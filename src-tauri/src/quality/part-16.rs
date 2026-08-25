const QR_COMMAND_RUN_VISIBLE_LIMIT: usize = 25;

fn import_qr_command_runs(repository: &RepositorySnapshot) -> QrCommandRunLedgerSnapshot {
    let runs_dir = Path::new(&repository.path).join(".quality-runner").join("runs");
    let Ok(entries) = fs::read_dir(runs_dir) else {
        return QrCommandRunLedgerSnapshot::default();
    };
    let mut runs = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().ok().is_some_and(|kind| kind.is_dir()) {
            continue;
        }
        let run_dir = entry.path();
        let manifest_path = run_dir.join("run-manifest.json");
        let manifest = read_json(&manifest_path);
        let directory_run_id = run_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_string();
        let run_id = manifest
            .as_ref()
            .and_then(|value| json_string_at(value, &["run_id"]))
            .unwrap_or(directory_run_id);
        let mode = manifest
            .as_ref()
            .and_then(|value| json_string_at(value, &["mode"]))
            .unwrap_or_else(|| "unknown".to_string());
        let observed_at = manifest
            .as_ref()
            .and_then(|value| json_string_at(value, &["created_at"]));
        let scanned_commit = manifest
            .as_ref()
            .and_then(|value| json_string_at(value, &["git", "head_sha"]));
        let scanned_branch = manifest
            .as_ref()
            .and_then(|value| json_string_at(value, &["git", "branch"]));
        let freshness = evaluate_freshness_at(
            observed_at.as_deref(),
            scanned_commit.as_deref(),
            scanned_branch.as_deref(),
            repository.workspace.last_commit.as_deref(),
            Some(repository.branch.as_str()),
            Utc::now(),
        );
        let artifact_count = manifest
            .as_ref()
            .and_then(|value| value.get("artifact_paths"))
            .and_then(Value::as_object)
            .map_or(0, |artifacts| artifacts.len() as u64);
        let valid = manifest.as_ref().is_some_and(|value| {
            json_string_at(value, &["schema"]).as_deref() == Some(QR_RUN_MANIFEST_SCHEMA)
                && canonical_path_matches(
                    json_string_at(value, &["repo_root"]).as_deref(),
                    &repository.path,
                )
                && json_string_at(value, &["run_id"]).is_some()
                && json_string_at(value, &["mode"]).is_some()
        });
        runs.push(QrCommandRunSnapshot {
            run_id,
            mode,
            status: if valid { "Imported" } else { "Invalid" }.to_string(),
            freshness,
            observed_at,
            scanned_commit,
            scanned_branch,
            artifact_count,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            detail: if valid {
                "Run manifest is available for repository-level roll-up.".to_string()
            } else {
                "Run manifest is missing, malformed, or belongs to another repository."
                    .to_string()
            },
        });
    }
    runs.sort_by(|left, right| {
        right
            .observed_at
            .cmp(&left.observed_at)
            .then_with(|| right.run_id.cmp(&left.run_id))
    });
    let total_count = runs.len() as u64;
    let invalid_count = runs.iter().filter(|run| run.status == "Invalid").count() as u64;
    let mut mode_counts = BTreeMap::new();
    for run in &runs {
        *mode_counts.entry(run.mode.clone()).or_insert(0) += 1;
    }
    runs.truncate(QR_COMMAND_RUN_VISIBLE_LIMIT);
    let visible_count = runs.len() as u64;
    QrCommandRunLedgerSnapshot {
        status: if invalid_count > 0 { "Warnings" } else { "Available" }.to_string(),
        implementation_allowed: false,
        total_count,
        visible_count,
        invalid_count,
        mode_counts,
        runs,
        detail: if invalid_count > 0 {
            "Every run is counted; one or more manifests need repair.".to_string()
        } else {
            "Every Quality Runner command run is counted by mode; the newest records are shown."
                .to_string()
        },
    }
}

