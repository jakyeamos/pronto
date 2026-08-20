fn refresh_remediation_at(
    path: &Path,
    target_query: Option<&str>,
    qr_executable: Option<&str>,
    dynamic: bool,
    changed_only: bool,
    skip_provider: bool,
    timeout_seconds: u64,
) -> Result<PortfolioSnapshot, String> {
    let initial_state = load_store(path)?;
    let initial_snapshot = snapshot_from_store(path, &initial_state);
    let scope = remediation_refresh_scope(&initial_snapshot, target_query)?;
    let eligible_paths = scope.repository_paths.clone();
    let eligible_ids = scope.repository_ids.clone();
    let repository_scoped = scope.is_repository_scoped();
    let scope_label = scope
        .target_name
        .as_deref()
        .map(|name| format!("repository '{name}'"))
        .unwrap_or_else(|| "eligible repositories".to_string());
    let refresh_id = format!("remediation-refresh-{}", iso_now().replace([':', '-'], ""));
    let mut steps = remediation_refresh_steps();
    let _ = persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps);

    let qr = resolve_qr_executable(qr_executable);
    set_remediation_refresh_step(
        &mut steps,
        "qr_doctor",
        "in_progress",
        format!("Running {qr} doctor before any QR audit."),
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let doctor = match run_json_command(&qr, &["doctor".to_string(), "--json".to_string()]) {
        Ok(payload) => payload,
        Err(error) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, "qr_doctor", error)
        }
    };
    let doctor_status = json_string(&doctor, &["status"]).unwrap_or_else(|| "unknown".to_string());
    if doctor_status != "ready" {
        return fail_remediation_refresh(
            path,
            &refresh_id,
            &mut steps,
            "qr_doctor",
            format!("Quality Runner doctor did not report ready (status: {doctor_status})."),
        );
    }
    set_remediation_refresh_step(
        &mut steps,
        "qr_doctor",
        "completed",
        "Quality Runner doctor reported ready.",
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    set_remediation_refresh_step(
        &mut steps,
        "local_scan",
        "in_progress",
        format!("Refreshing local Git/workspace evidence for {scope_label} only."),
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let mut state = load_store(path)?;
    if let Err(error) =
        audited_scan_and_persist_scoped(path, &mut state, Some(&eligible_ids), Some(&scope_label))
    {
        return fail_remediation_refresh(path, &refresh_id, &mut steps, "local_scan", error);
    }
    if eligible_paths.is_empty() {
        return fail_remediation_refresh(
            path,
            &refresh_id,
            &mut steps,
            "local_scan",
            "No eligible Git repositories remain for remediation refresh.".to_string(),
        );
    }
    set_remediation_refresh_step(
        &mut steps,
        "local_scan",
        "completed",
        "Local evidence refreshed for eligible repositories.",
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let projects_root = qr_projects_root(&eligible_paths)?;
    let all_projects_scope = !repository_scoped
        && dirs::home_dir()
        .map(|home| projects_root == home.join("projects"))
        .unwrap_or(false);
    let fleet_arguments = qr_fleet_run_arguments(
        &eligible_paths,
        &projects_root,
        all_projects_scope,
        dynamic,
        changed_only,
        timeout_seconds,
    );
    set_remediation_refresh_step(
        &mut steps,
        "qr_fleet_run",
        "in_progress",
        format!(
            "Running a fresh QR fleet audit from {}.",
            projects_root.display()
        ),
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let fleet = match run_json_command(&qr, &fleet_arguments) {
        Ok(payload) => payload,
        Err(error) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, "qr_fleet_run", error)
        }
    };
    let audit_id = match json_string(&fleet, &["audit_id"]) {
        Some(audit_id) => audit_id,
        None => {
            return fail_remediation_refresh(
                path,
                &refresh_id,
                &mut steps,
                "qr_fleet_run",
                "Quality Runner fleet run completed without an audit_id.".to_string(),
            )
        }
    };
    let artifact_root = json_string(&fleet, &["artifact_root"]);
    let scoped_artifact_root = artifact_root.clone();
    set_remediation_refresh_step(
        &mut steps,
        "qr_fleet_run",
        "completed",
        format!("Fresh QR audit {audit_id} completed."),
        artifact_root.clone(),
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let mut feed_published = match process_qr_audit_lifecycle(
        path,
        &refresh_id,
        &mut steps,
        &qr,
        &audit_id,
        artifact_root.clone(),
        !repository_scoped,
    ) {
        Ok(published) => published,
        Err((step_id, error)) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, &step_id, error)
        }
    };

    if !repository_scoped && !feed_published && !all_projects_scope {
        if let Some(canonical_root) = dirs::home_dir()
            .map(|home| home.join("projects"))
            .filter(|root| root.is_dir())
        {
            let canonical_arguments = {
                let mut arguments = vec![
                    "fleet".to_string(),
                    "audit".to_string(),
                    "run".to_string(),
                    "--all".to_string(),
                    "--projects-root".to_string(),
                    canonical_root.to_string_lossy().to_string(),
                ];
                append_qr_audit_runtime_arguments(
                    &mut arguments,
                    dynamic,
                    changed_only,
                    timeout_seconds,
                );
                arguments
            };
            set_remediation_refresh_step(
                &mut steps,
                "qr_fleet_run",
                "in_progress",
                format!(
                    "Scoped QR feed was not publishable; running the canonical all-projects QR audit from {}.",
                    canonical_root.display()
                ),
                None,
            );
            persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
            match run_json_command(&qr, &canonical_arguments) {
                Ok(canonical_fleet) => {
                    if let Some(canonical_audit_id) = json_string(&canonical_fleet, &["audit_id"]) {
                        let canonical_artifact_root =
                            json_string(&canonical_fleet, &["artifact_root"]);
                        set_remediation_refresh_step(
                            &mut steps,
                            "qr_fleet_run",
                            "completed",
                            format!(
                                "Scoped QR audit {audit_id} completed; canonical all-projects audit {canonical_audit_id} completed for maturity publication."
                            ),
                            scoped_artifact_root.clone(),
                        );
                        persist_remediation_refresh(
                            path,
                            &refresh_id,
                            "in_progress",
                            None,
                            &steps,
                        )?;
                        feed_published = match process_qr_audit_lifecycle(
                            path,
                            &refresh_id,
                            &mut steps,
                            &qr,
                            &canonical_audit_id,
                            canonical_artifact_root,
                            true,
                        ) {
                            Ok(published) => published,
                            Err((step_id, error)) => {
                                return fail_remediation_refresh(
                                    path,
                                    &refresh_id,
                                    &mut steps,
                                    &step_id,
                                    error,
                                )
                            }
                        };
                        if feed_published {
                            if let Some(step) = steps.iter_mut().find(|step| step.id == "qr_feed") {
                                step.detail = format!(
                                    "Canonical maturity feed published from all-projects audit {canonical_audit_id}; scoped audit {audit_id} remains the per-repository evidence run."
                                );
                            }
                        }
                    } else {
                        let error =
                            "Canonical Quality Runner fleet run completed without an audit_id."
                                .to_string();
                        set_remediation_refresh_step(
                            &mut steps,
                            "qr_fleet_run",
                            "completed",
                            format!(
                                "Scoped QR audit {audit_id} completed, but the canonical all-projects maturity audit was incomplete: {error}"
                            ),
                            artifact_root.clone(),
                        );
                        set_remediation_refresh_step(&mut steps, "qr_feed", "blocked", error, None);
                        persist_remediation_refresh(
                            path,
                            &refresh_id,
                            "partial",
                            Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                            &steps,
                        )?;
                    }
                }
                Err(error) => {
                    set_remediation_refresh_step(
                        &mut steps,
                        "qr_fleet_run",
                        "completed",
                        format!(
                            "Scoped QR audit {audit_id} completed, but the canonical all-projects maturity audit failed: {error}"
                        ),
                        artifact_root.clone(),
                    );
                    set_remediation_refresh_step(
                        &mut steps,
                        "qr_feed",
                        "blocked",
                        format!(
                            "Canonical all-projects maturity publication failed after scoped audit {audit_id}: {error}"
                        ),
                        None,
                    );
                    persist_remediation_refresh(
                        path,
                        &refresh_id,
                        "partial",
                        Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                        &steps,
                    )?;
                }
            }
        }
    }

    if skip_provider {
        set_remediation_refresh_step(
            &mut steps,
            "provider",
            "skipped",
            "Provider refresh was explicitly skipped; existing provider evidence was retained.",
            None,
        );
    } else {
        set_remediation_refresh_step(
            &mut steps,
            "provider",
            "in_progress",
            format!("Refreshing GitHub context for {scope_label}."),
            None,
        );
        persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
        match refresh_github_scoped_at(path, &eligible_ids) {
            Ok(_) => set_remediation_refresh_step(
                &mut steps,
                "provider",
                "completed",
                format!("GitHub provider context refreshed for {scope_label}."),
                None,
            ),
            Err(error) => {
                set_remediation_refresh_step(&mut steps, "provider", "blocked", error, None)
            }
        }
    }
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let mut final_state = load_store(path)?;
    let scoped_fleet_root = scoped_artifact_root.as_deref().map(Path::new);
    apply_quality_evidence_scoped(&mut final_state, Some(&eligible_ids), scoped_fleet_root);
    let scoped_repositories = final_state
        .repositories
        .iter()
        .filter(|repository| eligible_ids.contains(&repository.id))
        .cloned()
        .collect::<Vec<_>>();
    let maturity_repository_count = scoped_repositories
        .iter()
        .filter(|repository| remediation::repository_requires_maturity(repository))
        .count();
    let maturity_gaps = maturity_coverage_gaps(&scoped_repositories);
    let quality_source_ready = repository_scoped || feed_published;
    let quality_import_completed = quality_source_ready && maturity_gaps.is_empty();
    let quality_import_detail = if repository_scoped && maturity_gaps.is_empty() {
        format!(
            "Pronto imported replay-validated audit evidence for {scope_label}; all {maturity_repository_count} maturity-applicable repositories in scope have fresh scores, and CI ideal-state projections were refreshed."
        )
    } else if repository_scoped {
        format!(
            "The replay-validated audit for {scope_label} is incomplete because {} maturity-applicable repositories lack fresh scores: {}.",
            maturity_gaps.len(),
            maturity_gaps.join(", ")
        )
    } else if !feed_published {
        "The canonical QR maturity feed was not published; prior maturity evidence was retained."
            .to_string()
    } else if maturity_gaps.is_empty() {
        format!(
            "Pronto imported the canonical feed plus replay-validated scoped audit evidence; all {maturity_repository_count} maturity-applicable repositories have fresh scores, and CI ideal-state projections were refreshed."
        )
    } else {
        format!(
            "The canonical QR maturity feed was published, but the checkpoint is incomplete because {} maturity-applicable repositories lack fresh scores: {}.",
            maturity_gaps.len(),
            maturity_gaps.join(", ")
        )
    };
    set_remediation_refresh_step(
        &mut steps,
        "quality_import",
        if quality_import_completed {
            "completed"
        } else {
            "blocked"
        },
        quality_import_detail,
        scoped_artifact_root
            .clone()
            .or_else(|| final_state.quality.latest_audit_path.clone()),
    );
    set_remediation_refresh_step(
        &mut steps,
        "remediation_plan",
        "completed",
        "Ranked active repository plans and retained resolved-action history.",
        None,
    );
    let has_blockers = steps.iter().any(|step| step.status == "blocked");
    final_state.remediation = remediation::rebuild_run_with_fleet_root(
        &final_state.repositories,
        &final_state.remediation,
        final_state.quality.latest_audit_id.as_deref(),
        scoped_fleet_root,
    );
    remediation::set_refresh_metadata(
        &mut final_state.remediation,
        &refresh_id,
        if has_blockers { "partial" } else { "completed" },
        has_blockers.then(|| {
            steps
                .iter()
                .filter(|step| step.status == "blocked")
                .map(|step| format!("{}: {}", step.label, step.detail))
                .collect::<Vec<_>>()
                .join(" ")
        }),
        final_state
            .repositories
            .iter()
            .filter(|repository| eligible_ids.contains(&repository.id))
            .map(|repository| repository.id.clone())
            .collect(),
        final_state
            .repositories
            .iter()
            .filter(|repository| eligible_ids.contains(&repository.id))
            .map(|repository| repository.path.clone())
            .collect(),
        steps,
    );
    apply_release_threshold_conditions(&mut final_state);
    save_store(path, &final_state)?;
    Ok(snapshot_from_store(path, &final_state))
}

fn git_owned(path: &Path, arguments: Vec<String>) -> Option<String> {
    let result = run_git(path, arguments).ok()?;
    if result.success {
        Some(result.stdout.trim().to_string())
    } else {
        None
    }
}

fn path_id(prefix: &str, path: &Path) -> String {
    format!("{prefix}:{}", path.to_string_lossy())
}

fn sort_repositories_by_name(repositories: &mut [RepositorySnapshot]) {
    repositories.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
}
