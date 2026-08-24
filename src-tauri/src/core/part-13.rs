fn refresh_quality_detectors_at(
    path: &Path,
    qr_bin: Option<&str>,
    timeout_seconds: u64,
    agent_review_mode: &str,
) -> Result<QualityDetectorRefreshReport, String> {
    if !matches!(agent_review_mode, "off" | "auto" | "parallel" | "required") {
        return Err("--agent-review-mode must be off, auto, parallel, or required".to_string());
    }
    let mut state = load_store(path)?;
    audited_scan_and_persist(path, &mut state)?;
    let repositories = state
        .repositories
        .iter()
        .filter(|repository| Path::new(&repository.path).join(".git").exists())
        .collect::<Vec<_>>();
    if repositories.is_empty() {
        return Err(
            "No registered Git repositories are available for detector refresh.".to_string(),
        );
    }
    let repository_paths = repositories
        .iter()
        .map(|repository| repository.path.clone())
        .collect::<Vec<_>>();
    let projects_root = qr_projects_root(&repository_paths)?;
    let qr = resolve_qr_executable(qr_bin);
    let mut arguments = vec![
        "fleet".to_string(),
        "detector".to_string(),
        "refresh".to_string(),
        "--projects-root".to_string(),
        projects_root.to_string_lossy().to_string(),
        "--timeout-seconds".to_string(),
        timeout_seconds.to_string(),
        "--agent-review-mode".to_string(),
        agent_review_mode.to_string(),
        "--json".to_string(),
    ];
    for repository in repositories {
        arguments.extend(["--repo-path".to_string(), repository.path.clone()]);
        if repository.target_branch_configured {
            if let Some(target_branch) = repository.target_branch.as_ref() {
                arguments.extend([
                    "--target-path-override".to_string(),
                    repository.path.clone(),
                    target_branch.clone(),
                ]);
            }
        }
    }
    let qr_payload = run_json_command(&qr, &arguments)?;
    if qr_payload.get("schema").and_then(serde_json::Value::as_str)
        != Some("quality-runner-fleet-detector-refresh/v1")
    {
        return Err(
            "Quality Runner detector refresh returned an unsupported evidence contract."
                .to_string(),
        );
    }
    let mut state = load_store(path)?;
    let snapshot = audited_scan_and_persist(path, &mut state)?;
    let mut reconciliation = reconcile_published_detector_results(&qr_payload, &snapshot);
    let published_repositories = qr_payload
        .get("counts")
        .and_then(|counts| counts.get("published"))
        .and_then(serde_json::Value::as_u64)
        .map(|count| count as usize)
        .unwrap_or_else(|| reconciliation.len());
    if published_repositories != reconciliation.len() {
        reconciliation.push(QualityDetectorReconciliation {
            repository_path: None,
            target_branch: None,
            target_head: None,
            expected_findings: None,
            imported_findings: None,
            report_path: None,
            status: "rejected".to_string(),
            reason: format!(
                "QR declared {published_repositories} published repositories but returned {} published result rows.",
                reconciliation.len()
            ),
        });
    }
    let rejected_published_repositories = reconciliation
        .iter()
        .filter(|item| item.status == "rejected")
        .count();
    let ingested_published_repositories = reconciliation
        .iter()
        .filter(|item| item.status == "ingested")
        .count();
    let findings_evidence_repositories = snapshot
        .repositories
        .iter()
        .filter(|repository| quality_metric_is_available(repository))
        .count();
    let unsupported_paths = detector_unsupported_paths(&qr_payload);
    let detector_applicable_repositories = snapshot
        .repositories
        .iter()
        .filter(|repository| {
            let path = fs::canonicalize(&repository.path)
                .unwrap_or_else(|_| PathBuf::from(&repository.path));
            !unsupported_paths.contains(&path)
        })
        .count();
    let detector_excluded_repositories = snapshot
        .repositories
        .len()
        .saturating_sub(detector_applicable_repositories);
    let applicable_findings_evidence_repositories = snapshot
        .repositories
        .iter()
        .filter(|repository| {
            let path = fs::canonicalize(&repository.path)
                .unwrap_or_else(|_| PathBuf::from(&repository.path));
            !unsupported_paths.contains(&path) && quality_metric_is_available(repository)
        })
        .count();
    let missing_findings_evidence_repositories =
        detector_applicable_repositories.saturating_sub(applicable_findings_evidence_repositories);
    let qr_status = qr_payload
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("partial");
    Ok(QualityDetectorRefreshReport {
        schema_version: "pronto-quality-detector-refresh/v1".to_string(),
        generated_at: iso_now(),
        status: if qr_status == "completed" && rejected_published_repositories == 0 {
            "Completed".to_string()
        } else {
            "Partial".to_string()
        },
        qr: qr_payload,
        provenance_refreshes: 2,
        published_repositories,
        ingested_published_repositories,
        rejected_published_repositories,
        reconciliation,
        tracked_repositories: snapshot.repositories.len(),
        detector_applicable_repositories,
        detector_excluded_repositories,
        findings_evidence_repositories,
        applicable_findings_evidence_repositories,
        missing_findings_evidence_repositories,
        snapshot,
    })
}

