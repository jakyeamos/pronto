fn apply_provider_refresh_at(
    path: &Path,
    refresh: ProviderRefresh,
    target_repository_ids: Option<&HashSet<String>>,
) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let remote_by_name = refresh
        .repositories
        .iter()
        .filter_map(|repository| {
            normalize_remote_name(&repository.full_name).map(|name| (name, repository))
        })
        .collect::<HashMap<_, _>>();
    for local in &mut state.repositories {
        if target_repository_ids.is_some_and(|targets| !targets.contains(&local.id)) {
            continue;
        }
        let Some(remote_name) = local.remote_url.as_deref().and_then(normalize_remote_name) else {
            continue;
        };
        if let Some(remote) = remote_by_name.get(&remote_name) {
            local.provider_state = format!("GitHub connected as {}", remote.identity_id);
            local.locality = "Local and remote".to_string();
            local.pull_requests = refresh
                .pull_requests
                .iter()
                .filter(|pull_request| pull_request.repository_id == remote.id)
                .cloned()
                .collect();
            local.releases = refresh
                .releases
                .iter()
                .filter(|release| release.repository_id == remote.id)
                .cloned()
                .collect();
        } else if remote_name.starts_with("github.com/") || local.provider_state.contains("GitHub")
        {
            local.provider_state =
                "GitHub repository unavailable to the connected identity".to_string();
        }
    }
    let refreshed_remote_repositories =
        classify_remote_repositories(&state.repositories, refresh.repositories);
    let remote_repositories = if let Some(targets) = target_repository_ids {
        let target_names = state
            .repositories
            .iter()
            .filter(|repository| targets.contains(&repository.id))
            .filter_map(|repository| {
                repository
                    .remote_url
                    .as_deref()
                    .and_then(normalize_remote_name)
            })
            .collect::<HashSet<_>>();
        let mut merged = state
            .remote_repositories
            .into_iter()
            .filter(|remote| {
                normalize_remote_name(&remote.full_name)
                    .map_or(true, |name| !target_names.contains(&name))
            })
            .collect::<Vec<_>>();
        merged.extend(refreshed_remote_repositories.into_iter().filter(|remote| {
            normalize_remote_name(&remote.full_name)
                .is_some_and(|name| target_names.contains(&name))
        }));
        merged
    } else {
        refreshed_remote_repositories
    };
    state.provider_identities = refresh.identities;
    state.remote_repositories = remote_repositories;
    state.provider_status = ProviderStatus {
        provider: "GitHub".to_string(),
        state: "Ready".to_string(),
        message: "Read-only GitHub context refreshed for connected local repositories and GitHub-only candidates.".to_string(),
        last_refresh_at: Some(refresh.refreshed_at),
        identity_count: state.provider_identities.len(),
        repository_count: state.remote_repositories.len(),
    };
    apply_quality_evidence_scoped(&mut state, target_repository_ids, None);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn apply_quality_evidence(state: &mut StoreState) {
    apply_quality_evidence_scoped(state, None, None);
}

fn persisted_fleet_audit_root(state: &StoreState) -> Option<PathBuf> {
    state
        .remediation
        .refresh_steps
        .iter()
        .find(|step| step.id == "qr_fleet_run")
        .and_then(|step| step.evidence_path.as_deref())
        .map(PathBuf::from)
        .filter(|root| root.is_dir())
}

