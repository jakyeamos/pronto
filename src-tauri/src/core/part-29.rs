fn refresh_repository_target_evidence_at_with_lock_timeout(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
    lock_timeout: StdDuration,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock_with_timeout(path, lock_timeout)?;
    let target_branch = target_branch.trim();
    if target_branch.is_empty() || target_branch.contains('\0') {
        return Err("Choose a valid target branch".to_string());
    }
    let mut state = load_store(path)?;
    let repository_index = state
        .repositories
        .iter()
        .position(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let repository = state.repositories[repository_index].clone();
    if !repository
        .branches
        .iter()
        .any(|branch| branch.name == target_branch)
    {
        return Err(format!(
            "Target branch '{target_branch}' is not a local branch in {}",
            repository.name
        ));
    }
    let repository_path = Path::new(&repository.path);
    let target_ref = format!("refs/heads/{target_branch}");
    let target_head = run_git(
        repository_path,
        vec![
            "rev-parse".to_string(),
            "--verify".to_string(),
            target_ref.clone(),
        ],
    )?;
    if !target_head.success || target_head.stdout.trim().is_empty() {
        return Err(format!(
            "Target branch '{target_branch}' could not be resolved in {}: {}",
            repository.name,
            concise_target_command_error(&target_head.stderr)
        ));
    }
    let target_head = target_head.stdout.trim().to_string();
    let reuse_target_evidence =
        target_evidence_is_reusable(&repository, target_branch, &target_head);
    let mut configured = repository.clone();
    configured.target_branch = Some(target_branch.to_string());
    configured.target_branch_configured = true;
    let mut rescanned = scan_repository(
        repository_path,
        Some(&configured),
        &state.expected_conditions,
    );
    if !reuse_target_evidence {
        rescanned.quality.target_fleet_audit_root = None;
    }
    append_transition_event(&mut state, Some(&repository), &rescanned);
    state.repositories[repository_index] = rescanned;

    if reuse_target_evidence {
        let target_ids = [repository_id.to_string()]
            .into_iter()
            .collect::<HashSet<_>>();
        apply_quality_evidence_scoped(&mut state, Some(&target_ids), None);
        apply_release_threshold_conditions(&mut state);
        if let Some(repository) = state.repositories.get_mut(repository_index) {
            let short_head = target_head.chars().take(8).collect::<String>();
            repository.quality.ingestion_message = Some(format!(
                "Reused target evidence for {target_branch} @ {short_head}; target head is unchanged."
            ));
        }
        save_store(path, &state)?;
        return Ok(snapshot_from_store(path, &state));
    }

    let run_id_prefix = target_evidence_run_prefix(&repository, target_branch);
    let target_parent = target_evidence_artifact_parent();
    fs::create_dir_all(&target_parent).map_err(|error| {
        format!(
            "Could not create Pronto target evidence workspace {}: {error}",
            target_parent.display()
        )
    })?;
    let target_worktree = target_parent.join(format!("{run_id_prefix}-worktree"));
    if target_worktree.exists() {
        return Err(format!(
            "Target evidence workspace already exists: {}",
            target_worktree.display()
        ));
    }
    let add_result = run_git(
        repository_path,
        vec![
            "worktree".to_string(),
            "add".to_string(),
            "--force".to_string(),
            "--quiet".to_string(),
            target_worktree.to_string_lossy().to_string(),
            target_ref,
        ],
    )?;
    if !add_result.success {
        return Err(format!(
            "Could not create a clean target worktree for {target_branch}: {}",
            concise_target_command_error(&add_result.stderr)
        ));
    }

    let qr_executable = resolve_qr_executable(None);
    let fleet_output_base = repository_path
        .join(".quality-runner")
        .join("fleet-audit")
        .join("target")
        .join(&run_id_prefix);
    let mut outcomes = Vec::new();
    let mut fleet_root = None;
    match run_target_qr_refresh(
        &qr_executable,
        &target_worktree,
        repository_path,
        &run_id_prefix,
        target_branch,
    ) {
        Ok(outcome) => outcomes.push(outcome),
        Err(error) => outcomes.push(format!(
            "QR evidence unavailable: {}",
            concise_target_command_error(&error)
        )),
    }
    match run_target_fleet_audit(
        &qr_executable,
        &target_worktree,
        repository_path,
        target_worktree.parent().unwrap_or(&target_parent),
        &fleet_output_base,
        target_branch,
        &repository_feed_id(&repository),
    ) {
        Ok((outcome, root)) => {
            outcomes.push(outcome);
            fleet_root = Some(root);
        }
        Err(error) => outcomes.push(format!(
            "fleet evidence unavailable: {}",
            concise_target_command_error(&error)
        )),
    }

    remove_temporary_worktree_transactionally(repository_path, &target_worktree, &target_head)
        .map_err(|error| {
            format!(
                "Target evidence refresh could not complete transactional cleanup for {}: {error}",
                target_worktree.display()
            )
        })?;

    if fleet_root.is_none() {
        let _ = fs::remove_dir_all(&fleet_output_base);
    }
    state.repositories[repository_index]
        .quality
        .target_fleet_audit_root = fleet_root
        .as_ref()
        .map(|root| root.to_string_lossy().to_string());
    let target_ids = [repository_id.to_string()]
        .into_iter()
        .collect::<HashSet<_>>();
    let projection_root = fleet_root.as_deref().unwrap_or(&fleet_output_base);
    apply_quality_evidence_scoped(&mut state, Some(&target_ids), Some(projection_root));
    apply_release_threshold_conditions(&mut state);
    if let Some(repository) = state.repositories.get_mut(repository_index) {
        let short_head = target_head.chars().take(8).collect::<String>();
        let outcome = if outcomes.is_empty() {
            "No target evidence commands returned an outcome".to_string()
        } else {
            outcomes.join(" · ")
        };
        repository.quality.ingestion_message = Some(format!(
            "Target evidence refresh for {target_branch} @ {short_head}: {outcome}"
        ));
        repository.quality.target_fleet_audit_root = fleet_root
            .as_ref()
            .map(|root| root.to_string_lossy().to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_rule_at(
    path: &Path,
    repository_id: &str,
    release_rule: Option<ReleaseRuleConfig>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_rule = release_rule.map(normalize_release_rule).transpose()?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.release_rule = normalized_rule;
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_recipe_at(
    path: &Path,
    repository_id: &str,
    release_recipe: Option<ReleaseRecipeConfig>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_recipe = release_recipe.map(normalize_release_recipe).transpose()?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.release_recipe = normalized_recipe;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_version_at(
    path: &Path,
    repository_id: &str,
    release_version: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_version = match release_version {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(normalize_release_version(&value)?),
        None => None,
    };
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    if let Some(version) = normalized_version.as_ref() {
        let provider_available =
            provider_context_available(repository, state.provider_status.state == "Ready");
        let candidate = prepare_release(repository, &repository.workspace, provider_available)
            .candidate_version;
        if candidate.as_ref() != Some(version) {
            return Err(
                "Release version must match the current deterministic candidate".to_string(),
            );
        }
    }
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.confirmed_release_version = normalized_version;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_ai_permission_at(
    path: &Path,
    repository_id: &str,
    permission: &str,
) -> Result<PortfolioSnapshot, String> {
    let normalized_permission = normalize_ai_permission(permission)?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.ai_permission = normalized_permission;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_retention_days_at(path: &Path, retention_days: i64) -> Result<PortfolioSnapshot, String> {
    if !(1..=3_650).contains(&retention_days) {
        return Err("Retention must be between 1 and 3650 days".to_string());
    }
    let mut state = load_store(path)?;
    state.retention_days = retention_days;
    prune_events(&mut state);
    prune_action_audits(&mut state);
    save_store(path, &state)?;
    prune_analytics_samples(path, retention_days)?;
    Ok(snapshot_from_store(path, &state))
}

fn upsert_product_at(
    path: &Path,
    product_id: Option<&str>,
    name: &str,
    repository_ids: Vec<String>,
    release_mode: &str,
) -> Result<PortfolioSnapshot, String> {
    let clean_name = normalize_name(name, "Product")?;
    let clean_release_mode = normalize_release_mode(release_mode)?;
    let mut state = load_store(path)?;
    let clean_repository_ids = normalize_repository_ids(&state, repository_ids)?;
    if state.products.iter().any(|product| {
        product.name.eq_ignore_ascii_case(&clean_name) && product_id != Some(product.id.as_str())
    }) {
        return Err(format!("A product named '{clean_name}' already exists"));
    }
    let now = iso_now();
    if let Some(product_id) = product_id.filter(|value| !value.trim().is_empty()) {
        let product = state
            .products
            .iter_mut()
            .find(|product| product.id == product_id)
            .ok_or_else(|| "Product is not registered".to_string())?;
        product.name = clean_name;
        product.repository_ids = clean_repository_ids;
        product.release_mode = clean_release_mode;
        product.updated_at = now;
    } else {
        state.products.push(ProductConfig {
            id: generated_config_id("product", &clean_name),
            name: clean_name,
            repository_ids: clean_repository_ids,
            release_mode: clean_release_mode,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    state
        .products
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn upsert_group_at(
    path: &Path,
    group_id: Option<&str>,
    name: &str,
    repository_ids: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    let clean_name = normalize_name(name, "Group")?;
    let mut state = load_store(path)?;
    let clean_repository_ids = normalize_repository_ids(&state, repository_ids)?;
    if state.groups.iter().any(|group| {
        group.name.eq_ignore_ascii_case(&clean_name) && group_id != Some(group.id.as_str())
    }) {
        return Err(format!("A group named '{clean_name}' already exists"));
    }
    let now = iso_now();
    if let Some(group_id) = group_id.filter(|value| !value.trim().is_empty()) {
        let group = state
            .groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| "Group is not registered".to_string())?;
        group.name = clean_name;
        group.repository_ids = clean_repository_ids;
        group.updated_at = now;
    } else {
        state.groups.push(GroupConfig {
            id: generated_config_id("group", &clean_name),
            name: clean_name,
            repository_ids: clean_repository_ids,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    state
        .groups
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn delete_product_at(path: &Path, product_id: &str) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let original_len = state.products.len();
    state.products.retain(|product| product.id != product_id);
    if state.products.len() == original_len {
        return Err("Product is not registered".to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn delete_group_at(path: &Path, group_id: &str) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let original_len = state.groups.len();
    state.groups.retain(|group| group.id != group_id);
    if state.groups.len() == original_len {
        return Err("Group is not registered".to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn register_root_and_scan(path: &Path, root_path: &str) -> Result<PortfolioSnapshot, String> {
    let root = canonical_path(Path::new(root_path))
        .ok_or_else(|| "Choose an accessible folder for repository discovery".to_string())?;
    if !root.is_dir() {
        return Err("The selected repository root is not a folder".to_string());
    }
    with_store_write_state(path, |state| {
        let root_string = root.to_string_lossy().to_string();
        if !state.roots.iter().any(|item| item.path == root_string) {
            state.roots.push(RootConfig {
                id: path_id("root", &root),
                path: root_string,
                label: root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Repository root")
                    .to_string(),
                ignore_patterns: Vec::new(),
                refresh_policy: default_refresh_policy(),
                background_monitoring: false,
                registered_at: iso_now(),
            });
        }
        audited_scan_and_persist_scoped_locked(path, state, None, None)
    })
}

fn set_maturity_audit_root_at(
    path: &Path,
    audit_root: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    let canonical_feed = quality::canonical_maturity_feed_path()
        .ok_or_else(|| "Pronto could not resolve the user's home directory".to_string())?;
    let requested = audit_root
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "Pronto no longer accepts clearing or choosing an audit root; Quality Runner owns {}",
                canonical_feed.display()
            )
        })?;
    let requested_path = PathBuf::from(requested);
    let matches_canonical = requested_path == canonical_feed
        || fs::canonicalize(&requested_path)
            .ok()
            .is_some_and(|path| path == canonical_feed);
    if !matches_canonical {
        return Err(format!(
            "Pronto no longer accepts arbitrary audit roots; use Quality Runner's canonical feed at {}",
            canonical_feed.display()
        ));
    }
    let mut state = load_store_with_quality(path)?;
    apply_quality_evidence(&mut state);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}
