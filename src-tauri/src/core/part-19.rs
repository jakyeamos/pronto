fn parse_submodules(path: &Path) -> Vec<SubmoduleSummary> {
    let output = git_static(path, &["submodule", "status", "--recursive"]).unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let marker = line.chars().next()?;
            let mut fields = line.get(marker.len_utf8()..)?.split_whitespace();
            let commit = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let submodule_path = fields.next()?.to_string();
            let status = match line.chars().next() {
                Some('-') => "Uninitialized",
                Some('+') => "Modified commit",
                Some('U') => "Merge conflict",
                _ => "Checked out",
            };
            Some(SubmoduleSummary {
                path: submodule_path,
                commit,
                status: status.to_string(),
            })
        })
        .collect()
}

fn detect_default_branch(path: &Path, current: &str) -> Option<String> {
    git_static(
        path,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|value| value.strip_prefix("origin/").map(str::to_string))
    .or_else(|| {
        ["main", "master", "dev", "develop"]
            .iter()
            .find(|candidate| {
                git_static(
                    path,
                    &["show-ref", "--verify", &format!("refs/heads/{candidate}")],
                )
                .is_some()
            })
            .map(|candidate| (*candidate).to_string())
    })
    .or_else(|| (!current.is_empty() && current != "Detached HEAD").then(|| current.to_string()))
}

fn branch_role(branch: &str, default_branch: Option<&str>) -> (String, String) {
    if default_branch.is_some_and(|default| default == branch) {
        return ("Production".to_string(), "High".to_string());
    }
    let lower = branch.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "dev" | "develop" | "development" | "staging"
    ) {
        return ("Integration".to_string(), "Medium".to_string());
    }
    if lower.starts_with("agent/") || lower.starts_with("task/") || lower.starts_with("codex/") {
        return ("Agent task".to_string(), "Medium".to_string());
    }
    if lower.starts_with("release/") {
        return ("Release".to_string(), "Medium".to_string());
    }
    if lower.starts_with("hotfix/") {
        return ("Hotfix".to_string(), "Low".to_string());
    }
    ("Feature".to_string(), "Low".to_string())
}

fn target_for_branch(branch: &str, default_branch: Option<&str>) -> (Option<String>, String) {
    if default_branch.is_some_and(|default| default == branch) {
        (None, "High".to_string())
    } else {
        (default_branch.map(str::to_string), "Medium".to_string())
    }
}

fn activity_signal(
    source: &str,
    summary: &str,
    confidence: &str,
    process_name: Option<&str>,
    process_id: Option<u32>,
    started_at: Option<&str>,
    working_directory: Option<&Path>,
) -> ActivitySignal {
    ActivitySignal {
        source: source.to_string(),
        summary: summary.to_string(),
        confidence: confidence.to_string(),
        observed_at: iso_now(),
        process_name: process_name.map(str::to_string),
        process_id,
        started_at: started_at.map(str::to_string),
        working_directory: working_directory.map(|path| path.to_string_lossy().to_string()),
    }
}

fn manifest_value_is_safe(value: &Option<String>) -> bool {
    value
        .as_ref()
        .map(|value| value.chars().count() <= 512 && !value.contains('\0'))
        .unwrap_or(true)
}

fn manifest_is_safe(manifest: &AgentManifest) -> bool {
    [
        &manifest.task_id,
        &manifest.title,
        &manifest.target_branch,
        &manifest.agent_type,
        &manifest.start_time,
        &manifest.status,
        &manifest.source_session_id,
    ]
    .into_iter()
    .all(manifest_value_is_safe)
}

fn read_agent_manifest(path: &Path) -> (Option<AgentManifest>, Option<ActivitySignal>) {
    let candidates = [
        path.join(".pronto").join("agent.json"),
        path.join(".pronto").join("agent-manifest.json"),
    ];
    for manifest_path in candidates {
        let Ok(metadata) = fs::metadata(&manifest_path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > DEFAULT_MAX_MANIFEST_BYTES {
            return (
                None,
                Some(activity_signal(
                    "Manifest",
                    "Activity state uncertain",
                    "Low",
                    None,
                    None,
                    None,
                    None,
                )),
            );
        }
        let payload = match fs::read_to_string(&manifest_path) {
            Ok(payload) => payload,
            Err(_) => {
                return (
                    None,
                    Some(activity_signal(
                        "Manifest",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )),
                );
            }
        };
        let manifest = match serde_json::from_str::<AgentManifest>(&payload) {
            Ok(manifest) if manifest_is_safe(&manifest) => manifest,
            _ => {
                return (
                    None,
                    Some(activity_signal(
                        "Manifest",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )),
                );
            }
        };
        let summary = if manifest.status.as_deref().is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "active" | "running" | "started"
            )
        }) {
            "Agent manifest reports active task"
        } else {
            "Agent manifest found"
        };
        return (
            Some(manifest),
            Some(activity_signal(
                "Manifest", summary, "High", None, None, None, None,
            )),
        );
    }
    (None, None)
}

fn process_name_is_activity_candidate(name: &str) -> bool {
    let normalized = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(name)
        .to_ascii_lowercase();
    [
        "codex", "claude", "aider", "cursor", "continue", "opencode", "copilot",
    ]
    .iter()
    .any(|candidate| normalized == *candidate || normalized.contains(candidate))
}

