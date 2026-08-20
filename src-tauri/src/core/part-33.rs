fn agent_next_action(item: &AgentAttentionItem) -> AgentNextAction {
    let (recommended_projection, next_safe_step) = match item.category.as_str() {
        "quality_gate" => (
            "quality",
            "Read the repository quality projection before changing code or recording remediation status.".to_string(),
        ),
        "quality_findings" => (
            "quality",
            "Read the repository quality projection and source report before planning remediation.".to_string(),
        ),
        "quality_maturity" => (
            "quality",
            "Inspect quality freshness and source provenance; stale or missing maturity evidence is not a pass.".to_string(),
        ),
        "synchronization" => {
            let command = item
                .evidence
                .iter()
                .find(|evidence| evidence.label == "Next safe scoped refresh")
                .and_then(|evidence| evidence.value.clone())
                .unwrap_or_else(|| {
                    format!(
                        "pronto refresh {} --json",
                        shell_quote_for_display(&item.repository_path)
                    )
                });
            (
                "repo",
                format!(
                    "Inspect this workspace's sync detail, then run `{command}` only when a fresh local comparison is needed; reopen the repository projection afterward."
                ),
            )
        }
        "workspace" => (
            "repo",
            "Inspect the repository projection; preserve dirty workspace contents and active work.".to_string(),
        ),
        "condition" => (
            "repo",
            "Inspect the repository projection and condition evidence before choosing a workflow.".to_string(),
        ),
        _ => (
            "attention",
            "Inspect the linked evidence before taking any repository, provider, remediation, or release action.".to_string(),
        ),
    };
    AgentNextAction {
        attention_id: item.id.clone(),
        repository_id: item.repository_id.clone(),
        repository_name: item.repository_name.clone(),
        workspace_id: item.workspace_id.clone(),
        category: item.category.clone(),
        severity: item.severity.clone(),
        status: item.status.clone(),
        summary: item.summary.clone(),
        recommended_projection: recommended_projection.to_string(),
        next_safe_step,
        authorization: if item.category == "synchronization" {
            "Scoped refresh is a read-only local Git scan; it persists Pronto evidence but does not pull, push, merge, rebase, or edit repository files.".to_string()
        } else {
            "Inspection only; Git, provider, remediation, and release mutations require explicit authorization.".to_string()
        },
    }
}

fn agent_next_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    scope: &str,
    limit: usize,
) -> Result<AgentNextReport, String> {
    let current_repository = query
        .map(|value| find_cli_repository(snapshot, value).map(agent_repository_summary))
        .transpose()?;
    let summary = agent_summary(snapshot, scope);
    let mut attention = agent_attention_report(snapshot).items;
    attention.sort_by(|left, right| {
        agent_attention_priority(left)
            .cmp(&agent_attention_priority(right))
            .then_with(|| left.id.cmp(&right.id))
    });
    let attention_total = attention.len();
    attention.truncate(limit);
    let actions = attention.iter().take(3).map(agent_next_action).collect();
    Ok(AgentNextReport {
        schema_version: AGENT_NEXT_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        summary,
        current_repository,
        attention_total,
        attention,
        actions,
    })
}

fn agent_fold_target(
    repository: &RepositorySnapshot,
    requested_target: Option<&str>,
) -> (Option<String>, String, String) {
    if let Some(target) = requested_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return (
            Some(target.to_string()),
            "Explicit command target".to_string(),
            "Explicit".to_string(),
        );
    }
    match repository
        .target_branch
        .clone()
        .or_else(|| repository.default_branch.clone())
    {
        Some(target) => (
            Some(target),
            if repository.target_branch_configured {
                "Pronto configured repository target".to_string()
            } else {
                "Pronto observed default branch".to_string()
            },
            "High".to_string(),
        ),
        None => (
            None,
            "No observed default branch".to_string(),
            "Unknown".to_string(),
        ),
    }
}

