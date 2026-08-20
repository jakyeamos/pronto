fn agent_fold_preview_report_with_cursor_and_merge_preview(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    requested_target: Option<&str>,
    scope: &str,
    limit: usize,
    cursor: Option<&str>,
    include_merge_preview: bool,
) -> Result<AgentFoldPreview, String> {
    if limit == 0 {
        return Err("fold preview --limit must be greater than zero".to_string());
    }
    let repositories = if let Some(query) = query {
        vec![find_cli_repository(snapshot, query)?]
    } else {
        snapshot.repositories.iter().collect::<Vec<_>>()
    };
    let branch_total = repositories
        .iter()
        .map(|repository| repository.branches.len())
        .sum();
    let mut candidates = Vec::new();
    for repository in &repositories {
        let (target, target_source, target_confidence) =
            agent_fold_target(repository, requested_target);
        for branch in repository.branches.iter().filter(|branch| {
            target
                .as_deref()
                .map_or(true, |target| branch.name != target)
        }) {
            let mut candidate_branch = branch.clone();
            let explicit_local_target = requested_target
                .and_then(|_| target.as_deref())
                .filter(|target| repository.branches.iter().any(|item| &item.name == target));
            if let Some(target) = explicit_local_target {
                let workspace = branch.workspace_id.as_deref().and_then(|workspace_id| {
                    repository
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.id == workspace_id)
                });
                candidate_branch.target_branch = Some(target.to_string());
                candidate_branch.target_confidence = "Explicit".to_string();
                candidate_branch.integration_state = branch_integration_state(
                    Path::new(&repository.path),
                    &branch.name,
                    Some(target),
                    workspace,
                );
            }
            candidates.push(agent_fold_candidate(
                repository,
                &candidate_branch,
                target.as_deref(),
                &target_source,
                &target_confidence,
                false,
            ));
        }
    }
    candidates.sort_by(|left, right| {
        agent_fold_decision_priority(&left.decision)
            .cmp(&agent_fold_decision_priority(&right.decision))
            .then_with(|| left.repository_name.cmp(&right.repository_name))
            .then_with(|| left.source_branch.cmp(&right.source_branch))
    });
    let candidate_total = candidates.len();
    let cursor_fingerprint = agent_fold_cursor_fingerprint(
        scope,
        requested_target,
        repositories.len(),
        branch_total,
        &candidates,
    );
    let start = cursor
        .map(|value| agent_fold_cursor_offset(value, &cursor_fingerprint, candidate_total))
        .transpose()?
        .unwrap_or(0);
    let end = start.saturating_add(limit).min(candidate_total);
    let mut page_candidates = candidates.drain(start..end).collect::<Vec<_>>();
    if include_merge_preview {
        for candidate in &mut page_candidates {
            let merge_preview = agent_fold_candidate_merge_preview(candidate);
            candidate.merge_preview = merge_preview;
        }
    }
    let has_more = end < candidate_total;
    let next_cursor =
        has_more.then(|| agent_fold_cursor(start + page_candidates.len(), &cursor_fingerprint));
    Ok(AgentFoldPreview {
        schema_version: AGENT_FOLD_PREVIEW_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        repository_count: repositories.len(),
        branch_total,
        candidate_total,
        returned_count: page_candidates.len(),
        has_more,
        next_cursor,
        candidates: page_candidates,
        live_verification_required: true,
        authorization: "Inspection only; use fold-feature-branches for live ref classification, reviewed integration, and any authorized pruning.".to_string(),
    })
}

fn agent_fold_candidate_merge_preview(
    candidate: &AgentFoldCandidate,
) -> Option<AgentFoldMergePreview> {
    let target = candidate.target_branch.as_deref()?;
    let merge_path = candidate
        .workspace_path
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new(&candidate.repository_path));
    agent_fold_merge_preview(merge_path, &candidate.source_branch, target)
}

fn agent_fold_cursor_fingerprint(
    scope: &str,
    requested_target: Option<&str>,
    repository_count: usize,
    branch_total: usize,
    candidates: &[AgentFoldCandidate],
) -> String {
    let payload = serde_json::to_vec(&(
        scope,
        requested_target,
        repository_count,
        branch_total,
        candidates,
    ))
    .expect("fold preview candidates should be serializable");
    let digest = Sha256::digest(payload);
    format!("{digest:x}")
}

fn agent_fold_cursor(offset: usize, fingerprint: &str) -> String {
    format!("{AGENT_FOLD_CURSOR_VERSION}:{offset}:{fingerprint}")
}

fn agent_fold_cursor_offset(
    cursor: &str,
    expected_fingerprint: &str,
    candidate_total: usize,
) -> Result<usize, String> {
    let mut parts = cursor.split(':');
    let version = parts.next();
    let offset = parts.next();
    let fingerprint = parts.next();
    if version != Some(AGENT_FOLD_CURSOR_VERSION)
        || offset.is_none()
        || fingerprint.is_none()
        || parts.next().is_some()
    {
        return Err(
            "invalid fold preview cursor; use the next_cursor returned by a current preview"
                .to_string(),
        );
    }
    let offset = offset
        .expect("offset was checked above")
        .parse::<usize>()
        .map_err(|_| {
            "invalid fold preview cursor offset; use the next_cursor returned by a current preview"
                .to_string()
        })?;
    if fingerprint != Some(expected_fingerprint) {
        return Err(
            "fold preview cursor does not match this snapshot or scope; restart pagination"
                .to_string(),
        );
    }
    if offset > candidate_total {
        return Err("fold preview cursor is past the end of the candidate list".to_string());
    }
    Ok(offset)
}

