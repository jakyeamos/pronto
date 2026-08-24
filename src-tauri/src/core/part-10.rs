impl GitHubCliAdapter {
    fn for_repository_names(repository_names: HashSet<String>) -> Self {
        Self {
            executable: "gh".to_string(),
            target_repository_names: Some(repository_names),
        }
    }

    fn failure_message(stderr: &[u8]) -> String {
        let detail = String::from_utf8_lossy(stderr).to_ascii_lowercase();
        let network_failure = [
            "check your internet connection",
            "could not resolve host",
            "error connecting to api.github.com",
            "failed to connect",
            "network is unreachable",
            "no such host",
            "operation timed out",
            "proxyconnect tcp",
            "temporary failure in name resolution",
            "tls handshake timeout",
        ]
        .iter()
        .any(|marker| detail.contains(marker));
        if network_failure {
            return "GitHub provider unavailable: GitHub CLI could not reach GitHub; authentication was not verified. Check network access and retry.".to_string();
        }

        let authentication_failure = [
            "401 unauthorized",
            "bad credentials",
            "failed to log in",
            "http 401",
            "login required",
            "not logged in",
            "requires authentication",
            "status 401",
            "the token in",
            "token is expired",
            "token is invalid",
        ]
        .iter()
        .any(|marker| detail.contains(marker));
        if authentication_failure {
            return "GitHub provider unavailable: GitHub CLI authentication is invalid or expired. Run `gh auth status` and reauthenticate if needed.".to_string();
        }

        "GitHub provider unavailable: GitHub CLI request failed; authentication and network status could not be determined.".to_string()
    }

    fn json(&self, arguments: &[&str]) -> Result<serde_json::Value, String> {
        let output = Command::new(&self.executable)
            .args(arguments)
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    "GitHub provider unavailable: GitHub CLI is not installed or not on PATH."
                        .to_string()
                } else {
                    format!("GitHub provider unavailable: could not start GitHub CLI ({error}).")
                }
            })?;
        if !output.status.success() {
            return Err(Self::failure_message(&output.stderr));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_| "GitHub provider returned invalid JSON.".to_string())
    }
}

impl ProviderAdapter for GitHubCliAdapter {
    fn provider_id(&self) -> &str {
        "github"
    }

