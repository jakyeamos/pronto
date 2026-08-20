fn save_store(path: &Path, state: &StoreState) -> Result<(), String> {
    let mut connection = open_store(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not begin Pronto database transaction: {error}"))?;

    for table in [
        "roots",
        "repositories",
        "products",
        "groups_config",
        "expected_conditions",
        "events",
        "action_audits",
        "provider_identities",
        "remote_repositories",
        "remediation_runs",
    ] {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| format!("Could not clear Pronto {table} table: {error}"))?;
    }

    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params![
                "store_version",
                state.version.max(STORE_VERSION).to_string()
            ],
        )
        .map_err(|error| format!("Could not save Pronto store version: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["retention_days", state.retention_days.to_string()],
        )
        .map_err(|error| format!("Could not save Pronto retention setting: {error}"))?;
    let provider_status_json = serde_json::to_string(&state.provider_status)
        .map_err(|error| format!("Could not encode provider status: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["provider_status_json", provider_status_json],
        )
        .map_err(|error| format!("Could not save provider status: {error}"))?;
    let quality_summary_json = serde_json::to_string(&state.quality)
        .map_err(|error| format!("Could not encode quality summary: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["quality_summary_json", quality_summary_json],
        )
        .map_err(|error| format!("Could not save quality summary: {error}"))?;
    for root in &state.roots {
        let ignore_patterns_json = serde_json::to_string(&root.ignore_patterns)
            .map_err(|error| format!("Could not encode root ignore patterns: {error}"))?;
        transaction
            .execute(
                "INSERT INTO roots
                 (id, path, label, ignore_patterns_json, refresh_policy,
                  background_monitoring, registered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    root.id,
                    root.path,
                    root.label,
                    ignore_patterns_json,
                    root.refresh_policy,
                    i64::from(root.background_monitoring),
                    root.registered_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto root: {error}"))?;
    }

    for repository in &state.repositories {
        let payload = serde_json::to_string(repository)
            .map_err(|error| format!("Could not encode repository snapshot: {error}"))?;
        transaction
            .execute(
                "INSERT INTO repositories (id, payload_json) VALUES (?1, ?2)",
                params![repository.id, payload],
            )
            .map_err(|error| format!("Could not save Pronto repository snapshot: {error}"))?;
    }

    for product in &state.products {
        let repository_ids_json = serde_json::to_string(&product.repository_ids)
            .map_err(|error| format!("Could not encode product repositories: {error}"))?;
        transaction
            .execute(
                "INSERT INTO products
                 (id, name, repository_ids_json, release_mode, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    product.id,
                    product.name,
                    repository_ids_json,
                    product.release_mode,
                    product.created_at,
                    product.updated_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto product: {error}"))?;
    }

    for group in &state.groups {
        let repository_ids_json = serde_json::to_string(&group.repository_ids)
            .map_err(|error| format!("Could not encode group repositories: {error}"))?;
        transaction
            .execute(
                "INSERT INTO groups_config
                 (id, name, repository_ids_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    group.id,
                    group.name,
                    repository_ids_json,
                    group.created_at,
                    group.updated_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto group: {error}"))?;
    }

    for expected in &state.expected_conditions {
        transaction
            .execute(
                "INSERT INTO expected_conditions
                 (repository_id, condition_id, fingerprint, marked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    expected.repository_id,
                    expected.condition_id,
                    expected.fingerprint,
                    expected.marked_at
                ],
            )
            .map_err(|error| format!("Could not save expected condition: {error}"))?;
    }

    for event in &state.events {
        transaction
            .execute(
                "INSERT INTO events
                 (id, repository_id, kind, summary, fingerprint, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.id,
                    event.repository_id,
                    event.kind,
                    event.summary,
                    event.fingerprint,
                    event.created_at
                ],
            )
            .map_err(|error| format!("Could not save Pronto event: {error}"))?;
    }

    for audit in &state.action_audits {
        let target_ids_json = serde_json::to_string(&audit.target_ids)
            .map_err(|error| format!("Could not encode action audit targets: {error}"))?;
        transaction
            .execute(
                "INSERT INTO action_audits
                 (id, action, target_ids_json, risk, status, summary, created_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    audit.id,
                    audit.action,
                    target_ids_json,
                    audit.risk,
                    audit.status,
                    audit.summary,
                    audit.created_at,
                    audit.completed_at
                ],
            )
            .map_err(|error| format!("Could not save Pronto action audit: {error}"))?;
    }

    for identity in &state.provider_identities {
        let payload = serde_json::to_string(identity)
            .map_err(|error| format!("Could not encode provider identity: {error}"))?;
        transaction
            .execute(
                "INSERT INTO provider_identities (id, payload_json) VALUES (?1, ?2)",
                params![identity.id, payload],
            )
            .map_err(|error| format!("Could not save provider identity: {error}"))?;
    }

    for repository in &state.remote_repositories {
        let payload = serde_json::to_string(repository)
            .map_err(|error| format!("Could not encode remote repository: {error}"))?;
        transaction
            .execute(
                "INSERT INTO remote_repositories (id, payload_json) VALUES (?1, ?2)",
                params![repository.id, payload],
            )
            .map_err(|error| format!("Could not save remote repository: {error}"))?;
    }

    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    let remediation_payload = serde_json::to_string(&remediation_run)
        .map_err(|error| format!("Could not encode remediation run: {error}"))?;
    transaction
        .execute(
            "INSERT INTO remediation_runs (id, generated_at, payload_json)
             VALUES (?1, ?2, ?3)",
            params![
                remediation_run.id,
                remediation_run.generated_at,
                remediation_payload
            ],
        )
        .map_err(|error| format!("Could not save remediation run: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("Could not commit Pronto database transaction: {error}"))
}

fn quality_metric_freshness(repository: &RepositorySnapshot) -> Option<String> {
    let has_evidence = repository.quality.ci_readiness.score.is_some()
        || repository.quality.maturity.score.is_some()
        || repository.quality.findings.source.is_some()
        || repository.quality.findings.observed_at.is_some();
    if !has_evidence {
        return None;
    }
    let mut values = Vec::new();
    if repository.quality.ci_readiness.score.is_some() {
        values.push(QualityFreshness::Fresh);
    }
    if repository.quality.maturity.score.is_some() {
        values.push(repository.quality.maturity.freshness.clone());
    }
    if quality_metric_is_available(repository) {
        values.push(repository.quality.findings.freshness.clone());
    }
    if values
        .iter()
        .any(|value| *value == QualityFreshness::Conflicted)
    {
        return Some(QualityFreshness::Conflicted.as_str().to_string());
    }
    if values.iter().any(|value| *value == QualityFreshness::Stale) {
        return Some(QualityFreshness::Stale.as_str().to_string());
    }
    if values.iter().any(|value| *value == QualityFreshness::Fresh) {
        return Some(QualityFreshness::Fresh.as_str().to_string());
    }
    None
}

fn quality_metric_is_available(repository: &RepositorySnapshot) -> bool {
    repository.quality.findings.source.is_some()
        || repository.quality.findings.observed_at.is_some()
        || repository.quality.findings.freshness != QualityFreshness::Unknown
}

fn quality_evidence_fingerprint(repository: &RepositorySnapshot) -> Option<String> {
    let findings = &repository.quality.findings;
    let maturity = &repository.quality.maturity;
    let has_evidence = quality_metric_is_available(repository)
        || maturity.score.is_some()
        || !maturity.dimension_scores.is_empty()
        || !maturity.gaps.is_empty();
    if !has_evidence {
        return None;
    }
    let comparable = serde_json::json!({
        "detector_findings_total": findings.detector_findings_total,
        "detector_actionable_total": findings.detector_actionable_total,
        "detector_unreviewed_total": findings.detector_unreviewed_total,
        "enabled_detector_count": findings.enabled_detector_count,
        "enabled_rule_count": findings.enabled_rule_count,
        "producer_versions": findings.producer_versions,
        "producer_source_shas": findings.producer_source_shas,
        "ruleset_fingerprints": findings.ruleset_fingerprints,
        "configuration_fingerprints": findings.configuration_fingerprints,
        "qr_version": findings.qr_version,
        "target_sha": findings.target_sha,
        "refresh_required": findings.refresh_required,
        "detector_status": findings.detector_status,
        "maturity_score": maturity.score,
        "maturity_dimensions": maturity.dimension_scores,
        "maturity_gaps": maturity.gaps.iter().map(|gap| serde_json::json!({
            "dimension": gap.dimension,
            "status": gap.status,
            "score": gap.score,
        })).collect::<Vec<_>>(),
    });
    let encoded = serde_json::to_vec(&comparable).ok()?;
    let mut digest = Sha256::new();
    digest.update(encoded);
    Some(format!("{:x}", digest.finalize()))
}

fn aggregate_quality_evidence_fingerprint(samples: &[AnalyticsMetricSample]) -> Option<String> {
    let mut fingerprints = samples
        .iter()
        .filter_map(|sample| sample.quality_evidence_fingerprint.clone())
        .collect::<Vec<_>>();
    if fingerprints.is_empty() {
        return None;
    }
    fingerprints.sort();
    let encoded = serde_json::to_vec(&fingerprints).ok()?;
    let mut digest = Sha256::new();
    digest.update(encoded);
    Some(format!("{:x}", digest.finalize()))
}

fn local_commit_count_since(path: &Path, observed_at: &str) -> Option<u64> {
    let observed = DateTime::parse_from_rfc3339(observed_at)
        .ok()?
        .with_timezone(&Utc);
    let cutoff = observed - chrono::Duration::days(ANALYTICS_RANGE_DAYS);
    let cutoff = cutoff.to_rfc3339_opts(SecondsFormat::Secs, true);
    git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--all".to_string(),
            format!("--since={cutoff}"),
            "--count".to_string(),
        ],
    )
    .and_then(|value| value.parse::<u64>().ok())
}

fn analytics_workspace_activity_counts(repository: &RepositorySnapshot) -> (u64, u64, u64, u64) {
    let mut active = 0;
    let mut interrupted = 0;
    let mut idle = 0;
    let mut unknown = 0;
    for workspace in &repository.workspaces {
        if workspace.activity.state == "Active" {
            active += 1;
        } else if workspace.activity.state.starts_with("Interrupted") {
            interrupted += 1;
        } else if workspace.activity.state.starts_with("Unknown") {
            unknown += 1;
        } else {
            idle += 1;
        }
    }
    (active, interrupted, idle, unknown)
}

fn analytics_repository_sample(
    repository: &RepositorySnapshot,
    observed_at: &str,
) -> AnalyticsMetricSample {
    let (
        active_workspace_count,
        interrupted_workspace_count,
        idle_workspace_count,
        unknown_workspace_count,
    ) = analytics_workspace_activity_counts(repository);
    let active_condition_count = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .count() as u64;
    let dirty_workspace_count = repository
        .workspaces
        .iter()
        .filter(|workspace| workspace.dirty)
        .count() as u64;
    let unsynced_workspace_count = repository
        .workspaces
        .iter()
        .filter(|workspace| workspace_is_unsynced(workspace))
        .count() as u64;
    let release_ready = repository.conditions.iter().any(|condition| {
        condition.kind == "release-threshold"
            && matches!(condition.status.as_str(), "Active" | "Expected")
    });
    let ci_readiness_score = repository.quality.ci_readiness.score;
    let maturity_score = repository.quality.maturity.score;
    let findings_available = quality_metric_is_available(repository);
    AnalyticsMetricSample {
        observed_at: observed_at.to_string(),
        repository_count: 1,
        workspace_count: repository.workspaces.len() as u64,
        branch_count: repository.branches.len() as u64,
        active_condition_count,
        dirty_workspace_count,
        unsynced_workspace_count,
        active_workspace_count,
        interrupted_workspace_count,
        idle_workspace_count,
        unknown_workspace_count,
        ahead_commit_count: repository
            .workspaces
            .iter()
            .map(|workspace| workspace.ahead)
            .sum(),
        behind_commit_count: repository
            .workspaces
            .iter()
            .map(|workspace| workspace.behind)
            .sum(),
        commits_last_30_days: local_commit_count_since(Path::new(&repository.path), observed_at),
        ci_readiness_score,
        maturity_score,
        findings_total: findings_available.then_some(repository.quality.findings.total),
        high_severity_findings: findings_available
            .then_some(repository.quality.findings.high_severity_total),
        detector_findings_total: findings_available
            .then_some(repository.quality.findings.detector_findings_total),
        detector_actionable_findings: findings_available
            .then_some(repository.quality.findings.detector_actionable_total),
        detector_unreviewed_findings: findings_available
            .then_some(repository.quality.findings.detector_unreviewed_total),
        maturity_gap_total: repository
            .quality
            .maturity
            .score
            .is_some()
            .then_some(repository.quality.maturity.gaps.len() as u64),
        detector_refresh_required: findings_available
            .then_some(repository.quality.findings.refresh_required),
        quality_evidence_fingerprint: quality_evidence_fingerprint(repository),
        ci_readiness_scored_repository_count: u64::from(ci_readiness_score.is_some()),
        maturity_scored_repository_count: u64::from(maturity_score.is_some()),
        findings_repository_count: u64::from(findings_available),
        release_rule_repository_count: u64::from(repository.release_rule.is_some()),
        release_ready_repository_count: u64::from(release_ready),
        remediation_open_action_count: None,
        remediation_in_progress_action_count: None,
        remediation_blocked_action_count: None,
        remediation_deferred_action_count: None,
        remediation_verified_action_count: None,
        remediation_progress_percent: None,
        quality_freshness: quality_metric_freshness(repository),
        metrics: BTreeMap::new(),
    }
}

fn average_score(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<f64>,
) -> Option<f64> {
    let values = samples.iter().filter_map(selector).collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn sum_optional_metric(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<u64>,
) -> Option<u64> {
    let values = samples.iter().filter_map(selector).collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.into_iter().sum())
    }
}