fn agent_doctor_check(
    id: &str,
    status: &str,
    summary: String,
    evidence: Vec<String>,
    next_safe_step: String,
) -> AgentDoctorCheck {
    AgentDoctorCheck {
        id: id.to_string(),
        status: status.to_string(),
        summary,
        evidence,
        next_safe_step,
    }
}

fn agent_doctor_relevant_roots<'a>(
    snapshot: &'a PortfolioSnapshot,
    scope: &str,
) -> Vec<&'a RootConfig> {
    if scope == "fleet" {
        return snapshot.roots.iter().collect();
    }
    snapshot
        .roots
        .iter()
        .filter(|root| {
            snapshot.repositories.iter().any(|repository| {
                Path::new(&repository.path).starts_with(Path::new(&root.path))
                    || repository.workspaces.iter().any(|workspace| {
                        Path::new(&workspace.path).starts_with(Path::new(&root.path))
                    })
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MissingWorkspaceClassification {
    Blocked(String),
    Warning(String),
}

fn workspace_owner(workspace: &WorkspaceSummary) -> String {
    workspace
        .provenance
        .owner
        .clone()
        .or_else(|| {
            workspace
                .activity
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.task_id.clone())
        })
        .unwrap_or_else(|| "unknown owner".to_string())
}

fn workspace_lease_is_complete(workspace: &WorkspaceSummary) -> bool {
    workspace.provenance.lease.as_deref().is_some_and(|lease| {
        matches!(
            lease.trim().to_ascii_lowercase().as_str(),
            "complete" | "completed" | "closed" | "released"
        )
    })
}

fn classify_missing_workspace(
    repository_path: &Path,
    workspace: &WorkspaceSummary,
) -> MissingWorkspaceClassification {
    let path = workspace.path.clone();
    let owner = workspace_owner(workspace);
    let preservation_evidence = workspace
        .provenance
        .preservation_evidence
        .clone()
        .unwrap_or_else(|| "No preservation evidence was recorded.".to_string());
    let blocked = |reason: String| {
        MissingWorkspaceClassification::Blocked(format!(
            "Workspace '{path}' is unavailable: {reason} Owner: {owner}. Preservation action: retain the record and inspect ownership before any scoped refresh. {preservation_evidence}"
        ))
    };

    if workspace.provenance.kind != "temporary" {
        return blocked(format!(
            "recorded provenance is '{}', not a completed temporary workspace cleanup",
            workspace.provenance.kind
        ));
    }
    if !workspace.status_available {
        return blocked(
            "the last Git status was unknown, so cleanliness cannot be established".to_string(),
        );
    }
    if workspace.dirty {
        return blocked(
            "the last Git status was dirty; the legacy snapshot does not contain an exact dirty-file list"
                .to_string(),
        );
    }
    if let Some(operation) = workspace.operation.as_ref() {
        return blocked(format!(
            "the last snapshot recorded an interrupted operation: {operation}"
        ));
    }
    if owner == "unknown owner" {
        return blocked("the workspace owner is unknown".to_string());
    }
    if !workspace_lease_is_complete(workspace) {
        return blocked(format!(
            "the lease is not recorded as completed (observed {:?})",
            workspace.provenance.lease
        ));
    }

    match live_worktree_contains(repository_path, Path::new(&path)) {
        Some(false) => {}
        Some(true) => {
            return blocked(
                "live Git still registers the path, so the persisted record cannot be classified as stale"
                    .to_string(),
            )
        }
        None => {
            return blocked(
                "live Git worktree metadata could not be read, so absence cannot be proven"
                    .to_string(),
            )
        }
    }

    let Some(head) = workspace
        .provenance
        .head
        .as_deref()
        .or(workspace.last_commit.as_deref())
    else {
        return blocked("the workspace HEAD was not recorded".to_string());
    };
    match git_head_reachable(repository_path, head) {
        Some(true) => {}
        Some(false) => {
            return blocked(format!(
                "recorded HEAD {head} is not reachable from the repository's live refs"
            ))
        }
        None => {
            return blocked(format!(
                "reachability for recorded HEAD {head} could not be verified"
            ))
        }
    }

    MissingWorkspaceClassification::Warning(format!(
        "Temporary workspace '{path}' was last clean, is absent from live Git worktree metadata, and recorded HEAD {head} remains reachable. Owner: {owner}; lease: {}. The stale record is retained because route is read-only; run scoped `pronto refresh '{}' --json` to reconstruct workspaces from live Git metadata.",
        workspace
            .provenance
            .lease
            .as_deref()
            .unwrap_or("completed"),
        repository_path.display()
    ))
}
