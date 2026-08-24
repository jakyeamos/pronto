fn print_human_repository(detail: &AgentRepositoryDetail) {
    let repository = &detail.repository;
    println!(
        "{} · {} · {} · {}",
        repository.name, repository.lifecycle, repository.branch, repository.path
    );
    println!(
        "Quality: {} · maturity {} · {} active conditions",
        repository.quality.ingestion_status,
        repository
            .quality
            .maturity
            .score_display
            .as_deref()
            .unwrap_or("unknown"),
        repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
            .count()
    );
    println!(
        "Branch lifecycle: {} · {}/{} feature branches · admission {}",
        repository.branch_lifecycle.status,
        repository.branch_lifecycle.feature_branch_count,
        repository.branch_lifecycle.policy.hard_limit,
        repository.branch_lifecycle.admission
    );
    println!("  {}", repository.branch_lifecycle.next_safe_step);
    println!(
        "Task lanes: {} active · {} adoptable · {} stale · {} contested · {} unknown · {} total",
        detail.task_lanes.counts.active,
        detail.task_lanes.counts.adoptable,
        detail.task_lanes.counts.stale,
        detail.task_lanes.counts.contested,
        detail.task_lanes.counts.unknown,
        detail.task_lanes.counts.total
    );
    if detail.task_lanes.source.status != "available" {
        println!(
            "  custody evidence unavailable: {}",
            detail.task_lanes.source.detail
        );
    }
    for workspace in &repository.workspaces {
        let cleanliness = if !workspace.status_available {
            "unavailable"
        } else if workspace.dirty {
            "dirty"
        } else {
            "clean"
        };
        println!(
            "  {} · {} · {} · {}",
            workspace.branch, cleanliness, workspace.sync_state, workspace.path
        );
        if let Some(detail) = workspace.sync_detail.as_ref() {
            println!(
                "    {}: {}",
                if workspace.status_available {
                    "why unsynced"
                } else {
                    "why Git status is unavailable"
                },
                detail.reason
            );
            println!(
                "    evidence expires: {}",
                detail
                    .evidence_expires_at
                    .as_deref()
                    .unwrap_or("unavailable")
            );
            println!(
                "    next safe scoped refresh: {}",
                detail.scoped_refresh_command
            );
            println!("    authorization: {}", detail.authorization);
        }
    }
}

fn print_human_quality(report: &AgentQualityReport) {
    println!(
        "PRONTO QUALITY · {} repositories · {}",
        report.repositories.len(),
        report.portfolio.audit_status
    );
    println!(
        "Fleet maturity: {} · CI configuration: {}/{} configured · fresh passing evidence: {}/{}",
        report
            .portfolio
            .maturity_score_display
            .as_deref()
            .unwrap_or("unknown"),
        report.portfolio.ci_configuration_configured_gate_count,
        report.portfolio.ci_configuration_ideal_gate_count,
        report.portfolio.ci_evidence_fresh_passing_gate_count,
        report.portfolio.ci_evidence_ideal_gate_count,
    );
    for repository in &report.repositories {
        println!(
            "  {} · maturity {} · {}",
            repository.name,
            repository
                .quality
                .maturity
                .score_display
                .as_deref()
                .unwrap_or("unknown"),
            repository.quality.ingestion_status
        );
    }
}

fn print_human_remediation(run: &RemediationRun) {
    println!(
        "PRONTO REMEDIATION · {} · {} active · {} resolved-history entries · {} excluded · {} GitHub-only candidates",
        run.status,
        run.plans.len(),
        run.closures.len(),
        run.excluded_repositories.len(),
        run.github_only_candidates.len()
    );
    if let Some(refresh_id) = run.source_refresh_id.as_deref() {
        println!("Refresh: {refresh_id}");
    }
    if let Some(message) = run.message.as_deref() {
        println!("Message: {message}");
    }
    for step in &run.refresh_steps {
        println!(
            "  refresh · {} · {} · {}",
            step.id, step.status, step.detail
        );
    }
    for exclusion in &run.excluded_repositories {
        println!(
            "  excluded · {} · {}",
            exclusion.repository_name, exclusion.reason
        );
    }
    for candidate in &run.github_only_candidates {
        println!(
            "  github-only · {} · {} · last task {} · observed {}",
            candidate.full_name,
            candidate.status,
            candidate.last_remediation_task,
            candidate.observed_at
        );
    }
    for (index, plan) in run.plans.iter().enumerate() {
        println!(
            "  #{} · {} · goal {} ({}) · {} · {}% · {} · {} actions",
            index + 1,
            plan.repository_name,
            plan.goal.target_state,
            plan.goal.source,
            plan.status,
            plan.progress.percentage.round(),
            plan.current_stage,
            plan.actions.len()
        );
    }
    for closure in &run.closures {
        println!(
            "  resolved · {} · goal {} ({}) · {} · {} · {}",
            closure.repository_name,
            closure.target_state,
            closure.goal_source,
            closure.disposition,
            closure.closed_at,
            closure.summary
        );
    }
}

