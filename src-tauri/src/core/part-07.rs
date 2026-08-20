fn sum_complete_optional_metric(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<u64>,
) -> Option<u64> {
    if samples.is_empty() || samples.iter().any(|sample| selector(sample).is_none()) {
        None
    } else {
        Some(samples.iter().filter_map(selector).sum())
    }
}

fn aggregate_quality_freshness(samples: &[AnalyticsMetricSample]) -> Option<String> {
    let values = samples
        .iter()
        .filter_map(|sample| sample.quality_freshness.as_deref())
        .collect::<Vec<_>>();
    if values.iter().any(|value| *value == "Conflicted") {
        Some("Conflicted".to_string())
    } else if values.iter().any(|value| *value == "Stale") {
        Some("Stale".to_string())
    } else if values.iter().any(|value| *value == "Fresh") {
        Some("Fresh".to_string())
    } else {
        None
    }
}

fn analytics_portfolio_sample(
    repositories: &[RepositorySnapshot],
    remediation_run: &remediation::RemediationRun,
    observed_at: &str,
) -> AnalyticsMetricSample {
    let samples = repositories
        .iter()
        .map(|repository| analytics_repository_sample(repository, observed_at))
        .collect::<Vec<_>>();
    AnalyticsMetricSample {
        schema_version: ANALYTICS_SAMPLE_SCHEMA.to_string(),
        observed_at: observed_at.to_string(),
        repository_count: samples.iter().map(|sample| sample.repository_count).sum(),
        workspace_count: samples.iter().map(|sample| sample.workspace_count).sum(),
        branch_count: samples.iter().map(|sample| sample.branch_count).sum(),
        active_condition_count: samples
            .iter()
            .map(|sample| sample.active_condition_count)
            .sum(),
        dirty_workspace_count: samples
            .iter()
            .map(|sample| sample.dirty_workspace_count)
            .sum(),
        unsynced_workspace_count: samples
            .iter()
            .map(|sample| sample.unsynced_workspace_count)
            .sum(),
        active_workspace_count: samples
            .iter()
            .map(|sample| sample.active_workspace_count)
            .sum(),
        interrupted_workspace_count: samples
            .iter()
            .map(|sample| sample.interrupted_workspace_count)
            .sum(),
        idle_workspace_count: samples
            .iter()
            .map(|sample| sample.idle_workspace_count)
            .sum(),
        unknown_workspace_count: samples
            .iter()
            .map(|sample| sample.unknown_workspace_count)
            .sum(),
        ahead_commit_count: samples.iter().map(|sample| sample.ahead_commit_count).sum(),
        behind_commit_count: samples
            .iter()
            .map(|sample| sample.behind_commit_count)
            .sum(),
        commits_last_30_days: sum_complete_optional_metric(&samples, |sample| {
            sample.commits_last_30_days
        }),
        ci_readiness_score: average_score(&samples, |sample| sample.ci_readiness_score),
        maturity_score: average_score(&samples, |sample| sample.maturity_score),
        maturity_evidence_coverage: average_score(&samples, |sample| {
            sample.maturity_evidence_coverage
        }),
        findings_total: sum_optional_metric(&samples, |sample| sample.findings_total),
        high_severity_findings: sum_optional_metric(&samples, |sample| {
            sample.high_severity_findings
        }),
        detector_findings_total: sum_optional_metric(&samples, |sample| {
            sample.detector_findings_total
        }),
        detector_actionable_findings: sum_optional_metric(&samples, |sample| {
            sample.detector_actionable_findings
        }),
        detector_unreviewed_findings: sum_optional_metric(&samples, |sample| {
            sample.detector_unreviewed_findings
        }),
        maturity_gap_total: sum_optional_metric(&samples, |sample| sample.maturity_gap_total),
        detector_refresh_required: {
            let values = samples
                .iter()
                .filter_map(|sample| sample.detector_refresh_required)
                .collect::<Vec<_>>();
            (!values.is_empty()).then(|| values.into_iter().any(|value| value))
        },
        quality_evidence_fingerprint: aggregate_quality_evidence_fingerprint(&samples),
        ci_readiness_scored_repository_count: samples
            .iter()
            .map(|sample| sample.ci_readiness_scored_repository_count)
            .sum(),
        maturity_scored_repository_count: samples
            .iter()
            .map(|sample| sample.maturity_scored_repository_count)
            .sum(),
        findings_repository_count: samples
            .iter()
            .map(|sample| sample.findings_repository_count)
            .sum(),
        release_rule_repository_count: samples
            .iter()
            .map(|sample| sample.release_rule_repository_count)
            .sum(),
        release_ready_repository_count: samples
            .iter()
            .map(|sample| sample.release_ready_repository_count)
            .sum(),
        remediation_open_action_count: Some(
            remediation_run
                .plans
                .iter()
                .flat_map(|plan| &plan.actions)
                .filter(|action| action.status == "open")
                .count() as u64,
        ),
        remediation_in_progress_action_count: Some(
            remediation_run
                .plans
                .iter()
                .flat_map(|plan| &plan.actions)
                .filter(|action| action.status == "in_progress")
                .count() as u64,
        ),
        remediation_blocked_action_count: Some(
            remediation_run
                .plans
                .iter()
                .flat_map(|plan| &plan.actions)
                .filter(|action| action.status == "blocked")
                .count() as u64,
        ),
        remediation_deferred_action_count: Some(
            remediation_run
                .plans
                .iter()
                .flat_map(|plan| &plan.actions)
                .filter(|action| action.status == "deferred")
                .count() as u64,
        ),
        remediation_verified_action_count: Some(
            remediation_run
                .plans
                .iter()
                .flat_map(|plan| &plan.actions)
                .filter(|action| action.status == "verified")
                .count() as u64,
        ),
        remediation_progress_percent: (!remediation_run.plans.is_empty()).then(|| {
            let (verified, eligible) =
                remediation_run
                    .plans
                    .iter()
                    .fold((0_u64, 0_u64), |(verified, eligible), plan| {
                        (
                            verified + plan.progress.verified_weight,
                            eligible
                                + plan
                                    .progress
                                    .total_weight
                                    .saturating_sub(plan.progress.deferred_weight),
                        )
                    });
            if eligible == 0 {
                100.0
            } else {
                verified as f64 / eligible as f64 * 100.0
            }
        }),
        quality_freshness: aggregate_quality_freshness(&samples),
        metrics: BTreeMap::new(),
    }
}

