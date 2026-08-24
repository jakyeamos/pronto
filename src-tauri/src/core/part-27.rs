fn audited_scan_and_persist(
    path: &Path,
    state: &mut StoreState,
) -> Result<PortfolioSnapshot, String> {
    audited_scan_and_persist_scoped(path, state, None, None)
}

fn build_targeted_refresh_preflight(
    state: &StoreState,
    target_repository_ids: &HashSet<String>,
    target_label: &str,
) -> Result<ActionPreflight, String> {
    if target_repository_ids.is_empty() {
        return Err(format!("{target_label} has no registered repositories"));
    }
    if let Some(unknown) = target_repository_ids.iter().find(|repository_id| {
        !state
            .repositories
            .iter()
            .any(|repository| &repository.id == *repository_id)
    }) {
        return Err(format!("Repository {unknown} is not registered"));
    }
    let created_at = iso_now();
    let target_ids = target_repository_ids.iter().cloned().collect::<Vec<_>>();
    let audit = ActionAudit {
        id: action_audit_id("refresh", &created_at),
        action: "refresh".to_string(),
        target_ids,
        risk: "read-only".to_string(),
        status: "Preflighted".to_string(),
        summary: format!("Read-only refresh preflight for {target_label}."),
        created_at,
        completed_at: None,
    };
    Ok(ActionPreflight {
        audit,
        allowed: true,
        target_label: target_label.to_string(),
    })
}

fn audited_scan_and_persist_scoped(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    target_label: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    audited_scan_and_persist_scoped_locked(path, state, target_repository_ids, target_label)
}

fn audited_scan_and_persist_scoped_locked(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    target_label: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    let preflight = match (target_repository_ids, target_label) {
        (Some(repository_ids), Some(label)) => {
            build_targeted_refresh_preflight(state, repository_ids, label)?
        }
        (None, None) => build_action_preflight(state, "refresh", None)?,
        _ => return Err("Refresh target metadata is incomplete".to_string()),
    };
    if !preflight.allowed {
        return Err("Local refresh action is not permitted".to_string());
    }
    let audit_id = preflight.audit.id.clone();
    append_action_audit(state, &preflight);
    save_store(path, state)?;

    match scan_and_persist_scoped(path, state, target_repository_ids) {
        Ok(_) => {
            update_action_audit(
                state,
                &audit_id,
                "Completed",
                format!(
                    "Read-only refresh completed for {}.",
                    preflight.target_label
                ),
            )?;
            prune_action_audits(state);
            save_store(path, state)?;
            Ok(snapshot_from_store(path, state))
        }
        Err(error) => {
            if update_action_audit(
                state,
                &audit_id,
                "Failed",
                format!("Read-only refresh failed for {}.", preflight.target_label),
            )
            .is_ok()
            {
                let _ = save_store(path, state);
            }
            Err(error)
        }
    }
}

fn build_repository_path_refresh_preflight(
    repository_id: &str,
    repository_path: &Path,
) -> ActionPreflight {
    let created_at = iso_now();
    let target_label = format!("Repository {}", repository_path.display());
    let audit = ActionAudit {
        id: action_audit_id("refresh", &created_at),
        action: "refresh".to_string(),
        target_ids: vec![repository_id.to_string()],
        risk: "read-only".to_string(),
        status: "Preflighted".to_string(),
        summary: format!(
            "Read-only refresh preflight for {target_label}; the repository path was not previously registered."
        ),
        created_at,
        completed_at: None,
    };
    ActionPreflight {
        audit,
        allowed: true,
        target_label,
    }
}

#[cfg(test)]
fn audited_scan_and_persist_repository_path(
    path: &Path,
    state: &mut StoreState,
    repository_path: &Path,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    audited_scan_and_persist_repository_path_locked(path, state, repository_path)
}

fn audited_scan_and_persist_repository_path_locked(
    path: &Path,
    state: &mut StoreState,
    repository_path: &Path,
) -> Result<PortfolioSnapshot, String> {
    let repository_path = canonical_repository_path(repository_path)
        .ok_or_else(|| "The refresh target is not an accessible Git repository".to_string())?;
    let repository_id = path_id("repository", &repository_path);
    let preflight = build_repository_path_refresh_preflight(&repository_id, &repository_path);
    if !preflight.allowed {
        return Err("Local refresh action is not permitted".to_string());
    }
    let audit_id = preflight.audit.id.clone();
    append_action_audit(state, &preflight);
    save_store(path, state)?;

    match scan_and_persist_repository_path(path, state, &repository_path) {
        Ok(_) => {
            update_action_audit(
                state,
                &audit_id,
                "Completed",
                format!(
                    "Read-only refresh completed for {}.",
                    preflight.target_label
                ),
            )?;
            prune_action_audits(state);
            save_store(path, state)?;
            Ok(snapshot_from_store(path, state))
        }
        Err(error) => {
            if update_action_audit(
                state,
                &audit_id,
                "Failed",
                format!("Read-only refresh failed for {}.", preflight.target_label),
            )
            .is_ok()
            {
                let _ = save_store(path, state);
            }
            Err(error)
        }
    }
}