fn agent_fold_candidate_decision(
    branch: &BranchSummary,
    target: Option<&str>,
    workspace: Option<&WorkspaceSummary>,
) -> (String, String, String) {
    if let Some(workspace) = workspace {
        if !workspace.status_available {
            return (
                "status_unavailable".to_string(),
                workspace_status_unavailable_reason(workspace),
                "Restore live Git status access before evaluating this branch for integration or pruning."
                    .to_string(),
            );
        }
        if let Some(operation) = workspace.operation.as_deref() {
            return (
                "blocked_operation".to_string(),
                format!("Git operation in progress: {operation}"),
                "Resolve the in-progress Git operation before evaluating this branch.".to_string(),
            );
        }
        if workspace.activity.state == "Active" {
            return (
                "preserve_active".to_string(),
                "Agent activity is active for this workspace.".to_string(),
                "Wait for or hand off the active agent; do not integrate or prune this branch."
                    .to_string(),
            );
        }
        if workspace.dirty {
            return (
                "preserve_dirty".to_string(),
                "The linked workspace has uncommitted changes.".to_string(),
                "Preserve the workspace and inspect its complete diff before any fold decision."
                    .to_string(),
            );
        }
    }
    let Some(target) = target else {
        return (
            "target_unknown".to_string(),
            "No target branch was observed for this repository.".to_string(),
            "Identify the repository's canonical integration target before considering a fold."
                .to_string(),
        );
    };
    if branch.target_branch.as_deref() != Some(target) {
        return (
            "target_mismatch".to_string(),
            format!(
                "Pronto observed {} as this branch's target, not {target}.",
                branch.target_branch.as_deref().unwrap_or("an unknown branch")
            ),
            "Confirm the canonical integration target with the fold workflow before classifying this branch.".to_string(),
        );
    }
    let Some(workspace) = workspace else {
        return (
            "live_check_required".to_string(),
            "No registered workspace provides cleanliness or activity evidence for this branch."
                .to_string(),
            "Run live ref and worktree classification; do not treat this snapshot as fold authorization.".to_string(),
        );
    };
    if workspace.activity.confidence == "Low" {
        return (
            "activity_uncertain".to_string(),
            "Workspace activity evidence is explicitly uncertain.".to_string(),
            "Recheck live workspace ownership and activity before integration or pruning."
                .to_string(),
        );
    }
    match branch.integration_state.as_str() {
        "Already integrated" => (
            "prune_review".to_string(),
            "Pronto observed no unique commits relative to the observed target.".to_string(),
            "Verify remote ancestry or patch equivalence, PR/protection state, and worktree cleanliness before authorizing pruning.".to_string(),
        ),
        "Integration eligible" if workspace.upstream.is_none() => (
            "preserve_unpublished".to_string(),
            "The branch has unique commits but no tracked upstream is recorded.".to_string(),
            "Preserve the unpublished branch; stabilize and push it only with explicit task scope before integration.".to_string(),
        ),
        "Integration eligible" if workspace.sync_state != "Synced" => (
            "refresh_before_integration".to_string(),
            format!(
                "The branch has unique commits but its workspace is {}.",
                workspace.sync_state
            ),
            "Refresh scoped evidence and verify the live remote head before integration.".to_string(),
        ),
        "Integration eligible" => (
            "review_for_integration".to_string(),
            "Pronto observed a clean branch with unique commits relative to the observed target.".to_string(),
            "Run fold-feature-branches live classification and review the complete source diff before integration.".to_string(),
        ),
        "Blocked" => (
            "blocked".to_string(),
            "Pronto observed the branch as blocked for integration.".to_string(),
            "Inspect the linked workspace and condition evidence; preserve the branch until the blocker is resolved.".to_string(),
        ),
        "No unique commits" => (
            "no_unique_commits".to_string(),
            "Pronto observed no unique commits for this branch against its recorded target.".to_string(),
            "Verify live remote and worktree state; do not prune unless supersession is proven.".to_string(),
        ),
        _ => (
            "inspect".to_string(),
            format!(
                "Pronto recorded integration state: {}.",
                branch.integration_state
            ),
            "Inspect the live branch, worktree, ancestry, and remote evidence before choosing a workflow.".to_string(),
        ),
    }
}

fn agent_fold_decision_priority(decision: &str) -> u8 {
    match decision {
        "status_unavailable" | "preserve_dirty" | "preserve_active" | "blocked_operation"
        | "blocked" => 0,
        "target_unknown" | "target_mismatch" | "live_check_required" | "activity_uncertain" => 1,
        "preserve_unpublished" | "refresh_before_integration" => 2,
        "review_for_integration" => 3,
        "prune_review" => 4,
        "no_unique_commits" => 5,
        _ => 6,
    }
}

fn git_is_ancestor(path: &Path, ancestor: &str, descendant: &str) -> bool {
    run_git(
        path,
        vec![
            "merge-base".to_string(),
            "--is-ancestor".to_string(),
            ancestor.to_string(),
            descendant.to_string(),
        ],
    )
    .map(|result| result.success)
    .unwrap_or(false)
}

