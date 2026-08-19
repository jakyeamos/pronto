fn workspace_policy_repository_target(repository: &Path) -> Result<PathBuf, String> {
    let repository = canonical_path(repository)
        .ok_or_else(|| format!("repository path does not exist: {}", repository.display()))?;
    if !repository.is_dir() {
        return Err(format!(
            "repository path is not a directory: {}",
            repository.display()
        ));
    }
    let git_root = git_static(&repository, &["rev-parse", "--show-toplevel"])
        .map(|value| canonical_path(Path::new(&value)).unwrap_or_else(|| PathBuf::from(value)))
        .ok_or_else(|| format!("repository is not a Git worktree: {}", repository.display()))?;
    if git_root != repository {
        return Err(format!(
            "registered path is not the Git worktree root: {}",
            repository.display()
        ));
    }

    let agents = repository.join(".agents");
    match fs::symlink_metadata(&agents) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(".agents is a symlink: {}", agents.display()));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(format!(".agents is not a directory: {}", agents.display()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("could not inspect {}: {error}", agents.display())),
    }

    let policy_path = repository.join(crate::custody::WORKSPACE_POLICY_RELATIVE_PATH);
    match fs::symlink_metadata(&policy_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "workspace policy is a symlink: {}",
                policy_path.display()
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(format!(
                "workspace policy is not a regular file: {}",
                policy_path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect {}: {error}",
                policy_path.display()
            ));
        }
    }
    Ok(policy_path)
}

fn workspace_policy_bytes(policy: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(policy)
        .map_err(|error| format!("could not serialize workspace policy: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn workspace_policy_generation_plan(
    repository: &RepositorySnapshot,
    policy: Value,
    replace: bool,
) -> WorkspacePolicyGenerationPlan {
    let repository_path = PathBuf::from(&repository.path);
    let fallback_policy_path = repository_path.join(crate::custody::WORKSPACE_POLICY_RELATIVE_PATH);
    let repository_role = policy
        .get("repository_role")
        .and_then(Value::as_str)
        .unwrap_or("role_unresolved")
        .to_string();
    let policy_bytes = workspace_policy_bytes(&policy).unwrap_or_default();
    let policy_path = match workspace_policy_repository_target(&repository_path) {
        Ok(path) => path,
        Err(reason) => {
            return WorkspacePolicyGenerationPlan {
                repository_id: repository.id.clone(),
                repository_path,
                policy_path: fallback_policy_path,
                repository_role,
                policy_bytes,
                status: "blocked".to_string(),
                reason: Some(reason),
            };
        }
    };

    let (status, reason) = match fs::read(&policy_path) {
        Ok(existing) if existing == policy_bytes => ("unchanged", None),
        Ok(_) if replace => ("ready_replace", None),
        Ok(_) => (
            "conflict",
            Some(
                "existing workspace policy differs; use --replace with --write to replace it"
                    .to_string(),
            ),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ("ready_create", None),
        Err(error) => (
            "blocked",
            Some(format!("could not read {}: {error}", policy_path.display())),
        ),
    };
    WorkspacePolicyGenerationPlan {
        repository_id: repository.id.clone(),
        repository_path,
        policy_path,
        repository_role,
        policy_bytes,
        status: status.to_string(),
        reason,
    }
}

fn write_workspace_policy_file(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "workspace policy has no parent directory: {}",
            path.display()
        )
    })?;
    match fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "workspace policy parent is a symlink: {}",
                parent.display()
            ));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(format!(
                "workspace policy parent is not a directory: {}",
                parent.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        Err(error) => return Err(format!("could not inspect {}: {error}", parent.display())),
    }

    let nonce = NEXT_CONFIG_ID.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".workspace-policy.json.pronto-{}-{nonce}.tmp",
        std::process::id()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("could not create {}: {error}", temporary.display()))?;
        file.write_all(bytes)
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("could not sync {}: {error}", temporary.display()))?;
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temporary, metadata.permissions()).map_err(|error| {
                format!(
                    "could not preserve permissions for {}: {error}",
                    path.display()
                )
            })?;
        }
        if !replace && path.exists() {
            return Err(format!(
                "workspace policy appeared during generation: {}",
                path.display()
            ));
        }
        fs::rename(&temporary, path)
            .map_err(|error| format!("could not install {}: {error}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn workspace_policy_generation(
    state: &StoreState,
    role_map: &Value,
    repository_query: Option<&str>,
    write: bool,
    replace: bool,
) -> Result<(Value, bool), String> {
    if replace && !write {
        return Err("--replace requires --write".to_string());
    }
    let by_id = workspace_role_map_entries(state, role_map)?;
    for repository in &state.repositories {
        let entry = by_id
            .get(&repository.id)
            .expect("role map coverage was checked above");
        workspace_policy_for_role(&repository.id, entry)?;
    }

    let selected = state
        .repositories
        .iter()
        .filter(|repository| {
            repository_query.map_or(true, |query| repository_matches_query(repository, query))
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "workspace policy repository query did not match a registered repository: {}",
            repository_query.unwrap_or("<fleet>")
        ));
    }
    if let Some(query) = repository_query {
        let matches = state
            .repositories
            .iter()
            .filter(|repository| repository_matches_query(repository, query))
            .count();
        if matches > 1 {
            return Err(format!(
                "workspace policy repository query is ambiguous: {query}"
            ));
        }
    }

    let mut plans = selected
        .into_iter()
        .map(|repository| {
            let entry = by_id
                .get(&repository.id)
                .expect("role map coverage was checked above");
            let policy = workspace_policy_for_role(&repository.id, entry)
                .expect("all workspace role entries were validated above");
            workspace_policy_generation_plan(repository, policy, replace)
        })
        .collect::<Vec<_>>();
    let preflight_blocked = plans
        .iter()
        .any(|plan| matches!(plan.status.as_str(), "blocked" | "conflict"));
    let mut partial = false;
    let mut results = Vec::with_capacity(plans.len());
    for plan in plans.drain(..) {
        let mut status = plan.status.clone();
        let mut reason = plan.reason.clone();
        let mut write_applied = false;
        if write
            && !preflight_blocked
            && matches!(status.as_str(), "ready_create" | "ready_replace")
        {
            match write_workspace_policy_file(
                &plan.policy_path,
                &plan.policy_bytes,
                status == "ready_replace" && replace,
            ) {
                Ok(()) => {
                    status = if status == "ready_replace" {
                        "replaced"
                    } else {
                        "created"
                    }
                    .to_string();
                    write_applied = true;
                }
                Err(error) => {
                    status = "write_failed".to_string();
                    reason = Some(error);
                    partial = true;
                }
            }
        } else if !write {
            status = match status.as_str() {
                "ready_create" => "would_create",
                "ready_replace" => "would_replace",
                other => other,
            }
            .to_string();
        } else if preflight_blocked && matches!(status.as_str(), "ready_create" | "ready_replace") {
            status = "not_applied".to_string();
            reason =
                Some("fleet preflight contains a blocked or conflicting policy target".to_string());
        }
        let mut result = serde_json::json!({
            "repository_id": plan.repository_id,
            "repository_path": plan.repository_path,
            "policy_path": plan.policy_path,
            "repository_role": plan.repository_role,
            "status": status,
            "write_applied": write_applied,
            "policy_sha256": format!("{:x}", Sha256::digest(&plan.policy_bytes)),
        });
        if let Some(reason) = reason {
            result["reason"] = Value::String(reason);
        }
        results.push(result);
    }
    let mut counts = BTreeMap::<String, u64>::new();
    for result in &results {
        if let Some(status) = result.get("status").and_then(Value::as_str) {
            *counts.entry(status.to_string()).or_default() += 1;
        }
    }
    let blocked = preflight_blocked || partial;
    let status = if blocked {
        if partial {
            "partial"
        } else {
            "blocked"
        }
    } else if write {
        "written"
    } else {
        "ready"
    };
    let next_safe_step = if blocked {
        "Resolve blocked or conflicting repository policy targets, then rerun the exact scoped generation command.".to_string()
    } else if write {
        "Review the generated repository policy files and commit them through each repository's normal integration lane.".to_string()
    } else {
        "Review the plan, then rerun with --write to create missing policy files.".to_string()
    };
    Ok((
        serde_json::json!({
            "schema_version": WORKSPACE_POLICY_GENERATION_SCHEMA,
            "generated_at": iso_now(),
            "source": "pronto registered fleet plus explicit workspace-role-map/v1",
            "role_map_schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
            "registered_repository_count": state.repositories.len(),
            "repository_count": results.len(),
            "write_requested": write,
            "replace_requested": replace,
            "read_only": !write,
            "implementation_allowed": write && !blocked,
            "status": status,
            "next_safe_step": next_safe_step,
            "counts": counts,
            "repositories": results,
        }),
        blocked,
    ))
}