fn refresh_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    with_store_write_state(path, |state| {
        audited_scan_and_persist_scoped_locked(path, state, None, None)
    })
}

fn refresh_target_at(path: &Path, target: &str) -> Result<PortfolioSnapshot, String> {
    with_store_write_state(path, |state| {
        let current = snapshot_from_store(path, state);
        match resolve_local_refresh_target(&current, state, target)? {
            LocalRefreshTarget::Registered {
                repository_ids,
                label,
            } => audited_scan_and_persist_scoped_locked(
                path,
                state,
                Some(&repository_ids),
                Some(&label),
            )
            .map(|snapshot| filter_snapshot_to_repository_ids(snapshot, &repository_ids)),
            LocalRefreshTarget::RepositoryPath(repository_path) => {
                let repository_id = path_id("repository", &repository_path);
                audited_scan_and_persist_repository_path_locked(path, state, &repository_path).map(
                    |snapshot| {
                        let repository_ids = [repository_id].into_iter().collect();
                        filter_snapshot_to_repository_ids(snapshot, &repository_ids)
                    },
                )
            }
        }
    })
}

fn scan_and_persist_scoped(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
) -> Result<PortfolioSnapshot, String> {
    let mut discovered = HashMap::<String, PathBuf>::new();
    for root in &state.roots {
        for repository in discover_repositories(root) {
            let repository_id = path_id("repository", &repository);
            if target_repository_ids
                .map(|targets| targets.contains(&repository_id))
                .unwrap_or(true)
            {
                discovered.insert(repository_id, repository);
            }
        }
    }
    scan_discovered_and_persist(path, state, target_repository_ids, discovered)
}

fn scan_and_persist_repository_path(
    path: &Path,
    state: &mut StoreState,
    repository_path: &Path,
) -> Result<PortfolioSnapshot, String> {
    let repository_id = path_id("repository", repository_path);
    let target_repository_ids = [repository_id.clone()].into_iter().collect::<HashSet<_>>();
    let discovered = [(repository_id, repository_path.to_path_buf())]
        .into_iter()
        .collect::<HashMap<_, _>>();
    scan_discovered_and_persist(path, state, Some(&target_repository_ids), discovered)
}

fn scan_discovered_and_persist(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    discovered: HashMap<String, PathBuf>,
) -> Result<PortfolioSnapshot, String> {
    let old_by_id = state
        .repositories
        .iter()
        .map(|repository| (repository.id.clone(), repository.clone()))
        .collect::<HashMap<_, _>>();
    let mut repositories = Vec::new();
    let mut discovered = discovered.into_iter().collect::<Vec<_>>();
    discovered.sort_by(|left, right| left.0.cmp(&right.0));
    for (id, repository_path) in discovered {
        let repository = scan_repository(
            &repository_path,
            old_by_id.get(&id),
            &state.expected_conditions,
        );
        repositories.push(repository);
    }
    for (id, old) in &old_by_id {
        if !repositories
            .iter()
            .any(|repository| repository.id.as_str() == id.as_str())
            && target_repository_ids
                .map(|targets| targets.contains(id))
                .unwrap_or(true)
        {
            if target_repository_ids.is_none()
                && repository_is_ignored_by_existing_root(state, &old)
            {
                continue;
            }
            if Path::new(&old.path).exists() && has_git_metadata(Path::new(&old.path)) {
                let repository_path = PathBuf::from(&old.path);
                let repository =
                    scan_repository(&repository_path, Some(old), &state.expected_conditions);
                repositories.push(repository);
            }
        } else if target_repository_ids.is_some_and(|targets| !targets.contains(id)) {
            repositories.push(old.clone());
        }
    }
    merge_scanned_and_persist(path, state, target_repository_ids, repositories)
}

