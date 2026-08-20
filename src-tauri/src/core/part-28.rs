fn scan_refresh_inputs(
    inputs: Vec<RefreshScanInput>,
    parallelism: usize,
) -> Result<Vec<RepositorySnapshot>, String> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = parallelism.max(1).min(inputs.len());
    let input_count = inputs.len();
    let queue = Arc::new(Mutex::new(VecDeque::from(inputs)));
    let (sender, receiver) = mpsc::channel::<(String, RepositorySnapshot)>();
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let sender = sender.clone();
        workers.push(thread::spawn(move || loop {
            let input = match queue.lock() {
                Ok(mut queue) => queue.pop_front(),
                Err(_) => None,
            };
            let Some(input) = input else {
                break;
            };
            let repository = scan_repository(
                &input.repository_path,
                input.existing.as_ref(),
                &input.expected_conditions,
            );
            if sender.send((input.repository_id, repository)).is_err() {
                break;
            }
        }));
    }
    drop(sender);

    let mut scanned = Vec::with_capacity(input_count);
    for _ in 0..input_count {
        let (_, repository) = receiver.recv().map_err(|error| {
            format!("Parallel refresh scan worker exited before returning all results: {error}")
        })?;
        scanned.push(repository);
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "Parallel refresh scan worker panicked".to_string())?;
    }
    scanned.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(scanned)
}

fn refresh_batch_at(
    path: &Path,
    target: Option<&str>,
    parallelism: usize,
) -> Result<RefreshBatchReport, String> {
    let parallelism = parallelism.clamp(1, MAX_REFRESH_BATCH_PARALLELISM);
    let mut conflict_retries = 0;
    loop {
        let read_state = load_store_read_only(path)?;
        let plan = refresh_batch_plan(path, &read_state, target)?;
        let scanned = scan_refresh_inputs(plan.inputs.clone(), parallelism)?;

        let lock = acquire_store_write_lock(path)?;
        let mut state = load_store(path)?;
        if refresh_batch_revision(&state) != plan.revision {
            drop(lock);
            if conflict_retries >= MAX_REFRESH_BATCH_CONFLICT_RETRIES {
                return Err(
                    "The Pronto store changed during the parallel refresh scan; retry the batch refresh."
                        .to_string(),
                );
            }
            conflict_retries += 1;
            continue;
        }

        let preflight = match (
            plan.target_repository_ids.as_ref(),
            plan.repository_path_target.as_ref(),
        ) {
            (None, None) => build_action_preflight(&state, "refresh", None)?,
            (Some(repository_ids), None) => {
                build_targeted_refresh_preflight(&state, repository_ids, &plan.target_label)?
            }
            (Some(_), Some(repository_path)) => build_repository_path_refresh_preflight(
                &path_id("repository", repository_path),
                repository_path,
            ),
            (None, Some(_)) => {
                return Err("Refresh batch target metadata is incomplete".to_string())
            }
        };
        if !preflight.allowed {
            return Err("Local batch refresh action is not permitted".to_string());
        }
        let audit_id = preflight.audit.id.clone();
        append_action_audit(&mut state, &preflight);
        save_store(path, &state)?;
        let scan_results = scanned
            .iter()
            .enumerate()
            .map(|(scan_order, repository)| RefreshBatchRepositoryResult {
                repository_id: repository.id.clone(),
                name: repository.name.clone(),
                path: repository.path.clone(),
                status: "Scanned".to_string(),
                scan_order,
            })
            .collect::<Vec<_>>();
        let snapshot = match merge_scanned_and_persist(
            path,
            &mut state,
            plan.target_repository_ids.as_ref(),
            scanned,
        ) {
            Ok(snapshot) => {
                update_action_audit(
                    &mut state,
                    &audit_id,
                    "Completed",
                    format!(
                        "Parallel read-only refresh completed for {}.",
                        plan.target_label
                    ),
                )?;
                prune_action_audits(&mut state);
                save_store(path, &state)?;
                snapshot
            }
            Err(error) => {
                if update_action_audit(
                    &mut state,
                    &audit_id,
                    "Failed",
                    format!(
                        "Parallel read-only refresh failed for {}.",
                        plan.target_label
                    ),
                )
                .is_ok()
                {
                    let _ = save_store(path, &state);
                }
                return Err(error);
            }
        };
        drop(lock);
        return Ok(RefreshBatchReport {
            schema_version: REFRESH_BATCH_SCHEMA.to_string(),
            generated_at: iso_now(),
            status: "Completed".to_string(),
            scope: plan.target_label,
            parallelism,
            repository_count: scan_results.len(),
            conflict_retries,
            scan_phase: "Parallel read-only scan completed".to_string(),
            merge_phase: "Serialized locked merge committed".to_string(),
            repositories: scan_results,
            snapshot,
        });
    }
}