fn print_human_attention(report: &AgentAttentionReport) {
    println!("PRONTO ATTENTION · {} items", report.items.len());
    for item in &report.items {
        println!(
            "{} · {} · {} · {}",
            item.repository_name, item.category, item.status, item.summary
        );
    }
}

fn print_human_activity(report: &AgentActivityReport) {
    println!(
        "PRONTO ACTIVITY · {} events · {} action audits",
        report.events.len(),
        report.action_audits.len()
    );
    for event in &report.events {
        println!("  {} · {}", event.kind, event.summary);
    }
    for audit in &report.action_audits {
        println!("  {} · {} · {}", audit.action, audit.status, audit.summary);
    }
}

fn print_human_remediation_handoff_check(check: &RemediationHandoffCheck) {
    println!(
        "PRONTO REMEDIATION HANDOFF · {} · {}",
        check.repository_name, check.status
    );
    println!(
        "Workspace: {} · branch: {} · checkpoint required: {}",
        check.workspace_path, check.branch, check.checkpoint_required
    );
    for reason in &check.reasons {
        println!("  reason: {reason}");
    }
    println!("  next: {}", check.next_safe_step);
}

fn print_human_remediation_execution_gate(gate: &RemediationExecutionGate) {
    println!(
        "PRONTO REMEDIATION EXECUTION GATE · {} · {}",
        gate.repository_name, gate.status
    );
    println!(
        "Scope: {} · workspaces checked: {} · execution blockers: {}",
        gate.scope,
        gate.workspace_checks.len(),
        gate.blockers.len()
    );
    println!(
        "Closure: {} · {} active · {} explicitly blocked",
        gate.closure_gate.status,
        gate.closure_gate.active_action_count,
        gate.closure_gate.blocked_action_count
    );
    for blocker in &gate.blockers {
        println!(
            "  blocker · {} · {} · {}",
            blocker.kind, blocker.workspace_path, blocker.title
        );
        println!(
            "    evidence: {} · {}{}",
            blocker.evidence_state,
            blocker.source,
            blocker
                .observed_at
                .as_deref()
                .map(|observed_at| format!(" · observed {observed_at}"))
                .unwrap_or_default()
        );
        println!("    affects: {}", blocker.blocked_operations.join(", "));
        println!("    next: {}", blocker.next_safe_step);
    }
    println!("Authorization: {}", gate.authorization.status);
    println!("  next: {}", gate.next_safe_step);
}

fn print_human_preparation(report: &AgentPreparationReport) {
    let preparation = &report.preparation;
    println!(
        "PRONTO PREPARATION · {} · {}",
        preparation.repository_id, preparation.generated_at
    );
    println!(
        "Pull request: {} · release: {} · recipe: {}",
        preparation.pull_request.status, preparation.release.status, preparation.recipe.status
    );
}

fn print_human_release(report: &AgentReleaseReport) {
    println!(
        "PRONTO RELEASE · {} · {}",
        report.repository_id, report.release.status
    );
    println!(
        "Baseline: {} · recommendation: {} · recipe: {}",
        report.release.baseline_status, report.release.recommendation.label, report.recipe.status
    );
    for reason in &report.release.reasons {
        println!("  reason: {reason}");
    }
}

fn print_cli_json_error(command: &str, error: &str) {
    let payload = serde_json::json!({
        "schema_version": "pronto-cli-error/v1",
        "generated_at": iso_now(),
        "command": command,
        "status": "Blocked",
        "error": error,
        "next_safe_step": "Retry with the cached read path or resolve the reported storage, quality, detector prerequisite, or repository blocker."
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
            "{\"schema_version\":\"pronto-cli-error/v1\",\"status\":\"Blocked\"}".to_string()
        })
    );
}

fn required_workspace_role_string(
    entry: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    let value = entry
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("workspace role map field {field} must be a non-empty string"))?;
    Ok(value.to_string())
}

