#[tauri::command]
pub fn export_remediation(
    output_dir: Option<String>,
) -> Result<remediation::RemediationExport, String> {
    let path = store_path();
    let state = load_store_with_quality(&path)?;
    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    let root = output_dir
        .map(PathBuf::from)
        .or_else(|| {
            path.parent()
                .map(|parent| parent.join("remediation").join(&state.remediation.id))
        })
        .ok_or_else(|| "Pronto storage path has no export directory".to_string())?;
    remediation::export_run(&remediation_run, &root)
}

#[tauri::command]
pub fn set_maturity_audit_root(audit_root: Option<String>) -> Result<PortfolioSnapshot, String> {
    set_maturity_audit_root_at(&store_path(), audit_root.as_deref())
}

#[tauri::command]
pub fn open_quality_report(report_path: String) -> Result<PortfolioSnapshot, String> {
    open_quality_report_at(&store_path(), &report_path)
}

#[tauri::command]
pub fn open_workspace(
    repository_id: String,
    workspace_id: String,
    tool: String,
) -> Result<PortfolioSnapshot, String> {
    open_workspace_at(&store_path(), &repository_id, &workspace_id, &tool)
}

#[tauri::command]
pub fn prepare_repository(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<RepositoryPreparation, String> {
    prepare_repository_at(&store_path(), &repository_id, workspace_id.as_deref())
}

#[tauri::command]
pub fn preflight_action(
    action: String,
    repository_id: Option<String>,
) -> Result<ActionPreflight, String> {
    preflight_action_at(&store_path(), &action, repository_id.as_deref())
}

#[tauri::command]
pub fn mark_condition_expected(
    repository_id: String,
    condition_id: String,
) -> Result<PortfolioSnapshot, String> {
    mutate_expected(&store_path(), &repository_id, &condition_id, true)
}

#[tauri::command]
pub fn clear_condition_expected(
    repository_id: String,
    condition_id: String,
) -> Result<PortfolioSnapshot, String> {
    mutate_expected(&store_path(), &repository_id, &condition_id, false)
}

#[tauri::command]
pub fn update_root_settings(
    root_id: String,
    ignore_patterns: Vec<String>,
    refresh_policy: String,
    background_monitoring: bool,
) -> Result<PortfolioSnapshot, String> {
    update_root_settings_at(
        &store_path(),
        &root_id,
        ignore_patterns,
        &refresh_policy,
        background_monitoring,
    )
}

#[tauri::command]
pub fn set_repository_lifecycle(
    repository_id: String,
    lifecycle: String,
) -> Result<PortfolioSnapshot, String> {
    set_repository_lifecycle_at(&store_path(), &repository_id, &lifecycle)
}

#[tauri::command]
pub fn set_repository_target_branch(
    repository_id: String,
    target_branch: String,
) -> Result<PortfolioSnapshot, String> {
    set_repository_target_branch_at(&store_path(), &repository_id, &target_branch)
}

#[tauri::command]
pub fn set_release_rule(
    repository_id: String,
    release_rule: Option<ReleaseRuleConfig>,
) -> Result<PortfolioSnapshot, String> {
    set_release_rule_at(&store_path(), &repository_id, release_rule)
}

#[tauri::command]
pub fn set_release_recipe(
    repository_id: String,
    release_recipe: Option<ReleaseRecipeConfig>,
) -> Result<PortfolioSnapshot, String> {
    set_release_recipe_at(&store_path(), &repository_id, release_recipe)
}

#[tauri::command]
pub fn set_release_version(
    repository_id: String,
    release_version: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    set_release_version_at(&store_path(), &repository_id, release_version)
}

#[tauri::command]
pub fn set_ai_permission(
    repository_id: String,
    permission: String,
) -> Result<PortfolioSnapshot, String> {
    set_ai_permission_at(&store_path(), &repository_id, &permission)
}

#[tauri::command]
pub fn preview_ai_summary(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<AiPayloadPreview, String> {
    preview_ai_summary_at(&store_path(), &repository_id, workspace_id.as_deref())
}

#[tauri::command]
pub fn set_retention_days(retention_days: i64) -> Result<PortfolioSnapshot, String> {
    set_retention_days_at(&store_path(), retention_days)
}

#[tauri::command]
pub fn upsert_product(
    product_id: Option<String>,
    name: String,
    repository_ids: Vec<String>,
    release_mode: String,
) -> Result<PortfolioSnapshot, String> {
    upsert_product_at(
        &store_path(),
        product_id.as_deref(),
        &name,
        repository_ids,
        &release_mode,
    )
}

#[tauri::command]
pub fn delete_product(product_id: String) -> Result<PortfolioSnapshot, String> {
    delete_product_at(&store_path(), &product_id)
}

#[tauri::command]
pub fn upsert_group(
    group_id: Option<String>,
    name: String,
    repository_ids: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    upsert_group_at(&store_path(), group_id.as_deref(), &name, repository_ids)
}

#[tauri::command]
pub fn delete_group(group_id: String) -> Result<PortfolioSnapshot, String> {
    delete_group_at(&store_path(), &group_id)
}

fn cli_option(arguments: &[String], option: &str) -> Result<Option<String>, String> {
    let mut value = None;
    let mut index = 1;
    while index < arguments.len() {
        if arguments[index] == option {
            let next = arguments
                .get(index + 1)
                .ok_or_else(|| format!("{option} requires a value"))?;
            if next.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            if value.replace(next.clone()).is_some() {
                return Err(format!("{option} may only be provided once"));
            }
            index += 1;
        }
        index += 1;
    }
    Ok(value)
}

fn cli_repeated_option(arguments: &[String], option: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut index = 1;
    while index < arguments.len() {
        if arguments[index] == option {
            let next = arguments
                .get(index + 1)
                .ok_or_else(|| format!("{option} requires a value"))?;
            if next.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            values.push(next.clone());
            index += 1;
        }
        index += 1;
    }
    Ok(values)
}

fn cli_json_option<T: DeserializeOwned>(
    arguments: &[String],
    option: &str,
) -> Result<Option<T>, String> {
    let Some(value) = cli_option(arguments, option)? else {
        return Ok(None);
    };
    let payload = if let Some(path) = value.strip_prefix('@') {
        fs::read_to_string(path)
            .map_err(|error| format!("Could not read {option} file: {error}"))?
    } else {
        value
    };
    serde_json::from_str(&payload)
        .map(Some)
        .map_err(|error| format!("{option} must contain valid JSON: {error}"))
}

fn cli_bool_option(arguments: &[String], option: &str) -> Result<Option<bool>, String> {
    cli_option(arguments, option)?
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            _ => Err(format!("{option} must be true or false")),
        })
        .transpose()
}

