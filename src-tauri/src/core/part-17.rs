fn generated_config_id(kind: &str, name: &str) -> String {
    let slug = name
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character == '-' || character == '_' {
                Some(character)
            } else {
                None
            }
        })
        .take(48)
        .collect::<String>();
    let slug = if slug.is_empty() { "item" } else { &slug };
    let sequence = NEXT_CONFIG_ID.fetch_add(1, Ordering::Relaxed);
    format!("{kind}:{slug}:{sequence}")
}

fn normalize_name(value: &str, kind: &str) -> Result<String, String> {
    let name = value.trim();
    if name.is_empty() {
        return Err(format!("{kind} name cannot be empty"));
    }
    if name.chars().count() > 80 {
        return Err(format!("{kind} name must be 80 characters or fewer"));
    }
    Ok(name.to_string())
}

fn normalize_repository_ids(
    state: &StoreState,
    repository_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let known_ids = state
        .repositories
        .iter()
        .map(|repository| repository.id.as_str())
        .collect::<HashSet<_>>();
    let mut normalized = repository_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if let Some(unknown) = normalized
        .iter()
        .find(|repository_id| !known_ids.contains(repository_id.as_str()))
    {
        return Err(format!("Repository {unknown} is not registered"));
    }
    Ok(normalized)
}

