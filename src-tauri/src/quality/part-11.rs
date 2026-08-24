pub fn reconcile_finding_dispositions(repository_path: &Path, findings: &mut QualityFindings) {
    sync_detector_counts(findings);
    findings.actionable_total = findings.total;
    findings.reviewed_total = 0;
    findings.unreviewed_total = findings.total;
    findings.disposition_counts.clear();
    findings.stale_disposition_total = 0;
    findings.disposition_contract_path = Some(
        repository_path
            .join(FINDING_DISPOSITIONS_RELATIVE_PATH)
            .to_string_lossy()
            .to_string(),
    );
    findings.disposition_message = None;
    let report_paths = if findings.report_paths.is_empty() {
        findings.report_path.iter().cloned().collect::<Vec<_>>()
    } else {
        findings.report_paths.clone()
    };
    let report_inventory = report_finding_inventory(&report_paths);
    if let Some(inventory) = report_inventory.as_ref() {
        findings.category_counts = inventory.category_counts.clone();
        findings.actionable_category_counts = inventory.category_counts.clone();
    } else {
        findings.category_counts.clear();
        findings.actionable_category_counts.clear();
    }

    let contract = match load_finding_dispositions_contract(repository_path) {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            findings.disposition_status = "Missing".to_string();
            findings.disposition_message = Some(
                "No repository-owned finding disposition contract was found; every detected finding remains unreviewed and actionable."
                    .to_string(),
            );
            sync_detector_counts(findings);
            return;
        }
        Err(error) => {
            findings.disposition_status = "Invalid".to_string();
            findings.disposition_message = Some(error);
            sync_detector_counts(findings);
            return;
        }
    };

    let Some(report_inventory) = report_inventory else {
        findings.disposition_status = "Unreconcilable".to_string();
        findings.disposition_message = Some(
            "The current finding report does not expose stable fingerprints; saved dispositions were not applied."
                .to_string(),
        );
        findings.stale_disposition_total = contract.dispositions.len() as u64;
        sync_detector_counts(findings);
        return;
    };
    let identified_total = report_inventory.fingerprint_counts.values().sum::<u64>();
    if identified_total != findings.total {
        findings.disposition_status = "Unreconcilable".to_string();
        findings.disposition_message = Some(format!(
            "The report declares {} findings but only {} expose stable fingerprints in this scope; saved dispositions were not applied.",
            findings.total, identified_total
        ));
        findings.stale_disposition_total = contract.dispositions.len() as u64;
        sync_detector_counts(findings);
        return;
    }

    let now = Utc::now();
    let mut actionable_reviewed = 0_u64;
    for disposition in &contract.dispositions {
        let current_count = report_inventory
            .fingerprint_counts
            .get(&disposition.fingerprint)
            .copied()
            .unwrap_or(0);
        let expired = disposition_is_expired(disposition, now);
        let applies_to_current = current_count > 0
            && !expired
            && !matches!(disposition.status.as_str(), "fixed" | "superseded");
        if !applies_to_current {
            findings.stale_disposition_total += 1;
            continue;
        }
        *findings
            .disposition_counts
            .entry(disposition.status.clone())
            .or_insert(0) += current_count;
        findings.reviewed_total += current_count;
        if matches!(disposition.status.as_str(), "confirmed" | "deferred") {
            actionable_reviewed += current_count;
        }
        if matches!(
            disposition.status.as_str(),
            "false_positive" | "accepted_intentional" | "accepted_risk"
        ) {
            if let Some(categories) = report_inventory
                .fingerprint_category_counts
                .get(&disposition.fingerprint)
            {
                for (category, count) in categories {
                    if let Some(actionable) = findings.actionable_category_counts.get_mut(category)
                    {
                        *actionable = actionable.saturating_sub(*count);
                    }
                }
            }
        }
    }
    findings.unreviewed_total = findings.total.saturating_sub(findings.reviewed_total);
    findings.actionable_total = findings.unreviewed_total + actionable_reviewed;
    findings.disposition_status = "Ready".to_string();
    sync_detector_counts(findings);
}

