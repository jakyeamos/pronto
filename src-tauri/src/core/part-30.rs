fn open_external_tool(path: &Path, tool: &str) -> Result<(), String> {
    let tool = tool.trim().to_ascii_lowercase();
    let arguments = match tool.as_str() {
        "file_browser" => vec![path.to_string_lossy().to_string()],
        "terminal" => vec![
            "-a".to_string(),
            "Terminal".to_string(),
            path.to_string_lossy().to_string(),
        ],
        "editor" => vec![
            "-a".to_string(),
            "Visual Studio Code".to_string(),
            path.to_string_lossy().to_string(),
        ],
        "git_client" => vec![
            "-a".to_string(),
            "GitHub Desktop".to_string(),
            path.to_string_lossy().to_string(),
        ],
        _ => return Err("Choose a supported external handoff tool".to_string()),
    };

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/open")
            .args(arguments)
            .output()
            .map_err(|_| "Could not start the external handoff tool".to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "The external handoff tool could not open {}",
                path.display()
            ))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, arguments);
        Err("External handoff is currently implemented for macOS only".to_string())
    }
}

fn open_quality_report_at(path: &Path, report_path: &str) -> Result<PortfolioSnapshot, String> {
    let state = load_store_with_quality(path)?;
    let mut allowed_roots = Vec::new();
    if let Some(audit_root) = state.quality.audit_root.as_deref() {
        allowed_roots.push(PathBuf::from(audit_root));
    }
    if let Some(feed_path) = quality::canonical_maturity_feed_path() {
        allowed_roots.push(feed_path);
    }
    allowed_roots.extend(
        state
            .repositories
            .iter()
            .map(|repository| Path::new(&repository.path).join(".quality-runner")),
    );
    let report = quality::safe_report_path(Path::new(report_path), &allowed_roots)?;
    open_external_tool(&report, "file_browser")?;
    Ok(snapshot_from_store(path, &state))
}

fn open_workspace_at(
    path: &Path,
    repository_id: &str,
    workspace_id: &str,
    tool: &str,
) -> Result<PortfolioSnapshot, String> {
    let state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = repository
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| "Workspace is not registered for this repository".to_string())?;
    let workspace_path = canonical_path(Path::new(&workspace.path))
        .ok_or_else(|| "The workspace path is unavailable".to_string())?;
    if !workspace_path.is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }
    open_external_tool(&workspace_path, tool)?;
    Ok(snapshot_from_store(path, &state))
}

#[tauri::command]
pub fn get_snapshot() -> Result<PortfolioSnapshot, String> {
    let path = store_path();
    let state = load_store(&path)?;
    Ok(snapshot_from_store(&path, &state))
}

#[tauri::command]
pub fn get_analytics(range_days: Option<i64>) -> Result<AnalyticsSnapshot, String> {
    load_analytics_at(&store_path(), range_days)
}

fn validate_analytics_view(view: &AnalyticsView, retention_days: i64) -> Result<(), String> {
    if view.id.trim().is_empty() || view.name.trim().is_empty() {
        return Err("Analytics views require an id and name".to_string());
    }
    if view.id == "curated" || view.builtin {
        return Err("The built-in curated view cannot be overwritten".to_string());
    }
    validated_analytics_range(Some(view.filters.range_days), retention_days)?;
    let catalog = analytics_metric_catalog();
    for widget in &view.widgets {
        if widget.metric_ids.is_empty() {
            return Err(format!(
                "Analytics widget {} must select at least one metric",
                widget.id
            ));
        }
        if widget.chart_type == "dual-axis" {
            return Err("Dual-axis analytics charts are not supported".to_string());
        }
        if !metric_axis_compatible(&catalog, &widget.metric_ids) {
            return Err(format!("Analytics widget {} combines metrics with incompatible units, denominators, aggregations, or time windows", widget.id));
        }
        for metric_id in &widget.metric_ids {
            let metric = catalog
                .iter()
                .find(|metric| &metric.id == metric_id)
                .ok_or_else(|| format!("Unknown analytics metric {metric_id}"))?;
            if !metric.allowed_visualizations.contains(&widget.chart_type) {
                return Err(format!(
                    "Metric {metric_id} does not allow {} charts",
                    widget.chart_type
                ));
            }
        }
        if !(1..=2).contains(&widget.width) || !(1..=2).contains(&widget.height) {
            return Err("Analytics widget dimensions must be 1 or 2".to_string());
        }
    }
    Ok(())
}