    fn refresh(&self) -> Result<ProviderRefresh, String> {
        let refreshed_at = iso_now();
        let identity_payload = self.json(&["api", "user"])?;
        let login = identity_payload
            .get("login")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "GitHub provider did not return an authenticated login.".to_string())?;
        let identity_id = format!("github:{login}");
        let identity = ProviderIdentity {
            id: identity_id.clone(),
            provider: self.provider_id().to_string(),
            login: login.to_string(),
            display_name: identity_payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            organizations: Vec::new(),
            credential_state: "Authenticated".to_string(),
            updated_at: refreshed_at.clone(),
        };
        let repositories_payload = self.json(&["api", "user/repos", "--paginate", "--slurp"])?;
        let mut repositories =
            parse_github_repositories(&repositories_payload, &identity_id, &refreshed_at)?;
        let mut pull_requests = Vec::new();
        let mut releases = Vec::new();
        let mut ci_updates = HashMap::<String, (Vec<CheckSnapshot>, String, Option<String>)>::new();
        let mut ci_run_updates = HashMap::<String, Vec<CiRunSnapshot>>::new();
        let repositories_to_refresh = repositories
            .iter()
            .filter(|repository| {
                self.target_repository_names
                    .as_ref()
                    .map(|names| {
                        normalize_remote_name(&repository.full_name)
                            .is_some_and(|name| names.contains(&name))
                    })
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        for repository in repositories_to_refresh {
            let pull_request_endpoint = format!(
                "repos/{}/pulls?state=all&per_page=100",
                repository.full_name
            );
            if let Ok(payload) = self.json(&[
                "api",
                pull_request_endpoint.as_str(),
                "--paginate",
                "--slurp",
            ]) {
                if let Ok(parsed) =
                    parse_github_pull_requests(&payload, &repository.id, &refreshed_at)
                {
                    let mut parsed = parsed;
                    for pull_request in &mut parsed {
                        let Some(head_commit) = pull_request.head_commit.as_deref() else {
                            continue;
                        };
                        let check_endpoint = format!(
                            "repos/{}/commits/{head_commit}/check-runs?per_page=100",
                            repository.full_name
                        );
                        if let Ok(check_payload) = self.json(&["api", check_endpoint.as_str()]) {
                            if let Ok(checks) =
                                parse_github_check_runs(&check_payload, &refreshed_at)
                            {
                                pull_request.checks_state = summarize_check_state(&checks);
                                pull_request.checks = checks;
                            }
                        }
                    }
                    pull_requests.extend(parsed);
                }
            }
            if let Some(default_branch) = repository.default_branch.as_deref() {
                let check_endpoint = format!(
                    "repos/{}/commits/{default_branch}/check-runs?per_page=100",
                    repository.full_name
                );
                if let Ok(payload) = self.json(&["api", check_endpoint.as_str()]) {
                    if let Ok(checks) = parse_github_check_runs(&payload, &refreshed_at) {
                        let ci_commit = checks.iter().find_map(|check| check.head_sha.clone());
                        ci_updates.insert(
                            repository.id.clone(),
                            (checks, default_branch.to_string(), ci_commit),
                        );
                    }
                }
            }
            let workflow_endpoint = format!(
                "repos/{}/actions/runs?per_page={CI_RUN_LIMIT}",
                repository.full_name
            );
            if let Ok(payload) = self.json(&["api", workflow_endpoint.as_str()]) {
                if let Ok(mut runs) =
                    parse_github_workflow_runs(&payload, &repository.full_name, &refreshed_at)
                {
                    for (index, run) in runs.iter_mut().enumerate() {
                        if run.status != "completed" || ci_run_needs_artifact(run) {
                            let jobs_endpoint = format!(
                                "repos/{}/actions/runs/{}/jobs?per_page=100",
                                repository.full_name, run.id
                            );
                            if let Ok(job_payload) = self.json(&["api", jobs_endpoint.as_str()]) {
                                if let Ok(jobs) = parse_github_jobs(&job_payload) {
                                    run.jobs = jobs;
                                }
                            }
                        }
                        run.failure_summary = summarize_ci_failure(run);
                        run.failure_signature = ci_failure_signature(run);
                        if ci_run_needs_artifact(run) && index < CI_ARTIFACT_LOOKUP_LIMIT {
                            let artifact_endpoint = format!(
                                "repos/{}/actions/runs/{}/artifacts?per_page=100",
                                repository.full_name, run.id
                            );
                            if let Ok(artifact_payload) =
                                self.json(&["api", artifact_endpoint.as_str()])
                            {
                                run.prompt_artifact = parse_ci_prompt_artifact(
                                    &artifact_payload,
                                    run.id,
                                    run.run_attempt,
                                );
                            }
                        }
                    }
                    ci_run_updates.insert(repository.id.clone(), runs);
                }
            }
            let release_endpoint = format!("repos/{}/releases?per_page=100", repository.full_name);
            if let Ok(payload) =
                self.json(&["api", release_endpoint.as_str(), "--paginate", "--slurp"])
            {
                if let Ok(parsed) = parse_github_releases(&payload, &repository.id, &refreshed_at) {
                    releases.extend(parsed);
                }
            }
        }
        for repository in &mut repositories {
            if let Some((checks, branch, commit)) = ci_updates.remove(&repository.id) {
                repository.ci_checks = checks;
                repository.ci_branch = Some(branch);
                repository.ci_commit = commit;
            }
            if let Some(runs) = ci_run_updates.remove(&repository.id) {
                repository.ci_runs = runs;
            }
            repository.pull_requests = pull_requests
                .iter()
                .filter(|pull_request| pull_request.repository_id == repository.id)
                .cloned()
                .collect();
            repository.releases = releases
                .iter()
                .filter(|release| release.repository_id == repository.id)
                .cloned()
                .collect();
        }
        Ok(ProviderRefresh {
            identities: vec![identity],
            repositories,
            pull_requests,
            releases,
            refreshed_at,
        })
    }
}

fn parse_github_repositories(
    payload: &serde_json::Value,
    identity_id: &str,
    refreshed_at: &str,
) -> Result<Vec<RemoteRepositorySnapshot>, String> {
    let pages = github_array_items(payload, "repository")?;
    Ok(pages
        .into_iter()
        .filter_map(|repository| {
            let full_name = repository.get("full_name")?.as_str()?.to_string();
            let owner = repository
                .get("owner")
                .and_then(|value| value.get("login"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let name = repository
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| full_name.rsplit('/').next().unwrap_or_default())
                .to_string();
            Some(RemoteRepositorySnapshot {
                id: format!(
                    "github:{}",
                    repository
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| full_name.clone())
                ),
                provider: "github".to_string(),
                full_name,
                name,
                owner,
                html_url: repository
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                default_branch: repository
                    .get("default_branch")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                archived: repository
                    .get("archived")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                locality: "Remote only".to_string(),
                identity_id: identity_id.to_string(),
                last_refreshed_at: refreshed_at.to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
                ci_runs: Vec::new(),
            })
        })
        .collect())
}

fn github_array_items<'a>(
    payload: &'a serde_json::Value,
    resource: &str,
) -> Result<Vec<&'a serde_json::Value>, String> {
    match payload {
        serde_json::Value::Array(values) if values.iter().all(serde_json::Value::is_array) => {
            Ok(values
                .iter()
                .flat_map(|page| page.as_array().into_iter().flatten())
                .collect::<Vec<_>>())
        }
        serde_json::Value::Array(values) => Ok(values.iter().collect::<Vec<_>>()),
        _ => Err(format!("GitHub {resource} response was not an array.")),
    }
}