fn merge_tree_conflicts(output: &str) -> BTreeMap<String, u64> {
    let mut breakdown = BTreeMap::new();
    for line in output.lines() {
        let trimmed = line.trim();
        let kind = if let Some(kind) = trimmed
            .strip_prefix("CONFLICT (")
            .and_then(|value| value.split_once(')'))
            .map(|(kind, _)| kind)
        {
            Some(kind.to_string())
        } else {
            match trimmed {
                "changed in both" => Some("content".to_string()),
                "added in both" => Some("add/add".to_string()),
                "removed in local" | "removed in remote" => Some("modify/delete".to_string()),
                "removed in both" => Some("delete/delete".to_string()),
                _ => None,
            }
        };
        let Some(kind) = kind else {
            continue;
        };
        *breakdown.entry(kind).or_insert(0) += 1;
    }
    breakdown
}

fn agent_fold_merge_preview(
    path: &Path,
    source_branch: &str,
    target_branch: &str,
) -> Option<AgentFoldMergePreview> {
    let merge_base = git_owned(
        path,
        vec![
            "merge-base".to_string(),
            target_branch.to_string(),
            source_branch.to_string(),
        ],
    )?;
    let target_is_ancestor = git_is_ancestor(path, target_branch, source_branch);
    let source_is_ancestor = git_is_ancestor(path, source_branch, target_branch);
    let merge_strategy = if target_is_ancestor {
        "fast-forward"
    } else if source_is_ancestor {
        "already-integrated"
    } else {
        "three-way-merge"
    };
    let target_only_commits = git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{merge_base}..{target_branch}"),
        ],
    )?
    .parse()
    .ok()?;
    let source_only_commits = git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{merge_base}..{source_branch}"),
        ],
    )?
    .parse()
    .ok()?;
    let conflict_breakdown = if merge_strategy == "three-way-merge" {
        run_git(
            path,
            vec![
                "merge-tree".to_string(),
                merge_base.clone(),
                target_branch.to_string(),
                source_branch.to_string(),
            ],
        )
        .ok()
        .map(|result| merge_tree_conflicts(&result.stdout))
        .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let conflict_count = conflict_breakdown.values().sum();
    Some(AgentFoldMergePreview {
        merge_strategy: merge_strategy.to_string(),
        fast_forwardable: target_is_ancestor,
        target_is_ancestor,
        source_is_ancestor,
        merge_base,
        target_only_commits,
        source_only_commits,
        conflict_count,
        conflict_breakdown,
    })
}

fn agent_fold_candidate(
    repository: &RepositorySnapshot,
    branch: &BranchSummary,
    target: Option<&str>,
    target_source: &str,
    target_confidence: &str,
    include_merge_preview: bool,
) -> AgentFoldCandidate {
    let workspace = branch.workspace_id.as_deref().and_then(|workspace_id| {
        repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
    });
    let (decision, reason, next_safe_step) =
        agent_fold_candidate_decision(branch, target, workspace);
    AgentFoldCandidate {
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        source_branch: branch.name.clone(),
        target_branch: target.map(str::to_string),
        target_source: target_source.to_string(),
        target_confidence: target_confidence.to_string(),
        workspace_id: branch.workspace_id.clone(),
        workspace_path: workspace.map(|item| item.path.clone()),
        role: branch.role.clone(),
        role_confidence: branch.role_confidence.clone(),
        integration_state: branch.integration_state.clone(),
        dirty: workspace.map(|item| item.dirty),
        sync_state: workspace.map(|item| item.sync_state.clone()),
        ahead: workspace.map(|item| item.ahead).unwrap_or(branch.ahead),
        behind: workspace.map(|item| item.behind).unwrap_or(branch.behind),
        upstream: workspace.and_then(|item| item.upstream.clone()),
        operation: workspace.and_then(|item| item.operation.clone()),
        activity_state: workspace.map(|item| item.activity.state.clone()),
        activity_confidence: workspace.map(|item| item.activity.confidence.clone()),
        merge_preview: include_merge_preview.then(|| target).flatten().and_then(|target| {
            let merge_path = workspace
                .map(|item| Path::new(item.path.as_str()))
                .unwrap_or_else(|| Path::new(&repository.path));
            agent_fold_merge_preview(merge_path, &branch.name, target)
        }),
        decision,
        reason,
        next_safe_step: next_safe_step.to_string(),
        authorization: "Preview only; Git, provider, branch, worktree, merge, rebase, push, and delete mutations require explicit authorization.".to_string(),
    }
}

fn agent_fold_preview_report_with_merge_preview(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    requested_target: Option<&str>,
    scope: &str,
    limit: usize,
    include_merge_preview: bool,
) -> Result<AgentFoldPreview, String> {
    agent_fold_preview_report_with_cursor_and_merge_preview(
        snapshot,
        query,
        requested_target,
        scope,
        limit,
        None,
        include_merge_preview,
    )
}