fn project_qr_command_runs_for_target(
    snapshot: &mut QrCommandRunLedgerSnapshot,
    target_branch: &str,
    target_commit: &str,
) {
    for run in &mut snapshot.runs {
        run.freshness = evaluate_target_freshness(
            run.scanned_branch.as_deref(),
            run.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    }
}

fn import_test_portfolio(repository: &RepositorySnapshot) -> TestPortfolioSnapshot {
    let runs_dir = Path::new(&repository.path).join(".quality-runner").join("runs");
    let Ok(entries) = fs::read_dir(runs_dir) else {
        return TestPortfolioSnapshot::default();
    };
    let mut audits = Vec::new();
    let mut proofs = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().ok().is_some_and(|kind| kind.is_dir()) {
            continue;
        }
        let run_dir = entry.path();
        let Some(manifest) = read_json(&run_dir.join("run-manifest.json")) else {
            continue;
        };
        let observed_at = json_string_at(&manifest, &["created_at"]);
        if let Some(snapshot) = import_test_portfolio_run(
            repository,
            &run_dir,
            &manifest,
            observed_at,
            "test-portfolio-audit.json",
            "test-portfolio-audit",
            TEST_PORTFOLIO_AUDIT_SCHEMA,
        ) {
            audits.push(snapshot);
        }
        let observed_at = json_string_at(&manifest, &["created_at"]);
        if let Some(snapshot) = import_test_portfolio_run(
            repository,
            &run_dir,
            &manifest,
            observed_at,
            "test-removal-proof.json",
            "test-removal-proof",
            TEST_REMOVAL_PROOF_SCHEMA,
        ) {
            proofs.push(snapshot);
        }
    }
    audits.sort_by(test_portfolio_run_order);
    proofs.sort_by(test_portfolio_run_order);
    let mut snapshot = TestPortfolioSnapshot {
        audit: audits.pop(),
        removal_proof: proofs.pop(),
        ..TestPortfolioSnapshot::default()
    };
    update_test_portfolio_status(&mut snapshot);
    snapshot
}

