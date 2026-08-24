pub fn ingest_repository_quality(
    repository: &RepositorySnapshot,
    remote: Option<&RemoteRepositorySnapshot>,
    maturity: Option<QualityMaturity>,
    ci_gate_profile: Option<&CiGateProfile>,
) -> QualitySnapshot {
    let mut gates = default_quality_gates();
    let mut findings = QualityFindings::default();
    let mut last_ingested_at = None;
    let mut configured_gate_ids = Vec::new();
    let (web_readiness, web_readiness_evidence) = import_web_readiness(repository);
    let release_boundary = release_boundary::import_release_boundary(repository);

    if let Some(run) = latest_qr_run(Path::new(&repository.path)) {
        last_ingested_at = run.observed_at.clone();
        configured_gate_ids.extend(run.configured_gate_ids());
        for evidence in run.gate_evidence(repository) {
            configured_gate_ids.push(evidence.id.clone());
            add_evidence(&mut gates, evidence);
        }
        findings = run.findings(repository);
    }
    reconcile_finding_dispositions(Path::new(&repository.path), &mut findings);
    if let Some(evidence) = debloat_maturity_evidence(&findings) {
        configured_gate_ids.push(evidence.id.clone());
        add_evidence(&mut gates, evidence);
    }

    for evidence in ci_evidence(repository, remote) {
        configured_gate_ids.push(evidence.id.clone());
        add_evidence(&mut gates, evidence);
    }

    if let Some(evidence) = web_readiness_evidence {
        if web_readiness.applicability != "not_applicable" {
            configured_gate_ids.push(evidence.id.clone());
        }
        add_evidence(&mut gates, evidence);
        if last_ingested_at.as_deref() < web_readiness.observed_at.as_deref() {
            last_ingested_at = web_readiness.observed_at.clone();
        }
    }
    if let Some(profile) = ci_gate_profile {
        ensure_profile_gates(&mut gates, profile);
    }

    for gate in &mut gates {
        let (status, freshness) = aggregate_gate_status(&gate.evidence);
        gate.status = status;
        gate.freshness = freshness;
    }
    let effective_required_gate_ids = ci_gate_profile.map(|profile| {
        let mut gate_ids = profile.required_gate_ids.clone();
        if matches!(
            web_readiness.applicability.as_str(),
            "public_web" | "internal_web"
        ) && !gate_ids.iter().any(|gate_id| gate_id == "web_readiness")
        {
            gate_ids.push("web_readiness".to_string());
        }
        gate_ids
    });
    let mut ci_readiness = effective_required_gate_ids
        .as_deref()
        .filter(|gate_ids| !gate_ids.is_empty())
        .map(|gate_ids| {
            evaluate_ci_readiness_for_ideal_with_configuration(
                &gates,
                gate_ids,
                &configured_gate_ids,
            )
        })
        .unwrap_or_default();
    if let Some(profile) = ci_gate_profile {
        apply_ci_gate_profile(&mut ci_readiness, profile);
    }
    let maturity_available = maturity.is_some();
    let installed_runtime = installed_runtime::evaluate(
        Path::new(&repository.path),
        repository.workspace.last_commit.as_deref(),
    );
    let runtime_available = installed_runtime.applicability == "applicable";
    let evidence_available = gates.iter().any(|gate| !gate.evidence.is_empty())
        || findings.total > 0
        || runtime_available;
    QualitySnapshot {
        gates,
        findings,
        maturity: maturity.unwrap_or_default(),
        foundation_readiness: FoundationReadinessGate::default(),
        target_fleet_audit_root: None,
        ci_readiness,
        mac_control_ideal_state: MacControlRepositoryState::default(),
        behavior_assurance: BehaviorAssuranceRepositoryState::default(),
        evidence_contracts: Vec::new(),
        web_readiness,
        release_boundary,
        installed_runtime,
        last_ingested_at,
        ingestion_status: if evidence_available || maturity_available {
            "Available".to_string()
        } else {
            "No evidence".to_string()
        },
        ingestion_message: if evidence_available || maturity_available {
            None
        } else {
            Some("No QR artifacts or CI check runs were found for this repository.".to_string())
        },
    }
}