fn save_analytics_view_at(
    path: &Path,
    mut view: AnalyticsView,
) -> Result<Vec<AnalyticsView>, String> {
    let state = load_store_read_only(path)?;
    validate_analytics_view(&view, state.retention_days)?;
    let connection = open_store(path)?;
    let now = iso_now();
    if view.created_at.trim().is_empty() {
        view.created_at = now.clone();
    }
    view.updated_at = now;
    view.schema_version = "pronto-analytics-view/v1".to_string();
    if view.is_default {
        connection
            .execute("UPDATE analytics_views SET is_default = 0", [])
            .map_err(|error| format!("Could not clear analytics default view: {error}"))?;
    }
    let payload = serde_json::to_string(&view)
        .map_err(|error| format!("Could not encode analytics view: {error}"))?;
    connection.execute("INSERT INTO analytics_views (id, name, is_default, payload_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name, is_default=excluded.is_default, payload_json=excluded.payload_json, updated_at=excluded.updated_at", params![view.id, view.name, i64::from(view.is_default), payload, view.created_at, view.updated_at]).map_err(|error| format!("Could not save analytics view: {error}"))?;
    load_analytics_views(&connection, view.filters.range_days)
}

#[tauri::command]
pub fn save_analytics_view(view: AnalyticsView) -> Result<Vec<AnalyticsView>, String> {
    save_analytics_view_at(&store_path(), view)
}

#[tauri::command]
pub fn delete_analytics_view(view_id: String) -> Result<Vec<AnalyticsView>, String> {
    if view_id == "curated" {
        return Err("The built-in curated view cannot be deleted".to_string());
    }
    let path = store_path();
    let state = load_store_read_only(&path)?;
    let connection = open_store(&path)?;
    connection
        .execute(
            "DELETE FROM analytics_views WHERE id = ?1",
            params![view_id],
        )
        .map_err(|error| format!("Could not delete analytics view: {error}"))?;
    Ok(load_analytics_views(
        &connection,
        ANALYTICS_RANGE_DAYS.min(state.retention_days),
    )?)
}

fn set_default_analytics_view_at(path: &Path, view_id: &str) -> Result<Vec<AnalyticsView>, String> {
    let state = load_store_read_only(path)?;
    let mut connection = open_store(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not begin analytics view transaction: {error}"))?;
    transaction
        .execute("UPDATE analytics_views SET is_default = 0", [])
        .map_err(|error| format!("Could not clear analytics default view: {error}"))?;
    if view_id != "curated" {
        let changed = transaction
            .execute(
                "UPDATE analytics_views SET is_default = 1 WHERE id = ?1",
                params![view_id],
            )
            .map_err(|error| format!("Could not set analytics default view: {error}"))?;
        if changed == 0 {
            return Err("Analytics view was not found".to_string());
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("Could not commit analytics default view: {error}"))?;
    load_analytics_views(&connection, ANALYTICS_RANGE_DAYS.min(state.retention_days))
}

#[tauri::command]
pub fn set_default_analytics_view(view_id: String) -> Result<Vec<AnalyticsView>, String> {
    set_default_analytics_view_at(&store_path(), &view_id)
}

#[tauri::command]
pub fn get_skills() -> Result<SkillsSnapshot, String> {
    skills::load(&store_path())
}

#[tauri::command]
pub async fn refresh_skills() -> Result<SkillsSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| skills::refresh(&store_path()))
        .await
        .map_err(|error| format!("Skills refresh task failed: {error}"))?
}

#[tauri::command]
pub fn open_skill_source(path: String) -> Result<(), String> {
    skills::open_source(&path)
}

#[tauri::command]
pub fn register_root(path: String) -> Result<PortfolioSnapshot, String> {
    register_root_and_scan(&store_path(), &path)
}

#[tauri::command]
pub async fn refresh() -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| refresh_at(&store_path()))
        .await
        .map_err(|error| format!("Local refresh task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_batch(
    target: Option<String>,
    parallelism: Option<usize>,
) -> Result<RefreshBatchReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        refresh_batch_at(
            &store_path(),
            target.as_deref(),
            parallelism.unwrap_or(DEFAULT_REFRESH_BATCH_PARALLELISM),
        )
    })
    .await
    .map_err(|error| format!("Parallel refresh task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_quality() -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| refresh_quality_at(&store_path()))
        .await
        .map_err(|error| format!("Quality refresh task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_repository_target_evidence(
    repository_id: String,
    target_branch: String,
) -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        refresh_repository_target_evidence_at(&store_path(), &repository_id, &target_branch)
    })
    .await
    .map_err(|error| format!("Target evidence refresh task failed: {error}"))?
}

#[tauri::command]
pub fn refresh_github() -> Result<PortfolioSnapshot, String> {
    refresh_github_at(&store_path())
}

#[tauri::command]
pub fn start_ci_codex_handoff(
    repository: String,
    run_id: u64,
    run_attempt: u64,
) -> Result<CiCodexHandoffReceipt, String> {
    start_ci_codex_handoff_at(&store_path(), &repository, run_id, run_attempt)
}