fn migrate_analytics_sample(mut sample: AnalyticsMetricSample) -> AnalyticsMetricSample {
    let legacy_fresh_passing_ci_score = sample
        .metrics
        .get("quality.evidence_score")
        .copied()
        .flatten()
        .or(sample.ci_readiness_score);
    let mut insert = |id: &str, value: Option<f64>| {
        sample.metrics.entry(id.to_string()).or_insert(value);
    };
    insert(
        "git.commits.trailing_30_days",
        sample.commits_last_30_days.map(|v| v as f64),
    );
    insert("git.ahead_commits", Some(sample.ahead_commit_count as f64));
    insert(
        "git.behind_commits",
        Some(sample.behind_commit_count as f64),
    );
    insert(
        "workspaces.dirty",
        Some(sample.dirty_workspace_count as f64),
    );
    insert(
        "workspaces.unsynced",
        Some(sample.unsynced_workspace_count as f64),
    );
    insert(
        "conditions.active",
        Some(sample.active_condition_count as f64),
    );
    insert("quality.maturity_score", sample.maturity_score);
    insert(
        "quality.maturity_evidence_coverage",
        sample.maturity_evidence_coverage,
    );
    insert(
        "quality.fresh_passing_ci_score",
        legacy_fresh_passing_ci_score,
    );
    insert("quality.evidence_score", sample.ci_readiness_score);
    insert("findings.total", sample.findings_total.map(|v| v as f64));
    insert(
        "findings.high_severity",
        sample.high_severity_findings.map(|v| v as f64),
    );
    insert(
        "findings.detector_total",
        sample.detector_findings_total.map(|v| v as f64),
    );
    insert(
        "findings.detector_actionable",
        sample.detector_actionable_findings.map(|v| v as f64),
    );
    insert(
        "findings.detector_unreviewed",
        sample.detector_unreviewed_findings.map(|v| v as f64),
    );
    insert("maturity.gaps", sample.maturity_gap_total.map(|v| v as f64));
    insert(
        "release.ready_repositories",
        Some(sample.release_ready_repository_count as f64),
    );
    insert(
        "release.configured_repositories",
        Some(sample.release_rule_repository_count as f64),
    );
    insert(
        "workspaces.activity.active",
        Some(sample.active_workspace_count as f64),
    );
    insert(
        "workspaces.activity.interrupted",
        Some(sample.interrupted_workspace_count as f64),
    );
    insert(
        "workspaces.activity.idle",
        Some(sample.idle_workspace_count as f64),
    );
    insert(
        "workspaces.activity.unknown",
        Some(sample.unknown_workspace_count as f64),
    );
    insert(
        "remediation.actions.open",
        sample
            .remediation_open_action_count
            .map(|value| value as f64),
    );
    insert(
        "remediation.actions.in_progress",
        sample
            .remediation_in_progress_action_count
            .map(|value| value as f64),
    );
    insert(
        "remediation.actions.blocked",
        sample
            .remediation_blocked_action_count
            .map(|value| value as f64),
    );
    insert(
        "remediation.actions.deferred",
        sample
            .remediation_deferred_action_count
            .map(|value| value as f64),
    );
    insert(
        "remediation.actions.verified",
        sample
            .remediation_verified_action_count
            .map(|value| value as f64),
    );
    insert(
        "remediation.progress_percent",
        sample.remediation_progress_percent,
    );
    sample.schema_version = ANALYTICS_SAMPLE_SCHEMA.to_string();
    sample
}

