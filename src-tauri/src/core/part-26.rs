fn scan_repository(
    path: &Path,
    existing: Option<&RepositorySnapshot>,
    expected: &[ExpectedCondition],
) -> RepositorySnapshot {
    let observed_at = iso_now();
    let repository_id = path_id("repository", path);
    let remote_url = git_static(path, &["remote", "get-url", "origin"]);
    let remote_unchanged = existing.is_some_and(|repository| repository.remote_url == remote_url);
    let provider_state = existing
        .filter(|_| remote_unchanged)
        .map(|repository| repository.provider_state.clone())
        .unwrap_or_else(|| {
            if remote_url
                .as_ref()
                .is_some_and(|url| url.contains("github.com"))
            {
                "GitHub remote detected · provider not connected".to_string()
            } else if remote_url.is_some() {
                "Remote detected · provider not connected".to_string()
            } else {
                "No remote configured".to_string()
            }
        });
    let locality = existing
        .filter(|_| remote_unchanged)
        .map(|repository| repository.locality.clone())
        .unwrap_or_else(|| {
            if remote_url.is_some() {
                "Connected".to_string()
            } else {
                "Local only".to_string()
            }
        });
    let worktree_records = parse_worktrees(path);
    let provisional_branch = git_static(path, &["branch", "--show-current"])
        .unwrap_or_else(|| "Detached HEAD".to_string());
    let default_branch = detect_default_branch(path, &provisional_branch);
    let configured_target_branch = existing
        .filter(|repository| repository.target_branch_configured)
        .and_then(|repository| repository.target_branch.clone());
    let target_branch = configured_target_branch
        .clone()
        .or_else(|| default_branch.clone());
    let target_confidence = if configured_target_branch.is_some() {
        "High"
    } else {
        "Medium"
    };
    let recorded_fetch = existing
        .filter(|_| remote_unchanged)
        .and_then(|repository| repository.last_fetch_at.clone());
    let observed_fetch = remote_url.as_ref().and_then(|_| observed_fetch_at(path));
    let existing_last_fetch = [recorded_fetch, observed_fetch].into_iter().flatten().max();
    let mut workspaces = Vec::new();
    for record in worktree_records {
        let canonical = canonical_path(&record.path).unwrap_or(record.path.clone());
        let is_primary = canonical == canonical_path(path).unwrap_or_else(|| path.to_path_buf());
        let workspace_existing = existing.and_then(|repository| {
            repository
                .workspaces
                .iter()
                .find(|workspace| workspace.path == canonical.to_string_lossy())
        });
        let mut workspace = scan_workspace(
            &canonical,
            is_primary,
            default_branch.as_deref(),
            target_branch.as_deref(),
            target_confidence,
            existing,
        );
        if existing_last_fetch.is_some() {
            workspace.remote_freshness = existing_last_fetch
                .clone()
                .unwrap_or_else(|| "Not fetched by Pronto".to_string());
        }
        if let Some(existing_workspace) = workspace_existing {
            if workspace.last_activity_at.is_none() {
                workspace.last_activity_at = existing_workspace.last_activity_at.clone();
            }
            merge_workspace_provenance(&mut workspace.provenance, &existing_workspace.provenance);
        }
        workspaces.push(workspace);
    }
    if workspaces.is_empty() {
        workspaces.push(scan_workspace(
            path,
            true,
            default_branch.as_deref(),
            target_branch.as_deref(),
            target_confidence,
            existing,
        ));
    }
    let primary_index = workspaces
        .iter()
        .position(|workspace| workspace.is_primary)
        .unwrap_or(0);
    let primary = workspaces[primary_index].clone();
    let branch_records = parse_branches(path);
    let mut branches = Vec::new();
    for record in branch_records {
        let current_workspace = workspaces
            .iter()
            .find(|workspace| workspace.branch == record.name);
        let (role, role_confidence) = current_workspace
            .map(|workspace| (workspace.role.clone(), workspace.role_confidence.clone()))
            .unwrap_or_else(|| branch_role(&record.name, default_branch.as_deref()));
        let (branch_target, branch_target_confidence) = current_workspace
            .map(|workspace| {
                (
                    workspace.target_branch.clone(),
                    workspace.target_confidence.clone(),
                )
            })
            .unwrap_or_else(|| {
                let (branch_target, mut confidence) =
                    target_for_branch(&record.name, target_branch.as_deref());
                if branch_target.is_some() {
                    confidence = target_confidence.to_string();
                }
                (branch_target, confidence)
            });
        let integration_state = if primary.status_available {
            branch_integration_state(
                path,
                &record.name,
                target_branch.as_deref(),
                current_workspace,
            )
        } else {
            "Unknown".to_string()
        };
        let ahead = current_workspace
            .map(|workspace| workspace.ahead)
            .unwrap_or(0);
        let behind = current_workspace
            .map(|workspace| workspace.behind)
            .unwrap_or(0);
        let workspace_id = current_workspace.map(|workspace| workspace.id.clone());
        branches.push(BranchSummary {
            name: record.name,
            role,
            role_confidence,
            target_branch: branch_target,
            target_confidence: branch_target_confidence,
            ahead,
            behind,
            integration_state,
            workspace_id,
            last_commit: record.last_commit,
            last_commit_at: record.last_commit_at,
        });
    }
    let mut primary_for_conditions = primary.clone();
    if primary.status_available {
        primary_for_conditions.integration_state = branch_integration_state(
            path,
            &primary.branch,
            target_branch.as_deref(),
            Some(&primary),
        );
    }
    let conditions = build_conditions(
        &repository_id,
        &primary_for_conditions,
        target_branch.as_deref(),
        expected,
        &observed_at,
    );
    let submodules = parse_submodules(path);
    let custody = crate::custody::project(path).unwrap_or_default();
    let lifecycle_candidate = if primary.last_activity_at.as_ref().is_some_and(|value| {
        DateTime::parse_from_rfc3339(value)
            .map(|date| {
                Utc::now()
                    .signed_duration_since(date.with_timezone(&Utc))
                    .num_days()
                    < 90
            })
            .unwrap_or(false)
    }) {
        "Active"
    } else {
        "Maintenance"
    };
    RepositorySnapshot {
        id: repository_id,
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unnamed repository")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        locality: locality.to_string(),
        lifecycle: existing
            .map(|repository| repository.lifecycle.clone())
            .unwrap_or_else(|| "Unconfirmed".to_string()),
        lifecycle_candidate: lifecycle_candidate.to_string(),
        remote_url,
        provider_state,
        branch: primary.branch.clone(),
        default_branch,
        target_branch,
        target_branch_configured: configured_target_branch.is_some(),
        workspace: primary,
        workspaces,
        branches,
        submodules,
        pull_requests: existing
            .map(|repository| repository.pull_requests.clone())
            .unwrap_or_default(),
        releases: existing
            .map(|repository| repository.releases.clone())
            .unwrap_or_default(),
        quality: existing
            .map(|repository| repository.quality.clone())
            .unwrap_or_default(),
        project_compass: project_compass::inspect(path),
        custody,
        release_rule: existing.and_then(|repository| repository.release_rule.clone()),
        release_recipe: existing.and_then(|repository| repository.release_recipe.clone()),
        confirmed_release_version: existing
            .and_then(|repository| repository.confirmed_release_version.clone()),
        ai_permission: existing
            .map(|repository| repository.ai_permission.clone())
            .unwrap_or_else(default_ai_permission),
        conditions,
        last_scan_at: observed_at,
        last_fetch_at: existing_last_fetch,
        last_activity_at: primary_for_conditions.last_activity_at.clone(),
    }
}