fn print_cli_usage() {
    println!(
        "Usage: pronto . | pronto analytics [--range-days <days>] [--json] | pronto analytics view list|save --config-json <json|@file>|delete <id>|default <id> [--json] | pronto skills [<skill-id>] [--json] | pronto custody [<repository>] [--json] | pronto telescope <repository> [--json] | pronto telescope refresh <repository> [--json] | pronto papercuts list --json | pronto papercuts observe --stdin --json [--dry-run] | pronto papercuts contract --json | pronto papercuts digest --week current --json | pronto papercuts propose --stdin --json | pronto papercuts proposal set-status <id> <status> --json | pronto papercuts health --json | pronto behavior [<repository>] [--filter <missing|legacy|unprofiled|partially_verified|stale|failed|blocked|unknown|current|not_applicable>] [--fresh] [--json] | pronto change-matrix repo <repository> [--operation <add|change|remove>] [--json] | pronto change-matrix skill <skill-id> [--operation <add|change|remove>] [--json] | pronto attention [--json] | pronto route [<repository>] [--fresh] [--json] | pronto quality [<repository>] [--json] | pronto quality refresh [--json] | pronto quality detector-refresh [--qr-bin <path>] [--timeout-seconds <positive-integer>] [--agent-review-mode <off|auto|parallel|required>] [--json] | pronto refresh [<repository|group|product|repository-path>] [--json] | pronto refresh-batch [<repository|group|product|repository-path>] [--parallelism <positive-integer>] [--json] | pronto prepare <repository> [--workspace <id>] [--fresh] [--json] | pronto release preview <repository> [--workspace <id>] [--fresh] [--json] | pronto remediation gate <repository> [--workspace <id>] [--json] | pronto remediation handoff-check <repository> [--workspace <id>] [--json] | pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json] | pronto repo set-target <repository> <branch> [--json] | pronto status [--fresh] [--json] | pronto help"
    );
    println!("Telescope: pronto telescope <repository> [--json] | pronto telescope refresh <repository> [--json]");
    println!("Workspace manifest: pronto workspace-manifest --role-map <path|@json> [--json]");
    println!("Workspace policy: pronto workspace-policy generate --role-map <path|@json> [--repository <id|path|name>] [--write] [--replace] [--json]");
}