#[cfg(not(target_os = "windows"))]
fn process_working_directory_from_lsof(process_id: u32) -> Option<PathBuf> {
    let output = Command::new("lsof")
        .args(["-a", "-p", &process_id.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(PathBuf::from))
}

#[cfg(target_os = "linux")]
fn process_working_directory(process_id: u32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{process_id}/cwd"))
        .ok()
        .or_else(|| process_working_directory_from_lsof(process_id))
}

#[cfg(target_os = "macos")]
fn process_working_directory(process_id: u32) -> Option<PathBuf> {
    process_working_directory_from_lsof(process_id)
}

#[cfg(target_os = "windows")]
fn process_working_directory(_process_id: u32) -> Option<PathBuf> {
    None
}

fn workspace_contains(parent: &Path, candidate: &Path) -> bool {
    let Some(parent) = canonical_path(parent) else {
        return false;
    };
    let Some(candidate) = canonical_path(candidate) else {
        return false;
    };
    candidate == parent || candidate.starts_with(parent)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessActivityRow {
    process_id: u32,
    parent_process_id: u32,
    process_name: String,
    started_at: Option<String>,
}

fn parse_process_activity_rows(output: &str) -> Vec<ProcessActivityRow> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process_id = fields.next()?.parse::<u32>().ok()?;
            let parent_process_id = fields.next()?.parse::<u32>().ok()?;
            let process_name = fields.next()?.to_string();
            let started_at = fields.collect::<Vec<_>>().join(" ");
            Some(ProcessActivityRow {
                process_id,
                parent_process_id,
                process_name,
                started_at: (!started_at.is_empty()).then_some(started_at),
            })
        })
        .collect()
}

fn process_ancestor_ids(rows: &[ProcessActivityRow], process_id: u32) -> HashSet<u32> {
    let parents = rows
        .iter()
        .map(|row| (row.process_id, row.parent_process_id))
        .collect::<HashMap<_, _>>();
    let mut excluded = HashSet::new();
    let mut candidate = Some(process_id);
    while let Some(current) = candidate {
        if current == 0 || !excluded.insert(current) {
            break;
        }
        candidate = parents.get(&current).copied();
    }
    excluded
}

fn process_activity_signals(path: &Path) -> (Vec<ActivitySignal>, bool) {
    #[cfg(target_os = "windows")]
    {
        let _ = path;
        return (
            vec![activity_signal(
                "Process",
                "Activity state uncertain",
                "Low",
                None,
                None,
                None,
                None,
            )],
            false,
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = match Command::new("ps")
            .args(["-axo", "pid=,ppid=,comm=,lstart="])
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => {
                return (
                    vec![activity_signal(
                        "Process",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )],
                    false,
                );
            }
        };
        let rows = parse_process_activity_rows(&String::from_utf8_lossy(&output.stdout));
        let invoking_process_ids = process_ancestor_ids(&rows, std::process::id());
        let mut signals = Vec::new();
        let mut unresolved_candidate = false;
        for row in rows {
            if invoking_process_ids.contains(&row.process_id) {
                continue;
            }
            if !process_name_is_activity_candidate(&row.process_name) {
                continue;
            }
            let Some(working_directory) = process_working_directory(row.process_id) else {
                unresolved_candidate = true;
                continue;
            };
            if workspace_contains(path, &working_directory) {
                signals.push(activity_signal(
                    "Process",
                    "Process evidence found",
                    "Medium",
                    Some(&row.process_name),
                    Some(row.process_id),
                    row.started_at.as_deref(),
                    Some(&working_directory),
                ));
            }
        }
        if signals.is_empty() && unresolved_candidate {
            signals.push(activity_signal(
                "Process",
                "Activity state uncertain",
                "Low",
                None,
                None,
                None,
                None,
            ));
            return (signals, false);
        }
        (signals, true)
    }
}

fn workspace_activity_state(
    manifest_present: bool,
    manifest_active: bool,
    process_active: bool,
    process_inspection_complete: bool,
    uncertain: bool,
    dirty: bool,
    ahead: u64,
) -> &'static str {
    if manifest_active || process_active {
        "Active"
    } else if dirty {
        "Interrupted with dirty work"
    } else if ahead > 0 {
        "Interrupted with unpushed commits"
    } else if manifest_present {
        "Recently active"
    } else if process_inspection_complete && !uncertain {
        "Idle"
    } else {
        "Unknown"
    }
}

fn collect_workspace_activity(path: &Path, dirty: bool, ahead: u64) -> WorkspaceActivity {
    let (manifest, manifest_signal) = read_agent_manifest(path);
    let (mut signals, process_inspection_complete) = process_activity_signals(path);
    if let Some(signal) = manifest_signal {
        signals.push(signal);
    }
    let manifest_active = manifest
        .as_ref()
        .and_then(|manifest| manifest.status.as_deref())
        .is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "active" | "running" | "started"
            )
        });
    let process_active = signals
        .iter()
        .any(|signal| signal.summary == "Process evidence found");
    let uncertain = signals
        .iter()
        .any(|signal| signal.summary == "Activity state uncertain");
    if !process_active && process_inspection_complete && !uncertain {
        signals.push(activity_signal(
            "Process",
            "No associated process detected",
            "Medium",
            None,
            None,
            None,
            None,
        ));
    }
    let state = workspace_activity_state(
        manifest.is_some(),
        manifest_active,
        process_active,
        process_inspection_complete,
        uncertain,
        dirty,
        ahead,
    );
    let confidence = if manifest_active {
        "High"
    } else if process_active {
        "Medium"
    } else if uncertain {
        "Low"
    } else {
        "Medium"
    };
    WorkspaceActivity {
        state: state.to_string(),
        confidence: confidence.to_string(),
        signals,
        manifest,
    }
}
