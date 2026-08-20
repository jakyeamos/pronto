fn remediation_closure_gate(
    plan: Option<&remediation::RemediationPlan>,
    closure: Option<&remediation::RemediationClosure>,
) -> RemediationClosureGate {
    if let Some(plan) = plan {
        let active_actions = plan
            .actions
            .iter()
            .filter(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"))
            .collect::<Vec<_>>();
        let blocked_action_ids = active_actions
            .iter()
            .filter(|action| action.status == "blocked")
            .map(|action| action.id.clone())
            .collect::<Vec<_>>();
        let status = if !blocked_action_ids.is_empty() {
            "blocked"
        } else if !active_actions.is_empty() {
            "not_ready"
        } else {
            "ready"
        };
        let detail = match status {
            "blocked" => format!(
                "{} explicitly blocked action(s) prevent plan closure; this does not by itself block remediation execution.",
                blocked_action_ids.len()
            ),
            "not_ready" => format!(
                "{} active action(s) remain before the remediation plan can close.",
                active_actions.len()
            ),
            _ => "No active action prevents plan closure.".to_string(),
        };
        return RemediationClosureGate {
            status: status.to_string(),
            ready: status == "ready",
            plan_status: Some(plan.status.clone()),
            active_action_count: active_actions.len(),
            blocked_action_count: blocked_action_ids.len(),
            blocked_action_ids,
            source_generated_at: Some(plan.generated_at.clone()),
            detail,
        };
    }
    if let Some(closure) = closure {
        return RemediationClosureGate {
            status: "complete".to_string(),
            ready: true,
            plan_status: Some(closure.disposition.clone()),
            active_action_count: 0,
            blocked_action_count: 0,
            blocked_action_ids: Vec::new(),
            source_generated_at: Some(closure.closed_at.clone()),
            detail: "The latest persisted remediation plan is closed.".to_string(),
        };
    }
    RemediationClosureGate {
        status: "not_queued".to_string(),
        ready: true,
        plan_status: None,
        active_action_count: 0,
        blocked_action_count: 0,
        blocked_action_ids: Vec::new(),
        source_generated_at: None,
        detail: "No active or resolved remediation plan is recorded for this repository."
            .to_string(),
    }
}

fn remediation_execution_blocker(
    workspace: &WorkspaceSummary,
    kind: &str,
    title: &str,
    detail: String,
    blocked_operations: &[&str],
    source: &str,
    evidence_state: &str,
    observed_at: Option<String>,
    next_safe_step: &str,
) -> RemediationExecutionBlocker {
    RemediationExecutionBlocker {
        id: format!("execution:{kind}:{}", workspace.id),
        kind: kind.to_string(),
        title: title.to_string(),
        detail,
        workspace_id: workspace.id.clone(),
        workspace_path: workspace.path.clone(),
        branch: workspace.branch.clone(),
        blocked_operations: blocked_operations
            .iter()
            .map(|operation| (*operation).to_string())
            .collect(),
        source: source.to_string(),
        evidence_state: evidence_state.to_string(),
        observed_at,
        next_safe_step: next_safe_step.to_string(),
    }
}

fn remediation_execution_gate_for_repository(
    repository: &RepositorySnapshot,
    plan: Option<&remediation::RemediationPlan>,
    closure: Option<&remediation::RemediationClosure>,
    workspace_id: Option<&str>,
) -> Result<RemediationExecutionGate, String> {
    let workspaces = if let Some(workspace_id) =
        workspace_id.filter(|value| !value.trim().is_empty())
    {
        let workspace = repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .or_else(|| (repository.workspace.id == workspace_id).then_some(&repository.workspace))
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?;
        vec![workspace]
    } else if repository.workspaces.is_empty() {
        vec![&repository.workspace]
    } else {
        repository.workspaces.iter().collect::<Vec<_>>()
    };

    let mut workspace_checks = Vec::new();
    let mut blockers = Vec::new();
    let mut ready_workspace_count = 0usize;
    for workspace in workspaces {
        if !Path::new(&workspace.path).is_dir() {
            blockers.push(remediation_execution_blocker(
                workspace,
                "path_unavailable",
                "Workspace path is unavailable",
                format!(
                    "{} is not an accessible folder, so live remediation state cannot be checked.",
                    workspace.path
                ),
                &[
                    "inspect_workspace",
                    "mutate_workspace",
                    "integrate_branch",
                    "verify_remediation_action",
                ],
                "live_filesystem",
                "unavailable",
                None,
                "Restore the workspace path or refresh Pronto after intentionally removing the checkout, then rerun `pronto remediation gate`.",
            ));
            continue;
        }

        let selected_id =
            (workspace.id != repository.workspace.id).then_some(workspace.id.as_str());
        let check = remediation_handoff_check_for_repository(repository, selected_id)?;
        if check.ownership_coordination_required {
            blockers.push(remediation_execution_blocker(
                workspace,
                "ownership_coordination_required",
                "Workspace ownership requires coordination",
                "Persisted activity evidence reports an active, interrupted, or ambiguous owner."
                    .to_string(),
                &[
                    "mutate_workspace",
                    "integrate_branch",
                    "verify_remediation_action",
                ],
                "persisted_activity_snapshot",
                "observed",
                workspace
                    .last_activity_at
                    .clone()
                    .or_else(|| Some(repository.last_scan_at.clone())),
                "Coordinate with the current owner, refresh repository activity evidence, then rerun `pronto remediation gate`.",
            ));
        } else if check.ownership_status == "evidence_unavailable" {
            blockers.push(remediation_execution_blocker(
                workspace,
                "ownership_evidence_unavailable",
                "Workspace ownership evidence is unavailable",
                "Activity inspection could not establish whether another agent owns this workspace."
                    .to_string(),
                &[
                    "mutate_workspace",
                    "integrate_branch",
                    "verify_remediation_action",
                ],
                "persisted_activity_snapshot",
                "unavailable",
                workspace
                    .activity
                    .signals
                    .iter()
                    .find(|signal| signal.summary == "Activity state uncertain")
                    .map(|signal| signal.observed_at.clone())
                    .or_else(|| Some(repository.last_scan_at.clone())),
                "Restore workspace activity inspection, refresh the repository, then rerun `pronto remediation gate`.",
            ));
        }
        if !check.status_available {
            blockers.push(remediation_execution_blocker(
                workspace,
                "git_status_unavailable",
                "Live Git status is unavailable",
                check.reasons.join(" "),
                &[
                    "mutate_workspace",
                    "integrate_branch",
                    "verify_remediation_action",
                ],
                "live_git",
                "unavailable",
                Some(check.generated_at.clone()),
                "Restore live Git access, then rerun `pronto remediation gate`.",
            ));
        }
        if let Some(operation) = check.operation.as_deref() {
            blockers.push(remediation_execution_blocker(
                workspace,
                "interrupted_git_operation",
                "Interrupted Git operation must be resolved",
                format!("The workspace has an interrupted {operation} operation."),
                &[
                    "mutate_workspace",
                    "integrate_branch",
                    "verify_remediation_action",
                ],
                "live_git",
                "observed",
                Some(check.generated_at.clone()),
                "Intentionally complete or abort the interrupted operation without discarding unrelated work, then rerun `pronto remediation gate`.",
            ));
        }
        if check.workspace_dirty {
            blockers.push(remediation_execution_blocker(
                workspace,
                "uncommitted_changes",
                "Workspace changes require preservation",
                "Live Git reports uncommitted changes that must be understood and preserved before handoff or integration."
                    .to_string(),
                &["handoff", "integrate_branch", "verify_remediation_action"],
                "live_git",
                "observed",
                Some(check.generated_at.clone()),
                "Confirm ownership and preserve the intended changes in a coherent checkpoint, then rerun `pronto remediation gate`.",
            ));
        } else if check.persisted_snapshot_dirty {
            blockers.push(remediation_execution_blocker(
                workspace,
                "snapshot_reconciliation_required",
                "Persisted workspace state is stale",
                "Live Git is clean, but the persisted Pronto snapshot still reports dirty work."
                    .to_string(),
                &["handoff", "verify_remediation_action"],
                "persisted_repository_snapshot",
                "stale",
                Some(repository.last_scan_at.clone()),
                "Run a repository-scoped refresh, then rerun `pronto remediation gate`.",
            ));
        }
        if check.ready {
            ready_workspace_count += 1;
        }
        workspace_checks.push(check);
    }

    blockers.sort_by(|left, right| left.id.cmp(&right.id));
    blockers.dedup_by(|left, right| left.id == right.id);
    let blocked_operations = blockers
        .iter()
        .flat_map(|blocker| blocker.blocked_operations.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let status = if blockers.is_empty() {
        "ready"
    } else if ready_workspace_count > 0 {
        "partially_blocked"
    } else {
        "blocked"
    };
    let next_safe_step = blockers
        .first()
        .map(|blocker| blocker.next_safe_step.clone())
        .unwrap_or_else(|| {
            "Repository state is ready for scoped remediation. Confirm that the current request authorizes the intended mutation before proceeding."
                .to_string()
        });

    Ok(RemediationExecutionGate {
        schema_version: REMEDIATION_EXECUTION_GATE_SCHEMA.to_string(),
        generated_at: iso_now(),
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        scope: if workspace_id.is_some() {
            "workspace"
        } else {
            "repository"
        }
        .to_string(),
        selected_workspace_id: workspace_id.map(str::to_string),
        status: status.to_string(),
        ready: blockers.is_empty(),
        workspace_checks,
        blockers,
        blocked_operations,
        closure_gate: remediation_closure_gate(plan, closure),
        authorization: RemediationAuthorizationBoundary {
            status: "caller_scope_required".to_string(),
            evaluated: false,
            source: "current_request_and_policy".to_string(),
            detail: "Pronto verifies repository execution state but cannot infer whether the current request authorizes a mutation. Execution status intentionally excludes authorization."
                .to_string(),
        },
        next_safe_step,
    })
}

fn remediation_execution_gate_at(
    path: &Path,
    query: &str,
    workspace_id: Option<&str>,
) -> Result<RemediationExecutionGate, String> {
    let state = load_store_read_only(path)?;
    let snapshot = snapshot_from_store(path, &state);
    let repository = find_cli_repository(&snapshot, query)?;
    let plan = snapshot
        .remediation
        .plans
        .iter()
        .find(|plan| plan.repository_id == repository.id);
    let closure = snapshot
        .remediation
        .closures
        .iter()
        .filter(|closure| closure.repository_id == repository.id)
        .max_by(|left, right| left.closed_at.cmp(&right.closed_at));
    remediation_execution_gate_for_repository(repository, plan, closure, workspace_id)
}

fn branch_integration_state(
    path: &Path,
    branch: &str,
    default_branch: Option<&str>,
    current_workspace: Option<&WorkspaceSummary>,
) -> String {
    if current_workspace.is_some_and(|workspace| !workspace.status_available) {
        return "Unknown".to_string();
    }
    let Some(target) = default_branch else {
        return "Target unknown".to_string();
    };
    if branch == target {
        return "No unique commits".to_string();
    }
    let unique = unique_commits(path, branch, Some(target));
    if unique == 0 {
        return "Already integrated".to_string();
    }
    if let Some(workspace) = current_workspace {
        if workspace.operation.is_some() || workspace.dirty || workspace.activity.state == "Active"
        {
            return "Blocked".to_string();
        }
    }
    "Integration eligible".to_string()
}
