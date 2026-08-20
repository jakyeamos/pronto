fn apply_detector_evidence(findings: &mut QualityFindings, payload: &Value) {
    let receipts = payload
        .get("detector_evidence")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            payload
                .get("detector")
                .is_some()
                .then(|| vec![payload.clone()])
        })
        .unwrap_or_default();
    if receipts.is_empty() {
        sync_detector_counts(findings);
        return;
    }

    let mut detectors = BTreeSet::new();
    let mut rules = BTreeSet::new();
    let mut statuses = BTreeSet::new();
    for receipt in receipts.iter().filter(|receipt| receipt.is_object()) {
        let detector = json_string_at(receipt, &["detector"]);
        let status = json_string_at(receipt, &["status"]);
        if let Some(status) = status.as_deref() {
            statuses.insert(status.to_string());
        }
        let applicable = receipt
            .get("applicable")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if applicable && status.as_deref() != Some("not_applicable") {
            if let Some(detector) = detector.as_deref() {
                detectors.insert(detector.to_string());
            }
            if let Some(enabled_rules) = receipt.get("enabled_rules").and_then(Value::as_array) {
                rules.extend(
                    enabled_rules
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string),
                );
            }
        }
        if let Some(detector) = detector.as_deref() {
            if let Some(version) = json_string_at(receipt, &["producer", "version"]) {
                findings
                    .producer_versions
                    .insert(detector.to_string(), version);
            }
            if let Some(source_sha) = json_string_at(receipt, &["producer", "source_sha"]) {
                findings
                    .producer_source_shas
                    .insert(detector.to_string(), source_sha);
            }
            if let Some(ruleset_hash) = json_string_at(receipt, &["ruleset_hash"]) {
                findings
                    .ruleset_fingerprints
                    .insert(detector.to_string(), ruleset_hash);
            }
            if let Some(configuration_hash) = json_string_at(receipt, &["configuration_hash"]) {
                findings
                    .configuration_fingerprints
                    .insert(detector.to_string(), configuration_hash);
            }
        }
        if findings.qr_version.is_none() {
            findings.qr_version = json_string_at(receipt, &["qr_version"]);
        }
        if findings.target_sha.is_none() {
            findings.target_sha = json_string_at(receipt, &["target_sha"]);
        }
        if let Some(scan_time) = json_string_at(receipt, &["scan_time"]) {
            if findings
                .refresh_time
                .as_deref()
                .is_none_or(|current| current < scan_time.as_str())
            {
                findings.refresh_time = Some(scan_time);
            }
        }
        if receipt
            .get("refresh_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || status.as_deref() == Some("blocked")
        {
            findings.refresh_required = true;
            if findings.refresh_required_reason.is_none() {
                findings.refresh_required_reason = json_string_at(receipt, &["reason"]);
            }
        }
    }

    findings.enabled_detector_count = detectors.len() as u64;
    findings.enabled_rule_count = rules.len() as u64;
    findings.detector_status = if statuses.contains("blocked") {
        Some("blocked".to_string())
    } else if statuses.contains("passed") {
        Some("passed".to_string())
    } else if statuses.contains("not_applicable") {
        Some("not_applicable".to_string())
    } else {
        None
    };
    if findings.target_sha.is_none() {
        findings.target_sha = findings.scanned_commit.clone();
    }
    sync_detector_counts(findings);
}

pub fn preserve_detector_evidence_on_refresh_failure(
    prior: &QualityFindings,
    current: &mut QualityFindings,
) {
    if !current.refresh_required
        || current.detector_status.as_deref() != Some("blocked")
        || prior.source.is_none()
    {
        return;
    }
    let mut preserved = prior.clone();
    preserved.refresh_required = true;
    preserved.refresh_required_reason = current
        .refresh_required_reason
        .clone()
        .or_else(|| Some("The latest detector refresh was blocked.".to_string()));
    preserved.detector_status = Some("blocked".to_string());
    preserved.refresh_time = current.refresh_time.clone().or(prior.refresh_time.clone());
    preserved.target_sha = current.target_sha.clone().or(prior.target_sha.clone());
    preserved.qr_version = current.qr_version.clone().or(prior.qr_version.clone());
    if current.report_path.is_some() && !is_stable_detector_report(current.report_path.as_deref()) {
        preserved.refresh_required_reason = preserved.refresh_required_reason.or_else(|| {
            Some(
                "The detector receipt is blocked; the prior valid scan is retained for review."
                    .to_string(),
            )
        });
    }
    *current = preserved;
}