fn maturity_coverage_gaps(repositories: &[RepositorySnapshot]) -> Vec<String> {
    repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .filter(|repository| remediation::repository_requires_maturity(repository))
        .filter_map(|repository| {
            let maturity = &repository.quality.maturity;
            if maturity.score.is_none() {
                Some(format!("{} (missing)", repository.name))
            } else if maturity.freshness != QualityFreshness::Fresh {
                Some(format!(
                    "{} ({})",
                    repository.name,
                    maturity.freshness.as_str().to_ascii_lowercase()
                ))
            } else if remediation::repository_requires_maturity_gate(
                repository,
                mac_control_maturity::MAC_CONTROL_GATE_ID,
            ) && !(repository.quality.mac_control_ideal_state.ideal_state
                || (repository.quality.mac_control_ideal_state.status == "Not applicable"
                    && repository.quality.mac_control_ideal_state.freshness == "Fresh"))
            {
                Some(format!(
                    "{} (Mac Control ideal state: {} / {})",
                    repository.name,
                    repository.quality.mac_control_ideal_state.status,
                    repository.quality.mac_control_ideal_state.freshness,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn refresh_github_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    let adapter = GitHubCliAdapter::default();
    match adapter.refresh() {
        Ok(refresh) => apply_provider_refresh_at(path, refresh, None),
        Err(error) => {
            let mut state = load_store(path)?;
            state.provider_status = ProviderStatus {
                provider: "GitHub".to_string(),
                state: "Unavailable".to_string(),
                message: error.clone(),
                last_refresh_at: state.provider_status.last_refresh_at.clone(),
                identity_count: state.provider_identities.len(),
                repository_count: state.remote_repositories.len(),
            };
            apply_release_threshold_conditions(&mut state);
            save_store(path, &state)?;
            Ok(snapshot_from_store(path, &state))
        }
    }
}

fn refresh_github_scoped_at(
    path: &Path,
    target_repository_ids: &HashSet<String>,
) -> Result<PortfolioSnapshot, String> {
    let target_repository_names = load_store(path)?
        .repositories
        .iter()
        .filter(|repository| target_repository_ids.contains(&repository.id))
        .filter_map(|repository| {
            repository
                .remote_url
                .as_deref()
                .and_then(normalize_remote_name)
        })
        .collect::<HashSet<_>>();
    let adapter = GitHubCliAdapter::for_repository_names(target_repository_names);
    match adapter.refresh() {
        Ok(refresh) => apply_provider_refresh_at(path, refresh, Some(target_repository_ids)),
        Err(error) => {
            let mut state = load_store(path)?;
            state.provider_status = ProviderStatus {
                provider: "GitHub".to_string(),
                state: "Unavailable".to_string(),
                message: error.clone(),
                last_refresh_at: state.provider_status.last_refresh_at.clone(),
                identity_count: state.provider_identities.len(),
                repository_count: state.remote_repositories.len(),
            };
            apply_release_threshold_conditions(&mut state);
            save_store(path, &state)?;
            Err(error)
        }
    }
}

fn remediation_refresh_steps() -> Vec<remediation::RemediationRefreshStep> {
    [
        ("qr_doctor", "Quality Runner doctor"),
        ("local_scan", "Scoped local repository scan"),
        ("qr_fleet_run", "Fresh Quality Runner fleet run"),
        ("qr_replay", "Quality Runner replay verification"),
        ("qr_report", "Quality Runner aggregate report"),
        ("qr_feed", "Quality Runner maturity feed"),
        ("provider", "GitHub provider refresh"),
        ("quality_import", "Pronto quality and maturity import"),
        ("remediation_plan", "Ranked active remediation queue"),
    ]
    .into_iter()
    .map(|(id, label)| remediation::RemediationRefreshStep {
        id: id.to_string(),
        label: label.to_string(),
        status: "pending".to_string(),
        ..remediation::RemediationRefreshStep::default()
    })
    .collect()
}

fn set_remediation_refresh_step(
    steps: &mut [remediation::RemediationRefreshStep],
    step_id: &str,
    status: &str,
    detail: impl Into<String>,
    evidence_path: Option<String>,
) {
    let Some(step) = steps.iter_mut().find(|step| step.id == step_id) else {
        return;
    };
    let now = iso_now();
    if step.started_at.is_none() {
        step.started_at = Some(now.clone());
    }
    step.status = status.to_string();
    step.detail = detail.into();
    step.evidence_path = evidence_path;
    if matches!(status, "completed" | "blocked" | "skipped") {
        step.completed_at = Some(now);
    }
}

fn persist_remediation_refresh(
    path: &Path,
    refresh_id: &str,
    status: &str,
    message: Option<String>,
    steps: &[remediation::RemediationRefreshStep],
) -> Result<(), String> {
    let mut state = load_store(path)?;
    remediation::sync_scope_metadata(&mut state.remediation, &state.repositories);
    if state.remediation.id.is_empty() {
        state.remediation = remediation::rebuild_run(
            &state.repositories,
            &state.remediation,
            state.quality.latest_audit_id.as_deref(),
        );
    }
    let eligible = state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .collect::<Vec<_>>();
    remediation::set_refresh_metadata(
        &mut state.remediation,
        refresh_id,
        status,
        message,
        eligible
            .iter()
            .map(|repository| repository.id.clone())
            .collect(),
        eligible
            .iter()
            .map(|repository| repository.path.clone())
            .collect(),
        steps.to_vec(),
    );
    save_store(path, &state)
}

fn fail_remediation_refresh(
    path: &Path,
    refresh_id: &str,
    steps: &mut [remediation::RemediationRefreshStep],
    step_id: &str,
    error: String,
) -> Result<PortfolioSnapshot, String> {
    set_remediation_refresh_step(steps, step_id, "blocked", error.clone(), None);
    let _ = persist_remediation_refresh(path, refresh_id, "blocked", Some(error.clone()), steps);
    Err(error)
}

fn json_from_process_output(output: &std::process::Output) -> Result<serde_json::Value, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&stdout) {
        return Ok(payload);
    }
    let payload = stdout
        .find('{')
        .and_then(|start| stdout.rfind('}').map(|end| (start, end)))
        .and_then(|(start, end)| serde_json::from_str(&stdout[start..=end]).ok());
    payload.ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            "Quality Runner returned invalid or missing JSON on stdout.".to_string()
        } else {
            format!("Quality Runner returned invalid or missing JSON on stdout ({stderr})")
        }
    })
}