fn apply_quality_evidence_scoped(
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    fleet_audit_root: Option<&Path>,
) {
    let persisted_fleet_root = persisted_fleet_audit_root(state);
    let fleet_audit_root = fleet_audit_root.or(persisted_fleet_root.as_deref());
    let checkpoint_path = quality::canonical_maturity_checkpoint_path();
    let coordinated =
        quality::maturity_checkpoint_import(checkpoint_path.as_deref(), &state.repositories);
    let (audit, mac_control, checkpoint) = if let Some(import) = coordinated {
        (import.audit, import.mac_control, import.checkpoint)
    } else {
        let feed_path = quality::canonical_maturity_feed_path();
        let audit = quality::maturity_feed_import(feed_path.as_deref(), &state.repositories);
        let mac_control_scope = state.repositories.clone();
        let mac_control = mac_control_maturity::evaluate_canonical(&mac_control_scope);
        (
            audit,
            mac_control,
            quality::MaturityCheckpointSnapshot::legacy(),
        )
    };
    let fleet = quality::fleet_audit_import(fleet_audit_root, &state.repositories);
    state.quality = audit.portfolio;
    state.quality.maturity_checkpoint = checkpoint;
    state.quality.mac_control_ideal_state = mac_control.portfolio.clone();
    state.quality.evidence_contracts = vec![mac_control.portfolio.evidence_contract.clone()];
    let remote_by_name = state
        .remote_repositories
        .iter()
        .filter_map(|remote| normalize_remote_name(&remote.full_name).map(|name| (name, remote)))
        .collect::<HashMap<_, _>>();
    for repository in &mut state.repositories {
        if target_repository_ids.is_some_and(|targets| !targets.contains(&repository.id)) {
            continue;
        }
        let prior_findings = repository.quality.findings.clone();
        let remote = repository
            .remote_url
            .as_deref()
            .and_then(normalize_remote_name)
            .and_then(|name| remote_by_name.get(&name).copied());
        let target_fleet_import =
            repository
                .quality
                .target_fleet_audit_root
                .as_deref()
                .map(|root| {
                    quality::fleet_audit_import(
                        Some(Path::new(root)),
                        std::slice::from_ref(repository),
                    )
                });
        let target_provenance = repository.target_branch.as_deref().and_then(|branch| {
            repository
                .branches
                .iter()
                .find(|candidate| candidate.name == branch)
                .and_then(|candidate| candidate.last_commit.as_deref())
                .map(|commit| (branch.to_string(), commit.to_string()))
        });
        let target_fleet_evidence = target_fleet_import
            .as_ref()
            .and_then(|import| import.evidence.get(&repository.id))
            .map(|evidence| {
                let mut scoped = evidence.clone();
                if let Some((branch, commit)) = target_provenance.as_ref() {
                    quality::scope_fleet_audit_evidence_to_target(&mut scoped, branch, commit);
                }
                scoped
            });
        let fleet_evidence = target_fleet_evidence
            .as_ref()
            .or_else(|| fleet.evidence.get(&repository.id));
        let maturity = target_fleet_evidence
            .as_ref()
            .map(|evidence| evidence.maturity.clone())
            .or_else(|| audit.maturities.get(&repository.id).cloned())
            .or_else(|| fleet_evidence.map(|evidence| evidence.maturity.clone()));
        let ci_gate_profile = quality::ci_gate_profile_for_repository(repository);
        let mut imported = quality::ingest_repository_quality(
            repository,
            remote,
            maturity,
            Some(&ci_gate_profile),
        );
        imported.mac_control_ideal_state = mac_control
            .by_repository
            .get(&repository.id)
            .cloned()
            .unwrap_or_default();
        imported.behavior_assurance = audit
            .behavior_assurance
            .get(&repository.id)
            .cloned()
            .unwrap_or_default();
        imported.evidence_contracts =
            vec![imported.mac_control_ideal_state.evidence_contract.clone()];
        imported.target_fleet_audit_root = repository.quality.target_fleet_audit_root.clone();
        if let Some(fleet_evidence) = fleet_evidence {
            if target_fleet_evidence.is_some()
                || imported.maturity.score.is_none()
                || (imported.maturity.freshness != quality::QualityFreshness::Fresh
                    && fleet_evidence.maturity.freshness == quality::QualityFreshness::Fresh)
            {
                imported.maturity = fleet_evidence.maturity.clone();
            }
            let stable_detector_report =
                quality::is_stable_detector_report(imported.findings.report_path.as_deref());
            if target_fleet_evidence.is_some()
                || (!stable_detector_report
                    && (imported.findings.source.is_none()
                        || (imported.findings.freshness != quality::QualityFreshness::Fresh
                            && fleet_evidence.findings.freshness
                                == quality::QualityFreshness::Fresh)))
            {
                imported.findings = fleet_evidence.findings.clone();
            }
            if imported.last_ingested_at.is_none() {
                imported.last_ingested_at = fleet_evidence.findings.observed_at.clone();
            }
            imported.ingestion_status = "Available".to_string();
            imported.ingestion_message = None;
        }
        if repository.target_branch_configured {
            if let Some((target_branch, target_commit)) = target_provenance.as_ref() {
                imported
                    .behavior_assurance
                    .project_to_target(target_branch, target_commit);
                quality::project_quality_snapshot_for_target(
                    &mut imported,
                    target_branch,
                    target_commit,
                );
            }
        }
        quality::preserve_detector_evidence_on_refresh_failure(
            &prior_findings,
            &mut imported.findings,
        );
        quality::reconcile_finding_dispositions(
            Path::new(&repository.path),
            &mut imported.findings,
        );
        quality::update_detector_delta(&prior_findings, &mut imported.findings);
        repository.quality = imported;
    }
    for repository in &mut state.repositories {
        if let Some(mac_control_state) = mac_control.by_repository.get(&repository.id) {
            repository.quality.mac_control_ideal_state = mac_control_state.clone();
            repository.quality.evidence_contracts =
                vec![mac_control_state.evidence_contract.clone()];
        }
    }
    quality::update_ci_readiness_summary(&mut state.quality, &state.repositories);
    quality::update_composite_maturity_summary(&mut state.quality, &mut state.repositories);
    state.remediation = remediation::rebuild_run_with_fleet_root(
        &state.repositories,
        &state.remediation,
        state.quality.latest_audit_id.as_deref(),
        fleet_audit_root,
    );
}