fn analytics_sample_fingerprint(sample: &AnalyticsMetricSample) -> Result<String, String> {
    let mut comparable = sample.clone();
    comparable.observed_at.clear();
    serde_json::to_string(&comparable)
        .map_err(|error| format!("Could not fingerprint analytics sample: {error}"))
}

fn analytics_scope_id(repository_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(repository_id.as_bytes());
    format!("repository:{:x}", digest.finalize())
}

fn latest_analytics_sample(
    connection: &SqliteConnection,
    repository_id: Option<&str>,
) -> Result<Option<AnalyticsMetricSample>, String> {
    let payload = match repository_id {
        Some(repository_id) => connection
            .query_row(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id = ?1 ORDER BY observed_at DESC, id DESC LIMIT 1",
                params![repository_id],
                |row| row.get::<_, String>(0),
            )
            .optional(),
        None => connection
            .query_row(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id IS NULL ORDER BY observed_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional(),
    }
    .map_err(|error| format!("Could not read latest analytics sample: {error}"))?;
    payload
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode analytics sample: {error}"))
                .map(migrate_analytics_sample)
        })
        .transpose()
}

fn should_deduplicate_analytics_sample(
    latest: Option<&AnalyticsMetricSample>,
    sample: &AnalyticsMetricSample,
) -> Result<bool, String> {
    let Some(latest) = latest else {
        return Ok(false);
    };
    if analytics_sample_fingerprint(latest)? != analytics_sample_fingerprint(sample)? {
        return Ok(false);
    }
    let observed_at = DateTime::parse_from_rfc3339(&sample.observed_at)
        .map_err(|error| format!("Could not parse analytics observation time: {error}"))?
        .with_timezone(&Utc);
    let previous = DateTime::parse_from_rfc3339(&latest.observed_at)
        .map_err(|error| format!("Could not parse latest analytics time: {error}"))?
        .with_timezone(&Utc);
    let elapsed = observed_at - previous;
    Ok(elapsed >= chrono::Duration::zero()
        && elapsed <= chrono::Duration::minutes(ANALYTICS_DEDUP_MINUTES))
}

fn record_analytics_samples_at(
    path: &Path,
    state: &StoreState,
    observed_at: &str,
) -> Result<(), String> {
    let portfolio = migrate_analytics_sample(analytics_portfolio_sample(
        &state.repositories,
        &state.remediation,
        observed_at,
    ));
    let mut samples = vec![(None, portfolio)];
    samples.extend(state.repositories.iter().map(|repository| {
        (
            Some(repository.id.clone()),
            migrate_analytics_sample(analytics_repository_sample(repository, observed_at)),
        )
    }));

    let mut connection = open_store(path)?;
    let latest_samples = samples
        .iter()
        .map(|(repository_id, _)| {
            let scope_id = repository_id.as_deref().map(analytics_scope_id);
            latest_analytics_sample(&connection, scope_id.as_deref())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not begin analytics transaction: {error}"))?;
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    transaction
        .execute(
            "DELETE FROM analytics_samples WHERE observed_at < ?1",
            params![cutoff.to_rfc3339_opts(SecondsFormat::Secs, true)],
        )
        .map_err(|error| format!("Could not prune analytics samples: {error}"))?;
    for ((repository_id, sample), latest) in samples.into_iter().zip(latest_samples) {
        if should_deduplicate_analytics_sample(latest.as_ref(), &sample)? {
            continue;
        }
        let payload = serde_json::to_string(&sample)
            .map_err(|error| format!("Could not encode analytics sample: {error}"))?;
        let sequence = NEXT_ANALYTICS_ID.fetch_add(1, Ordering::Relaxed);
        let scope_id = repository_id.as_deref().map(analytics_scope_id);
        let scope = scope_id.as_deref().unwrap_or("fleet");
        let id = format!("analytics:{scope}:{observed_at}:{sequence}");
        transaction
            .execute(
                "INSERT INTO analytics_samples
                 (id, repository_id, observed_at, payload_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, scope_id, observed_at, payload],
            )
            .map_err(|error| format!("Could not save analytics sample: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("Could not commit analytics samples: {error}"))
}

fn record_analytics_samples(path: &Path, state: &StoreState) -> Result<(), String> {
    record_analytics_samples_at(path, state, &iso_now())
}

fn prune_analytics_samples(path: &Path, retention_days: i64) -> Result<(), String> {
    let connection = open_store(path)?;
    let cutoff = Utc::now() - chrono::Duration::days(retention_days.max(1));
    connection
        .execute(
            "DELETE FROM analytics_samples WHERE observed_at < ?1",
            params![cutoff.to_rfc3339_opts(SecondsFormat::Secs, true)],
        )
        .map_err(|error| format!("Could not prune analytics samples: {error}"))?;
    Ok(())
}

fn analytics_payload(row: &Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}
