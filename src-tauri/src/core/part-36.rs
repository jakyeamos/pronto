fn agent_route_report(
    snapshot: &PortfolioSnapshot,
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    query: Option<&str>,
    limit: usize,
) -> Result<AgentRouteReport, String> {
    let doctor = agent_doctor_report(snapshot, storage_path, max_age_minutes, scope);
    if !doctor.ready {
        return Ok(agent_route_from_doctor(
            doctor, scope, None, None, None, None,
        ));
    }

    let next = agent_next_report(snapshot, query, scope, limit)?;
    let repository = query
        .map(|value| {
            find_cli_repository(snapshot, value)
                .map(|repository| agent_repository_detail(snapshot, repository))
        })
        .transpose()?;
    let quality = Some(agent_quality_report_with_scope(snapshot, None, scope)?);
    let fold_preview = Some(agent_fold_preview_report_with_merge_preview(
        snapshot, query, None, scope, limit, false,
    )?);
    Ok(agent_route_from_doctor(
        doctor,
        scope,
        Some(next),
        repository,
        quality,
        fold_preview,
    ))
}

fn agent_route_error_report(
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    check_id: &str,
    error: String,
) -> AgentRouteReport {
    let doctor = agent_doctor_error_report(storage_path, max_age_minutes, scope, check_id, error);
    agent_route_from_doctor(doctor, scope, None, None, None, None)
}

fn agent_summary(snapshot: &PortfolioSnapshot, scope: &str) -> AgentSummary {
    let repositories = snapshot
        .repositories
        .iter()
        .map(agent_repository_summary)
        .collect::<Vec<_>>();
    let attention_count = agent_attention_report(snapshot).items.len();
    AgentSummary {
        schema_version: AGENT_SUMMARY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        repository_count: repositories.len(),
        active_condition_count: repositories
            .iter()
            .map(|repository| repository.active_conditions.len())
            .sum(),
        dirty_workspace_count: repositories
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .filter(|workspace| workspace.dirty)
            .count(),
        unsynced_workspace_count: repositories
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .filter(|workspace| workspace.sync_state != "Synced")
            .count(),
        attention_count,
        provider_status: snapshot.provider_status.clone(),
        quality: snapshot.quality.clone(),
        showcase: snapshot.showcase.clone(),
        repositories,
    }
}

fn agent_repository_detail(
    snapshot: &PortfolioSnapshot,
    repository: &RepositorySnapshot,
) -> AgentRepositoryDetail {
    AgentRepositoryDetail {
        schema_version: AGENT_REPOSITORY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        repository: repository.clone(),
        products: snapshot
            .products
            .iter()
            .filter(|product| product.repository_ids.iter().any(|id| id == &repository.id))
            .cloned()
            .collect(),
        groups: snapshot
            .groups
            .iter()
            .filter(|group| group.repository_ids.iter().any(|id| id == &repository.id))
            .cloned()
            .collect(),
        task_lanes: task_lanes::inspect(Path::new(&repository.path)),
    }
}

fn agent_quality_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
) -> Result<AgentQualityReport, String> {
    let scope = query
        .map(|value| format!("repository:{value}"))
        .unwrap_or_else(|| "fleet".to_string());
    agent_quality_report_with_scope(snapshot, query, &scope)
}

fn agent_quality_report_with_scope(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    scope: &str,
) -> Result<AgentQualityReport, String> {
    let repositories = if let Some(query) = query {
        vec![find_cli_repository(snapshot, query)?]
    } else {
        snapshot.repositories.iter().collect::<Vec<_>>()
    };
    Ok(AgentQualityReport {
        schema_version: AGENT_QUALITY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        portfolio: snapshot.quality.clone(),
        repositories: repositories
            .into_iter()
            .map(|repository| AgentRepositoryQuality {
                id: repository.id.clone(),
                name: repository.name.clone(),
                path: repository.path.clone(),
                branch: repository.branch.clone(),
                quality: repository.quality.clone(),
            })
            .collect(),
    })
}