fn cli_positive_u64_option(arguments: &[String], option: &str) -> Result<Option<u64>, String> {
    cli_option(arguments, option)?
        .map(|value| {
            value
                .parse::<u64>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| format!("{option} must be a positive integer"))
        })
        .transpose()
}

fn cli_positive_usize_option(arguments: &[String], option: &str) -> Result<Option<usize>, String> {
    cli_option(arguments, option)?
        .map(|value| {
            value
                .parse::<usize>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| format!("{option} must be a positive integer"))
        })
        .transpose()
}

fn append_qr_audit_runtime_arguments(
    arguments: &mut Vec<String>,
    dynamic: bool,
    changed_only: bool,
    timeout_seconds: u64,
) {
    if dynamic {
        arguments.push("--dynamic".to_string());
        if !changed_only {
            arguments.push("--no-changed-only".to_string());
        }
    }
    arguments.extend([
        "--timeout-seconds".to_string(),
        timeout_seconds.to_string(),
        "--json".to_string(),
    ]);
}

fn cli_positionals(arguments: &[String], value_options: &[&str]) -> Result<Vec<String>, String> {
    let mut positionals = Vec::new();
    let mut expecting_value = None;
    for argument in arguments.iter().skip(1) {
        if expecting_value.take().is_some() {
            if argument.starts_with("--") {
                return Err("An option value is missing".to_string());
            }
            continue;
        }
        if argument == "--json" {
            continue;
        }
        if value_options.iter().any(|option| option == argument) {
            expecting_value = Some(argument.as_str());
        } else if argument.starts_with("--") {
            return Err(format!("Unknown option {argument}"));
        } else {
            positionals.push(argument.clone());
        }
    }
    if expecting_value.is_some() {
        return Err("An option value is missing".to_string());
    }
    Ok(positionals)
}

