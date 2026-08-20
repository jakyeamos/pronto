fn discover_in_directory(
    directory: &Path,
    patterns: &[String],
    visited: &mut HashSet<PathBuf>,
    repositories: &mut HashSet<PathBuf>,
) {
    let canonical = canonical_path(directory).unwrap_or_else(|| directory.to_path_buf());
    if !visited.insert(canonical.clone()) {
        return;
    }
    if canonical.join(".git").exists() {
        if let Some(repository) = canonical_repository_path(&canonical) {
            repositories.insert(repository);
        }
        return;
    }
    let entries = match fs::read_dir(&canonical) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if matches_ignore(&name, patterns) {
            continue;
        }
        let metadata = match fs::metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            discover_in_directory(&entry.path(), patterns, visited, repositories);
        }
    }
}

fn discover_repositories(root: &RootConfig) -> Vec<PathBuf> {
    let mut visited = HashSet::new();
    let mut repositories = HashSet::new();
    discover_in_directory(
        Path::new(&root.path),
        &root.ignore_patterns,
        &mut visited,
        &mut repositories,
    );
    let mut sorted = repositories.into_iter().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

fn parse_status(output: &str) -> ParsedStatus {
    let mut status = ParsedStatus::default();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            status.branch = if value == "(detached)" {
                "Detached HEAD".to_string()
            } else {
                value.to_string()
            };
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            let mut values = value.split_whitespace();
            status.ahead = values
                .next()
                .unwrap_or("+0")
                .trim_start_matches('+')
                .parse()
                .unwrap_or(0);
            status.behind = values
                .next()
                .unwrap_or("-0")
                .trim_start_matches('-')
                .parse()
                .unwrap_or(0);
        } else if !line.is_empty() && !line.starts_with('#') {
            status.dirty = true;
        }
    }
    if status.branch.is_empty() {
        status.branch = "Detached HEAD".to_string();
    }
    status
}

fn parse_git_status(result: GitOutput) -> Result<ParsedStatus, String> {
    if !result.success {
        let detail = result.stderr.trim();
        let detail = if detail.is_empty() {
            result
                .exit_code
                .map(|code| format!("Git status exited with code {code}."))
                .unwrap_or_else(|| "Git status exited unsuccessfully.".to_string())
        } else {
            detail.to_string()
        };
        return Err(format!("Git status failed: {detail}"));
    }
    Ok(parse_status(&result.stdout))
}

fn parse_numstat(output: &str) -> DiffTotals {
    let mut totals = DiffTotals::default();
    for line in output.lines() {
        let mut fields = line.split('\t');
        let added = fields.next().unwrap_or_default();
        let removed = fields.next().unwrap_or_default();
        if added == "-" || removed == "-" {
            totals.partial = true;
            continue;
        }
        totals.added += added.parse::<u64>().unwrap_or(0);
        totals.removed += removed.parse::<u64>().unwrap_or(0);
    }
    totals
}

fn count_untracked_lines(path: &Path) -> DiffTotals {
    let mut totals = DiffTotals::default();
    let result = match run_git(
        path,
        ["ls-files", "--others", "--exclude-standard", "-z"].iter(),
    ) {
        Ok(result) if result.success => result,
        _ => return totals,
    };
    let workspace = canonical_path(path).unwrap_or_else(|| path.to_path_buf());
    for relative in result.stdout.split('\0').filter(|value| !value.is_empty()) {
        let candidate = path.join(relative);
        let canonical = match canonical_path(&candidate) {
            Some(value) if value.starts_with(&workspace) => value,
            _ => {
                totals.partial = true;
                continue;
            }
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(metadata) => metadata,
            Err(_) => {
                totals.partial = true;
                continue;
            }
        };
        if metadata.len() > DEFAULT_MAX_UNTRACKED_BYTES {
            totals.partial = true;
            continue;
        }
        let bytes = match fs::read(&canonical) {
            Ok(bytes) => bytes,
            Err(_) => {
                totals.partial = true;
                continue;
            }
        };
        if bytes.contains(&0) {
            totals.partial = true;
            continue;
        }
        totals.added += bytes.iter().filter(|byte| **byte == b'\n').count() as u64
            + u64::from(!bytes.is_empty() && !bytes.ends_with(b"\n"));
    }
    totals
}