fn parse_github_pull_requests(
    payload: &serde_json::Value,
    repository_id: &str,
    refreshed_at: &str,
) -> Result<Vec<PullRequestSnapshot>, String> {
    Ok(github_array_items(payload, "pull-request")?
        .into_iter()
        .filter_map(|pull_request| {
            let number = pull_request
                .get("number")
                .and_then(serde_json::Value::as_u64)?;
            let head_branch = pull_request
                .get("head")
                .and_then(|value| value.get("ref"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let base_branch = pull_request
                .get("base")
                .and_then(|value| value.get("ref"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let head_commit = pull_request
                .get("head")
                .and_then(|value| value.get("sha"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            Some(PullRequestSnapshot {
                id: format!("github:pr:{repository_id}:{number}"),
                provider: "github".to_string(),
                repository_id: repository_id.to_string(),
                number,
                html_url: pull_request
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                title: pull_request
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                head_branch,
                base_branch,
                state: pull_request
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                draft: pull_request
                    .get("draft")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                checks_state: "Unknown — provider snapshot unavailable".to_string(),
                reviews_state: "Unknown — provider snapshot unavailable".to_string(),
                mergeability: pull_request
                    .get("mergeable_state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unknown — provider snapshot unavailable")
                    .to_string(),
                checks: Vec::new(),
                last_refreshed_at: refreshed_at.to_string(),
                head_commit,
            })
        })
        .collect())
}

fn parse_github_check_runs(
    payload: &serde_json::Value,
    refreshed_at: &str,
) -> Result<Vec<CheckSnapshot>, String> {
    let values = match payload {
        serde_json::Value::Array(values) if values.iter().all(serde_json::Value::is_array) => {
            values
                .iter()
                .flat_map(|page| page.as_array().into_iter().flatten())
                .collect::<Vec<_>>()
        }
        serde_json::Value::Array(values) => values.iter().collect::<Vec<_>>(),
        serde_json::Value::Object(_) => vec![payload],
        _ => return Err("GitHub check-run response was not an object or array.".to_string()),
    };
    Ok(values
        .into_iter()
        .flat_map(|value| {
            value
                .get("check_runs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
        .filter_map(|check| {
            let context = check
                .get("name")
                .or_else(|| check.get("context"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            Some(CheckSnapshot {
                context,
                state: check
                    .get("status")
                    .or_else(|| check.get("state"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                required: false,
                conclusion: check
                    .get("conclusion")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                last_refreshed_at: refreshed_at.to_string(),
                html_url: check
                    .get("html_url")
                    .or_else(|| check.get("details_url"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                head_sha: check
                    .get("head_sha")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect())
}

fn github_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
            .or_else(|| value.as_str().and_then(|number| number.parse::<u64>().ok()))
    })
}
