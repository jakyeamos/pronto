fn parse_github_workflow_runs(
    payload: &serde_json::Value,
    repository_full_name: &str,
    refreshed_at: &str,
) -> Result<Vec<CiRunSnapshot>, String> {
    let runs = payload
        .get("workflow_runs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "GitHub workflow-run response did not contain workflow_runs.".to_string())?;
    Ok(runs
        .iter()
        .take(CI_RUN_LIMIT)
        .filter_map(|run| {
            let id = github_u64(run.get("id"))?;
            let base_repository = run
                .get("repository")
                .and_then(|repository| repository.get("full_name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(repository_full_name);
            let head_repository = run
                .get("head_repository")
                .and_then(|repository| repository.get("full_name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(base_repository);
            let pull_request_number = run
                .get("pull_requests")
                .and_then(serde_json::Value::as_array)
                .and_then(|pull_requests| pull_requests.first())
                .and_then(|pull_request| github_u64(pull_request.get("number")));
            Some(CiRunSnapshot {
                id,
                workflow_name: run
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unnamed workflow")
                    .to_string(),
                workflow_path: run
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                display_title: run
                    .get("display_title")
                    .or_else(|| run.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unnamed workflow run")
                    .to_string(),
                run_number: github_u64(run.get("run_number")).unwrap_or_default(),
                run_attempt: github_u64(run.get("run_attempt")).unwrap_or(1),
                event: run
                    .get("event")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                status: run
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                conclusion: run
                    .get("conclusion")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                head_branch: run
                    .get("head_branch")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                head_sha: run
                    .get("head_sha")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                html_url: run
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                created_at: run
                    .get("created_at")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                updated_at: run
                    .get("updated_at")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                pull_request_number,
                is_fork: normalize_remote_name(head_repository)
                    .zip(normalize_remote_name(base_repository))
                    .is_some_and(|(head, base)| head != base),
                jobs: Vec::new(),
                failure_summary: None,
                failure_signature: None,
                prompt_artifact: None,
                last_refreshed_at: refreshed_at.to_string(),
            })
        })
        .collect())
}

fn parse_github_jobs(payload: &serde_json::Value) -> Result<Vec<CiJobSnapshot>, String> {
    let jobs = payload
        .get("jobs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "GitHub job response did not contain jobs.".to_string())?;
    Ok(jobs
        .iter()
        .filter_map(|job| {
            let id = github_u64(job.get("id"))?;
            let failed_steps = job
                .get("steps")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter(|step| {
                    !matches!(
                        step.get("conclusion").and_then(serde_json::Value::as_str),
                        Some("success" | "skipped" | "neutral" | "cancelled")
                    )
                })
                .filter_map(|step| {
                    step.get("name")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .take(5)
                .collect::<Vec<_>>();
            Some(CiJobSnapshot {
                id,
                name: job
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unnamed job")
                    .to_string(),
                status: job
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                conclusion: job
                    .get("conclusion")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                html_url: job
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                failed_steps,
            })
        })
        .collect())
}

fn ci_conclusion_is_failure(conclusion: Option<&str>) -> bool {
    matches!(
        conclusion,
        Some(
            "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure" | "stale"
        )
    )
}

fn ci_run_needs_artifact(run: &CiRunSnapshot) -> bool {
    ci_conclusion_is_failure(run.conclusion.as_deref())
}

fn summarize_ci_failure(run: &CiRunSnapshot) -> Option<String> {
    if !ci_run_needs_artifact(run) {
        return None;
    }
    let details = run
        .jobs
        .iter()
        .filter(|job| ci_conclusion_is_failure(job.conclusion.as_deref()))
        .take(3)
        .map(|job| {
            if job.failed_steps.is_empty() {
                format!(
                    "{} ({})",
                    job.name,
                    job.conclusion.as_deref().unwrap_or(job.status.as_str())
                )
            } else {
                format!("{}: {}", job.name, job.failed_steps.join(", "))
            }
        })
        .collect::<Vec<_>>();
    if details.is_empty() {
        Some(format!(
            "{} concluded {}",
            run.workflow_name,
            run.conclusion.as_deref().unwrap_or("unsuccessfully")
        ))
    } else {
        Some(details.join("; "))
    }
}

fn ci_failure_signature(run: &CiRunSnapshot) -> Option<String> {
    let summary = run.failure_summary.as_deref()?;
    let material = format!("{}|{}|{}", run.workflow_name, run.head_sha, summary);
    let digest = Sha256::digest(material.as_bytes());
    let short_digest = digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(format!("ci-{short_digest}"))
}

fn parse_ci_prompt_artifact(
    payload: &serde_json::Value,
    run_id: u64,
    run_attempt: u64,
) -> Option<CiPromptArtifactSnapshot> {
    let artifacts = payload
        .get("artifacts")
        .and_then(serde_json::Value::as_array)?;
    let expected = format!("codex-ci-prompt-{run_id}-{run_attempt}");
    let legacy = format!("codex-ci-prompt-{run_id}");
    artifacts.iter().find_map(|artifact| {
        let name = artifact.get("name").and_then(serde_json::Value::as_str)?;
        let exact_name = name == expected || (run_attempt == 1 && name == legacy);
        let expired = artifact
            .get("expired")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !exact_name || expired {
            return None;
        }
        Some(CiPromptArtifactSnapshot {
            id: github_u64(artifact.get("id"))?,
            name: name.to_string(),
            expired,
        })
    })
}

fn summarize_check_state(checks: &[CheckSnapshot]) -> String {
    if checks.is_empty() {
        return "Not configured".to_string();
    }
    if checks.iter().any(|check| {
        matches!(
            check.conclusion.as_deref(),
            Some("failure" | "timed_out" | "cancelled" | "action_required")
        )
    }) {
        "Failed".to_string()
    } else if checks
        .iter()
        .all(|check| matches!(check.conclusion.as_deref(), Some("success" | "neutral")))
    {
        "Passed".to_string()
    } else {
        "Blocked".to_string()
    }
}

fn parse_github_releases(
    payload: &serde_json::Value,
    repository_id: &str,
    refreshed_at: &str,
) -> Result<Vec<ReleaseSnapshot>, String> {
    Ok(github_array_items(payload, "release")?
        .into_iter()
        .filter_map(|release| {
            let id = release
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .map(|value| value.to_string())
                .or_else(|| {
                    release
                        .get("tag_name")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })?;
            Some(ReleaseSnapshot {
                id: format!("github:release:{repository_id}:{id}"),
                provider: "github".to_string(),
                repository_id: repository_id.to_string(),
                tag: release
                    .get("tag_name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: release
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                target_commit: release
                    .get("target_commitish")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                published_at: release
                    .get("published_at")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                draft: release
                    .get("draft")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                prerelease: release
                    .get("prerelease")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                last_refreshed_at: refreshed_at.to_string(),
            })
        })
        .collect())
}

fn normalize_remote_name(value: &str) -> Option<String> {
    let mut normalized = value.trim().trim_end_matches('/').to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some(value) = normalized.strip_prefix("git@github.com:") {
        normalized = value.to_string();
    } else {
        for prefix in ["https://github.com/", "http://github.com/", "github.com/"] {
            if let Some(value) = normalized.strip_prefix(prefix) {
                normalized = value.to_string();
                break;
            }
        }
    }
    Some(normalized.trim_end_matches(".git").to_string())
}

fn quality_runner_identity_key(repository: &RepositorySnapshot) -> String {
    if let Some(remote) = repository.remote_url.as_deref() {
        let mut normalized = remote.trim().trim_end_matches('/').to_ascii_lowercase();
        if let Some(value) = normalized.strip_prefix("git@") {
            if let Some((host, path)) = value.split_once(':') {
                normalized = format!("{host}/{path}");
            }
        } else {
            for prefix in ["https://", "http://", "ssh://"] {
                if let Some(value) = normalized.strip_prefix(prefix) {
                    normalized = value.trim_start_matches('/').to_string();
                    if let Some(value) = normalized.strip_prefix("git@") {
                        normalized = value.to_string();
                    }
                    break;
                }
            }
            if let Some((host, path)) = normalized.split_once(':') {
                if !path.contains('/') || host.contains('.') {
                    normalized = format!("{host}/{path}");
                }
            }
        }
        return format!(
            "origin:{}",
            normalized.trim_start_matches('/').trim_end_matches(".git")
        );
    }
    let common = git_static(
        Path::new(&repository.path),
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .and_then(|path| canonical_path(Path::new(&path)).or_else(|| Some(PathBuf::from(path))));
    common
        .map(|path| format!("common:{}", path.display()))
        .unwrap_or_else(|| format!("path:{}", repository.path))
}

fn repository_feed_id(repository: &RepositorySnapshot) -> String {
    let identity = quality_runner_identity_key(repository);
    let payload = serde_json::to_string(&[identity]).unwrap_or_else(|_| "[]".to_string());
    let digest = Sha256::digest(payload.as_bytes());
    let hex = format!("{digest:x}");
    format!("repo-{}", &hex[..16])
}

fn classify_remote_repositories(
    repositories: &[RepositorySnapshot],
    remote_repositories: Vec<RemoteRepositorySnapshot>,
) -> Vec<RemoteRepositorySnapshot> {
    let local_names = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .remote_url
                .as_deref()
                .and_then(normalize_remote_name)
        })
        .collect::<HashSet<_>>();

    remote_repositories
        .into_iter()
        .filter_map(|mut remote| {
            let normalized_name = normalize_remote_name(&remote.full_name)?;
            remote.locality = if local_names.contains(&normalized_name) {
                "Local and remote".to_string()
            } else if remote.provider.eq_ignore_ascii_case("github") {
                remediation::GITHUB_ONLY_LOCALITY.to_string()
            } else {
                "Remote only".to_string()
            };
            Some(remote)
        })
        .collect()
}