fn diff_totals(path: &Path) -> DiffTotals {
    let tracked_output = git_static(path, &["diff", "--numstat", "HEAD", "--"])
        .or_else(|| git_static(path, &["diff", "--numstat", "--"]))
        .unwrap_or_default();
    let mut totals = parse_numstat(&tracked_output);
    let untracked = count_untracked_lines(path);
    totals.added += untracked.added;
    totals.removed += untracked.removed;
    totals.partial |= untracked.partial;
    totals
}

fn interrupted_operation(path: &Path) -> Option<String> {
    let markers = [
        ("Merge in progress", "MERGE_HEAD"),
        ("Cherry-pick in progress", "CHERRY_PICK_HEAD"),
        ("Revert in progress", "REVERT_HEAD"),
        ("Rebase in progress", "rebase-merge"),
        ("Rebase in progress", "rebase-apply"),
        ("Bisect in progress", "BISECT_LOG"),
    ];
    for (label, marker) in markers {
        let marker_path = git_static(path, &["rev-parse", "--git-path", marker]).map(|value| {
            let candidate = PathBuf::from(value);
            if candidate.is_absolute() {
                candidate
            } else {
                path.join(candidate)
            }
        });
        if marker_path
            .as_ref()
            .is_some_and(|candidate| candidate.exists())
        {
            return Some(label.to_string());
        }
    }
    None
}

fn parse_log(raw: Option<String>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw else {
        return (None, None);
    };
    let mut fields = raw.split('\t');
    let commit = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let committed_at = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    (commit, committed_at)
}

fn workspace_status(
    path: &Path,
) -> (
    Result<ParsedStatus, String>,
    DiffTotals,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let status = run_git(
        path,
        [
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ]
        .iter(),
    )
    .and_then(parse_git_status);
    let totals = diff_totals(path);
    let operation = interrupted_operation(path);
    let (last_commit, last_commit_at) =
        parse_log(git_static(path, &["log", "-1", "--format=%H\t%cI"]));
    let last_activity = last_commit_at.clone();
    (
        status,
        totals,
        operation,
        last_commit,
        last_commit_at,
        last_activity,
    )
}

fn live_worktree_paths(path: &Path) -> Option<Vec<PathBuf>> {
    let result = run_git(path, ["worktree", "list", "--porcelain"].iter()).ok()?;
    if !result.success {
        return None;
    }

    let mut records = Vec::new();
    for block in result.stdout.split("\n\n") {
        let mut worktree_path = None;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                let candidate = PathBuf::from(value);
                worktree_path = Some(if candidate.is_absolute() {
                    candidate
                } else {
                    path.join(candidate)
                });
            }
        }
        if let Some(path) = worktree_path {
            records.push(path);
        }
    }
    Some(records)
}

fn parse_worktrees(path: &Path) -> Vec<WorktreeRecord> {
    let mut records = live_worktree_paths(path)
        .unwrap_or_default()
        .into_iter()
        .map(|path| WorktreeRecord { path })
        .collect::<Vec<_>>();
    if records.is_empty() {
        records.push(WorktreeRecord {
            path: path.to_path_buf(),
        });
    }
    records
}