pub fn update_detector_delta(prior: &QualityFindings, current: &mut QualityFindings) {
    current.delta_total = None;
    if current.refresh_required
        || prior.refresh_required
        || prior.source.is_none()
        || current.source.is_none()
        || prior.target_sha.is_none()
        || current.target_sha.is_none()
    {
        return;
    }
    let same_identity = prior.target_sha == current.target_sha
        && prior.qr_version == current.qr_version
        && prior.producer_versions == current.producer_versions
        && prior.producer_source_shas == current.producer_source_shas
        && prior.ruleset_fingerprints == current.ruleset_fingerprints
        && prior.configuration_fingerprints == current.configuration_fingerprints;
    if same_identity {
        current.delta_total =
            Some(current.detector_findings_total as i64 - prior.detector_findings_total as i64);
    }
}

pub fn fleet_audit_import(
    root: Option<&Path>,
    repositories: &[RepositorySnapshot],
) -> FleetAuditImport {
    let Some(root) = root else {
        return FleetAuditImport::default();
    };
    let summary = read_json(&root.join("summary.json"));
    let audit_id = summary
        .as_ref()
        .and_then(|value| json_string_at(value, &["audit_id"]));
    let observed_at = summary
        .as_ref()
        .and_then(|value| json_string_at(value, &["as_of"]));
    let Some(entries) = fs::read_dir(root.join("findings")).ok() else {
        return FleetAuditImport {
            audit_id,
            observed_at,
            evidence: HashMap::new(),
        };
    };
    let mut evidence = HashMap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let repository_payload = payload.get("repository");
        let candidate_path = repository_payload
            .and_then(|value| json_string_at(value, &["primary_path"]))
            .or_else(|| {
                repository_payload
                    .and_then(|value| value.get("checkouts"))
                    .and_then(Value::as_array)
                    .and_then(|checkouts| checkouts.first())
                    .and_then(|checkout| json_string_at(checkout, &["path"]))
            });
        let candidate_remote = repository_payload.and_then(|value| {
            ["identity_key", "remote_url", "remote_identity"]
                .iter()
                .find_map(|key| json_string_at(value, &[key]))
        });
        let Some(repository) = repositories.iter().find(|repository| {
            canonical_path_matches(candidate_path.as_deref(), &repository.path)
                || candidate_remote
                    .as_deref()
                    .and_then(remote_identity)
                    .zip(repository.remote_url.as_deref().and_then(remote_identity))
                    .is_some_and(|(candidate, local)| candidate == local)
        }) else {
            continue;
        };
        let run_id = payload
            .get("audit_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| audit_id.clone())
            .unwrap_or_else(|| path.display().to_string());
        let run_observed_at = payload
            .get("as_of")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| observed_at.clone());
        let checkouts = repository_payload
            .and_then(|value| value.get("checkouts"))
            .and_then(Value::as_array);
        let target_branch = repository_payload
            .and_then(|value| value.get("target_branch"))
            .and_then(|value| json_string_at(value, &["branch"]));
        let checkout = checkouts.and_then(|items| {
            target_branch
                .as_deref()
                .and_then(|branch| {
                    items.iter().find(|checkout| {
                        json_string_at(checkout, &["branch"]).as_deref() == Some(branch)
                    })
                })
                .or_else(|| items.first())
        });
        let scanned_commit = checkout
            .and_then(|value| json_string_at(value, &["head"]))
            .or_else(|| checkout.and_then(|value| json_string_at(value, &["fingerprint", "head"])));
        let scanned_branch = checkout.and_then(|value| json_string_at(value, &["branch"]));
        let findings = payload
            .get("findings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let agent_usability = payload
            .get("agent_usability")
            .filter(|value| value.is_object())
            .and_then(|value| serde_json::from_value(value.clone()).ok());
        let (mut dimension_scores, _) = fleet_dimension_scores(&findings);
        let mut gaps = fleet_maturity_gaps(&findings);
        if let Some(assessment) = agent_usability.as_ref() {
            merge_agent_usability_dimensions(&mut dimension_scores, &mut gaps, assessment);
        }
        gaps.sort_by(|left, right| left.dimension.cmp(&right.dimension));
        gaps.truncate(MAX_MATURITY_GAPS);
        let score = fleet_score(&dimension_scores)
            .or_else(|| payload.get("mean_maturity").and_then(Value::as_f64));
        let maturity = QualityMaturity {
            score,
            score_display: score.map(|value| format!("{value:.3}")),
            scored_dimension_count: Some(dimension_scores.len() as u64),
            dimension_scores,
            gaps,
            quality_outcome: payload
                .get("quality_outcome")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            agent_usability,
            repository_maturity: None,
            cache_design: None,
            ci_gate_audit: None,
            audit_id: Some(run_id.clone()),
            observed_at: run_observed_at.clone(),
            scanned_commit: scanned_commit.clone(),
            scanned_branch: scanned_branch.clone(),
            freshness: evaluate_audit_freshness_at(run_observed_at.as_deref(), Utc::now()),
            report_path: Some(path.to_string_lossy().to_string()),
        };
        // Fleet maturity rows are evidence for the maturity projection, not code
        // findings. Counting the raw array here duplicates maturity remediation and
        // can manufacture an aggregate findings action when every dimension passes.
        let quality_findings = findings
            .iter()
            .filter(|finding| !is_fleet_maturity_finding(finding))
            .cloned()
            .collect::<Vec<_>>();
        let severity_counts = fleet_severity_counts(&quality_findings);
        let high_severity_total = severity_counts
            .iter()
            .filter(|(severity, _)| matches!(severity.as_str(), "critical" | "high"))
            .map(|(_, count)| *count)
            .sum();
        let findings_freshness = evaluate_freshness_at(
            run_observed_at.as_deref(),
            scanned_commit.as_deref(),
            scanned_branch.as_deref(),
            repository.workspace.last_commit.as_deref(),
            Some(repository.branch.as_str()),
            Utc::now(),
        );
        let mut imported_findings = QualityFindings {
            total: quality_findings.len() as u64,
            severity_counts,
            high_severity_total,
            source: Some(QualitySource::Qr),
            observed_at: run_observed_at.clone(),
            scanned_commit,
            scanned_branch,
            freshness: findings_freshness,
            report_path: Some(path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };
        apply_detector_evidence(&mut imported_findings, &payload);
        evidence.insert(
            repository.id.clone(),
            FleetAuditEvidence {
                maturity,
                findings: imported_findings,
            },
        );
    }
    FleetAuditImport {
        audit_id,
        observed_at,
        evidence,
    }
}

pub(crate) fn is_fleet_maturity_finding(finding: &Value) -> bool {
    ["schema", "pack", "pack_id"].iter().any(|key| {
        finding
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with(FLEET_MATURITY_FINDING_SCHEMA_PREFIX))
    })
}