fn merge_scanned_and_persist(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    scanned: Vec<RepositorySnapshot>,
) -> Result<PortfolioSnapshot, String> {
    let old_by_id = state
        .repositories
        .iter()
        .map(|repository| (repository.id.clone(), repository.clone()))
        .collect::<HashMap<_, _>>();
    let mut scanned_by_id = HashMap::<String, RepositorySnapshot>::new();
    for repository in scanned {
        scanned_by_id.insert(repository.id.clone(), repository);
    }
    let scanned_ids = scanned_by_id.keys().cloned().collect::<HashSet<_>>();
    for (id, old) in &old_by_id {
        if scanned_ids.contains(id) {
            continue;
        }
        if target_repository_ids.is_some_and(|targets| !targets.contains(id)) {
            scanned_by_id.insert(id.clone(), old.clone());
        }
    }
    let mut repositories = scanned_by_id.into_values().collect::<Vec<_>>();
    repositories.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    for repository in &repositories {
        append_transition_event(state, old_by_id.get(&repository.id), repository);
    }
    sort_repositories_by_name(&mut repositories);
    state.repositories = repositories;
    apply_quality_evidence_scoped(state, target_repository_ids, None);
    apply_release_threshold_conditions(state);
    prune_events(state);
    save_store(path, state)?;
    record_analytics_samples(path, state)?;
    Ok(snapshot_from_store(path, state))
}

#[derive(Debug, Clone)]
struct RefreshScanInput {
    repository_id: String,
    repository_path: PathBuf,
    existing: Option<RepositorySnapshot>,
    expected_conditions: Vec<ExpectedCondition>,
}

#[derive(Debug, Clone)]
struct RefreshBatchPlan {
    target_repository_ids: Option<HashSet<String>>,
    target_label: String,
    repository_path_target: Option<PathBuf>,
    inputs: Vec<RefreshScanInput>,
    revision: String,
}

fn refresh_batch_revision(state: &StoreState) -> String {
    let payload = serde_json::to_vec(state).unwrap_or_default();
    let digest = Sha256::digest(payload);
    format!("{digest:x}")
}

fn refresh_batch_plan(
    path: &Path,
    state: &StoreState,
    target: Option<&str>,
) -> Result<RefreshBatchPlan, String> {
    let snapshot = snapshot_from_store(path, state);
    let (target_repository_ids, target_label, repository_path_target) = match target {
        None => (None, "All registered repositories".to_string(), None),
        Some(query) => match resolve_local_refresh_target(&snapshot, state, query)? {
            LocalRefreshTarget::Registered {
                repository_ids,
                label,
            } => (Some(repository_ids), label, None),
            LocalRefreshTarget::RepositoryPath(repository_path) => {
                let repository_id = path_id("repository", &repository_path);
                (
                    Some([repository_id].into_iter().collect()),
                    format!("Repository {}", repository_path.display()),
                    Some(repository_path),
                )
            }
        },
    };

    let mut discovered = HashMap::<String, PathBuf>::new();
    if let Some(repository_path) = repository_path_target.as_ref() {
        discovered.insert(
            path_id("repository", repository_path),
            repository_path.clone(),
        );
    } else {
        for root in &state.roots {
            for repository_path in discover_repositories(root) {
                let repository_id = path_id("repository", &repository_path);
                if target_repository_ids
                    .as_ref()
                    .map(|targets| targets.contains(&repository_id))
                    .unwrap_or(true)
                {
                    discovered.insert(repository_id, repository_path);
                }
            }
        }
    }

    let existing_by_id = state
        .repositories
        .iter()
        .map(|repository| (repository.id.clone(), repository))
        .collect::<HashMap<_, _>>();
    for (repository_id, repository) in existing_by_id {
        let in_scope = target_repository_ids
            .as_ref()
            .map(|targets| targets.contains(&repository_id))
            .unwrap_or(true);
        if !in_scope || discovered.contains_key(&repository_id) {
            continue;
        }
        let repository_path = Path::new(&repository.path);
        if repository_path.exists()
            && has_git_metadata(repository_path)
            && (target_repository_ids.is_some()
                || !repository_is_ignored_by_existing_root(state, repository))
        {
            discovered.insert(repository_id, repository_path.to_path_buf());
        }
    }

    let mut discovered = discovered.into_iter().collect::<Vec<_>>();
    discovered.sort_by(|left, right| left.0.cmp(&right.0));
    let inputs = discovered
        .into_iter()
        .map(|(repository_id, repository_path)| RefreshScanInput {
            existing: state
                .repositories
                .iter()
                .find(|repository| repository.id == repository_id)
                .cloned(),
            repository_id,
            repository_path,
            expected_conditions: state.expected_conditions.clone(),
        })
        .collect();

    Ok(RefreshBatchPlan {
        target_repository_ids,
        target_label,
        repository_path_target,
        inputs,
        revision: refresh_batch_revision(state),
    })
}