#[tauri::command]
pub fn refresh_remediation() -> Result<PortfolioSnapshot, String> {
    refresh_remediation_at(
        &store_path(),
        None,
        false,
        true,
        false,
        DEFAULT_QR_AUDIT_TIMEOUT_SECONDS,
    )
}

fn remediation_dependencies_are_terminal<'a>(mut statuses: impl Iterator<Item = &'a str>) -> bool {
    statuses.all(|status| matches!(status, "verified" | "deferred"))
}

fn remediation_action_workspace_id(action: &remediation::RemediationAction) -> Option<&str> {
    [
        "branch_hygiene:activity:",
        "branch_hygiene:operation:",
        "branch_hygiene:dirty:",
        "branch_hygiene:sync:",
        "branch_hygiene:remote-freshness:",
    ]
    .iter()
    .find_map(|prefix| action.stable_key.strip_prefix(prefix))
}

#[tauri::command]
pub fn set_remediation_action_status(
    action_id: String,
    status: String,
    notes: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_status = status.trim().to_ascii_lowercase();
    if !matches!(
        normalized_status.as_str(),
        "open" | "in_progress" | "blocked" | "deferred" | "verified"
    ) {
        return Err(
            "Remediation status must be open, in_progress, blocked, deferred, or verified."
                .to_string(),
        );
    }
    let path = store_path();
    let mut state = load_store_with_quality(&path)?;
    let mut found = false;
    for plan in &mut state.remediation.plans {
        let Some(action_index) = plan
            .actions
            .iter()
            .position(|action| action.id == action_id)
        else {
            continue;
        };
        if normalized_status == "verified" {
            let action = &plan.actions[action_index];
            if action.stable_key != remediation::GITHUB_ONLY_VERIFICATION_ACTION_KEY {
                let repository = state
                    .repositories
                    .iter()
                    .find(|repository| repository.id == plan.repository_id)
                    .ok_or_else(|| {
                        format!(
                            "Repository {} is no longer registered; remediation advancement is blocked.",
                            plan.repository_id
                        )
                    })?;
                let handoff = remediation_handoff_check_for_repository(
                    repository,
                    remediation_action_workspace_id(action),
                )?;
                if !handoff.ready {
                    return Err(format!(
                        "Remediation advancement is blocked by the handoff checkpoint ({}). {}",
                        handoff.status,
                        handoff.reasons.join(" ")
                    ));
                }
            }
            let verification_is_ready = if action.domain == "verification" {
                remediation_dependencies_are_terminal(
                    plan.actions
                        .iter()
                        .filter(|candidate| candidate.id != action.id)
                        .map(|candidate| candidate.status.as_str()),
                ) && plan
                    .actions
                    .iter()
                    .flat_map(|candidate| candidate.evidence.iter())
                    .any(|item| item.freshness.eq_ignore_ascii_case("fresh"))
            } else {
                remediation::action_has_fresh_evidence(action)
            };
            if !verification_is_ready {
                return Err(
                    "An action cannot be verified until its evidence is fresh. Refresh the source and recheck the plan first."
                        .to_string(),
                );
            }
        }
        let action = &mut plan.actions[action_index];
        action.status = normalized_status.clone();
        action.notes = notes.clone();
        action.updated_at = iso_now();
        action.completed_at = (normalized_status == "verified").then(iso_now);
        remediation::recompute_plan_derived(plan);
        found = true;
        break;
    }
    if !found {
        return Err(format!("Remediation action {action_id} was not found."));
    }
    let updated_at = iso_now();
    remediation::normalize_queue(&mut state.remediation, &updated_at);
    state.remediation.generated_at = updated_at;
    save_store(&path, &state)?;
    Ok(snapshot_from_store(&path, &state))
}

#[tauri::command]
pub fn check_remediation_handoff(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<RemediationHandoffCheck, String> {
    let path = store_path();
    let state = load_store_read_only(&path)?;
    let snapshot = snapshot_from_store(&path, &state);
    let repository = snapshot
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    remediation_handoff_check_for_repository(repository, workspace_id.as_deref())
}

#[tauri::command]
pub fn check_remediation_execution_gate(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<RemediationExecutionGate, String> {
    let path = store_path();
    let state = load_store_read_only(&path)?;
    let snapshot = snapshot_from_store(&path, &state);
    let repository = snapshot
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let plan = snapshot
        .remediation
        .plans
        .iter()
        .find(|plan| plan.repository_id == repository.id);
    let closure = snapshot
        .remediation
        .closures
        .iter()
        .filter(|closure| closure.repository_id == repository.id)
        .max_by(|left, right| left.closed_at.cmp(&right.closed_at));
    remediation_execution_gate_for_repository(repository, plan, closure, workspace_id.as_deref())
}