fn transition_fingerprint(repository: &RepositorySnapshot) -> String {
    let condition_state = repository
        .conditions
        .iter()
        .map(|condition| format!("{}:{}", condition.id, condition.fingerprint))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        repository.branch,
        repository.target_branch.as_deref().unwrap_or_default(),
        repository.target_branch_configured,
        repository.workspace.dirty,
        repository.workspace.added,
        repository.workspace.removed,
        repository.workspace.sync_state,
        repository.workspace.activity.state,
        condition_state
    )
}

fn event_summary(repository: &RepositorySnapshot) -> String {
    if repository.conditions.is_empty() {
        format!("{} has no active conditions", repository.name)
    } else {
        format!(
            "{} · {}",
            repository.name,
            repository
                .conditions
                .iter()
                .map(|condition| condition.title.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        )
    }
}

fn append_transition_event(
    state: &mut StoreState,
    old: Option<&RepositorySnapshot>,
    new: &RepositorySnapshot,
) {
    let new_fingerprint = transition_fingerprint(new);
    let changed = old
        .map(|repository| transition_fingerprint(repository) != new_fingerprint)
        .unwrap_or(true);
    if !changed {
        return;
    }
    let sequence = NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed);
    state.events.push(EventRecord {
        id: format!("event:{}:{}:{}", new.id, new.last_scan_at, sequence),
        repository_id: new.id.clone(),
        kind: if old.is_some() {
            "state-transition".to_string()
        } else {
            "repository-discovered".to_string()
        },
        summary: event_summary(new),
        fingerprint: new_fingerprint,
        created_at: new.last_scan_at.clone(),
    });
}