fn debloat_maturity_evidence(findings: &QualityFindings) -> Option<QualityEvidence> {
    let detected = findings.category_counts.get("debloat").copied()?;
    let actionable = findings
        .actionable_category_counts
        .get("debloat")
        .copied()
        .unwrap_or(detected);
    let status = if actionable == 0 && findings.freshness == QualityFreshness::Fresh {
        QualityGateStatus::Passed
    } else {
        QualityGateStatus::Blocked
    };
    let detail = if actionable > 0 {
        format!(
            "QR's structural scan reported {detected} debloat signal(s), with {actionable} unresolved. Each signal requires a broader ownership-pressure audit with confidence assessed separately from implementation readiness; no signal authorizes deletion."
        )
    } else if findings.freshness != QualityFreshness::Fresh {
        format!(
            "QR reported no unresolved structural debloat signals, but the evidence is {}; refresh the QR scan before treating this review gate as passed.",
            findings.freshness.as_str()
        )
    } else {
        format!(
            "QR's structural debloat review has no unresolved signals ({detected} detected). This clears the candidate-review gate only; it does not prove that an ownership-pressure audit found no architectural opportunity, establish deletion readiness, or authorize deletion."
        )
    };
    Some(QualityEvidence {
        id: "debloat".to_string(),
        source: QualitySource::Qr,
        status,
        freshness: findings.freshness.clone(),
        observed_at: findings.observed_at.clone(),
        scanned_commit: findings.scanned_commit.clone(),
        scanned_branch: findings.scanned_branch.clone(),
        command: None,
        source_label: "Quality Runner structural debloat signals".to_string(),
        report_path: findings.report_path.clone(),
        report_url: None,
        report_kind: Some("code-quality-scan".to_string()),
        detail,
        verification_level: QualityVerificationLevel::SourceInferred,
        target_kind: Some("source".to_string()),
        target_url: None,
        target_provider: None,
        deployment_id: None,
    })
}

