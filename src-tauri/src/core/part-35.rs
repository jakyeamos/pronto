fn agent_doctor_report(
    snapshot: &PortfolioSnapshot,
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
) -> AgentDoctorReport {
    let max_age_minutes = max_age_minutes.max(0);
    let now = Utc::now();
    let relevant_roots = agent_doctor_relevant_roots(snapshot, scope);
    let mut missing_root_paths = Vec::new();
    let mut unavailable_paths = BTreeSet::new();
    let mut workspace_blockers = Vec::new();
    let mut workspace_warnings = Vec::new();
    for root in &relevant_roots {
        if !Path::new(&root.path).is_dir() {
            missing_root_paths.push(root.path.clone());
            unavailable_paths.insert(root.path.clone());
        }
    }
    for repository in &snapshot.repositories {
        let repository_path = Path::new(&repository.path);
        if !repository_path.is_dir() {
            unavailable_paths.insert(repository.path.clone());
            continue;
        }
        for workspace in &repository.workspaces {
            if !Path::new(&workspace.path).is_dir() {
                match classify_missing_workspace(repository_path, workspace) {
                    MissingWorkspaceClassification::Blocked(reason) => {
                        unavailable_paths.insert(workspace.path.clone());
                        workspace_blockers.push(reason);
                    }
                    MissingWorkspaceClassification::Warning(warning) => {
                        workspace_warnings.push(warning);
                    }
                }
            }
        }
    }

    let mut stale_repository_ids = Vec::new();
    let mut invalid_scan_repository_ids = Vec::new();
    let mut oldest_scan: Option<(DateTime<Utc>, i64)> = None;
    for repository in &snapshot.repositories {
        let Ok(parsed) = DateTime::parse_from_rfc3339(&repository.last_scan_at) else {
            invalid_scan_repository_ids.push(repository.id.clone());
            continue;
        };
        let observed_at = parsed.with_timezone(&Utc);
        let age_minutes = now.signed_duration_since(observed_at).num_minutes();
        if age_minutes < 0 {
            invalid_scan_repository_ids.push(repository.id.clone());
            continue;
        }
        if age_minutes > max_age_minutes {
            stale_repository_ids.push(repository.id.clone());
        }
        if oldest_scan
            .as_ref()
            .map_or(true, |(oldest, _)| observed_at < *oldest)
        {
            oldest_scan = Some((observed_at, age_minutes));
        }
    }

    let mut checks = vec![agent_doctor_check(
        "storage",
        "Passed",
        "The Pronto database loaded through a read-only connection.".to_string(),
        vec![storage_path.display().to_string()],
        "Continue using focused Pronto projections for the requested scope.".to_string(),
    )];
    if relevant_roots.is_empty() && (scope == "fleet" || snapshot.repositories.is_empty()) {
        checks.push(agent_doctor_check(
            "roots",
            "Blocked",
            "No discovery roots are registered for this scope.".to_string(),
            Vec::new(),
            "Register an explicit discovery root and run a scoped refresh before routing work."
                .to_string(),
        ));
    } else if relevant_roots.is_empty() {
        checks.push(agent_doctor_check(
            "roots",
            "Warning",
            "No registered discovery root covers the scoped repositories.".to_string(),
            snapshot
                .repositories
                .iter()
                .map(|repository| repository.path.clone())
                .collect(),
            "Keep the repository scope explicit; register a covering root before relying on discovery refresh."
                .to_string(),
        ));
    } else if missing_root_paths.is_empty() {
        checks.push(agent_doctor_check(
            "roots",
            "Passed",
            format!(
                "{} registered discovery roots are available.",
                relevant_roots.len()
            ),
            relevant_roots
                .iter()
                .map(|root| root.path.clone())
                .collect(),
            "Keep refreshes scoped to the repository or root required by the task.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "roots",
            "Blocked",
            format!(
                "{} registered discovery root(s) are unavailable.",
                missing_root_paths.len()
            ),
            missing_root_paths.clone(),
            "Inspect the registered roots and restore or explicitly reconfigure unavailable paths before routing work.".to_string(),
        ));
    }

    if snapshot.repositories.is_empty() {
        checks.push(agent_doctor_check(
            "snapshot",
            "Blocked",
            "The persisted snapshot contains no repositories.".to_string(),
            Vec::new(),
            "Run a scoped refresh after confirming the discovery root; do not infer an empty portfolio from this report.".to_string(),
        ));
    } else if !stale_repository_ids.is_empty() || !invalid_scan_repository_ids.is_empty() {
        let mut evidence = stale_repository_ids.clone();
        evidence.extend(
            invalid_scan_repository_ids
                .iter()
                .map(|id| format!("invalid timestamp: {id}")),
        );
        checks.push(agent_doctor_check(
            "snapshot",
            "Blocked",
            format!(
                "{} repository snapshot(s) exceed the {} minute freshness window or have invalid scan timestamps.",
                stale_repository_ids.len() + invalid_scan_repository_ids.len(),
                max_age_minutes
            ),
            evidence,
            "Run a scoped `pronto refresh <repository> --json` for every stale or invalid repository, then rerun doctor.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "snapshot",
            "Passed",
            format!(
                "All {} repository snapshots are within the {} minute freshness window.",
                snapshot.repositories.len(),
                max_age_minutes
            ),
            snapshot
                .repositories
                .iter()
                .map(|repository| format!("{}: {}", repository.id, repository.last_scan_at))
                .collect(),
            "Use the focused projection that matches the task scope.".to_string(),
        ));
    }

    let unavailable_paths = unavailable_paths.into_iter().collect::<Vec<_>>();
    if unavailable_paths.is_empty() && workspace_warnings.is_empty() {
        checks.push(agent_doctor_check(
            "paths",
            "Passed",
            format!(
                "All {} repository and workspace paths are available.",
                snapshot.repositories.len()
                    + snapshot
                        .repositories
                        .iter()
                        .map(|repository| repository.workspaces.len())
                        .sum::<usize>()
            ),
            Vec::new(),
            "Treat the persisted paths as local evidence and still recheck live state before mutation.".to_string(),
        ));
    } else if !unavailable_paths.is_empty() {
        let mut evidence = unavailable_paths.clone();
        evidence.extend(workspace_blockers.clone());
        checks.push(agent_doctor_check(
            "paths",
            "Blocked",
            format!(
                "{} registered repository or workspace path(s) are unavailable.",
                unavailable_paths.len()
            ),
            evidence,
            "Preserve the affected work and inspect path ownership before refreshing or folding anything.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "paths",
            "Warning",
            format!(
                "{} missing temporary workspace record(s) are eligible for scoped stale-record refresh.",
                workspace_warnings.len()
            ),
            workspace_warnings.clone(),
            "Run `pronto refresh <repository> --json` for each warning, then rerun the same scoped route; do not use a fleet-wide refresh just to remove stale records.".to_string(),
        ));
    }

    if snapshot.quality.audit_status == "Ready" {
        checks.push(agent_doctor_check(
            "quality",
            "Passed",
            "Quality evidence reports Ready in the persisted portfolio snapshot.".to_string(),
            vec![snapshot.quality.audit_status.clone()],
            "Keep quality evidence separate from fresh local execution proof.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "quality",
            "Warning",
            format!(
                "Quality evidence is {} and does not block local portfolio routing.",
                snapshot.quality.audit_status
            ),
            vec![snapshot.quality.audit_status.clone()],
            "Do not treat quality or maturity as passing evidence until its source is fresh and verified.".to_string(),
        ));
    }

    let blocking = checks
        .iter()
        .any(|check| check.status == "Blocked" || check.status == "Unknown");
    let warning = checks.iter().any(|check| check.status == "Warning");
    let ready = !blocking;
    let status = if blocking {
        "Blocked"
    } else if warning {
        "Ready with warnings"
    } else {
        "Ready"
    };
    let next_safe_step = checks
        .iter()
        .find(|check| check.status == "Blocked" || check.status == "Unknown")
        .map(|check| check.next_safe_step.clone())
        .unwrap_or_else(|| {
            "Proceed with the focused Pronto projection appropriate to the task.".to_string()
        });

    AgentDoctorReport {
        schema_version: AGENT_DOCTOR_SCHEMA.to_string(),
        generated_at: iso_now(),
        scope: scope.to_string(),
        status: status.to_string(),
        ready,
        storage_path: storage_path.to_string_lossy().to_string(),
        max_age_minutes,
        root_count: relevant_roots.len(),
        repository_count: snapshot.repositories.len(),
        workspace_count: snapshot
            .repositories
            .iter()
            .map(|repository| repository.workspaces.len())
            .sum(),
        oldest_scan_at: oldest_scan
            .as_ref()
            .map(|(observed_at, _)| observed_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
        oldest_scan_age_minutes: oldest_scan.map(|(_, age_minutes)| age_minutes),
        stale_repository_ids,
        invalid_scan_repository_ids,
        unavailable_paths,
        workspace_warnings,
        checks,
        next_safe_step,
        authorization: "Inspection only; doctor does not refresh, write Pronto state, modify Git, access provider state, or authorize repository mutations.".to_string(),
    }
}

fn agent_doctor_error_report(
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    check_id: &str,
    error: String,
) -> AgentDoctorReport {
    let next_safe_step = if error.contains("Fresh quality projection") {
        "Rerun without `--fresh` for the cached snapshot or run `pronto quality refresh` separately before retrying a fresh projection.".to_string()
    } else if check_id == "storage" {
        "Inspect or repair the local Pronto database; do not route work from this failed state."
            .to_string()
    } else {
        "Resolve the doctor scope query and rerun doctor before routing work.".to_string()
    };
    AgentDoctorReport {
        schema_version: AGENT_DOCTOR_SCHEMA.to_string(),
        generated_at: iso_now(),
        scope: scope.to_string(),
        status: "Blocked".to_string(),
        ready: false,
        storage_path: storage_path.to_string_lossy().to_string(),
        max_age_minutes: max_age_minutes.max(0),
        root_count: 0,
        repository_count: 0,
        workspace_count: 0,
        oldest_scan_at: None,
        oldest_scan_age_minutes: None,
        stale_repository_ids: Vec::new(),
        invalid_scan_repository_ids: Vec::new(),
        unavailable_paths: Vec::new(),
        workspace_warnings: Vec::new(),
        checks: vec![agent_doctor_check(
            check_id,
            "Blocked",
            error,
            vec![storage_path.display().to_string()],
            next_safe_step.clone(),
        )],
        next_safe_step,
        authorization: "Inspection only; doctor does not refresh, write Pronto state, modify Git, access provider state, or authorize repository mutations.".to_string(),
    }
}

fn agent_route_from_doctor(
    doctor: AgentDoctorReport,
    scope: &str,
    next: Option<AgentNextReport>,
    repository: Option<AgentRepositoryDetail>,
    quality: Option<AgentQualityReport>,
    fold_preview: Option<AgentFoldPreview>,
) -> AgentRouteReport {
    let change_maturity = repository.as_ref().map(|detail| {
        let maturity = &detail.repository.quality.maturity;
        let score = maturity
            .dimension_scores
            .get("change_surface_coverage")
            .copied();
        let gaps = maturity
            .gaps
            .iter()
            .filter(|gap| {
                matches!(
                    gap.dimension.as_str(),
                    "change_surface_coverage" | "skill_contract_quality"
                )
            })
            .take(3)
            .map(|gap| gap.message.clone())
            .collect::<Vec<_>>();
        AgentChangeMaturitySummary {
            score,
            status: match score {
                Some(value) if value >= 4.0 => "proven",
                Some(value) if value >= 3.0 => "validated",
                Some(value) if value > 0.0 => "attention",
                Some(_) => "missing",
                None => "unknown",
            }
            .to_string(),
            gaps,
            recommended_inspection: format!(
                "pronto change-matrix repo '{}' --json",
                detail.repository.path.replace('\'', "'\\''")
            ),
        }
    });
    let developer_legibility = repository
        .as_ref()
        .map(|detail| agent_maturity_gate_summary(detail, "developer_legibility"));
    let change_surface_hotspots = repository
        .as_ref()
        .map(|detail| agent_maturity_gate_summary(detail, "change_surface_hotspots"));
    let next_safe_step = if doctor.ready {
        next.as_ref()
            .and_then(|report| report.actions.first())
            .map(|action| action.next_safe_step.clone())
            .unwrap_or_else(|| {
                "No active attention action was observed; choose the next bounded inspection for this scope.".to_string()
            })
    } else {
        doctor.next_safe_step.clone()
    };
    AgentRouteReport {
        schema_version: AGENT_ROUTE_SCHEMA.to_string(),
        generated_at: doctor.generated_at.clone(),
        scope: scope.to_string(),
        status: doctor.status.clone(),
        ready: doctor.ready,
        doctor,
        next,
        repository,
        quality,
        fold_preview,
        change_maturity,
        developer_legibility,
        change_surface_hotspots,
        next_safe_step,
        authorization: "Inspection only; this route does not refresh, modify Git, change provider state, update remediation status, or authorize repository or release mutations.".to_string(),
    }
}

fn agent_maturity_gate_summary(
    detail: &AgentRepositoryDetail,
    dimension: &str,
) -> AgentMaturityGateSummary {
    let maturity = &detail.repository.quality.maturity;
    let score = maturity.dimension_scores.get(dimension).copied();
    let gaps = maturity
        .gaps
        .iter()
        .filter(|gap| gap.dimension == dimension)
        .take(3)
        .map(|gap| gap.message.clone())
        .collect::<Vec<_>>();
    AgentMaturityGateSummary {
        score,
        status: match score {
            Some(value) if value >= 4.0 && dimension == "developer_legibility" => {
                "newcomer_verified"
            }
            Some(value) if value >= 4.0 => "maintained",
            Some(value) if value >= 3.0 && dimension == "developer_legibility" => "enforced",
            Some(value) if value >= 3.0 => "validated",
            Some(value) if value > 0.0 => "attention",
            Some(_) => "missing",
            None => "unknown",
        }
        .to_string(),
        gaps,
        recommended_inspection: format!(
            "qr fleet audit run --repo-path '{}'{} --json",
            detail.repository.path.replace('\'', "'\\''"),
            if dimension == "developer_legibility" {
                " --standard developer-legibility"
            } else {
                ""
            }
        ),
    }
}
