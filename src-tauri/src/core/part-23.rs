fn prepare_release_recipe(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    release: &ReleasePreparation,
) -> ReleaseRecipePreview {
    let recipe = repository
        .release_recipe
        .clone()
        .unwrap_or_else(default_release_recipe);
    let starting_state_ready = workspace.status_available
        && !workspace.dirty
        && workspace.operation.is_none()
        && workspace.activity.state != "Active";
    let release_evidence_ready = release.status == "Evidence ready";
    let version_confirmed = release.version_status == "Candidate version confirmed";
    let has_release_changes =
        !recipe.release_commands.is_empty() || !recipe.generated_paths.is_empty();
    let has_validation = !recipe.validation_commands.is_empty();
    let mut reasons = release.reasons.clone();
    if !starting_state_ready {
        if !workspace.status_available {
            reasons.push(workspace_status_unavailable_reason(workspace));
        } else if workspace.dirty {
            reasons.push("Workspace has uncommitted changes".to_string());
        }
        if let Some(operation) = workspace.operation.as_ref() {
            reasons.push(format!("Git operation is active: {operation}"));
        }
        if workspace.activity.state == "Active" {
            reasons.push("Associated agent or process activity is active".to_string());
        }
    }
    if !release_evidence_ready {
        reasons.push(format!("Release evidence is not ready: {}", release.status));
    }
    if release.candidate_version.is_none() {
        reasons.push(release.version_status.clone());
    } else if !version_confirmed {
        reasons.push(release.version_status.clone());
    }
    if !has_release_changes {
        reasons.push(
            "Release recipe has no release commands or generated paths configured".to_string(),
        );
    }
    if !has_validation {
        reasons.push("Release recipe has no validation commands configured".to_string());
    }
    reasons.push(
        "Preview only; no worktree, script, commit, push, pull request, or release publication is performed."
            .to_string(),
    );
    reasons.dedup();

    let mut steps = Vec::new();
    steps.push(ReleaseRecipeStep {
        order: 1,
        label: "Starting-state check".to_string(),
        status: if starting_state_ready {
            "Passed".to_string()
        } else {
            "Blocked".to_string()
        },
        detail: if starting_state_ready {
            format!(
                "Workspace is clean; operation is clear; activity state is {}.",
                workspace.activity.state
            )
        } else {
            "Release preparation cannot start until the workspace is safe to isolate.".to_string()
        },
    });
    steps.push(ReleaseRecipeStep {
        order: 2,
        label: "Create clean isolated release worktree".to_string(),
        status: "Deferred".to_string(),
        detail: "Preview only; no worktree was created.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 3,
        label: "Confirm candidate version".to_string(),
        status: if version_confirmed {
            "Passed".to_string()
        } else {
            "Blocked".to_string()
        },
        detail: release.version_status.clone(),
    });
    steps.push(ReleaseRecipeStep {
        order: 4,
        label: "Apply configured release changes".to_string(),
        status: if has_release_changes {
            "Configured".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: format!(
            "{} release command(s), {} generated path(s); commit message template: {}",
            recipe.release_commands.len(),
            recipe.generated_paths.len(),
            recipe.commit_message
        ),
    });
    steps.push(ReleaseRecipeStep {
        order: 5,
        label: "Run configured validation".to_string(),
        status: if has_validation {
            "Configured".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: if has_validation {
            format!(
                "{} validation command(s) would be reviewed before execution.",
                recipe.validation_commands.len()
            )
        } else {
            "Add at least one validation command before release preparation.".to_string()
        },
    });
    let blocked = !starting_state_ready
        || !release_evidence_ready
        || release.candidate_version.is_none()
        || !version_confirmed;
    steps.push(ReleaseRecipeStep {
        order: 6,
        label: "Review exact generated diff".to_string(),
        status: if blocked {
            "Blocked".to_string()
        } else if has_release_changes && has_validation {
            "Pending".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: "A user must inspect the exact generated files before any commit.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 7,
        label: "Commit generated files only".to_string(),
        status: "Deferred".to_string(),
        detail: "No commit is created by this preview.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 8,
        label: "Push and open pull request".to_string(),
        status: "Deferred".to_string(),
        detail: "Provider mutation remains outside this local preview.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 9,
        label: "Prepare draft GitHub Release".to_string(),
        status: "Deferred".to_string(),
        detail: "Release publication is not enabled by this preview.".to_string(),
    });

    let status = if blocked {
        "Blocked"
    } else if !has_release_changes || !has_validation {
        "Needs configuration"
    } else {
        "Ready for user review"
    };
    ReleaseRecipePreview {
        repository_id: repository.id.clone(),
        recipe_name: recipe.name,
        candidate_version: release.candidate_version.clone(),
        version_status: release.version_status.clone(),
        status: status.to_string(),
        reasons,
        steps,
        actions_performed: false,
        generated_at: iso_now(),
    }
}

fn prepare_repository_from_state(
    state: &StoreState,
    repository_id: &str,
    workspace_id: Option<&str>,
) -> Result<RepositoryPreparation, String> {
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    if !Path::new(&workspace.path).is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }
    let provider_available =
        provider_context_available(repository, state.provider_status.state == "Ready");
    let pull_request = prepare_pull_request(repository, workspace, provider_available);
    let release = prepare_release(repository, workspace, provider_available);
    let recipe = prepare_release_recipe(repository, workspace, &release);
    Ok(RepositoryPreparation {
        repository_id: repository.id.clone(),
        pull_request,
        release,
        recipe,
        generated_at: iso_now(),
    })
}

fn preparation_state(path: &Path, fresh_quality: bool) -> Result<StoreState, String> {
    if fresh_quality {
        load_store_read_only_with_quality_bounded(path)
    } else {
        load_store_read_only(path)
    }
}

fn prepare_repository_at_with_quality(
    path: &Path,
    repository_id: &str,
    workspace_id: Option<&str>,
    fresh_quality: bool,
) -> Result<RepositoryPreparation, String> {
    let state = preparation_state(path, fresh_quality)?;
    prepare_repository_from_state(&state, repository_id, workspace_id)
}

fn prepare_repository_by_query_at(
    path: &Path,
    query: &str,
    workspace_id: Option<&str>,
    fresh_quality: bool,
) -> Result<RepositoryPreparation, String> {
    let state = preparation_state(path, fresh_quality)?;
    let snapshot = snapshot_from_store(path, &state);
    let repository_id = find_cli_repository(&snapshot, query)?.id.clone();
    prepare_repository_from_state(&state, &repository_id, workspace_id)
}

fn prepare_repository_at(
    path: &Path,
    repository_id: &str,
    workspace_id: Option<&str>,
) -> Result<RepositoryPreparation, String> {
    prepare_repository_at_with_quality(path, repository_id, workspace_id, false)
}

fn remediation_handoff_check_for_repository(
    repository: &RepositorySnapshot,
    workspace_id: Option<&str>,
) -> Result<RemediationHandoffCheck, String> {
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    let workspace_path = Path::new(&workspace.path);
    if !workspace_path.is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }
    let ownership_status = remediation::workspace_activity_execution_state(&workspace.activity);
    let ownership_coordination_required = ownership_status == "coordination_required";
    let ownership_evidence_unavailable = ownership_status == "evidence_unavailable";

    let generated_at = iso_now();
    let live_status = run_git(
        workspace_path,
        [
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ]
        .iter(),
    )
    .and_then(parse_git_status);
    let live_head_commit = git_static(workspace_path, &["rev-parse", "HEAD"]);
    let live_operation = live_status
        .as_ref()
        .ok()
        .and_then(|_| interrupted_operation(workspace_path));
    let (
        status,
        ready,
        checkpoint_required,
        workspace_dirty,
        branch,
        operation,
        next_safe_step,
        reasons,
    ) = match live_status {
        Ok(live_status) => {
            let operation = live_operation.or_else(|| workspace.operation.clone());
            let mut reasons = Vec::new();
            if ownership_coordination_required {
                reasons.push(
                        "The persisted activity evidence reports an active workspace owner; coordinate with that owner before changing this workspace."
                            .to_string(),
                    );
            }
            if ownership_evidence_unavailable {
                reasons.push(
                    "Workspace ownership could not be established because activity inspection evidence is unavailable; restore that evidence before changing this workspace."
                        .to_string(),
                );
            }
            if live_status.dirty {
                reasons.push(
                        "The workspace contains uncommitted changes; create a local checkpoint commit before handoff."
                            .to_string(),
                    );
            }
            if let Some(operation) = operation.as_ref() {
                reasons.push(format!(
                        "The workspace has an interrupted Git operation ({operation}) that must be resolved before handoff."
                    ));
            }
            if !live_status.dirty && workspace.dirty {
                reasons.push(
                        "Live Git is clean, but the persisted Pronto snapshot still reports dirty work; run a scoped refresh before handoff."
                            .to_string(),
                    );
            }
            let ready = ownership_status == "clear"
                && !live_status.dirty
                && operation.is_none()
                && !workspace.dirty;
            let next_safe_step = if ownership_coordination_required {
                "Coordinate workspace ownership, refresh the repository activity evidence, then rerun `pronto remediation handoff-check`."
            } else if ownership_evidence_unavailable {
                "Restore workspace activity inspection, refresh the repository, then rerun `pronto remediation handoff-check`."
            } else if live_status.dirty {
                "Review ownership, commit the intended changes on this branch, then rerun `pronto remediation handoff-check`."
            } else if operation.is_some() {
                "Resolve the interrupted Git operation without discarding unrelated work, then rerun `pronto remediation handoff-check`."
            } else if workspace.dirty {
                "Run the repository-scoped `pronto refresh` after the checkpoint commit, then rerun `pronto remediation handoff-check`."
            } else {
                "Proceed with the scoped remediation handoff; this check performed no repository mutation."
            };
            (
                if ready { "ready" } else { "blocked" },
                ready,
                live_status.dirty || operation.is_some(),
                live_status.dirty,
                live_status.branch,
                operation,
                next_safe_step.to_string(),
                reasons,
            )
        }
        Err(status_error) => {
            let mut reasons = Vec::new();
            if ownership_coordination_required {
                reasons.push(
                        "The persisted activity evidence reports an active workspace owner; coordinate with that owner before changing this workspace."
                            .to_string(),
                    );
            }
            if ownership_evidence_unavailable {
                reasons.push(
                    "Workspace ownership could not be established because activity inspection evidence is unavailable; restore that evidence before changing this workspace."
                        .to_string(),
                );
            }
            reasons.push(format!(
                    "Live Git status could not be established: {status_error} Remediation advancement is blocked until the workspace can be checked."
                ));
            (
                    "unknown",
                    false,
                    true,
                    false,
                    workspace.branch.clone(),
                    workspace.operation.clone(),
                    "Restore live Git access, then rerun `pronto remediation handoff-check`; no repository mutation was attempted."
                        .to_string(),
                    reasons,
                )
        }
    };

    Ok(RemediationHandoffCheck {
        schema_version: REMEDIATION_HANDOFF_SCHEMA.to_string(),
        generated_at,
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        workspace_id: workspace.id.clone(),
        workspace_path: workspace.path.clone(),
        branch,
        head_commit: live_head_commit.or_else(|| workspace.last_commit.clone()),
        status: status.to_string(),
        ready,
        status_available: status != "unknown",
        ownership_status: ownership_status.to_string(),
        ownership_coordination_required,
        checkpoint_required,
        workspace_dirty,
        persisted_snapshot_dirty: workspace.dirty,
        operation,
        reasons,
        next_safe_step,
        authorization: "Read-only live Git check; no commit, stash, merge, rebase, push, or file edit was performed."
            .to_string(),
    })
}

fn remediation_handoff_check_at(
    path: &Path,
    query: &str,
    workspace_id: Option<&str>,
) -> Result<RemediationHandoffCheck, String> {
    let state = load_store_read_only(path)?;
    let snapshot = snapshot_from_store(path, &state);
    let repository = find_cli_repository(&snapshot, query)?;
    remediation_handoff_check_for_repository(repository, workspace_id)
}
