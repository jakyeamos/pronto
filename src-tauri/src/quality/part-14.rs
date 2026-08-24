struct QrRun {
    run_dir: PathBuf,
    manifest: Value,
    verification: Value,
    execution_plan: Value,
    capability_matrix: Value,
    repo_scan: Value,
    observed_at: Option<String>,
}

fn repository_provenance_for_branch<'a>(
    repository: &'a RepositorySnapshot,
    scanned_branch: Option<&str>,
) -> (Option<&'a str>, Option<&'a str>) {
    if let Some(scanned_branch) = scanned_branch {
        if let Some(branch) = repository
            .branches
            .iter()
            .find(|branch| branch.name == scanned_branch)
        {
            return (branch.last_commit.as_deref(), Some(branch.name.as_str()));
        }
    }
    (
        repository.workspace.last_commit.as_deref(),
        Some(repository.branch.as_str()),
    )
}

impl QrRun {
    fn finding_reports(&self) -> Vec<(PathBuf, Value)> {
        let report_names = [
            "code-quality-scan.json",
            "quality-audit.json",
            "completed-report.json",
            "repo-scan.json",
            "run-summary.json",
        ];
        let mut run_dirs = vec![self.run_dir.clone()];

        let publication = read_json(&self.run_dir.join("fleet-detector-publication.json"));
        let is_fleet_detector_run = publication.as_ref().is_some_and(|payload| {
            json_string_at(payload, &["schema"]).as_deref()
                == Some("quality-runner-fleet-detector-publication/v1")
        });
        if is_fleet_detector_run {
            let run_name = self.run_dir.file_name().and_then(|name| name.to_str());
            let group_name = run_name.and_then(|name| {
                ["-inspect", "-run", "-verify"]
                    .iter()
                    .find_map(|suffix| name.strip_suffix(suffix))
            });
            if let (Some(parent), Some(group_name)) = (self.run_dir.parent(), group_name) {
                let mut siblings = fs::read_dir(parent)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path != &self.run_dir)
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| {
                                ["-inspect", "-run", "-verify"]
                                    .iter()
                                    .any(|suffix| name == format!("{group_name}{suffix}"))
                            })
                    })
                    .filter(|path| {
                        read_json(&path.join("fleet-detector-publication.json")).is_some_and(
                            |payload| {
                                json_string_at(&payload, &["schema"]).as_deref()
                                    == Some("quality-runner-fleet-detector-publication/v1")
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                siblings.sort();
                run_dirs.extend(siblings);
            }
        }

        let mut seen = HashSet::new();
        report_names
            .iter()
            .flat_map(|name| {
                run_dirs.iter().filter_map(move |run_dir| {
                    let path = run_dir.join(name);
                    read_json(&path).map(|payload| (path, payload))
                })
            })
            .filter(|(path, _)| seen.insert(path.clone()))
            .collect()
    }

    // Compatibility accessor retained for consumers that expect one selected report;
    // finding_reports is the authoritative fleet-aware aggregation path.
    #[allow(dead_code)]
    fn finding_report(&self) -> Option<(PathBuf, Value)> {
        self.finding_reports().into_iter().next()
    }

    fn configured_gate_ids(&self) -> Vec<String> {
        let mut configured_gate_ids = Vec::new();
        let mut seen_gate_ids = HashSet::new();
        let mut append_entries = |entries: Option<&Vec<Value>>| {
            if let Some(entries) = entries {
                for entry in entries {
                    let Some(raw_id) =
                        json_string_at(entry, &["id"]).or_else(|| json_string_at(entry, &["name"]))
                    else {
                        continue;
                    };
                    let id = normalize_gate_id(&raw_id);
                    if seen_gate_ids.insert(id.clone()) {
                        configured_gate_ids.push(id);
                    }
                }
            }
        };
        append_entries(
            self.capability_matrix
                .get("available")
                .and_then(Value::as_array),
        );
        append_entries(
            self.repo_scan
                .get("quality_commands")
                .and_then(Value::as_array),
        );
        append_entries(self.verification.get("gates").and_then(Value::as_array));
        append_entries(
            self.verification
                .get("execution_plan")
                .and_then(Value::as_array),
        );
        append_entries(self.execution_plan.as_array());
        configured_gate_ids
    }

    fn gate_evidence(&self, repository: &RepositorySnapshot) -> Vec<QualityEvidence> {
        let branch = self.branch();
        let commit = self.commit();
        let (current_commit, current_branch) =
            repository_provenance_for_branch(repository, branch.as_deref());
        let report_path = artifact_path(&self.run_dir, "gate-verification.json")
            .or_else(|| artifact_path(&self.run_dir, "gate-execution-plan.json"));
        let observed_at = self.observed_at.clone();
        let mut entries = self
            .verification
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if entries.is_empty() {
            entries = self
                .verification
                .get("execution_plan")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        if entries.is_empty() {
            entries = self.execution_plan.as_array().cloned().unwrap_or_default();
        }
        entries
            .into_iter()
            .filter_map(|gate| {
                let raw_id =
                    json_string_at(&gate, &["id"]).or_else(|| json_string_at(&gate, &["name"]))?;
                let id = normalize_gate_id(&raw_id);
                let capability_kind = json_string_at(&gate, &["capability_kind"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let source = QualitySource::parse(
                    json_string_at(&gate, &["source"])
                        .as_deref()
                        .unwrap_or_default(),
                )
                .or_else(|| (capability_kind == "ci_only").then_some(QualitySource::Ci))
                .unwrap_or_else(|| {
                    if capability_kind == "local_command"
                        || capability_kind == "command"
                        || json_string_at(&gate, &["command"]).is_some()
                    {
                        QualitySource::Local
                    } else {
                        QualitySource::Qr
                    }
                });
                let mut status = parse_qr_status(json_string_at(&gate, &["status"]).as_deref());
                if matches!(
                    capability_kind.as_str(),
                    "evidence" | "evidence_file" | "agent_review" | "ci_only"
                ) {
                    status = QualityGateStatus::Blocked;
                }
                let failure_type = json_string_at(&gate, &["failure_type"]);
                let skip_type = json_string_at(&gate, &["skip_type"]);
                if skip_type.is_some() {
                    status = QualityGateStatus::Blocked;
                }
                let gate_observed_at = json_string_at(&gate, &["completed_at"])
                    .or_else(|| json_string_at(&gate, &["observed_at"]))
                    .or_else(|| observed_at.clone());
                let freshness = evaluate_freshness_at(
                    gate_observed_at.as_deref(),
                    commit.as_deref(),
                    branch.as_deref(),
                    current_commit,
                    current_branch,
                    Utc::now(),
                );
                let command = json_string_at(&gate, &["command"]);
                let source_name = json_string_at(&gate, &["source"])
                    .or_else(|| command.clone())
                    .unwrap_or_else(|| "QR gate-verification".to_string());
                let detail = failure_type.or(skip_type).unwrap_or_else(|| {
                    json_string_at(&gate, &["status"]).unwrap_or_else(|| "No result".to_string())
                });
                Some(QualityEvidence {
                    id,
                    source,
                    status,
                    freshness,
                    observed_at: gate_observed_at,
                    scanned_commit: commit.clone(),
                    scanned_branch: branch.clone(),
                    command,
                    source_label: format!(
                        "{} · {}",
                        gate_label(&normalize_gate_id(&raw_id)),
                        source_name
                    ),
                    report_path: report_path.clone(),
                    report_url: None,
                    report_kind: Some("QR gate verification".to_string()),
                    detail,
                    verification_level: QualityVerificationLevel::SourceInferred,
                    target_kind: Some("source".to_string()),
                    target_url: None,
                    target_provider: None,
                    deployment_id: None,
                })
            })
            .collect()
    }

    fn branch(&self) -> Option<String> {
        json_string_at(&self.manifest, &["git", "branch"])
            .or_else(|| json_string_at(&self.manifest, &["git_provenance", "branch"]))
            .or_else(|| json_string_at(&self.manifest, &["provenance", "branch"]))
            .or_else(|| json_string_at(&self.manifest, &["branch"]))
            .or_else(|| json_string_at(&self.verification, &["provenance", "branch"]))
    }

    fn commit(&self) -> Option<String> {
        json_string_at(&self.manifest, &["git", "head_sha"])
            .or_else(|| json_string_at(&self.manifest, &["git_provenance", "head_sha"]))
            .or_else(|| json_string_at(&self.manifest, &["provenance", "head_sha"]))
            .or_else(|| json_string_at(&self.manifest, &["head_sha"]))
            .or_else(|| json_string_at(&self.verification, &["provenance", "head_sha"]))
    }

    fn findings(&self, repository: &RepositorySnapshot) -> QualityFindings {
        let reports = self.finding_reports();
        let Some((report_path, payload)) = reports.first() else {
            let detector_path = self.run_dir.join("anti-slop-detector.json");
            if let Some(payload) = read_json(&detector_path) {
                let branch = self.branch();
                let commit = self.commit();
                let mut findings = QualityFindings {
                    source: Some(QualitySource::Qr),
                    observed_at: self.observed_at.clone(),
                    scanned_commit: commit.clone(),
                    scanned_branch: branch.clone(),
                    target_sha: json_string_at(&payload, &["target_sha"]),
                    freshness: evaluate_freshness_at(
                        self.observed_at.as_deref(),
                        commit.as_deref(),
                        branch.as_deref(),
                        repository.workspace.last_commit.as_deref(),
                        Some(repository.branch.as_str()),
                        Utc::now(),
                    ),
                    report_path: Some(detector_path.to_string_lossy().to_string()),
                    ..QualityFindings::default()
                };
                apply_detector_evidence(&mut findings, &payload);
                return findings;
            }
            return QualityFindings::default();
        };
        let mut merged_findings = Vec::new();
        let mut prior_report_fingerprints = HashSet::new();
        for (_, report) in &reports {
            let mut current_report_fingerprints = HashSet::new();
            for finding in report
                .get("findings")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let fingerprint = finding
                    .get("fingerprint")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                if fingerprint.is_some_and(|value| prior_report_fingerprints.contains(value)) {
                    continue;
                }
                if let Some(fingerprint) = fingerprint {
                    current_report_fingerprints.insert(fingerprint.to_string());
                }
                merged_findings.push(finding.clone());
            }
            prior_report_fingerprints.extend(current_report_fingerprints);
        }
        let (total, severity_counts) = if merged_findings.is_empty() {
            let severity_counts = severity_counts(payload);
            let total = json_u64_at(payload, &["finding_count"])
                .or_else(|| json_u64_at(payload, &["summary", "finding_count"]))
                .or_else(|| json_u64_at(payload, &["finding_counts", "total"]))
                .or_else(|| json_u64_at(payload, &["summary", "finding_counts", "total"]))
                .unwrap_or_else(|| severity_counts.values().sum());
            (total, severity_counts)
        } else {
            (
                merged_findings.len() as u64,
                fleet_severity_counts(&merged_findings),
            )
        };
        let high_severity_total = severity_counts
            .iter()
            .filter(|(severity, _)| matches!(severity.as_str(), "critical" | "high"))
            .map(|(_, count)| *count)
            .sum();
        let branch = self.branch();
        let commit = self.commit();
        let (current_commit, current_branch) =
            repository_provenance_for_branch(repository, branch.as_deref());
        let mut findings = QualityFindings {
            total,
            severity_counts,
            high_severity_total,
            source: Some(QualitySource::Qr),
            observed_at: self.observed_at.clone(),
            scanned_commit: commit.clone(),
            scanned_branch: branch.clone(),
            freshness: evaluate_freshness_at(
                self.observed_at.as_deref(),
                commit.as_deref(),
                branch.as_deref(),
                current_commit,
                current_branch,
                Utc::now(),
            ),
            report_path: Some(report_path.to_string_lossy().to_string()),
            report_paths: reports
                .iter()
                .map(|(path, _)| path.to_string_lossy().to_string())
                .collect(),
            ..QualityFindings::default()
        };
        apply_detector_evidence(&mut findings, &payload);
        findings
    }
}

fn latest_qr_run(repository_path: &Path) -> Option<QrRun> {
    let runs = repository_path.join(".quality-runner").join("runs");
    let entries = fs::read_dir(runs).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let run_dir = entry.path();
            let manifest = read_json(&run_dir.join("run-manifest.json"))?;
            let verification =
                read_json(&run_dir.join("gate-verification.json")).unwrap_or(Value::Null);
            let execution_plan =
                read_json(&run_dir.join("gate-execution-plan.json")).unwrap_or(Value::Null);
            let capability_matrix =
                read_json(&run_dir.join("capability-matrix.json")).unwrap_or(Value::Null);
            let repo_scan = read_json(&run_dir.join("repo-scan.json")).unwrap_or(Value::Null);
            let observed_at = json_string_at(&manifest, &["created_at"])
                .or_else(|| json_string_at(&manifest, &["started_at"]))
                .or_else(|| json_string_at(&manifest, &["completed_at"]))
                .or_else(|| json_string_at(&manifest, &["finished_at"]))
                .or_else(|| json_string_at(&manifest, &["generated_at"]))
                .or_else(|| json_string_at(&manifest, &["as_of"]))
                .or_else(|| json_string_at(&verification, &["provenance", "captured_at"]));
            Some(QrRun {
                run_dir,
                manifest,
                verification,
                execution_plan,
                capability_matrix,
                repo_scan,
                observed_at,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.run_dir.cmp(&right.run_dir))
    });
    candidates.pop()
}

#[derive(Debug, Clone)]
struct AuditFinding {
    path: PathBuf,
    canonical_path: Option<String>,
    remote_key: Option<String>,
    mean_maturity: Option<f64>,
    mean_maturity_display: Option<String>,
    scored_dimension_count: Option<u64>,
    dimension_scores: BTreeMap<String, f64>,
}

#[derive(Debug)]
struct AuditRun {
    audit_id: Option<String>,
    as_of: Option<String>,
    summary_path: PathBuf,
    mean_maturity: Option<f64>,
    mean_maturity_display: Option<String>,
    scored_dimension_count: Option<u64>,
    findings: Vec<AuditFinding>,
}

fn latest_audit_run(root: &Path) -> Option<AuditRun> {
    let mut directories = Vec::new();
    if root.join("summary.json").is_file() {
        directories.push(root.to_path_buf());
    } else {
        directories = fs::read_dir(root)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect();
    }
    let mut runs = directories
        .into_iter()
        .filter_map(|directory| parse_audit_run(&directory))
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| {
        left.as_of
            .cmp(&right.as_of)
            .then_with(|| left.summary_path.cmp(&right.summary_path))
    });
    runs.pop()
}