fn quality_refresh_import_was_accepted(quality: &QualityPortfolioSnapshot) -> bool {
    quality.latest_audit_id.is_some()
        && matches!(
            quality.audit_status.as_str(),
            "Ready" | "Ready with blockers" | "Stale" | "Unknown"
        )
}

fn refresh_quality_at_with(
    path: &Path,
    apply_quality: impl FnOnce(&mut StoreState),
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    let mut state = load_store(path)?;
    apply_quality(&mut state);
    apply_release_threshold_conditions(&mut state);
    let accepted_import = quality_refresh_import_was_accepted(&state.quality);
    save_store(path, &state)?;
    if accepted_import {
        record_analytics_samples(path, &state)?;
    }
    Ok(snapshot_from_store(path, &state))
}

fn refresh_quality_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    refresh_quality_at_with(path, apply_quality_evidence)
}

#[derive(Debug, Clone, Serialize)]
struct QualityDetectorRefreshReport {
    schema_version: String,
    generated_at: String,
    status: String,
    qr: serde_json::Value,
    provenance_refreshes: usize,
    published_repositories: usize,
    ingested_published_repositories: usize,
    rejected_published_repositories: usize,
    reconciliation: Vec<QualityDetectorReconciliation>,
    tracked_repositories: usize,
    detector_applicable_repositories: usize,
    detector_excluded_repositories: usize,
    findings_evidence_repositories: usize,
    applicable_findings_evidence_repositories: usize,
    missing_findings_evidence_repositories: usize,
    snapshot: PortfolioSnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct QualityDetectorReconciliation {
    repository_path: Option<String>,
    target_branch: Option<String>,
    target_head: Option<String>,
    expected_findings: Option<u64>,
    imported_findings: Option<u64>,
    report_path: Option<String>,
    status: String,
    reason: String,
}

fn detector_result_string(result: &serde_json::Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(result, |value, key| value.get(*key))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn detector_unsupported_paths(qr_payload: &serde_json::Value) -> HashSet<PathBuf> {
    qr_payload
        .get("results")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|result| {
            result.get("status").and_then(serde_json::Value::as_str) == Some("unsupported")
        })
        .filter_map(|result| {
            result
                .get("primary_path")
                .and_then(serde_json::Value::as_str)
        })
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)))
        .collect()
}