fn agent_activity_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    limit: usize,
) -> Result<AgentActivityReport, String> {
    let repository_id = query
        .map(|value| find_cli_repository(snapshot, value).map(|repository| repository.id.clone()))
        .transpose()?;
    let events = snapshot
        .events
        .iter()
        .filter(|event| {
            repository_id
                .as_deref()
                .map_or(true, |id| event.repository_id == id)
        })
        .take(limit)
        .cloned()
        .collect();
    let action_audits = snapshot
        .action_audits
        .iter()
        .filter(|audit| {
            repository_id.as_deref().map_or(true, |id| {
                audit.target_ids.iter().any(|target| target == id)
            })
        })
        .take(limit)
        .cloned()
        .collect();
    Ok(AgentActivityReport {
        schema_version: AGENT_ACTIVITY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: query
            .map(|value| format!("repository:{value}"))
            .unwrap_or_else(|| "fleet".to_string()),
        events,
        action_audits,
    })
}

fn launch_desktop_focus(repository: Option<&RepositorySnapshot>) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err(
            "Desktop focus from the companion CLI is currently implemented for macOS bundles."
                .to_string(),
        );
    }
    let status = Command::new("open")
        .args(["-a", "Pronto"])
        .status()
        .map_err(|error| format!("Could not launch the Pronto desktop app: {error}"))?;
    if !status.success() {
        return Err("The installed Pronto desktop app could not be opened".to_string());
    }
    if let Some(repository) = repository {
        println!("Opened Pronto for {}.", repository.name);
    } else {
        println!("Opened Pronto.");
    }
    Ok(())
}

fn print_human_status(snapshot: &PortfolioSnapshot) {
    if snapshot.repositories.is_empty() {
        println!("Pronto · no repositories registered");
        println!("Add a discovery root in the desktop app, then refresh.");
        return;
    }
    println!(
        "PRONTO STATUS · {} repositories",
        snapshot.repositories.len()
    );
    for repository in &snapshot.repositories {
        let active_conditions = repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
            .map(|condition| condition.title.as_str())
            .collect::<Vec<_>>();
        let condition_text = if active_conditions.is_empty() {
            "No active conditions".to_string()
        } else {
            active_conditions.join(" · ")
        };
        println!(
            "{} · {} · {} · {}",
            repository.name, repository.locality, repository.branch, condition_text
        );
    }
}

fn print_human_refresh_batch(report: &RefreshBatchReport) {
    println!(
        "PRONTO REFRESH BATCH · {} · {} repositories · parallelism {} · {} conflict retries",
        report.status, report.repository_count, report.parallelism, report.conflict_retries
    );
    println!("Scope: {}", report.scope);
    println!("Scan: {}", report.scan_phase);
    println!("Merge: {}", report.merge_phase);
    for repository in &report.repositories {
        println!(
            "  #{} · {} · {} · {}",
            repository.scan_order + 1,
            repository.name,
            repository.status,
            repository.path
        );
    }
}

fn print_human_groups(groups: &[GroupConfig]) {
    if groups.is_empty() {
        println!("PRONTO GROUPS · no groups registered");
        return;
    }
    println!("PRONTO GROUPS · {} groups", groups.len());
    for group in groups {
        println!(
            "{} · {} repositories · {}",
            group.name,
            group.repository_ids.len(),
            group.id
        );
    }
}

fn print_human_products(products: &[ProductConfig]) {
    if products.is_empty() {
        println!("PRONTO PRODUCTS · no products registered");
        return;
    }
    println!("PRONTO PRODUCTS · {} products", products.len());
    for product in products {
        println!(
            "{} · {} repositories · {} · {}",
            product.name,
            product.repository_ids.len(),
            product.release_mode,
            product.id
        );
    }
}

fn print_human_summary(summary: &AgentSummary) {
    println!(
        "PRONTO SUMMARY · {} repositories · {} attention items",
        summary.repository_count, summary.attention_count
    );
    println!(
        "Conditions: {} active · workspaces: {} dirty, {} unsynced",
        summary.active_condition_count,
        summary.dirty_workspace_count,
        summary.unsynced_workspace_count
    );
    println!(
        "Quality: {} · maturity {}",
        summary.quality.audit_status,
        summary
            .quality
            .maturity_score_display
            .as_deref()
            .unwrap_or("unknown")
    );
}