#[allow(clippy::too_many_arguments)]
fn import_test_portfolio_run(
    repository: &RepositorySnapshot,
    run_dir: &Path,
    manifest: &Value,
    observed_at: Option<String>,
    filename: &str,
    expected_mode: &str,
    expected_schema: &str,
) -> Option<TestPortfolioRunSnapshot> {
    let report_path = run_dir.join(filename);
    if !report_path.is_file() || report_path.is_symlink() {
        return None;
    }
    let payload = read_json(&report_path).unwrap_or(Value::Null);
    let run_id = json_string_at(manifest, &["run_id"])
        .unwrap_or_else(|| {
            run_dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
    let scanned_commit = json_string_at(manifest, &["git", "head_sha"]);
    let scanned_branch = json_string_at(manifest, &["git", "branch"]);
    let freshness = evaluate_freshness_at(
        observed_at.as_deref(),
        scanned_commit.as_deref(),
        scanned_branch.as_deref(),
        repository.workspace.last_commit.as_deref(),
        Some(repository.branch.as_str()),
        Utc::now(),
    );
    let mut invalid_reasons = Vec::new();
    if json_string_at(manifest, &["schema"]).as_deref() != Some(QR_RUN_MANIFEST_SCHEMA) {
        invalid_reasons.push("run manifest schema is unsupported");
    }
    if json_string_at(manifest, &["mode"]).as_deref() != Some(expected_mode) {
        invalid_reasons.push("run manifest mode does not match the artifact");
    }
    if !canonical_path_matches(
        json_string_at(manifest, &["repo_root"]).as_deref(),
        &repository.path,
    ) {
        invalid_reasons.push("run manifest repository does not match this repository");
    }
    if json_string_at(&payload, &["schema"]).as_deref() != Some(expected_schema) {
        invalid_reasons.push("artifact schema is unsupported or malformed");
    }
    if json_string_at(&payload, &["revision", "head"]) != scanned_commit {
        invalid_reasons.push("artifact revision does not match the run manifest");
    }
    let artifact_status = json_string_at(&payload, &["status"])
        .unwrap_or_else(|| "unknown".to_string());
    let status = if invalid_reasons.is_empty() {
        artifact_status.clone()
    } else {
        "invalid".to_string()
    };
    let operation = if expected_schema == TEST_PORTFOLIO_AUDIT_SCHEMA {
        "portfolio-audit"
    } else {
        "removal-proof"
    };
    let detail = if invalid_reasons.is_empty() {
        if operation == "portfolio-audit" {
            "Reviewer judgments are advisory; candidates require a separate removal proof."
                .to_string()
        } else if artifact_status == "passed" {
            "Declared signals were preserved; this proof does not authorize source mutation."
                .to_string()
        } else {
            "Removal evidence is blocked and does not authorize source mutation.".to_string()
        }
    } else {
        invalid_reasons.join("; ")
    };
    Some(TestPortfolioRunSnapshot {
        schema: json_string_at(&payload, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        run_id,
        operation: operation.to_string(),
        status,
        freshness,
        observed_at,
        scanned_commit,
        scanned_branch,
        report_path: report_path.to_string_lossy().to_string(),
        artifact_hash: if operation == "portfolio-audit" {
            json_string_at(&payload, &["audit_hash"])
        } else {
            json_string_at(&payload, &["proof_hash"])
        },
        source_audit_hash: json_string_at(&payload, &["portfolio_audit_hash"]),
        removal_candidate_count: json_u64_at(&payload, &["summary", "removal_candidates"])
            .unwrap_or(0),
        insufficient_evidence_count: json_u64_at(
            &payload,
            &["summary", "dispositions", "insufficient_evidence"],
        )
        .unwrap_or(0),
        removed_test_count: json_u64_at(&payload, &["summary", "removed_test_count"])
            .unwrap_or(0),
        blocker_ids: payload
            .get("blockers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        detail,
    })
}

fn test_portfolio_run_order(
    left: &TestPortfolioRunSnapshot,
    right: &TestPortfolioRunSnapshot,
) -> std::cmp::Ordering {
    left.observed_at
        .cmp(&right.observed_at)
        .then_with(|| left.run_id.cmp(&right.run_id))
}

fn update_test_portfolio_status(snapshot: &mut TestPortfolioSnapshot) {
    let audit_invalid = snapshot.audit.as_ref().is_some_and(|run| run.status == "invalid");
    let proof_invalid = snapshot
        .removal_proof
        .as_ref()
        .is_some_and(|run| run.status == "invalid");
    let proof_matches_audit = snapshot.audit.as_ref().is_some_and(|audit| {
        snapshot
            .removal_proof
            .as_ref()
            .is_some_and(|proof| match (
                proof.source_audit_hash.as_deref(),
                audit.artifact_hash.as_deref(),
            ) {
                (Some(source_hash), Some(audit_hash)) => source_hash == audit_hash,
                _ => false,
            })
    });
    if audit_invalid || proof_invalid {
        snapshot.status = "Invalid".to_string();
        snapshot.detail = "The latest test-portfolio command evidence failed its import contract."
            .to_string();
    } else if snapshot
        .removal_proof
        .as_ref()
        .is_some_and(|proof| proof.status == "blocked")
        && proof_matches_audit
    {
        snapshot.status = "Blocked".to_string();
        snapshot.detail = "The latest matching removal proof is blocked.".to_string();
    } else if snapshot
        .removal_proof
        .as_ref()
        .is_some_and(|proof| proof.status == "passed")
        && proof_matches_audit
    {
        snapshot.status = "Proof passed".to_string();
        snapshot.detail =
            "The matching proof preserved declared signals but grants no mutation authority."
                .to_string();
    } else if snapshot.audit.as_ref().is_some_and(|audit| {
        audit.removal_candidate_count > 0 || audit.insufficient_evidence_count > 0
    }) {
        snapshot.status = "Review required".to_string();
        snapshot.detail = if snapshot.removal_proof.is_some() {
            "The latest proof belongs to a different audit; current candidates remain advisory."
                .to_string()
        } else {
            "The latest audit has candidate or insufficient-evidence dispositions to review."
                .to_string()
        };
    } else if snapshot.audit.is_some() {
        snapshot.status = "Audited".to_string();
        snapshot.detail = "The latest audit has no removal candidate requiring proof.".to_string();
    }
}

fn project_test_portfolio_for_target(
    snapshot: &mut TestPortfolioSnapshot,
    target_branch: &str,
    target_commit: &str,
) {
    for run in [&mut snapshot.audit, &mut snapshot.removal_proof]
        .into_iter()
        .flatten()
    {
        run.freshness = evaluate_target_freshness(
            run.scanned_branch.as_deref(),
            run.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    }
    update_test_portfolio_status(snapshot);
}
