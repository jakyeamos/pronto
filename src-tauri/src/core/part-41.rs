fn repository_path_available(repository: &RepositorySnapshot) -> bool {
    Path::new(&repository.path).is_dir()
}

fn agent_repository_change_delta(
    repository: &RepositorySnapshot,
    max_change_commits: u64,
    max_changed_paths: u64,
) -> AgentRepositoryChangeDelta {
    let repository_path = Path::new(&repository.path);
    let target_branch = repository
        .target_branch
        .clone()
        .or_else(|| repository.default_branch.clone());
    let target_ref = target_branch
        .as_ref()
        .map(|branch| format!("refs/heads/{branch}"));
    let scanned_commit = target_branch.as_ref().and_then(|branch| {
        repository
            .branches
            .iter()
            .find(|candidate| candidate.name == *branch)
            .and_then(|candidate| candidate.last_commit.clone())
    });
    let current_commit = target_ref.as_ref().and_then(|reference| {
        git_owned(
            repository_path,
            vec![
                "rev-parse".to_string(),
                "--verify".to_string(),
                reference.clone(),
            ],
        )
    });
    let report = |status: &str,
                  commits_since_scan: Option<u64>,
                  changed_paths_since_scan: Option<u64>,
                  ancestry: &str,
                  detail: String| AgentRepositoryChangeDelta {
        repository_id: repository.id.clone(),
        projection_type: "fleet_target_branch".to_string(),
        target_ref: target_ref.clone(),
        status: status.to_string(),
        commits_since_scan,
        changed_paths_since_scan,
        scanned_commit: scanned_commit.clone(),
        current_commit: current_commit.clone(),
        ancestry: ancestry.to_string(),
        detail,
    };
    let Some(target_branch) = target_branch else {
        return report(
            "unavailable",
            None,
            None,
            "unknown",
            "The scan did not identify an explicit target or default branch.".to_string(),
        );
    };
    let Some(scanned) = scanned_commit.clone() else {
        return report(
            "unavailable",
            None,
            None,
            "unknown",
            format!("The scan did not record a commit for target branch {target_branch}."),
        );
    };
    let Some(current) = current_commit.clone() else {
        return report(
            "unavailable",
            None,
            None,
            "unknown",
            format!("The current target ref refs/heads/{target_branch} could not be resolved."),
        );
    };
    if scanned == current {
        return report(
            "current",
            Some(0),
            Some(0),
            "same",
            format!("The scanned and current {target_branch} target commits match."),
        );
    }
    let ancestry = run_git(
        repository_path,
        vec![
            "merge-base".to_string(),
            "--is-ancestor".to_string(),
            scanned.clone(),
            current.clone(),
        ],
    );
    match ancestry {
        Ok(result) if result.success => {
            let commit_count = git_owned(
                repository_path,
                vec![
                    "rev-list".to_string(),
                    "--first-parent".to_string(),
                    "--count".to_string(),
                    format!("{scanned}..{current}"),
                ],
            )
            .and_then(|value| value.parse::<u64>().ok());
            let changed_paths = git_owned(
                repository_path,
                vec![
                    "diff".to_string(),
                    "--name-only".to_string(),
                    "--no-renames".to_string(),
                    format!("{scanned}..{current}"),
                ],
            )
            .map(|value| value.lines().filter(|line| !line.trim().is_empty()).count() as u64);
            let (Some(commit_count), Some(changed_paths)) = (commit_count, changed_paths) else {
                return report(
                    "unavailable",
                    None,
                    None,
                    "ancestor",
                    "The target commit or changed-path delta could not be measured.".to_string(),
                );
            };
            let stale = commit_count >= max_change_commits || changed_paths >= max_changed_paths;
            report(
                if stale { "stale" } else { "current" },
                Some(commit_count),
                Some(changed_paths),
                "ancestor",
                if stale {
                    format!("Target {target_branch} advanced by {commit_count} first-parent commits and {changed_paths} paths; at least one change threshold was met.")
                } else {
                    format!("Target {target_branch} advanced by {commit_count} first-parent commits and {changed_paths} paths; both remain below their change thresholds.")
                },
            )
        }
        Ok(result) if result.exit_code == Some(1) => report(
            "unavailable",
            None,
            None,
            "diverged",
            format!("Target {target_branch} diverged from or rewrote the scanned target commit; refresh is required."),
        ),
        _ => report(
            "unavailable",
            None,
            None,
            "unknown",
            format!("Git ancestry could not be established for target {target_branch}."),
        ),
    }
}