fn print_human_next(report: &AgentNextReport) {
    println!(
        "PRONTO NEXT · {} · {} attention items",
        report.scope, report.attention_total
    );
    if let Some(repository) = &report.current_repository {
        println!(
            "Current repository: {} · {} · {}",
            repository.name, repository.branch, repository.path
        );
    }
    for action in &report.actions {
        println!(
            "  {} · {} · {} · {}",
            action.repository_name, action.category, action.severity, action.next_safe_step
        );
    }
}

fn print_human_fold_preview(report: &AgentFoldPreview) {
    println!(
        "PRONTO FOLD PREVIEW · {} · {} candidates ({} returned)",
        report.scope, report.candidate_total, report.returned_count
    );
    println!(
        "Branches: {} observed · more: {} · live verification required: {}",
        report.branch_total, report.has_more, report.live_verification_required
    );
    if let Some(cursor) = &report.next_cursor {
        println!("Next cursor: {cursor}");
    }
    for candidate in &report.candidates {
        println!(
            "  {} · {} -> {} · {} · {}",
            candidate.repository_name,
            candidate.source_branch,
            candidate
                .target_branch
                .as_deref()
                .unwrap_or("unknown target"),
            candidate.decision,
            candidate.reason
        );
        if let Some(preview) = &candidate.merge_preview {
            let breakdown = if preview.conflict_breakdown.is_empty() {
                "none".to_string()
            } else {
                preview
                    .conflict_breakdown
                    .iter()
                    .map(|(kind, count)| format!("{kind} {count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!(
                "    Merge: {} · fast-forwardable: {} · base: {} · target-only: {} · source-only: {} · conflicts: {} ({})",
                preview.merge_strategy,
                preview.fast_forwardable,
                preview.merge_base,
                preview.target_only_commits,
                preview.source_only_commits,
                preview.conflict_count,
                breakdown
            );
        }
    }
}

fn print_human_doctor(report: &AgentDoctorReport) {
    println!("PRONTO DOCTOR · {} · {}", report.status, report.scope);
    println!(
        "Snapshot: {} roots · {} repositories · {} workspaces · max age {} minutes",
        report.root_count, report.repository_count, report.workspace_count, report.max_age_minutes
    );
    if let (Some(oldest_scan_at), Some(oldest_scan_age_minutes)) = (
        report.oldest_scan_at.as_deref(),
        report.oldest_scan_age_minutes,
    ) {
        println!("Oldest scan: {oldest_scan_at} · {oldest_scan_age_minutes} minutes ago");
    }
    for check in &report.checks {
        println!("  {} · {} · {}", check.status, check.id, check.summary);
    }
    println!("Next: {}", report.next_safe_step);
}

fn print_human_route(report: &AgentRouteReport) {
    println!("PRONTO ROUTE · {} · {}", report.status, report.scope);
    println!(
        "Doctor: {} · next projection: {} · repository: {} · quality: {} · fold preview: {}",
        report.doctor.status,
        if report.next.is_some() {
            "available"
        } else {
            "blocked"
        },
        if report.repository.is_some() {
            "available"
        } else {
            "not selected"
        },
        if report.quality.is_some() {
            "available"
        } else {
            "blocked"
        },
        if report.fold_preview.is_some() {
            "available"
        } else {
            "blocked"
        },
    );
    if let Some(change) = &report.change_maturity {
        println!(
            "Change maturity: {} · {}",
            change
                .score
                .map(|score| format!("{score:.0}/4"))
                .unwrap_or_else(|| "unknown".into()),
            change.status
        );
        for gap in &change.gaps {
            println!("  gap: {gap}");
        }
        println!("Inspect: {}", change.recommended_inspection);
    }
    for (label, gate) in [
        ("Developer legibility", &report.developer_legibility),
        ("Change-surface hotspots", &report.change_surface_hotspots),
    ] {
        if let Some(gate) = gate {
            println!(
                "{label}: {} · {}",
                gate.score
                    .map(|score| format!("{score:.0}/4"))
                    .unwrap_or_else(|| "unknown".into()),
                gate.status
            );
            for gap in &gate.gaps {
                println!("  gap: {gap}");
            }
            println!("Inspect: {}", gate.recommended_inspection);
        }
    }
    println!("Next: {}", report.next_safe_step);
    println!("Authorization: {}", report.authorization);
}