pub fn project_quality_snapshot_for_target(
    snapshot: &mut QualitySnapshot,
    target_branch: &str,
    target_commit: &str,
) {
    for gate in &mut snapshot.gates {
        gate.evidence.retain(|evidence| {
            target_provenance_matches(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            )
        });
        for evidence in &mut gate.evidence {
            evidence.freshness = evaluate_target_freshness(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            );
        }
        let (status, freshness) = aggregate_gate_status(&gate.evidence);
        gate.status = status;
        gate.freshness = freshness;
    }

    let findings_match = target_provenance_matches(
        snapshot.findings.scanned_branch.as_deref(),
        snapshot.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    if findings_match {
        snapshot.findings.freshness = evaluate_target_freshness(
            snapshot.findings.scanned_branch.as_deref(),
            snapshot.findings.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    } else if snapshot.findings.refresh_required
        && snapshot.findings.detector_status.as_deref() == Some("blocked")
        && snapshot.findings.detector_findings_total > 0
    {
        // Keep the prior valid report available as stale/raw evidence when a
        // replacement detector run is blocked. The target UI still refuses to
        // present it as a verified result because refresh_required remains set.
        snapshot.findings.freshness = QualityFreshness::Stale;
    } else {
        snapshot.findings = QualityFindings::default();
    }

    let maturity_match = target_provenance_matches(
        snapshot.maturity.scanned_branch.as_deref(),
        snapshot.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    if maturity_match {
        snapshot.maturity.freshness = evaluate_target_freshness(
            snapshot.maturity.scanned_branch.as_deref(),
            snapshot.maturity.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    } else {
        snapshot.maturity = QualityMaturity::default();
    }

    if target_provenance_matches(
        snapshot.web_readiness.scanned_branch.as_deref(),
        snapshot.web_readiness.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    ) {
        snapshot.web_readiness.freshness = QualityFreshness::Fresh;
    } else if snapshot.web_readiness.report_path.is_some() {
        snapshot.web_readiness.freshness = evaluate_target_freshness(
            snapshot.web_readiness.scanned_branch.as_deref(),
            snapshot.web_readiness.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    }

    release_boundary::project_for_target(
        &mut snapshot.release_boundary,
        target_branch,
        target_commit,
    );

    let configured_gate_ids = snapshot
        .gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    snapshot.ci_readiness = evaluate_ci_readiness_for_ideal_with_configuration(
        &snapshot.gates,
        &snapshot.ci_readiness.applicable_gate_ids,
        &configured_gate_ids,
    );
    snapshot.ingestion_status = if snapshot.maturity.score.is_some()
        || snapshot.findings.source.is_some()
        || snapshot.gates.iter().any(|gate| !gate.evidence.is_empty())
    {
        "Available".to_string()
    } else {
        "No evidence".to_string()
    };
    snapshot.ingestion_message = (snapshot.ingestion_status == "No evidence").then(|| {
        "No target-scoped QR or fleet evidence was found for this branch and commit.".to_string()
    });
}

fn normalize_disposition_status(value: &str) -> Option<&'static str> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "_")
        .as_str()
    {
        "confirmed" => Some("confirmed"),
        "false_positive" => Some("false_positive"),
        "accepted_intentional" => Some("accepted_intentional"),
        "accepted_risk" => Some("accepted_risk"),
        "deferred" => Some("deferred"),
        "fixed" => Some("fixed"),
        "superseded" => Some("superseded"),
        _ => None,
    }
}

fn validate_finding_dispositions_contract(
    contract: &mut QualityFindingDispositionsContract,
) -> Result<(), String> {
    if contract.schema_version != FINDING_DISPOSITIONS_SCHEMA {
        return Err(format!(
            "Expected schema_version {FINDING_DISPOSITIONS_SCHEMA}, found {}",
            contract.schema_version
        ));
    }
    DateTime::parse_from_rfc3339(&contract.updated_at)
        .map_err(|error| format!("updated_at is not RFC 3339: {error}"))?;
    let mut fingerprints = HashSet::new();
    for disposition in &mut contract.dispositions {
        disposition.fingerprint = disposition.fingerprint.trim().to_string();
        if disposition.fingerprint.is_empty() {
            return Err("A finding disposition has an empty fingerprint".to_string());
        }
        if !fingerprints.insert(disposition.fingerprint.clone()) {
            return Err(format!(
                "Finding fingerprint {} is dispositioned more than once",
                disposition.fingerprint
            ));
        }
        disposition.status = normalize_disposition_status(&disposition.status)
            .ok_or_else(|| {
                format!(
                    "Finding {} has unsupported disposition status '{}'",
                    disposition.fingerprint, disposition.status
                )
            })?
            .to_string();
        if disposition.reason.trim().is_empty() {
            return Err(format!(
                "Finding {} is missing a disposition reason",
                disposition.fingerprint
            ));
        }
        if disposition.reviewer.trim().is_empty() {
            return Err(format!(
                "Finding {} is missing a reviewer",
                disposition.fingerprint
            ));
        }
        DateTime::parse_from_rfc3339(&disposition.reviewed_at).map_err(|error| {
            format!(
                "Finding {} reviewed_at is not RFC 3339: {error}",
                disposition.fingerprint
            )
        })?;
        if let Some(expires_at) = disposition.expires_at.as_deref() {
            DateTime::parse_from_rfc3339(expires_at).map_err(|error| {
                format!(
                    "Finding {} expires_at is not RFC 3339: {error}",
                    disposition.fingerprint
                )
            })?;
        }
    }
    contract
        .dispositions
        .sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
    Ok(())
}

fn load_finding_dispositions_contract(
    repository_path: &Path,
) -> Result<Option<QualityFindingDispositionsContract>, String> {
    let path = repository_path.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
    if !path.is_file() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut contract = serde_json::from_str::<QualityFindingDispositionsContract>(&contents)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))?;
    validate_finding_dispositions_contract(&mut contract)?;
    Ok(Some(contract))
}

struct ReportFindingInventory {
    fingerprint_counts: HashMap<String, u64>,
    fingerprint_category_counts: HashMap<String, BTreeMap<String, u64>>,
    category_counts: BTreeMap<String, u64>,
}

fn report_finding_inventory(report_paths: &[String]) -> Option<ReportFindingInventory> {
    let mut fingerprint_counts = HashMap::new();
    let mut fingerprint_category_counts: HashMap<String, BTreeMap<String, u64>> = HashMap::new();
    let mut derived_category_counts = BTreeMap::new();
    let mut prior_report_fingerprints: HashSet<String> = HashSet::new();
    let mut saw_findings = false;
    for payload in report_paths
        .iter()
        .filter_map(|path| read_json(Path::new(path)))
    {
        let Some(findings) = payload.get("findings").and_then(Value::as_array) else {
            continue;
        };
        saw_findings = true;
        let mut current_report_fingerprints = HashSet::new();
        for finding in findings {
            let fingerprint = finding
                .get("fingerprint")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if fingerprint.is_some_and(|value| prior_report_fingerprints.contains(value)) {
                continue;
            }
            if let Some(fingerprint) = fingerprint {
                current_report_fingerprints.insert(fingerprint.to_string());
            }
            let category = finding
                .get("category")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(category) = category {
                *derived_category_counts
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
            let Some(fingerprint) = fingerprint else {
                continue;
            };
            *fingerprint_counts
                .entry(fingerprint.to_string())
                .or_insert(0) += 1;
            if let Some(category) = category {
                *fingerprint_category_counts
                    .entry(fingerprint.to_string())
                    .or_default()
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
        }
        prior_report_fingerprints.extend(current_report_fingerprints);
    }
    if !saw_findings {
        return None;
    }
    Some(ReportFindingInventory {
        fingerprint_counts,
        fingerprint_category_counts,
        category_counts: derived_category_counts,
    })
}

fn disposition_is_expired(disposition: &QualityFindingDisposition, now: DateTime<Utc>) -> bool {
    disposition
        .expires_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|expires_at| expires_at.with_timezone(&Utc) <= now)
}

pub fn non_actionable_finding_fingerprints(
    repository_path: &Path,
) -> Result<HashSet<String>, String> {
    let Some(contract) = load_finding_dispositions_contract(repository_path)? else {
        return Ok(HashSet::new());
    };
    let now = Utc::now();
    Ok(contract
        .dispositions
        .into_iter()
        .filter(|disposition| !disposition_is_expired(disposition, now))
        .filter(|disposition| {
            matches!(
                disposition.status.as_str(),
                "false_positive" | "accepted_intentional" | "accepted_risk"
            )
        })
        .map(|disposition| disposition.fingerprint)
        .collect())
}