fn comparable_path(path: &Path) -> PathBuf {
    if let Some(path) = canonical_path(path) {
        return path;
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn live_worktree_contains(repository_path: &Path, workspace_path: &Path) -> Option<bool> {
    let workspace_path = comparable_path(workspace_path);
    Some(
        live_worktree_paths(repository_path)?
            .iter()
            .map(|path| comparable_path(path))
            .any(|path| path == workspace_path),
    )
}

fn git_head_reachable(repository_path: &Path, head: &str) -> Option<bool> {
    let arguments = vec![
        "for-each-ref".to_string(),
        "--contains".to_string(),
        head.to_string(),
        "--format=%(refname)".to_string(),
        "refs/heads".to_string(),
        "refs/remotes".to_string(),
        "refs/tags".to_string(),
    ];
    let result = run_git(repository_path, arguments.iter()).ok()?;
    if !result.success {
        return None;
    }
    Some(result.stdout.lines().any(|line| !line.trim().is_empty()))
}

fn fresh_clean_status_for_worktree(path: &Path) -> Result<Vec<String>, String> {
    let result = run_git(
        path,
        ["status", "--porcelain=v2", "--untracked-files=all"].iter(),
    )?;
    if !result.success {
        return Err(format!(
            "Git clean-status failed for {}: {}",
            path.display(),
            concise_target_command_error(&result.stderr)
        ));
    }

    Ok(result
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let record_type = line.as_bytes().first().copied();
            let fields = match record_type {
                Some(b'1') => line.splitn(9, ' ').nth(8),
                Some(b'2') => line.splitn(10, ' ').nth(9),
                Some(b'u') => line.splitn(11, ' ').nth(10),
                Some(b'?') | Some(b'!') => line.split_once(' ').map(|(_, path)| path),
                _ => None,
            };
            fields
                .map(|path| path.split_once('\t').map_or(path, |(path, _)| path))
                .map_or_else(|| line.to_string(), ToString::to_string)
        })
        .collect())
}

fn remove_temporary_worktree_transactionally(
    repository_path: &Path,
    worktree_path: &Path,
    head: &str,
) -> Result<(), String> {
    let dirty_paths = fresh_clean_status_for_worktree(worktree_path)?;
    if !dirty_paths.is_empty() {
        return Err(format!(
            "Temporary worktree cleanup blocked for '{}': fresh clean-status found dirty files [{}]. Preserve the worktree and inspect it before retrying.",
            worktree_path.display(),
            dirty_paths.join(", ")
        ));
    }

    let cleanup = run_git(
        repository_path,
        [
            "worktree",
            "remove",
            worktree_path.to_string_lossy().as_ref(),
        ]
        .iter(),
    )?;
    if !cleanup.success {
        return Err(format!(
            "Temporary worktree cleanup blocked for '{}': Git removal failed: {}. Preserve the worktree and inspect it before retrying.",
            worktree_path.display(),
            concise_target_command_error(&cleanup.stderr)
        ));
    }

    if worktree_path.exists() {
        return Err(format!(
            "Temporary worktree cleanup blocked for '{}': the path still exists after removal. Preserve it and inspect the incomplete cleanup.",
            worktree_path.display()
        ));
    }
    if live_worktree_contains(repository_path, worktree_path) != Some(false) {
        return Err(format!(
            "Temporary worktree cleanup blocked for '{}': live Git metadata still contains the worktree or could not be read. Preserve the cleanup receipt and inspect Git metadata.",
            worktree_path.display()
        ));
    }
    if git_head_reachable(repository_path, head) != Some(true) {
        return Err(format!(
            "Temporary worktree cleanup blocked for '{}': HEAD {} is not reachable from a live branch, remote, or tag. Preserve the cleanup receipt and inspect refs.",
            worktree_path.display(),
            head
        ));
    }

    Ok(())
}

fn parse_branches(path: &Path) -> Vec<BranchRecord> {
    let output = git_static(
        path,
        &[
            "for-each-ref",
            "--format=%(refname:short)%09%(objectname)%09%(authordate:iso-strict)",
            "refs/heads",
        ],
    )
    .unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let last_commit = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let last_commit_at = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            Some(BranchRecord {
                name,
                last_commit,
                last_commit_at,
            })
        })
        .collect()
}