fn run_json_command(executable: &str, arguments: &[String]) -> Result<serde_json::Value, String> {
    run_json_command_in(executable, arguments, None)
}

fn run_json_command_in_with_status(
    executable: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> Result<(serde_json::Value, bool, Option<String>), String> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    let output = command.output().map_err(|error| {
        current_dir.map_or_else(
            || format!("Could not run {executable}: {error}"),
            |path| format!("Could not run {executable} in {}: {error}", path.display()),
        )
    })?;
    let success = output.status.success();
    let detail = {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!stderr.is_empty())
            .then_some(stderr)
            .or_else(|| (!stdout.is_empty()).then_some(stdout))
    };
    let payload = json_from_process_output(&output).map_err(|error| {
        if success {
            error
        } else {
            format!(
                "{executable} {} failed with status {}{}",
                arguments.join(" "),
                output.status,
                detail
                    .as_deref()
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default()
            )
        }
    })?;
    Ok((payload, success, detail))
}

fn run_json_command_in(
    executable: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let (payload, success, detail) =
        run_json_command_in_with_status(executable, arguments, current_dir)?;
    if !success {
        return Err(format!(
            "{executable} {} failed with status {}{}",
            arguments.join(" "),
            "non-zero",
            if let Some(detail) = detail {
                format!(": {detail}")
            } else {
                String::new()
            }
        ));
    }
    Ok(payload)
}

fn target_evidence_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "target".to_string()
    } else {
        slug.chars().take(80).collect()
    }
}

fn target_evidence_run_prefix(repository: &RepositorySnapshot, target_branch: &str) -> String {
    let sequence = NEXT_TARGET_EVIDENCE_ID.fetch_add(1, Ordering::Relaxed);
    format!(
        "pronto-target-{}-{}-{}-{}",
        target_evidence_slug(&repository.id),
        target_evidence_slug(target_branch),
        target_evidence_slug(&iso_now()),
        sequence,
    )
}

fn target_evidence_artifact_parent() -> PathBuf {
    std::env::temp_dir().join("pronto-target-evidence")
}
