#[derive(Debug)]
struct RemediationRefreshScope {
    repository_ids: HashSet<String>,
    repository_paths: Vec<String>,
    target_name: Option<String>,
}

impl RemediationRefreshScope {
    fn is_repository_scoped(&self) -> bool {
        self.target_name.is_some()
    }
}

fn remediation_refresh_scope(
    snapshot: &PortfolioSnapshot,
    target_query: Option<&str>,
) -> Result<RemediationRefreshScope, String> {
    let selected_repository = target_query
        .map(|query| find_cli_repository(snapshot, query).cloned())
        .transpose()?;
    if let Some(repository) = selected_repository.as_ref() {
        if remediation::is_excluded_repository(repository) {
            return Err(format!(
                "Repository '{}' is excluded from remediation refresh.",
                repository.name
            ));
        }
    }
    let repositories = snapshot
        .repositories
        .iter()
        .filter(|repository| {
            selected_repository
                .as_ref()
                .map(|selected| selected.id == repository.id)
                .unwrap_or(true)
        })
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .collect::<Vec<_>>();
    if repositories.is_empty() {
        return Err("No eligible repositories are registered for remediation refresh.".to_string());
    }
    Ok(RemediationRefreshScope {
        repository_ids: repositories
            .iter()
            .map(|repository| repository.id.clone())
            .collect(),
        repository_paths: repositories
            .iter()
            .map(|repository| repository.path.clone())
            .collect(),
        target_name: selected_repository.map(|repository| repository.name),
    })
}

fn qr_fleet_run_arguments(
    repository_paths: &[String],
    projects_root: &Path,
    all_projects_scope: bool,
    dynamic: bool,
    changed_only: bool,
    timeout_seconds: u64,
) -> Vec<String> {
    let mut arguments = vec!["fleet".to_string(), "audit".to_string(), "run".to_string()];
    if all_projects_scope {
        arguments.extend(["--all".to_string(), "--projects-root".to_string()]);
        arguments.push(projects_root.to_string_lossy().to_string());
    } else {
        arguments.extend([
            "--projects-root".to_string(),
            projects_root.to_string_lossy().to_string(),
        ]);
        for repository_path in repository_paths {
            arguments.push("--repo-path".to_string());
            arguments.push(repository_path.clone());
        }
    }
    append_qr_audit_runtime_arguments(&mut arguments, dynamic, changed_only, timeout_seconds);
    arguments
}