fn prune_events(state: &mut StoreState) {
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    state.events.retain(|event| {
        DateTime::parse_from_rfc3339(&event.created_at)
            .map(|date| date.with_timezone(&Utc) >= cutoff)
            .unwrap_or(true)
    });
    if state.events.len() > 2_000 {
        let keep_from = state.events.len() - 2_000;
        state.events = state.events.split_off(keep_from);
    }
}

fn prune_action_audits(state: &mut StoreState) {
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    state.action_audits.retain(|audit| {
        DateTime::parse_from_rfc3339(&audit.created_at)
            .map(|date| date.with_timezone(&Utc) >= cutoff)
            .unwrap_or(true)
    });
    if state.action_audits.len() > 2_000 {
        let keep_from = state.action_audits.len() - 2_000;
        state.action_audits = state.action_audits.split_off(keep_from);
    }
}

fn action_audit_id(action: &str, created_at: &str) -> String {
    let sequence = NEXT_ACTION_AUDIT_ID.fetch_add(1, Ordering::Relaxed);
    format!("audit:{action}:{created_at}:{sequence}")
}

fn action_targets(
    state: &StoreState,
    action: &str,
    repository_id: Option<&str>,
) -> Result<(Vec<String>, String), String> {
    match action {
        "refresh" => {
            if repository_id.is_some() {
                return Err(
                    "Refresh preflight targets all registered discovery roots; omit repository_id"
                        .to_string(),
                );
            }
            let target_ids = state
                .roots
                .iter()
                .map(|root| root.id.clone())
                .collect::<Vec<_>>();
            let target_label = if target_ids.is_empty() {
                "No registered discovery roots".to_string()
            } else {
                "All registered discovery roots".to_string()
            };
            Ok((target_ids, target_label))
        }
        "inspect" => {
            if let Some(repository_id) = repository_id {
                let repository = state
                    .repositories
                    .iter()
                    .find(|repository| repository.id == repository_id)
                    .ok_or_else(|| "Repository is not registered".to_string())?;
                return Ok((
                    vec![repository.id.clone()],
                    format!("Repository {}", repository.name),
                ));
            }
            let target_ids = state
                .repositories
                .iter()
                .map(|repository| repository.id.clone())
                .collect::<Vec<_>>();
            let target_label = if target_ids.is_empty() {
                "No scanned repositories".to_string()
            } else {
                "All scanned repositories".to_string()
            };
            Ok((target_ids, target_label))
        }
        _ => Ok((Vec::new(), "No target selected".to_string())),
    }
}

fn build_action_preflight(
    state: &StoreState,
    action: &str,
    repository_id: Option<&str>,
) -> Result<ActionPreflight, String> {
    let normalized_action = action.trim().to_ascii_lowercase();
    let allowed = matches!(normalized_action.as_str(), "refresh" | "inspect");
    let (target_ids, target_label) = if allowed {
        action_targets(state, &normalized_action, repository_id)?
    } else {
        action_targets(state, "unsupported", None)?
    };
    let created_at = iso_now();
    let risk = if allowed { "read-only" } else { "blocked" }.to_string();
    let status = if allowed { "Preflighted" } else { "Rejected" }.to_string();
    let summary = if allowed {
        format!("Read-only {normalized_action} preflight for {target_label}.")
    } else {
        format!(
            "Action '{normalized_action}' is not enabled; Git mutation and provider writes remain blocked."
        )
    };
    let audit = ActionAudit {
        id: action_audit_id(&normalized_action, &created_at),
        action: normalized_action,
        target_ids,
        risk,
        status,
        summary,
        created_at,
        completed_at: None,
    };
    Ok(ActionPreflight {
        audit,
        allowed,
        target_label,
    })
}

fn append_action_audit(state: &mut StoreState, preflight: &ActionPreflight) {
    state.action_audits.push(preflight.audit.clone());
    prune_action_audits(state);
}

fn update_action_audit(
    state: &mut StoreState,
    audit_id: &str,
    status: &str,
    summary: String,
) -> Result<(), String> {
    let audit = state
        .action_audits
        .iter_mut()
        .find(|audit| audit.id == audit_id)
        .ok_or_else(|| "Action audit record is no longer available".to_string())?;
    audit.status = status.to_string();
    audit.summary = summary;
    audit.completed_at = Some(iso_now());
    Ok(())
}

fn preflight_action_at(
    path: &Path,
    action: &str,
    repository_id: Option<&str>,
) -> Result<ActionPreflight, String> {
    let mut state = load_store(path)?;
    let preflight = build_action_preflight(&state, action, repository_id)?;
    append_action_audit(&mut state, &preflight);
    save_store(path, &state)?;
    Ok(preflight)
}