fn cli_positionals_with_flags(
    arguments: &[String],
    value_options: &[&str],
    flags: &[&str],
) -> Result<Vec<String>, String> {
    let mut positionals = Vec::new();
    let mut expecting_value = None;
    for argument in arguments.iter().skip(1) {
        if expecting_value.take().is_some() {
            if argument.starts_with("--") {
                return Err("An option value is missing".to_string());
            }
            continue;
        }
        if argument == "--json" || flags.iter().any(|flag| flag == argument) {
            continue;
        }
        if value_options.iter().any(|option| option == argument) {
            expecting_value = Some(argument.as_str());
        } else if argument.starts_with("--") {
            return Err(format!("Unknown option {argument}"));
        } else {
            positionals.push(argument.clone());
        }
    }
    if expecting_value.is_some() {
        return Err("An option value is missing".to_string());
    }
    Ok(positionals)
}

fn repository_matches_query(repository: &RepositorySnapshot, query: &str) -> bool {
    repository.id == query
        || repository.path == query
        || repository.name.eq_ignore_ascii_case(query)
        || repository.workspaces.iter().any(|workspace| {
            workspace.path == query || workspace.id == query || workspace.path.ends_with(query)
        })
}

fn find_cli_repository<'a>(
    snapshot: &'a PortfolioSnapshot,
    query: &str,
) -> Result<&'a RepositorySnapshot, String> {
    let matches = snapshot
        .repositories
        .iter()
        .filter(|repository| repository_matches_query(repository, query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [repository] => Ok(repository),
        [] => Err(format!("Repository '{query}' is not registered")),
        _ => Err(format!("Repository query '{query}' is ambiguous")),
    }
}

fn find_cli_group<'a>(state: &'a StoreState, query: &str) -> Result<&'a GroupConfig, String> {
    let matches = state
        .groups
        .iter()
        .filter(|group| group.id == query || group.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [group] => Ok(group),
        [] => Err(format!("Group '{query}' is not registered")),
        _ => Err(format!("Group query '{query}' is ambiguous")),
    }
}

fn find_cli_product<'a>(state: &'a StoreState, query: &str) -> Result<&'a ProductConfig, String> {
    let matches = state
        .products
        .iter()
        .filter(|product| product.id == query || product.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [product] => Ok(product),
        [] => Err(format!("Product '{query}' is not registered")),
        _ => Err(format!("Product query '{query}' is ambiguous")),
    }
}

fn merge_repository_ids(existing: &[String], additions: Vec<String>) -> Vec<String> {
    existing
        .iter()
        .cloned()
        .chain(additions)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn find_repository_for_directory<'a>(
    snapshot: &'a PortfolioSnapshot,
    directory: &Path,
) -> Option<&'a RepositorySnapshot> {
    let canonical_directory = canonical_path(directory).unwrap_or_else(|| directory.to_path_buf());
    snapshot.repositories.iter().find(|repository| {
        repository
            .workspaces
            .iter()
            .any(|workspace| canonical_directory.starts_with(&workspace.path))
            || canonical_directory.starts_with(&repository.path)
    })
}

fn filter_snapshot_to_repository_ids(
    mut snapshot: PortfolioSnapshot,
    repository_ids: &HashSet<String>,
) -> PortfolioSnapshot {
    snapshot
        .repositories
        .retain(|repository| repository_ids.contains(&repository.id));
    snapshot
}

fn filter_snapshot_by_collection(
    mut snapshot: PortfolioSnapshot,
    product_name: Option<&str>,
    group_name: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    if product_name.is_some() && group_name.is_some() {
        return Err("Choose either --product or --group, not both".to_string());
    }
    if let Some(name) = product_name {
        let product = snapshot
            .products
            .iter()
            .find(|product| product.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Product '{name}' is not configured"))?;
        let repository_ids = product
            .repository_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        snapshot
            .repositories
            .retain(|repository| repository_ids.contains(&repository.id));
        snapshot.products = vec![product];
        snapshot.groups.clear();
    } else if let Some(name) = group_name {
        let group = snapshot
            .groups
            .iter()
            .find(|group| group.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Group '{name}' is not configured"))?;
        let repository_ids = group.repository_ids.iter().cloned().collect::<HashSet<_>>();
        snapshot
            .repositories
            .retain(|repository| repository_ids.contains(&repository.id));
        snapshot.groups = vec![group];
        snapshot.products.clear();
    }
    Ok(snapshot)
}
