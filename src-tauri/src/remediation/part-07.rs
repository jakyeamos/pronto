fn add_branch_hygiene_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let workspaces = if repository.workspaces.is_empty() {
        vec![&repository.workspace]
    } else {
        repository.workspaces.iter().collect::<Vec<_>>()
    };
    for workspace in workspaces {
        if workspace_activity_requires_coordination(&workspace.activity) {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:activity:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Coordinate workspace ownership · {}", workspace.branch),
                summary: format!(
                    "{} reports {} activity; preserve ownership before changing its branch or worktree.",
                    workspace.path, workspace.activity.state
                ),
                severity: "ownership".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "The active or interrupted owner is identified and coordinated with."
                        .to_string(),
                    "No worktree, branch, or unpublished work is changed while ownership is ambiguous."
                        .to_string(),
                    "A refreshed snapshot no longer reports unresolved active ownership.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Workspace activity · {}", workspace.branch),
                    &workspace.activity.state,
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &format!(
                        "{} activity signal(s); confidence {}.",
                        workspace.activity.signals.len(),
                        workspace.activity.confidence
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if let Some(operation) = workspace.operation.as_deref() {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:operation:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Resolve interrupted Git operation · {}", workspace.branch),
                summary: format!(
                    "{} has an interrupted {operation} operation that must be resolved before reconciliation.",
                    workspace.path
                ),
                severity: "operation".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "Inspect and intentionally complete or abort the interrupted Git operation."
                        .to_string(),
                    "Preserve all unrelated and unpublished work.".to_string(),
                    "A refreshed snapshot reports no interrupted operation.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Git operation · {}", workspace.branch),
                    operation,
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    "An interrupted Git operation is a hard stop for branch mutation.",
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if !workspace.status_available {
            let status_error = workspace.status_error.clone().unwrap_or_else(|| {
                "Git status could not be established for this workspace.".to_string()
            });
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:status-unavailable:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Restore Git status access · {}", workspace.branch),
                summary: format!(
                    "{} cannot be safely classified because Git status is unavailable: {status_error}",
                    workspace.path
                ),
                severity: "status".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "Restore readable local Git status for the workspace.".to_string(),
                    "A scoped Pronto refresh records branch, cleanliness, upstream, and sync evidence again.".to_string(),
                    "No integration, cleanup, or publication decision is made while status evidence is unavailable.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Git status · {}", workspace.branch),
                    "Unavailable",
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &status_error,
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        } else if workspace.dirty {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:dirty:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Resolve dirty workspace · {}", workspace.branch),
                summary: format!(
                    "{} has uncommitted changes that must be locally checkpointed and handoff-checked before remediation can be verified.",
                    workspace.path
                ),
                severity: "workspace".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "The intended scoped changes are committed on the isolated working branch before this action is verified.".to_string(),
                    "Owner-ambiguous or unrelated changes remain preserved and explicitly reported; they are not stashed, overwritten, or silently folded.".to_string(),
                    "A fresh `pronto remediation handoff-check` receipt reports `ready: true`.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Dirty workspace · {}", workspace.branch),
                    "Dirty",
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &format!("{} changed lines added and {} removed.", workspace.added, workspace.removed),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if workspace.status_available && workspace.sync_state != "Synced" {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:sync:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Reconcile branch sync · {}", workspace.branch),
                summary: format!(
                    "The workspace is {} relative to its upstream/default branch.",
                    workspace.sync_state
                ),
                severity: "sync".to_string(),
                priority: "P2".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "The branch relationship is understood and intentional.".to_string(),
                    "The final verification run records the resulting branch state.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch sync · {}", workspace.branch),
                    &workspace.sync_state,
                    "Fresh",
                    workspace.last_commit_at.as_deref(),
                    None,
                    &format!("Ahead {} · behind {}", workspace.ahead, workspace.behind),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        let remote_freshness = workspace.remote_freshness.to_ascii_lowercase();
        if workspace.status_available
            && (remote_freshness.contains("not fetched")
                || remote_freshness.contains("stale")
                || remote_freshness.contains("unknown")
                || remote_freshness.contains("unavailable"))
        {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:remote-freshness:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Refresh remote branch evidence · {}", workspace.branch),
                summary: "The workspace's remote comparison is not fresh enough to support reconciliation or pruning decisions.".to_string(),
                severity: "freshness".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Fetch remote evidence through the authorized Pronto workflow.".to_string(),
                    "Recompute ahead, behind, upstream, and integration state from fresh evidence."
                        .to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Remote freshness · {}", workspace.branch),
                    &workspace.remote_freshness,
                    &workspace.remote_freshness,
                    workspace.last_commit_at.as_deref(),
                    None,
                    "Stale or unavailable remote evidence cannot prove branch closure.",
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if workspace.integration_state == "Integration eligible" {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:integrate:{}", workspace.branch),
                domain: "branch_hygiene".to_string(),
                title: format!("Classify integration-ready branch · {}", workspace.branch),
                summary: format!(
                    "{} has unique commits eligible for integration into {}.",
                    workspace.branch,
                    workspace.target_branch.as_deref().unwrap_or("the canonical branch")
                ),
                severity: "integration".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Review the complete diff and confirm whether the work is wanted, superseded, or intentionally preserved.".to_string(),
                    "Fold wanted work through the documented integration lane and verify the canonical branch.".to_string(),
                    "Treat pruning as a separate, later authorization after integration equivalence is proven.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch integration · {}", workspace.branch),
                    &workspace.integration_state,
                    "Fresh",
                    workspace.last_commit_at.as_deref(),
                    None,
                    &format!(
                        "Target: {} · confidence: {}.",
                        workspace.target_branch.as_deref().unwrap_or("Unknown"),
                        workspace.target_confidence
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
    }
    for branch in &repository.branches {
        if branch.integration_state == "Integration eligible"
            && !seeds
                .iter()
                .any(|seed| seed.stable_key == format!("branch_hygiene:integrate:{}", branch.name))
        {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:integrate:{}", branch.name),
                domain: "branch_hygiene".to_string(),
                title: format!("Classify integration-ready branch · {}", branch.name),
                summary: format!(
                    "{} has unique commits eligible for integration into {}.",
                    branch.name,
                    branch.target_branch.as_deref().unwrap_or("the canonical branch")
                ),
                severity: "integration".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Review the complete diff and confirm whether the work is wanted, superseded, or intentionally preserved.".to_string(),
                    "Fold wanted work through the documented integration lane and verify the canonical branch.".to_string(),
                    "Treat pruning as a separate, later authorization after integration equivalence is proven.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch integration · {}", branch.name),
                    &branch.integration_state,
                    "Fresh",
                    branch.last_commit_at.as_deref(),
                    None,
                    &format!(
                        "Ahead {} · behind {} · target {}.",
                        branch.ahead,
                        branch.behind,
                        branch.target_branch.as_deref().unwrap_or("Unknown")
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
    }
    const REPRESENTED_CONDITIONS: [&str; 9] = [
        "active-agent-workspace",
        "interrupted-operation",
        "dirty-workspace",
        "diverged-branch",
        "unpushed-commits",
        "behind-remote",
        "remote-stale",
        "no-upstream",
        "integration-eligible",
    ];
    for condition in repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .filter(|condition| !REPRESENTED_CONDITIONS.contains(&condition.kind.as_str()))
    {
        seeds.push(ActionSeed {
            stable_key: format!("repository_health:condition:{}", condition.id),
            domain: "repository_health".to_string(),
            title: format!("Resolve repository condition · {}", condition.title),
            summary: condition.summary.clone(),
            severity: "condition".to_string(),
            priority: if condition.priority <= 1 { "P1" } else { "P2" }.to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Inspect the condition's rule, evidence, and missing prerequisites.".to_string(),
                "Resolve or explicitly disposition the condition and refresh Pronto.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto condition",
                &condition.title,
                &condition.status,
                condition.freshness.as_deref().unwrap_or("Unknown"),
                Some(&repository.last_scan_at),
                None,
                &condition.summary,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

pub(crate) fn workspace_activity_requires_coordination(
    activity: &crate::core::WorkspaceActivity,
) -> bool {
    workspace_activity_execution_state(activity) != "clear"
}

pub(crate) fn workspace_activity_execution_state(
    activity: &crate::core::WorkspaceActivity,
) -> &'static str {
    let explicitly_active = activity.state.eq_ignore_ascii_case("active")
        || activity
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.status.as_deref())
            .is_some_and(|status| {
                matches!(
                    status.to_ascii_lowercase().as_str(),
                    "active" | "running" | "started" | "paused" | "interrupted"
                )
            });
    if explicitly_active {
        return "coordination_required";
    }
    if activity
        .signals
        .iter()
        .any(|signal| signal.summary == "Activity state uncertain")
    {
        return "evidence_unavailable";
    }
    "clear"
}

fn add_submodule_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for submodule in &repository.submodules {
        if submodule.status == "Checked out" {
            continue;
        }
        seeds.push(ActionSeed {
            stable_key: format!("repository_health:submodule:{}", submodule.path),
            domain: "repository_health".to_string(),
            title: format!("Resolve submodule state · {}", submodule.path),
            summary: format!(
                "The submodule is '{}', so the repository snapshot is not in its expected checked-out state.",
                submodule.status
            ),
            severity: "repository_health".to_string(),
            priority: if submodule.status == "Merge conflict" {
                "P0"
            } else {
                "P1"
            }
            .to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Confirm the intended submodule commit from repository-owned evidence.".to_string(),
                "Resolve the submodule without discarding unrelated local work.".to_string(),
                "A refreshed Pronto snapshot reports the submodule as Checked out.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                &format!("Submodule · {}", submodule.path),
                &submodule.status,
                "Fresh",
                Some(&repository.last_scan_at),
                Some(&submodule.path),
                submodule
                    .commit
                    .as_deref()
                    .unwrap_or("No checked-out submodule commit was recorded."),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}