fn reconcile_published_detector_results(
    qr_payload: &serde_json::Value,
    snapshot: &PortfolioSnapshot,
) -> Vec<QualityDetectorReconciliation> {
    qr_payload
        .get("results")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|result| {
            result.get("status").and_then(serde_json::Value::as_str) == Some("published")
        })
        .map(|result| {
            let repository_path = detector_result_string(result, &["primary_path"]);
            let target_branch = detector_result_string(result, &["target", "branch"]);
            let target_head = detector_result_string(result, &["target", "head"]);
            let expected_findings = result
                .get("finding_count")
                .and_then(serde_json::Value::as_u64);
            let published_paths = result
                .get("published_paths")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .filter_map(|path| canonical_path(Path::new(path)))
                .collect::<Vec<_>>();
            let repository = repository_path.as_deref().and_then(|path| {
                let expected = canonical_path(Path::new(path))?;
                snapshot.repositories.iter().find(|repository| {
                    canonical_path(Path::new(&repository.path)).as_ref() == Some(&expected)
                })
            });
            let findings = repository.map(|repository| &repository.quality.findings);
            let report_path = findings.and_then(|findings| findings.report_path.clone());
            let report_is_published = report_path.as_deref().is_some_and(|report| {
                canonical_path(Path::new(report)).is_some_and(|report| {
                    published_paths
                        .iter()
                        .any(|published| report.starts_with(published))
                })
            });
            let mut failures = Vec::new();
            if repository_path.is_none() {
                failures.push("QR publication omitted primary_path".to_string());
            }
            if target_branch.is_none() {
                failures.push("QR publication omitted target branch".to_string());
            }
            if target_head.is_none() {
                failures.push("QR publication omitted target commit".to_string());
            }
            if expected_findings.is_none() {
                failures.push("QR publication omitted findings total".to_string());
            }
            if published_paths.is_empty() {
                failures.push("QR publication omitted published run paths".to_string());
            }
            if repository.is_none() {
                failures.push("repository was not present after provenance refresh".to_string());
            }
            if findings.and_then(|findings| findings.source.as_ref())
                != Some(&quality::QualitySource::Qr)
            {
                failures.push("Pronto did not import QR findings evidence".to_string());
            }
            if findings.and_then(|findings| findings.scanned_branch.as_deref())
                != target_branch.as_deref()
            {
                failures.push("imported branch does not match the published target".to_string());
            }
            if findings.and_then(|findings| findings.scanned_commit.as_deref())
                != target_head.as_deref()
            {
                failures.push("imported commit does not match the published target".to_string());
            }
            if findings.map(|findings| &findings.freshness) != Some(&QualityFreshness::Fresh) {
                failures.push("imported findings are not fresh for the target ref".to_string());
            }
            if expected_findings
                .is_some_and(|expected| findings.map(|findings| findings.total) != Some(expected))
            {
                failures.push("imported findings total does not match QR publication".to_string());
            }
            if !report_is_published {
                failures
                    .push("Pronto selected a report outside QR's published run set".to_string());
            }
            QualityDetectorReconciliation {
                repository_path,
                target_branch,
                target_head,
                expected_findings,
                imported_findings: findings.map(|findings| findings.total),
                report_path,
                status: if failures.is_empty() {
                    "ingested".to_string()
                } else {
                    "rejected".to_string()
                },
                reason: if failures.is_empty() {
                    "Published exact-target QR findings were imported and verified.".to_string()
                } else {
                    failures.join("; ")
                },
            }
        })
        .collect()
}
