fn agent_project_compass_attention(repository: &RepositorySnapshot) -> Option<AgentAttentionItem> {
    let compass = &repository.project_compass;
    let coverage_incomplete = compass.status == "Ready"
        && (compass.mvp.total_pillar_count == 0
            || compass.mvp.covered_pillar_count < compass.mvp.total_pillar_count);
    if compass.status == "Ready"
        && !coverage_incomplete
        && compass.open_blockers == 0
        && compass.open_drift == 0
    {
        return None;
    }

    let mut details = Vec::new();
    if compass.status != "Ready" {
        details.push(format!(
            "contract is {}",
            compass.status.to_lowercase()
        ));
    }
    if coverage_incomplete {
        if compass.mvp.total_pillar_count == 0 {
            details.push("MVP pillar coverage is unavailable".to_string());
        } else {
            details.push(format!(
                "MVP pillar coverage is incomplete ({}/{})",
                compass.mvp.covered_pillar_count, compass.mvp.total_pillar_count
            ));
        }
    }
    if compass.open_blockers > 0 {
        details.push(format!("{} open blocker(s)", compass.open_blockers));
    }
    if compass.open_drift > 0 {
        details.push(format!("{} open drift item(s)", compass.open_drift));
    }

    let contract_path = Path::new(&repository.path)
        .join(&compass.contract_path)
        .to_string_lossy()
        .to_string();
    let evidence_reference =
        |label: &str, status: &str, value: Option<String>| AgentEvidenceReference {
            source: "Project Compass".to_string(),
            label: label.to_string(),
            status: Some(status.to_string()),
            freshness: None,
            observed_at: compass.updated_at.clone(),
            value,
            report_path: Some(contract_path.clone()),
        };
    let mut evidence = vec![evidence_reference(
        "Contract status",
        &compass.status,
        compass.error.clone(),
    )];
    if compass.status == "Ready" {
        evidence.push(evidence_reference(
            "MVP pillar coverage",
            if coverage_incomplete {
                "Incomplete"
            } else {
                "Complete"
            },
            Some(format!(
                "{}/{} pillars covered across {} scoped outcomes",
                compass.mvp.covered_pillar_count,
                compass.mvp.total_pillar_count,
                compass.mvp.scored_outcome_count
            )),
        ));
    }
    if compass.open_blockers > 0 {
        evidence.push(evidence_reference(
            "Open blockers",
            "Open",
            Some(compass.open_blockers.to_string()),
        ));
    }
    if compass.open_drift > 0 {
        evidence.push(evidence_reference(
            "Open drift",
            "Open",
            Some(compass.open_drift.to_string()),
        ));
    }

    Some(AgentAttentionItem {
        id: format!("{}:project-compass", repository.id),
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        workspace_id: None,
        workspace_path: None,
        category: "project_compass".to_string(),
        severity: "warning".to_string(),
        status: if compass.status == "Ready" {
            "Attention".to_string()
        } else {
            compass.status.clone()
        },
        freshness: None,
        summary: format!(
            "Project Compass requires attention: {}",
            details.join("; ")
        ),
        evidence,
    })
}