fn mutate_expected(
    path: &Path,
    repository_id: &str,
    condition_id: &str,
    should_mark: bool,
) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    if should_mark {
        let repository = state
            .repositories
            .iter()
            .find(|repository| repository.id == repository_id)
            .ok_or_else(|| "Repository is not registered".to_string())?;
        let condition = repository
            .conditions
            .iter()
            .find(|condition| condition.id == condition_id)
            .ok_or_else(|| "Condition is no longer active".to_string())?;
        state.expected_conditions.retain(|item| {
            !(item.repository_id == repository_id && item.condition_id == condition_id)
        });
        state.expected_conditions.push(ExpectedCondition {
            repository_id: repository_id.to_string(),
            condition_id: condition_id.to_string(),
            fingerprint: condition.fingerprint.clone(),
            marked_at: iso_now(),
        });
    } else {
        state.expected_conditions.retain(|item| {
            !(item.repository_id == repository_id && item.condition_id == condition_id)
        });
    }
    for repository in &mut state.repositories {
        for condition in &mut repository.conditions {
            let expected = state.expected_conditions.iter().any(|item| {
                item.repository_id == repository.id
                    && item.condition_id == condition.id
                    && item.fingerprint == condition.fingerprint
            });
            condition.status = if expected {
                "Expected".to_string()
            } else {
                "Active".to_string()
            };
        }
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn update_root_settings_at(
    path: &Path,
    root_id: &str,
    ignore_patterns: Vec<String>,
    refresh_policy: &str,
    background_monitoring: bool,
) -> Result<PortfolioSnapshot, String> {
    let normalized_patterns = normalize_ignore_patterns(ignore_patterns)?;
    let normalized_policy = normalize_refresh_policy(refresh_policy)?;
    with_store_write_state(path, |state| {
        let root = state
            .roots
            .iter_mut()
            .find(|root| root.id == root_id)
            .ok_or_else(|| "Discovery root is not registered".to_string())?;
        root.ignore_patterns = normalized_patterns;
        root.refresh_policy = normalized_policy;
        root.background_monitoring = background_monitoring;
        audited_scan_and_persist_scoped_locked(path, state, None, None)
    })
}

fn exclude_root_patterns_at(
    path: &Path,
    root_path: &str,
    patterns: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    let canonical_root = canonical_path(Path::new(root_path))
        .ok_or_else(|| "Choose an accessible folder for repository discovery".to_string())?;
    let root_string = canonical_root.to_string_lossy().to_string();
    with_store_write_state(path, |state| {
        let root = state
            .roots
            .iter_mut()
            .find(|root| root.path == root_string)
            .ok_or_else(|| format!("Discovery root '{root_string}' is not registered"))?;
        let mut combined_patterns = root.ignore_patterns.clone();
        combined_patterns.extend(patterns);
        root.ignore_patterns = normalize_ignore_patterns(combined_patterns)?;
        audited_scan_and_persist_scoped_locked(path, state, None, None)
    })
}

fn set_repository_lifecycle_at(
    path: &Path,
    repository_id: &str,
    lifecycle: &str,
) -> Result<PortfolioSnapshot, String> {
    let normalized_lifecycle = normalize_lifecycle(lifecycle)?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.lifecycle = normalized_lifecycle;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_repository_target_branch_at(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
) -> Result<PortfolioSnapshot, String> {
    set_repository_target_branch_at_with_lock_timeout(
        path,
        repository_id,
        target_branch,
        StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS),
    )
}

fn set_repository_target_branch_at_with_lock_timeout(
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
    let mut configured = repository.clone();
    configured.target_branch = Some(target_branch.to_string());
    configured.target_branch_configured = true;
    let rescanned = scan_repository(
        Path::new(&configured.path),
        Some(&configured),
        &state.expected_conditions,
    );
    append_transition_event(&mut state, Some(&repository), &rescanned);
    prune_events(&mut state);
    state.repositories[repository_index] = rescanned;
    if !state.remediation.id.is_empty() {
        state.remediation = remediation::rebuild_run(
            &state.repositories,
            &state.remediation,
            state.quality.latest_audit_id.as_deref(),
        );
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn refresh_repository_target_evidence_at(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
) -> Result<PortfolioSnapshot, String> {
    refresh_repository_target_evidence_at_with_lock_timeout(
        path,
        repository_id,
        target_branch,
        StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS),
    )
}

fn target_evidence_is_reusable(
    repository: &RepositorySnapshot,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    repository
        .quality
        .target_fleet_audit_root
        .as_deref()
        .is_some_and(|root| Path::new(root).is_dir())
        && quality::target_evidence_is_current(&repository.quality, target_branch, target_commit)
}
