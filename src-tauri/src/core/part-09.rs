fn metric_axis_compatible(definitions: &[MetricDefinition], metric_ids: &[String]) -> bool {
    let selected = metric_ids
        .iter()
        .filter_map(|id| definitions.iter().find(|metric| &metric.id == id))
        .collect::<Vec<_>>();
    selected.first().is_none_or(|first| {
        selected.iter().all(|metric| {
            metric.unit == first.unit
                && metric.denominator == first.denominator
                && metric.aggregation == first.aggregation
                && metric.time_semantics == first.time_semantics
                && metric.window_days == first.window_days
        })
    })
}

fn builtin_analytics_view(range_days: i64) -> AnalyticsView {
    let now = iso_now();
    AnalyticsView {
        schema_version: "pronto-analytics-view/v1".to_string(),
        id: "curated".to_string(),
        name: "Curated evidence story".to_string(),
        builtin: true,
        is_default: true,
        filters: AnalyticsViewFilters {
            range_days,
            repository_ids: vec![],
            group_ids: vec![],
            product_ids: vec![],
            freshness: "all".to_string(),
        },
        widgets: vec![],
        created_at: now.clone(),
        updated_at: now,
    }
}

fn load_analytics_views(
    connection: &SqliteConnection,
    range_days: i64,
) -> Result<Vec<AnalyticsView>, String> {
    let mut views = vec![builtin_analytics_view(range_days)];
    let mut statement = connection
        .prepare("SELECT payload_json, is_default FROM analytics_views ORDER BY name, id")
        .map_err(|error| format!("Could not prepare analytics views query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| format!("Could not read analytics views: {error}"))?;
    for row in rows {
        let (payload, is_default) =
            row.map_err(|error| format!("Could not decode analytics view row: {error}"))?;
        let mut view: AnalyticsView = serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode analytics view: {error}"))?;
        view.is_default = is_default != 0;
        views.push(view);
    }
    if views.iter().skip(1).any(|view| view.is_default) {
        views[0].is_default = false;
    }
    Ok(views)
}

fn deterministic_analytics_findings(
    samples: &[AnalyticsMetricSample],
    repositories: &[AnalyticsRepositorySeries],
) -> Vec<AnalyticsFinding> {
    let mut findings = Vec::new();
    if samples.len() < 2 {
        findings.push(AnalyticsFinding {
            id: "insufficient-history".to_string(),
            kind: "coverage-gap".to_string(),
            severity: "info".to_string(),
            title: "More history is needed".to_string(),
            detail:
                "At least two refresh observations are required before Pronto can describe changes."
                    .to_string(),
            metric_ids: vec![],
            repository_id: None,
            observed_at: samples.last().map(|sample| sample.observed_at.clone()),
        });
    }
    if let Some(latest) = samples.last() {
        if matches!(
            latest.quality_freshness.as_deref(),
            Some("Stale") | Some("Conflicted")
        ) {
            findings.push(AnalyticsFinding { id: "quality-evidence-state".to_string(), kind: if latest.quality_freshness.as_deref() == Some("Conflicted") { "conflict" } else { "stale" }.to_string(), severity: "attention".to_string(), title: format!("Quality evidence is {}", latest.quality_freshness.as_deref().unwrap_or("unavailable").to_lowercase()), detail: "The chart retains the observation and labels its evidence state; it does not infer a cause.".to_string(), metric_ids: vec!["quality.maturity_score".to_string(), "quality.evidence_score".to_string()], repository_id: None, observed_at: Some(latest.observed_at.clone()) });
        }
    }
    let missing = repositories
        .iter()
        .filter(|series| {
            series.samples.last().is_none_or(|sample| {
                sample.ci_readiness_score.is_none() || sample.maturity_score.is_none()
            })
        })
        .count();
    if missing > 0 {
        findings.push(AnalyticsFinding {
            id: "quality-coverage-gap".to_string(),
            kind: "coverage-gap".to_string(),
            severity: "attention".to_string(),
            title: format!("{missing} repositories lack quality coverage"),
            detail: "No score is shown where timestamped quality evidence is unavailable."
                .to_string(),
            metric_ids: vec![
                "quality.maturity_score".to_string(),
                "quality.evidence_score".to_string(),
            ],
            repository_id: None,
            observed_at: samples.last().map(|sample| sample.observed_at.clone()),
        });
    }
    findings
}

fn validated_analytics_range(requested: Option<i64>, retention_days: i64) -> Result<i64, String> {
    let requested = requested.unwrap_or(ANALYTICS_RANGE_DAYS);
    if requested < ANALYTICS_MIN_RANGE_DAYS {
        return Err("Analytics range must be at least 1 day".to_string());
    }
    Ok(requested.min(retention_days.max(1)))
}

fn load_analytics_at(
    path: &Path,
    requested_range_days: Option<i64>,
) -> Result<AnalyticsSnapshot, String> {
    let state = load_store(path)?;
    let connection = open_store(path)?;
    let range_days = validated_analytics_range(requested_range_days, state.retention_days)?;
    let range_cutoff = Utc::now() - chrono::Duration::days(range_days);
    let retention_cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    let cutoff = range_cutoff.max(retention_cutoff);
    let cutoff = cutoff.to_rfc3339_opts(SecondsFormat::Secs, true);
    let portfolio_samples = load_analytics_samples(&connection, None, &cutoff)?;
    let repositories = state
        .repositories
        .iter()
        .map(|repository| {
            let scope_id = analytics_scope_id(repository.id.as_str());
            Ok(AnalyticsRepositorySeries {
                repository_id: repository.id.clone(),
                name: repository.name.clone(),
                samples: load_analytics_samples(&connection, Some(scope_id.as_str()), &cutoff)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let latest_observed_at = portfolio_samples
        .last()
        .map(|sample| sample.observed_at.clone())
        .or_else(|| {
            repositories
                .iter()
                .flat_map(|repository| repository.samples.last())
                .map(|sample| sample.observed_at.clone())
                .max()
        });
    let history_available_from = portfolio_samples
        .first()
        .map(|sample| sample.observed_at.clone())
        .or_else(|| {
            repositories
                .iter()
                .flat_map(|repository| repository.samples.first())
                .map(|sample| sample.observed_at.clone())
                .min()
        });
    let metric_catalog = analytics_metric_catalog();
    let findings = deterministic_analytics_findings(&portfolio_samples, &repositories);
    let views = load_analytics_views(&connection, range_days)?;
    let default_view_id = views
        .iter()
        .find(|view| view.is_default)
        .map(|view| view.id.clone())
        .unwrap_or_else(|| "curated".to_string());
    Ok(AnalyticsSnapshot {
        schema_version: ANALYTICS_SCHEMA.to_string(),
        generated_at: iso_now(),
        source: "Local refresh snapshots".to_string(),
        freshness: latest_observed_at
            .map(|observed_at| format!("Observed through {observed_at}"))
            .unwrap_or_else(|| "Unavailable until the first local refresh".to_string()),
        range_days,
        retention_days: state.retention_days,
        history_available_from,
        portfolio_samples,
        repositories,
        metric_catalog,
        findings,
        views,
        default_view_id,
    })
}

fn snapshot_from_store(path: &Path, state: &StoreState) -> PortfolioSnapshot {
    let mut repositories = state.repositories.clone();
    sort_repositories_by_name(&mut repositories);
    for repository in &mut repositories {
        hydrate_workspace_sync_details(repository);
        repository.quality.behavior_assurance.normalize_state();
    }
    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    let showcase = showcase::inspect(&repositories);
    PortfolioSnapshot {
        roots: state.roots.clone(),
        repositories,
        products: state.products.clone(),
        groups: state.groups.clone(),
        events: state.events.iter().rev().take(24).cloned().collect(),
        action_audits: state.action_audits.iter().rev().take(24).cloned().collect(),
        provider_identities: state.provider_identities.clone(),
        remote_repositories: state.remote_repositories.clone(),
        provider_status: state.provider_status.clone(),
        quality: state.quality.clone(),
        remediation: remediation_run,
        showcase,
        retention_days: state.retention_days,
        generated_at: iso_now(),
        storage_path: path.to_string_lossy().to_string(),
    }
}

fn shell_quote_for_display(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn workspace_sync_reason(workspace: &WorkspaceSummary) -> String {
    if !workspace.status_available {
        return workspace_status_unavailable_reason(workspace);
    }
    match (
        workspace.upstream.as_deref(),
        workspace.ahead,
        workspace.behind,
    ) {
        (None, _, _) => format!(
            "Workspace branch '{}' has no tracked upstream, so Pronto cannot compare it to a remote branch.",
            workspace.branch
        ),
        (Some(upstream), ahead, behind) if ahead > 0 && behind > 0 => format!(
            "Workspace branch '{}' is ahead by {} commit{} and behind by {} commit{} relative to '{}'.",
            workspace.branch,
            ahead,
            if ahead == 1 { "" } else { "s" },
            behind,
            if behind == 1 { "" } else { "s" },
            upstream
        ),
        (Some(upstream), ahead, _) if ahead > 0 => format!(
            "Workspace branch '{}' is ahead by {} commit{} relative to '{}'.",
            workspace.branch,
            ahead,
            if ahead == 1 { "" } else { "s" },
            upstream
        ),
        (Some(upstream), _, behind) if behind > 0 => format!(
            "Workspace branch '{}' is behind by {} commit{} relative to '{}'.",
            workspace.branch,
            behind,
            if behind == 1 { "" } else { "s" },
            upstream
        ),
        _ => format!(
            "Pronto recorded sync state '{}' for workspace branch '{}', but the comparison counts are unavailable.",
            workspace.sync_state, workspace.branch
        ),
    }
}

fn workspace_status_unavailable_reason(workspace: &WorkspaceSummary) -> String {
    workspace
        .status_error
        .clone()
        .unwrap_or_else(|| "Git status could not be established for this workspace.".to_string())
}

fn workspace_sync_detail(
    workspace: &WorkspaceSummary,
    repository_path: &str,
    observed_at: &str,
) -> Option<WorkspaceSyncDetail> {
    if !workspace_requires_sync_attention(workspace) {
        return None;
    }

    let observed = DateTime::parse_from_rfc3339(observed_at)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc));
    let evidence_observed_at =
        observed.map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    let evidence_expires_at = observed.map(|timestamp| {
        (timestamp + Duration::minutes(WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES))
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });

    Some(WorkspaceSyncDetail {
        reason: workspace_sync_reason(workspace),
        evidence_observed_at,
        evidence_expires_at,
        evidence_window_minutes: WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES,
        next_safe_action: "Run the repository-scoped local refresh command below, then reopen this detail to compare the newly observed evidence. Do not choose a merge, rebase, pull, or push from this view.".to_string(),
        scoped_refresh_command: format!(
            "pronto refresh {} --json",
            shell_quote_for_display(repository_path)
        ),
        authorization: "Read-only local Git scan; it persists Pronto evidence only and does not pull, push, merge, rebase, or edit repository files.".to_string(),
    })
}

fn hydrate_workspace_sync_details(repository: &mut RepositorySnapshot) {
    let repository_path = repository.path.clone();
    let observed_at = repository.last_scan_at.clone();

    if repository.workspaces.is_empty() {
        repository.workspace.sync_detail =
            workspace_sync_detail(&repository.workspace, &repository_path, &observed_at);
        return;
    }

    for workspace in &mut repository.workspaces {
        workspace.sync_detail = workspace_sync_detail(workspace, &repository_path, &observed_at);
    }
    repository.workspace.sync_detail = repository
        .workspaces
        .iter()
        .find(|workspace| workspace.is_primary)
        .or_else(|| {
            repository
                .workspaces
                .iter()
                .find(|workspace| workspace.id == repository.workspace.id)
        })
        .and_then(|workspace| workspace.sync_detail.clone())
        .or_else(|| workspace_sync_detail(&repository.workspace, &repository_path, &observed_at));
}

fn git_process(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE");
    command
}

fn run_git<I, S>(path: &Path, arguments: I) -> Result<GitOutput, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_process(path)
        .args(arguments)
        .output()
        .map_err(|error| format!("Could not run Git in {}: {error}", path.display()))?;
    Ok(GitOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code(),
    })
}

fn run_git_bounded<I, S>(
    path: &Path,
    arguments: I,
    timeout: StdDuration,
) -> Result<GitOutput, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = git_process(path)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run Git in {}: {error}", path.display()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Git stdout was unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Git stderr was unavailable".to_string())?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut stdout = stdout;
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut stderr = stderr;
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started_at = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started_at.elapsed() < timeout => {
                thread::sleep(StdDuration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Git command exceeded the {} second release-preview deadline in {}",
                    timeout.as_secs(),
                    path.display()
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Could not wait for Git in {}: {error}",
                    path.display()
                ));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| "Git stdout reader stopped unexpectedly".to_string())?
        .map_err(|error| format!("Could not read Git stdout: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "Git stderr reader stopped unexpectedly".to_string())?
        .map_err(|error| format!("Could not read Git stderr: {error}"))?;
    Ok(GitOutput {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        exit_code: status.code(),
    })
}

fn git_static(path: &Path, arguments: &[&str]) -> Option<String> {
    let result = run_git(path, arguments.iter()).ok()?;
    if result.success {
        Some(result.stdout.trim().to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct GitHubCliAdapter {
    executable: String,
    target_repository_names: Option<HashSet<String>>,
}

impl Default for GitHubCliAdapter {
    fn default() -> Self {
        Self {
            executable: "gh".to_string(),
            target_repository_names: None,
        }
    }
}