pub fn default_quality_gates() -> Vec<QualityGate> {
    CANONICAL_GATE_DEFINITIONS
        .iter()
        .map(|(id, label)| QualityGate {
            id: (*id).to_string(),
            label: (*label).to_string(),
            status: QualityGateStatus::NotConfigured,
            freshness: QualityFreshness::Unknown,
            evidence: Vec::new(),
        })
        .collect()
}

pub fn evaluate_ci_readiness(gates: &[QualityGate]) -> QualityReadiness {
    let mut applicable_gate_ids = CI_READINESS_BASELINE_GATE_IDS
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    applicable_gate_ids.extend(
        CI_READINESS_CONDITIONAL_GATE_IDS
            .iter()
            .filter(|id| {
                gates
                    .iter()
                    .find(|gate| gate.id == **id)
                    .is_some_and(|gate| !gate.evidence.is_empty())
            })
            .map(|id| (*id).to_string()),
    );

    let configured_gate_ids = gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    let mut readiness = evaluate_ci_readiness_for_ideal_with_configuration(
        gates,
        &applicable_gate_ids,
        &configured_gate_ids,
    );
    let passing_gate_count = applicable_gate_ids
        .iter()
        .filter(|gate_id| {
            gates.iter().any(|gate| {
                gate.id == **gate_id
                    && gate.status == QualityGateStatus::Passed
                    && gate.freshness == QualityFreshness::Fresh
            })
        })
        .count();
    readiness.score = (applicable_gate_ids.len() > 0).then(|| {
        let score = (passing_gate_count as f64 / applicable_gate_ids.len() as f64) * 4.0;
        (score * 100.0).round() / 100.0
    });
    readiness.score_display = readiness.score.map(format_quality_score);
    readiness
}

pub fn evaluate_ci_readiness_for_ideal(
    gates: &[QualityGate],
    ideal_gate_ids: &[String],
) -> QualityReadiness {
    let configured_gate_ids = gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    evaluate_ci_readiness_for_ideal_with_configuration(gates, ideal_gate_ids, &configured_gate_ids)
}