pub fn set_finding_disposition(
    repository_path: &Path,
    fingerprint: &str,
    status: &str,
    reason: &str,
    reviewer: &str,
    evidence: Vec<String>,
    expires_at: Option<String>,
) -> Result<QualityFindingDispositionsContract, String> {
    let fingerprint = fingerprint.trim();
    let reason = reason.trim();
    let reviewer = reviewer.trim();
    if fingerprint.is_empty() {
        return Err("Finding fingerprint must not be empty".to_string());
    }
    let status = normalize_disposition_status(status).ok_or_else(|| {
        "Disposition status must be confirmed, false_positive, accepted_intentional, accepted_risk, deferred, fixed, or superseded"
            .to_string()
    })?;
    if reason.is_empty() {
        return Err("A disposition reason is required".to_string());
    }
    if reviewer.is_empty() {
        return Err("A reviewer is required".to_string());
    }
    if let Some(value) = expires_at.as_deref() {
        DateTime::parse_from_rfc3339(value)
            .map_err(|error| format!("expires_at must be RFC 3339: {error}"))?;
    }
    let now = Utc::now().to_rfc3339();
    let mut contract = load_finding_dispositions_contract(repository_path)?.unwrap_or(
        QualityFindingDispositionsContract {
            schema_version: FINDING_DISPOSITIONS_SCHEMA.to_string(),
            updated_at: now.clone(),
            dispositions: Vec::new(),
        },
    );
    contract
        .dispositions
        .retain(|item| item.fingerprint != fingerprint);
    contract.dispositions.push(QualityFindingDisposition {
        fingerprint: fingerprint.to_string(),
        status: status.to_string(),
        reason: reason.to_string(),
        reviewer: reviewer.to_string(),
        reviewed_at: now.clone(),
        evidence: evidence
            .into_iter()
            .map(|item| item.trim().chars().take(512).collect::<String>())
            .filter(|item| !item.is_empty())
            .take(16)
            .collect(),
        expires_at,
    });
    contract.updated_at = now;
    validate_finding_dispositions_contract(&mut contract)?;
    let path = repository_path.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
    let parent = path
        .parent()
        .ok_or_else(|| format!("Could not resolve parent directory for {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    let payload = serde_json::to_string_pretty(&contract)
        .map_err(|error| format!("Could not encode finding dispositions: {error}"))?;
    let temporary_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temporary_path, format!("{payload}\n")).map_err(|error| {
        format!(
            "Could not write temporary finding dispositions {}: {error}",
            temporary_path.display()
        )
    })?;
    fs::rename(&temporary_path, &path)
        .map_err(|error| format!("Could not replace {} atomically: {error}", path.display()))?;
    Ok(contract)
}

pub fn evaluate_requirement(
    repository: &RepositorySnapshot,
    requirement: &QualityGateRequirement,
) -> (QualityGateStatus, QualityFreshness, String) {
    let gate_id = normalize_gate_id(&requirement.gate_id);
    let Some(gate) = repository
        .quality
        .gates
        .iter()
        .find(|gate| gate.id == gate_id)
    else {
        return (
            QualityGateStatus::NotConfigured,
            QualityFreshness::Unknown,
            format!("{} has no imported evidence", gate_label(&gate_id)),
        );
    };
    let source_evidence = gate
        .evidence
        .iter()
        .filter(|item| item.source == requirement.source)
        .collect::<Vec<_>>();
    if source_evidence.is_empty() {
        return (
            QualityGateStatus::NotConfigured,
            QualityFreshness::Unknown,
            format!(
                "{} has no {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    let evidence = source_evidence
        .into_iter()
        .filter(|item| {
            requirement
                .minimum_verification_level
                .as_ref()
                .is_none_or(|minimum| item.verification_level.satisfies(minimum))
        })
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        let minimum = requirement
            .minimum_verification_level
            .as_ref()
            .map(QualityVerificationLevel::as_str)
            .unwrap_or("unknown");
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Unknown,
            format!(
                "{} has no {} evidence at or above {} verification",
                gate.label,
                requirement.source.as_str(),
                minimum
            ),
        );
    }
    let has_conflict = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed)
        && evidence.iter().any(|item| {
            item.status == QualityGateStatus::Failed || item.status == QualityGateStatus::Blocked
        });
    if has_conflict {
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Conflicted,
            format!(
                "{} has conflicting {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Blocked)
    {
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Unknown,
            format!(
                "{} is blocked by {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Failed)
    {
        return (
            QualityGateStatus::Failed,
            evidence_freshness(&evidence),
            format!(
                "{} failed in {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    let freshness = evidence_freshness(&evidence);
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed)
    {
        (
            QualityGateStatus::Passed,
            freshness,
            format!(
                "{} passed in {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        )
    } else {
        (
            QualityGateStatus::NotConfigured,
            freshness,
            format!(
                "{} has no passing {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        )
    }
}

pub fn audit_import(root: Option<&Path>, repositories: &[RepositorySnapshot]) -> AuditImport {
    let Some(root) = root else {
        return AuditImport::default();
    };
    let mut portfolio = QualityPortfolioSnapshot {
        audit_root: Some(root.to_string_lossy().to_string()),
        ..QualityPortfolioSnapshot::default()
    };
    let Some(run) = latest_audit_run(root) else {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    };
    portfolio.latest_audit_id = run.audit_id.clone();
    portfolio.latest_audit_at = run.as_of.clone();
    portfolio.latest_audit_path = Some(run.summary_path.to_string_lossy().to_string());
    portfolio.maturity_score = run.mean_maturity;
    portfolio.maturity_score_display = run.mean_maturity_display.clone();
    portfolio.scored_dimension_count = run.scored_dimension_count;
    portfolio.audit_status = "Ready".to_string();

    let mut matches = HashMap::new();
    for repository in repositories {
        let candidates = run
            .findings
            .iter()
            .filter(|finding| {
                canonical_path_matches(finding.canonical_path.as_deref(), &repository.path)
            })
            .collect::<Vec<_>>();
        let selected = if candidates.len() == 1 {
            candidates.first().copied()
        } else if candidates.is_empty() {
            let remote_key = repository.remote_url.as_deref().and_then(remote_identity);
            let remote_matches = remote_key.as_deref().map_or_else(Vec::new, |key| {
                run.findings
                    .iter()
                    .filter(|finding| finding.remote_key.as_deref() == Some(key))
                    .collect::<Vec<_>>()
            });
            (remote_matches.len() == 1)
                .then(|| remote_matches.first().copied())
                .flatten()
        } else {
            None
        };
        if let Some(finding) = selected {
            let maturity = QualityMaturity {
                score: finding.mean_maturity,
                score_display: finding.mean_maturity_display.clone(),
                scored_dimension_count: finding.scored_dimension_count,
                dimension_scores: finding.dimension_scores.clone(),
                gaps: Vec::new(),
                quality_outcome: None,
                agent_usability: None,
                repository_maturity: None,
                cache_design: None,
                ci_gate_audit: None,
                audit_id: run.audit_id.clone(),
                observed_at: run.as_of.clone(),
                scanned_commit: None,
                scanned_branch: None,
                freshness: evaluate_audit_freshness_at(run.as_of.as_deref(), Utc::now()),
                report_path: Some(finding.path.to_string_lossy().to_string()),
            };
            matches.insert(repository.id.clone(), maturity);
        }
    }
    portfolio.matched_repository_count = matches.len();
    AuditImport {
        portfolio,
        maturities: matches,
        behavior_assurance: HashMap::new(),
    }
}