fn workspace_policy_for_role(
    repository_id: &str,
    entry: &serde_json::Map<String, Value>,
) -> Result<Value, String> {
    let role = required_workspace_role_string(entry, "repository_role")?;
    let canonical_workspaces = match role.as_str() {
        "production_product" => {
            let release_ref = required_workspace_role_string(entry, "release_ref")?;
            if !matches!(release_ref.as_str(), "main" | "master") {
                return Err(format!(
                    "{repository_id}: production release_ref must be main or master"
                ));
            }
            let integration_ref = required_workspace_role_string(entry, "integration_ref")?;
            if integration_ref != "dev" {
                return Err(format!(
                    "{repository_id}: production integration_ref must be dev"
                ));
            }
            vec![
                serde_json::json!({
                    "id": "release",
                    "role": "release",
                    "ref": release_ref,
                    "path": Value::Null,
                    "protected": true,
                }),
                serde_json::json!({
                    "id": "integration",
                    "role": "integration",
                    "ref": integration_ref,
                    "path": Value::Null,
                    "protected": true,
                }),
            ]
        }
        "supporting_project" => {
            let working_ref = required_workspace_role_string(entry, "working_ref")?;
            vec![serde_json::json!({
                "id": "working",
                "role": "working",
                "ref": working_ref,
                "path": Value::Null,
                "protected": true,
            })]
        }
        "role_unresolved" => Vec::new(),
        _ => {
            return Err(format!(
                "{repository_id}: repository_role must be production_product, supporting_project, or role_unresolved"
            ));
        }
    };
    let retention_exceptions = entry
        .get("retention_exceptions")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    if !retention_exceptions.is_array() {
        return Err(format!(
            "{repository_id}: retention_exceptions must be an array"
        ));
    }
    Ok(serde_json::json!({
        "schema_version": "workspace-policy/v1",
        "repository_id": repository_id,
        "repository_role": role,
        "canonical_workspaces": canonical_workspaces,
        "retention_exceptions": retention_exceptions,
    }))
}

fn workspace_role_map_entries<'a>(
    state: &StoreState,
    role_map: &'a Value,
) -> Result<BTreeMap<String, &'a serde_json::Map<String, Value>>, String> {
    let role_map = role_map
        .as_object()
        .ok_or_else(|| "workspace role map must contain an object".to_string())?;
    if role_map.get("schema_version").and_then(Value::as_str) != Some(WORKSPACE_ROLE_MAP_SCHEMA) {
        return Err(format!(
            "workspace role map schema_version must be {WORKSPACE_ROLE_MAP_SCHEMA}"
        ));
    }
    let entries = role_map
        .get("repositories")
        .and_then(Value::as_array)
        .ok_or_else(|| "workspace role map repositories must be an array".to_string())?;
    let mut by_id = BTreeMap::new();
    for (index, value) in entries.iter().enumerate() {
        let entry = value
            .as_object()
            .ok_or_else(|| format!("workspace role map repositories[{index}] must be an object"))?;
        let repository_id = required_workspace_role_string(entry, "repository_id")?;
        if by_id.insert(repository_id.clone(), entry).is_some() {
            return Err(format!(
                "workspace role map contains duplicate repository_id {repository_id}"
            ));
        }
    }
    let registered_ids: BTreeSet<String> = state
        .repositories
        .iter()
        .map(|repository| repository.id.clone())
        .collect();
    let mapped_ids: BTreeSet<String> = by_id.keys().cloned().collect();
    let missing: Vec<String> = registered_ids.difference(&mapped_ids).cloned().collect();
    let extra: Vec<String> = mapped_ids.difference(&registered_ids).cloned().collect();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(format!(
            "workspace role map must cover the registered fleet exactly; missing={missing:?}, extra={extra:?}"
        ));
    }
    Ok(by_id)
}

fn workspace_fleet_manifest(state: &StoreState, role_map: &Value) -> Result<Value, String> {
    let by_id = workspace_role_map_entries(state, role_map)?;

    let mut repositories = Vec::with_capacity(state.repositories.len());
    for repository in &state.repositories {
        let entry = by_id
            .get(&repository.id)
            .expect("role map coverage was checked above");
        let policy = workspace_policy_for_role(&repository.id, entry)?;
        let live = crate::custody::project(Path::new(&repository.path))?;
        repositories.push(serde_json::json!({
            "repository_id": repository.id,
            "repository_path": repository.path,
            "active_temporary_lanes": live.workspace_policy.active_temporary_lanes,
            "policy": policy,
        }));
    }
    Ok(serde_json::json!({
        "schema_version": WORKSPACE_FLEET_MANIFEST_SCHEMA,
        "generated_at": iso_now(),
        "source": "pronto registered fleet plus explicit workspace-role-map/v1",
        "role_map_schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
        "repository_count": repositories.len(),
        "repositories": repositories,
        "read_only": true,
        "implementation_allowed": false,
    }))
}

struct WorkspacePolicyGenerationPlan {
    repository_id: String,
    repository_path: PathBuf,
    policy_path: PathBuf,
    repository_role: String,
    policy_bytes: Vec<u8>,
    status: String,
    reason: Option<String>,
}