fn normalize_ignore_patterns(patterns: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for pattern in patterns {
        let value = pattern.trim().trim_matches('/').to_string();
        if value.is_empty() {
            continue;
        }
        if value == "."
            || value == ".."
            || value.contains('/')
            || value.contains('\\')
            || value.chars().count() > 120
        {
            return Err(format!(
                "Ignore pattern '{value}' must be a repository-relative name or suffix pattern"
            ));
        }
        normalized.push(value);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_refresh_policy(value: &str) -> Result<String, String> {
    match value.trim() {
        "Manual" => Ok("Manual".to_string()),
        "On open" => Ok("On open".to_string()),
        "Periodic" => Ok("Periodic".to_string()),
        _ => Err("Refresh policy must be Manual, On open, or Periodic".to_string()),
    }
}

fn normalize_lifecycle(value: &str) -> Result<String, String> {
    match value.trim() {
        "Unconfirmed" | "Active" | "Maintenance" | "Paused" | "Archived" => {
            Ok(value.trim().to_string())
        }
        _ => Err(
            "Lifecycle must be Unconfirmed, Active, Maintenance, Paused, or Archived".to_string(),
        ),
    }
}

fn normalize_release_mode(value: &str) -> Result<String, String> {
    match value.trim() {
        "Independent" => Ok("Independent".to_string()),
        "Coordinated independent versions" => Ok("Coordinated independent versions".to_string()),
        "Unified product version" => Ok("Unified product version".to_string()),
        _ => Err("Release mode is not supported".to_string()),
    }
}

fn normalize_ai_permission(value: &str) -> Result<String, String> {
    match value.trim() {
        "Disabled" => Ok("Disabled".to_string()),
        "Commit metadata only" => Ok("Commit metadata only".to_string()),
        "Committed diff allowed" => Ok("Committed diff allowed".to_string()),
        _ => Err(
            "AI permission must be Disabled, Commit metadata only, or Committed diff allowed"
                .to_string(),
        ),
    }
}

fn normalize_release_rule(rule: ReleaseRuleConfig) -> Result<ReleaseRuleConfig, String> {
    let name = normalize_name(&rule.name, "Release rule")?;
    let operator = match rule.operator.trim().to_ascii_uppercase().as_str() {
        "AND" => "AND".to_string(),
        "OR" => "OR".to_string(),
        _ => return Err("Release rule operator must be AND or OR".to_string()),
    };
    if rule.min_commits == Some(0) || rule.min_commits.is_some_and(|value| value > 100_000) {
        return Err("Minimum commits must be between 1 and 100000".to_string());
    }
    if rule.min_elapsed_days == Some(0) || rule.min_elapsed_days.is_some_and(|value| value > 36_500)
    {
        return Err("Minimum elapsed days must be between 1 and 36500".to_string());
    }
    let allowed_types = [
        "breaking", "feat", "fix", "perf", "docs", "refactor", "test", "chore",
    ];
    let mut required_commit_types = rule
        .required_commit_types
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    required_commit_types.sort();
    required_commit_types.dedup();
    if let Some(unknown) = required_commit_types
        .iter()
        .find(|value| !allowed_types.contains(&value.as_str()))
    {
        return Err(format!("Unsupported conventional commit type '{unknown}'"));
    }
    let mut required_quality_gates = rule
        .required_quality_gates
        .into_iter()
        .map(|requirement| QualityGateRequirement {
            gate_id: crate::quality::normalize_gate_id(&requirement.gate_id),
            source: requirement.source,
            minimum_verification_level: requirement.minimum_verification_level,
            policy: requirement.policy,
        })
        .collect::<Vec<_>>();
    required_quality_gates.sort_by(|left, right| {
        left.gate_id
            .cmp(&right.gate_id)
            .then_with(|| left.source.as_str().cmp(right.source.as_str()))
    });
    if required_quality_gates
        .windows(2)
        .any(|requirements| requirements[0].gate_id == requirements[1].gate_id)
    {
        return Err("Each release rule gate may specify only one evidence source".to_string());
    }
    if rule.min_commits.is_none()
        && rule.min_elapsed_days.is_none()
        && required_commit_types.is_empty()
        && required_quality_gates.is_empty()
    {
        return Err(
            "Release rule needs a commit count, elapsed time, commit type, or quality gate clause"
                .to_string(),
        );
    }
    Ok(ReleaseRuleConfig {
        name,
        operator,
        min_commits: rule.min_commits,
        min_elapsed_days: rule.min_elapsed_days,
        required_commit_types,
        allow_first_release: rule.allow_first_release,
        required_quality_gates,
    })
}

fn default_release_recipe() -> ReleaseRecipeConfig {
    ReleaseRecipeConfig {
        name: "Single repository release".to_string(),
        validation_commands: Vec::new(),
        release_commands: Vec::new(),
        generated_paths: Vec::new(),
        commit_message: "chore(release): prepare {version}".to_string(),
    }
}

fn normalize_release_commands(commands: Vec<String>, label: &str) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for command in commands {
        let value = command.trim();
        if value.is_empty() {
            continue;
        }
        if value.contains('\0') || value.contains('\n') || value.contains('\r') {
            return Err(format!("{label} cannot contain line breaks or null bytes"));
        }
        if value.chars().count() > 500 {
            return Err(format!(
                "{label} must be 500 characters or fewer per command"
            ));
        }
        let value = value.to_string();
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn normalize_generated_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for path in paths {
        let value = path.trim();
        if value.is_empty() {
            continue;
        }
        if value.starts_with('/')
            || value.contains('\\')
            || value.contains(':')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(format!(
                "Generated path '{value}' must be a repository-relative file path"
            ));
        }
        if value.chars().count() > 240 {
            return Err("Generated paths must be 240 characters or fewer".to_string());
        }
        let value = value.to_string();
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn normalize_release_recipe(recipe: ReleaseRecipeConfig) -> Result<ReleaseRecipeConfig, String> {
    let name = normalize_name(&recipe.name, "Release recipe")?;
    let commit_message = recipe.commit_message.trim();
    if commit_message.is_empty() {
        return Err("Release recipe commit message cannot be empty".to_string());
    }
    if commit_message.contains('\0')
        || commit_message.contains('\n')
        || commit_message.contains('\r')
    {
        return Err(
            "Release recipe commit message cannot contain line breaks or null bytes".to_string(),
        );
    }
    if commit_message.chars().count() > 160 {
        return Err("Release recipe commit message must be 160 characters or fewer".to_string());
    }
    Ok(ReleaseRecipeConfig {
        name,
        validation_commands: normalize_release_commands(
            recipe.validation_commands,
            "Validation commands",
        )?,
        release_commands: normalize_release_commands(recipe.release_commands, "Release commands")?,
        generated_paths: normalize_generated_paths(recipe.generated_paths)?,
        commit_message: commit_message.to_string(),
    })
}

fn canonical_path(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn canonical_repository_path(path: &Path) -> Option<PathBuf> {
    let top_level = git_static(path, &["rev-parse", "--show-toplevel"])?;
    let top = canonical_path(Path::new(&top_level)).unwrap_or_else(|| PathBuf::from(top_level));
    let common_raw = git_static(
        &top,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .or_else(|| git_static(&top, &["rev-parse", "--git-common-dir"]))?;
    let common = {
        let candidate = PathBuf::from(&common_raw);
        if candidate.is_absolute() {
            candidate
        } else {
            top.join(candidate)
        }
    };
    let common = canonical_path(&common).unwrap_or(common);
    if common.file_name().and_then(|name| name.to_str()) == Some(".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        Some(top)
    }
}

fn workspace_path_is_temporary(path: &Path) -> bool {
    let candidate = canonical_path(path).unwrap_or_else(|| path.to_path_buf());
    let mut temporary_roots = vec![PathBuf::from("/tmp"), PathBuf::from("/private/tmp")];
    if let Ok(root) = std::env::var("TMPDIR") {
        temporary_roots.push(PathBuf::from(root));
    }
    temporary_roots.push(std::env::temp_dir());

    temporary_roots.into_iter().any(|root| {
        let root = canonical_path(&root).unwrap_or(root);
        candidate == root || candidate.starts_with(&root)
    }) || candidate
        .components()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| {
            pair[0].as_os_str() == OsStr::new(".codex")
                && pair[1].as_os_str() == OsStr::new("worktrees")
        })
}

fn workspace_provenance(
    path: &Path,
    is_primary: bool,
    head: Option<String>,
    activity: &WorkspaceActivity,
) -> WorkspaceProvenance {
    let kind = if is_primary {
        "canonical"
    } else if workspace_path_is_temporary(path) {
        "temporary"
    } else {
        "linked"
    };
    let manifest = activity.manifest.as_ref();
    let canonical_repository = canonical_repository_path(path)
        .or_else(|| is_primary.then(|| canonical_path(path)).flatten())
        .map(|repository| repository.to_string_lossy().to_string())
        .unwrap_or_default();

    WorkspaceProvenance {
        kind: kind.to_string(),
        owner: manifest.and_then(|manifest| {
            manifest
                .task_id
                .clone()
                .or_else(|| manifest.source_session_id.clone())
        }),
        lease: manifest.and_then(|manifest| manifest.status.clone()),
        canonical_repository,
        head,
        preservation_evidence: None,
        cleanup_state: if kind == "temporary" {
            "present".to_string()
        } else {
            "not_applicable".to_string()
        },
    }
}

fn merge_workspace_provenance(observed: &mut WorkspaceProvenance, existing: &WorkspaceProvenance) {
    if observed.kind == "unknown" {
        observed.kind = existing.kind.clone();
    }
    if observed.owner.is_none() {
        observed.owner = existing.owner.clone();
    }
    if observed.lease.is_none() {
        observed.lease = existing.lease.clone();
    }
    if observed.canonical_repository.is_empty() {
        observed.canonical_repository = existing.canonical_repository.clone();
    }
    if observed.head.is_none() {
        observed.head = existing.head.clone();
    }
    if observed.preservation_evidence.is_none() {
        observed.preservation_evidence = existing.preservation_evidence.clone();
    }
    if observed.cleanup_state == "unknown" {
        observed.cleanup_state = existing.cleanup_state.clone();
    }
}

fn has_git_metadata(path: &Path) -> bool {
    path.join(".git").exists()
}

fn default_ignore(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "node_modules"
            | ".pnpm"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".cache"
            | "coverage"
            | "vendor"
            | ".idea"
            | ".vscode"
    )
}

fn matches_ignore(name: &str, patterns: &[String]) -> bool {
    let normalized_name = name.to_ascii_lowercase();
    default_ignore(&normalized_name)
        || patterns.iter().any(|pattern| {
            let trimmed = pattern.trim_matches('/').to_ascii_lowercase();
            trimmed == normalized_name
                || (trimmed.starts_with('*')
                    && normalized_name.ends_with(trimmed.trim_start_matches('*')))
        })
}

fn path_is_within(root: &Path, candidate: &Path) -> bool {
    let canonical_root = canonical_path(root).unwrap_or_else(|| root.to_path_buf());
    let canonical_candidate = canonical_path(candidate).unwrap_or_else(|| candidate.to_path_buf());
    canonical_candidate == canonical_root || canonical_candidate.starts_with(&canonical_root)
}

fn path_is_ignored_by_root(root: &RootConfig, candidate: &Path) -> bool {
    let root_path = Path::new(&root.path);
    if !path_is_within(root_path, candidate) {
        return false;
    }
    let candidate_path = canonical_path(candidate).unwrap_or_else(|| candidate.to_path_buf());
    let root_path = canonical_path(root_path).unwrap_or_else(|| root_path.to_path_buf());
    candidate_path
        .strip_prefix(root_path)
        .map(|relative| {
            relative.components().any(|component| {
                let name = component.as_os_str().to_string_lossy();
                matches_ignore(&name, &root.ignore_patterns)
            })
        })
        .unwrap_or(false)
}

fn repository_is_ignored_by_existing_root(
    state: &StoreState,
    repository: &RepositorySnapshot,
) -> bool {
    state
        .roots
        .iter()
        .any(|root| path_is_ignored_by_root(root, Path::new(&repository.path)))
}
