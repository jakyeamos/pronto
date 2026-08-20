fn resolve_refresh_target(
    snapshot: &PortfolioSnapshot,
    query: &str,
) -> Result<(HashSet<String>, String), String> {
    let repository_matches = snapshot
        .repositories
        .iter()
        .filter(|repository| repository_matches_query(repository, query))
        .collect::<Vec<_>>();
    let product_matches = snapshot
        .products
        .iter()
        .filter(|product| product.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    let group_matches = snapshot
        .groups
        .iter()
        .filter(|group| group.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    let match_count = repository_matches.len() + product_matches.len() + group_matches.len();
    if match_count == 0 {
        return Err(format!(
            "Refresh target '{query}' is not a repository, product, or group"
        ));
    }
    if match_count > 1 {
        return Err(format!("Refresh target '{query}' is ambiguous"));
    }
    if let Some(repository) = repository_matches.first() {
        return Ok((
            [repository.id.clone()].into_iter().collect(),
            format!("Repository {}", repository.name),
        ));
    }
    if let Some(product) = product_matches.first() {
        return Ok((
            product.repository_ids.iter().cloned().collect(),
            format!("Product {}", product.name),
        ));
    }
    let group = group_matches
        .first()
        .expect("target count guarantees a group");
    Ok((
        group.repository_ids.iter().cloned().collect(),
        format!("Group {}", group.name),
    ))
}

#[derive(Debug)]
enum LocalRefreshTarget {
    Registered {
        repository_ids: HashSet<String>,
        label: String,
    },
    RepositoryPath(PathBuf),
}

fn resolve_local_refresh_target(
    snapshot: &PortfolioSnapshot,
    state: &StoreState,
    query: &str,
) -> Result<LocalRefreshTarget, String> {
    match resolve_refresh_target(snapshot, query) {
        Ok((repository_ids, label)) => {
            return Ok(LocalRefreshTarget::Registered {
                repository_ids,
                label,
            });
        }
        Err(error) if error.contains("ambiguous") => return Err(error),
        Err(_) => {}
    }

    let repository_path = canonical_repository_path(Path::new(query)).ok_or_else(|| {
        format!(
            "Refresh target '{query}' is not a registered repository, group, product, or repository path"
        )
    })?;
    let covered_by_root = state
        .roots
        .iter()
        .any(|root| path_is_within(Path::new(&root.path), &repository_path));
    if !covered_by_root {
        return Err(format!(
            "Repository path '{}' is not covered by a registered discovery root; register its parent root before refreshing it",
            repository_path.display()
        ));
    }
    Ok(LocalRefreshTarget::RepositoryPath(repository_path))
}

fn agent_condition_summary(condition: &Condition) -> AgentConditionSummary {
    AgentConditionSummary {
        id: condition.id.clone(),
        kind: condition.kind.clone(),
        title: condition.title.clone(),
        summary: condition.summary.clone(),
        priority: condition.priority,
        status: condition.status.clone(),
        missing: condition.missing.clone(),
        confidence: condition.confidence.clone(),
        freshness: condition.freshness.clone(),
    }
}

fn agent_workspace_summary(workspace: &WorkspaceSummary) -> AgentWorkspaceSummary {
    AgentWorkspaceSummary {
        id: workspace.id.clone(),
        path: workspace.path.clone(),
        is_primary: workspace.is_primary,
        branch: workspace.branch.clone(),
        status_available: workspace.status_available,
        status_error: workspace.status_error.clone(),
        dirty: workspace.dirty,
        sync_state: workspace.sync_state.clone(),
        ahead: workspace.ahead,
        behind: workspace.behind,
        upstream: workspace.upstream.clone(),
        operation: workspace.operation.clone(),
        integration_state: workspace.integration_state.clone(),
        target_branch: workspace.target_branch.clone(),
        target_confidence: workspace.target_confidence.clone(),
        activity_state: workspace.activity.state.clone(),
        activity_confidence: workspace.activity.confidence.clone(),
        last_commit: workspace.last_commit.clone(),
        last_commit_at: workspace.last_commit_at.clone(),
        last_activity_at: workspace.last_activity_at.clone(),
        provenance: workspace.provenance.clone(),
        sync_detail: workspace.sync_detail.clone(),
    }
}

fn workspace_requires_sync_attention(workspace: &WorkspaceSummary) -> bool {
    !workspace.status_available || workspace_is_unsynced(workspace)
}

fn workspace_is_unsynced(workspace: &WorkspaceSummary) -> bool {
    workspace.status_available && workspace.sync_state != "Synced"
}

fn agent_repository_summary(repository: &RepositorySnapshot) -> AgentRepositorySummary {
    let active_conditions = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .map(agent_condition_summary)
        .collect::<Vec<_>>();
    AgentRepositorySummary {
        id: repository.id.clone(),
        name: repository.name.clone(),
        path: repository.path.clone(),
        locality: repository.locality.clone(),
        lifecycle: repository.lifecycle.clone(),
        branch: repository.branch.clone(),
        default_branch: repository.default_branch.clone(),
        target_branch: repository
            .target_branch
            .clone()
            .or_else(|| repository.default_branch.clone()),
        target_branch_configured: repository.target_branch_configured,
        branch_lifecycle: repository.branch_lifecycle.clone(),
        workspaces: repository
            .workspaces
            .iter()
            .map(agent_workspace_summary)
            .collect(),
        active_conditions,
        quality_status: repository.quality.ingestion_status.clone(),
        installed_runtime_status: repository.quality.installed_runtime.status.clone(),
        installed_runtime_summary: repository.quality.installed_runtime.summary.clone(),
        maturity_score: repository.quality.maturity.score,
        maturity_score_display: repository.quality.maturity.score_display.clone(),
        maturity_freshness: repository.quality.maturity.freshness.as_str().to_string(),
        ci_readiness_score: repository.quality.ci_readiness.score,
        ci_readiness_score_display: repository.quality.ci_readiness.score_display.clone(),
        ci_readiness_fresh_passing_gate_count: repository
            .quality
            .ci_readiness
            .fresh_passing_gate_ids
            .len(),
        ci_readiness_ideal_gate_count: repository.quality.ci_readiness.applicable_gate_ids.len(),
        ci_configuration_configured_gate_count: repository
            .quality
            .ci_readiness
            .configured_gate_ids
            .len(),
        ci_configuration_ideal_gate_count: repository
            .quality
            .ci_readiness
            .applicable_gate_ids
            .len(),
        findings_total: repository.quality.findings.total,
        high_severity_findings: repository.quality.findings.high_severity_total,
        project_compass: repository.project_compass.clone(),
        last_scan_at: repository.last_scan_at.clone(),
        last_activity_at: repository.last_activity_at.clone(),
    }
}

fn agent_condition_evidence(condition: &Condition) -> Vec<AgentEvidenceReference> {
    condition
        .evidence
        .iter()
        .map(|evidence| AgentEvidenceReference {
            source: evidence.source.clone(),
            label: evidence.label.clone(),
            status: None,
            freshness: condition.freshness.clone(),
            observed_at: Some(evidence.observed_at.clone()),
            value: Some(evidence.value.clone()),
            report_path: None,
        })
        .collect()
}

fn agent_gate_evidence(gate: &quality::QualityGate) -> Vec<AgentEvidenceReference> {
    gate.evidence
        .iter()
        .map(|evidence| AgentEvidenceReference {
            source: evidence.source.as_str().to_string(),
            label: evidence.source_label.clone(),
            status: Some(evidence.status.as_str().to_string()),
            freshness: Some(evidence.freshness.as_str().to_string()),
            observed_at: evidence.observed_at.clone(),
            value: Some(evidence.detail.clone()),
            report_path: evidence.report_path.clone(),
        })
        .collect()
}

fn agent_workspace_sync_evidence(workspace: &WorkspaceSummary) -> Vec<AgentEvidenceReference> {
    let Some(detail) = workspace.sync_detail.as_ref() else {
        return Vec::new();
    };
    vec![
        AgentEvidenceReference {
            source: "Local workspace scan".to_string(),
            label: if workspace.status_available {
                "Why unsynced".to_string()
            } else {
                "Why Git status unavailable".to_string()
            },
            status: Some(workspace.sync_state.clone()),
            freshness: None,
            observed_at: detail.evidence_observed_at.clone(),
            value: Some(detail.reason.clone()),
            report_path: None,
        },
        AgentEvidenceReference {
            source: "Local workspace scan".to_string(),
            label: "Evidence expires".to_string(),
            status: Some("Expiry timestamp".to_string()),
            freshness: None,
            observed_at: detail.evidence_observed_at.clone(),
            value: detail.evidence_expires_at.clone(),
            report_path: None,
        },
        AgentEvidenceReference {
            source: "Pronto scoped refresh contract".to_string(),
            label: "Next safe scoped refresh".to_string(),
            status: Some("Read-only local scan".to_string()),
            freshness: None,
            observed_at: None,
            value: Some(detail.scoped_refresh_command.clone()),
            report_path: None,
        },
    ]
}

fn agent_attention_report(snapshot: &PortfolioSnapshot) -> AgentAttentionReport {
    let mut items = Vec::new();
    for repository in &snapshot.repositories {
        for condition in repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
        {
            items.push(AgentAttentionItem {
                id: format!("{}:condition:{}", repository.id, condition.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "condition".to_string(),
                severity: format!("P{}", condition.priority),
                status: condition.status.clone(),
                freshness: condition.freshness.clone(),
                summary: condition.summary.clone(),
                evidence: agent_condition_evidence(condition),
            });
        }

        for workspace in repository
            .workspaces
            .iter()
            .filter(|workspace| workspace.dirty)
        {
            items.push(AgentAttentionItem {
                id: format!("{}:workspace-dirty:{}", repository.id, workspace.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: Some(workspace.id.clone()),
                workspace_path: Some(workspace.path.clone()),
                category: "workspace".to_string(),
                severity: "warning".to_string(),
                status: "Dirty".to_string(),
                freshness: None,
                summary: format!("Workspace {} has uncommitted changes", workspace.branch),
                evidence: Vec::new(),
            });
        }

        for workspace in repository
            .workspaces
            .iter()
            .filter(|workspace| workspace_requires_sync_attention(workspace))
        {
            let summary = if workspace.status_available {
                format!(
                    "Workspace {} is {} (ahead {}, behind {})",
                    workspace.branch, workspace.sync_state, workspace.ahead, workspace.behind
                )
            } else {
                format!(
                    "Workspace {} Git status unavailable: {}",
                    workspace.branch,
                    workspace_status_unavailable_reason(workspace)
                )
            };
            items.push(AgentAttentionItem {
                id: format!("{}:workspace-sync:{}", repository.id, workspace.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: Some(workspace.id.clone()),
                workspace_path: Some(workspace.path.clone()),
                category: "synchronization".to_string(),
                severity: "warning".to_string(),
                status: workspace.sync_state.clone(),
                freshness: workspace
                    .status_available
                    .then_some(workspace.remote_freshness.clone()),
                summary,
                evidence: agent_workspace_sync_evidence(workspace),
            });
        }

        for gate in &repository.quality.gates {
            let missing = gate.status == QualityGateStatus::NotConfigured
                && repository
                    .quality
                    .ci_readiness
                    .applicable_gate_ids
                    .iter()
                    .any(|gate_id| gate_id == &gate.id);
            let stale = gate.freshness != QualityFreshness::Fresh;
            let failed_or_blocked = matches!(
                gate.status,
                QualityGateStatus::Failed | QualityGateStatus::Blocked
            );
            if missing || stale || failed_or_blocked {
                let status = if missing {
                    "Missing".to_string()
                } else {
                    gate.status.as_str().to_string()
                };
                let severity = if failed_or_blocked {
                    "error"
                } else {
                    "warning"
                };
                items.push(AgentAttentionItem {
                    id: format!("{}:quality-gate:{}", repository.id, gate.id),
                    repository_id: repository.id.clone(),
                    repository_name: repository.name.clone(),
                    repository_path: repository.path.clone(),
                    workspace_id: None,
                    workspace_path: None,
                    category: "quality_gate".to_string(),
                    severity: severity.to_string(),
                    status,
                    freshness: Some(gate.freshness.as_str().to_string()),
                    summary: format!("{} gate requires attention", gate.label),
                    evidence: agent_gate_evidence(gate),
                });
            }
        }

        if repository.quality.findings.high_severity_total > 0 {
            items.push(AgentAttentionItem {
                id: format!("{}:quality-findings", repository.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "quality_findings".to_string(),
                severity: "error".to_string(),
                status: "Open".to_string(),
                freshness: Some(repository.quality.findings.freshness.as_str().to_string()),
                summary: format!(
                    "{} high-severity quality findings remain open",
                    repository.quality.findings.high_severity_total
                ),
                evidence: Vec::new(),
            });
        }

        if repository.quality.maturity.score.is_none()
            || repository.quality.maturity.freshness != QualityFreshness::Fresh
        {
            items.push(AgentAttentionItem {
                id: format!("{}:quality-maturity", repository.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "quality_maturity".to_string(),
                severity: "warning".to_string(),
                status: if repository.quality.maturity.score.is_some() {
                    "Stale".to_string()
                } else {
                    "Unknown".to_string()
                },
                freshness: Some(repository.quality.maturity.freshness.as_str().to_string()),
                summary: "Repository maturity evidence is missing or not fresh".to_string(),
                evidence: Vec::new(),
            });
        }

        if let Some(item) = agent_project_compass_attention(repository) {
            items.push(item);
        }
    }
    for plan in &snapshot.remediation.plans {
        let Some(repository) = snapshot
            .repositories
            .iter()
            .find(|repository| repository.id == plan.repository_id)
        else {
            continue;
        };
        for action in plan.actions.iter().filter(|action| {
            action.domain == "telescope_readiness"
                && matches!(action.status.as_str(), "open" | "in_progress" | "blocked")
        }) {
            items.push(AgentAttentionItem {
                id: format!(
                    "{}:telescope-readiness:{}",
                    repository.id, action.stable_key
                ),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: Some(repository.workspace.id.clone()),
                workspace_path: Some(repository.workspace.path.clone()),
                category: "telescope_readiness".to_string(),
                severity: action.priority.clone(),
                status: action.status.clone(),
                freshness: action.evidence.first().map(|item| item.freshness.clone()),
                summary: action.title.clone(),
                evidence: action
                    .evidence
                    .iter()
                    .map(|item| AgentEvidenceReference {
                        source: item.source.clone(),
                        label: item.label.clone(),
                        status: Some(item.status.clone()),
                        freshness: Some(item.freshness.clone()),
                        observed_at: item.observed_at.clone(),
                        value: Some(item.detail.clone()),
                        report_path: item.report_path.clone(),
                    })
                    .collect(),
            });
        }
    }
    AgentAttentionReport {
        schema_version: AGENT_ATTENTION_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        items,
    }
}

fn agent_attention_report_for_query(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
) -> Result<AgentAttentionReport, String> {
    let mut report = agent_attention_report(snapshot);
    if let Some(query) = query {
        let repository_id = find_cli_repository(snapshot, query)?.id.clone();
        report
            .items
            .retain(|item| item.repository_id == repository_id);
    }
    Ok(report)
}

fn agent_attention_priority(item: &AgentAttentionItem) -> u16 {
    if item.severity == "error" {
        return 0;
    }
    if let Some(priority) = item
        .severity
        .strip_prefix('P')
        .and_then(|value| value.parse::<u16>().ok())
    {
        return priority;
    }
    if item.severity == "warning" {
        return 100;
    }
    200
}
