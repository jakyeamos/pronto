fn add_maturity_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let maturity = &repository.quality.maturity;
    let freshness = maturity.freshness.as_str().to_string();
    let observed_at = maturity.observed_at.as_deref();
    let report_path = maturity.report_path.as_deref();
    match maturity.score {
        None => seeds.push(maturity_seed(
            "maturity:score",
            "Get a current maturity score",
            "No repository maturity score is available in the imported feed.",
            "Missing",
            &freshness,
            observed_at,
            report_path,
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
            2,
        )),
        Some(score) if freshness != "Fresh" => seeds.push(maturity_seed(
            "maturity:score",
            "Refresh the maturity score",
            &format!("The maturity score is {score:.3}/4 but its evidence is {freshness}."),
            "Stale",
            &freshness,
            observed_at,
            report_path,
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
            2,
        )),
        Some(score) if score < MATURITY_CLOSURE_TARGET => seeds.push(maturity_seed(
            "maturity:score",
            "Raise the repository maturity score",
            &format!(
                "The current maturity score is {score:.3}/4; the minimum closure target is {MATURITY_CLOSURE_TARGET:.1}/4 and the evidence-backed ideal is {MATURITY_IDEAL_SCORE:.1}/4."
            ),
            "Below target",
            &freshness,
            observed_at,
            report_path,
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
            3,
        )),
        Some(_) => {}
    }
    for (dimension, score) in &maturity.dimension_scores {
        if *score < MATURITY_CLOSURE_TARGET {
            let (title, first_acceptance_criterion) = maturity_dimension_action_copy(dimension);
            seeds.push(ActionSeed {
                stable_key: format!("maturity:dimension:{dimension}"),
                domain: "maturity".to_string(),
                title,
                summary: format!(
                    "The dimension score is {score:.3}/4; the minimum closure target is {MATURITY_CLOSURE_TARGET:.1}/4 and the evidence-backed ideal is {MATURITY_IDEAL_SCORE:.1}/4."
                ),
                severity: "maturity".to_string(),
                priority: "P2".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    first_acceptance_criterion,
                    format!(
                        "Fresh imported maturity evidence reports {dimension} at or above {MATURITY_CLOSURE_TARGET:.1}/4."
                    ),
                    maturity_improvement_rule(),
                    maturity_integrity_rule(),
                ],
                evidence: vec![evidence_with_provenance(
                    "Quality Runner maturity evidence",
                    &format!("Maturity dimension · {dimension}"),
                    &format!("{score:.3}/4"),
                    &freshness,
                    observed_at,
                    report_path,
                    "Dimension-level score imported from the latest QR maturity evidence.",
                    maturity.scanned_branch.as_deref(),
                    maturity.scanned_commit.as_deref(),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: maturity.audit_id.clone(),
            });
        }
    }
}

fn maturity_dimension_action_copy(dimension: &str) -> (String, String) {
    match dimension {
        "long_running_task_observability" => (
            "Add agent-readable progress to long-running tasks".to_string(),
            "Expose machine-readable heartbeat or progress state and actionable diagnostics for every qualified long-running task.".to_string(),
        ),
        "long_running_task_optimization" => (
            "Review long-running tasks for avoidable repeated work".to_string(),
            "Document or implement bounded execution and work-reduction evidence such as batching, caching, checkpointing, cursors, incremental work, or one-time materialization.".to_string(),
        ),
        _ => (
            format!("Improve the {dimension} maturity dimension"),
            format!("Address the evidence-backed gaps for {dimension}."),
        ),
    }
}

fn add_evidence_contract_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for contract in repository
        .quality
        .evidence_contracts
        .iter()
        .filter(|contract| contract.status != "current")
    {
        let observed = contract.observed_schema.as_deref().unwrap_or("missing");
        seeds.push(ActionSeed {
            stable_key: format!("evidence-contract:{}", contract.contract_id),
            domain: "evidence".to_string(),
            title: format!("Re-audit {} against {}", contract.label, contract.target_schema),
            summary: contract.message.clone(),
            severity: "maturity".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                format!(
                    "The owning producer emits {} evidence for this repository.",
                    contract.target_schema
                ),
                "Pronto refreshes the evidence and reports this repository's contract status as current.".to_string(),
                "The audit preserves positive, negative, and ambiguous evidence; do not convert missing evidence into a pass.".to_string(),
            ],
            evidence: vec![evidence(
                "Evidence contract",
                &contract.label,
                observed,
                "Contract audit required",
                None,
                None,
                &contract.message,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_maturity_gate_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if !goal
        .maturity_gate_ids
        .iter()
        .any(|gate_id| gate_id == mac_control_maturity::MAC_CONTROL_GATE_ID)
    {
        return;
    }
    if repository
        .quality
        .evidence_contracts
        .iter()
        .any(|contract| {
            contract.contract_id == mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_ID
                && contract.status != "current"
        })
    {
        return;
    }
    let gate = &repository.quality.mac_control_ideal_state;
    if (gate.status == "Passed" && gate.freshness == "Fresh")
        || (gate.status == "Not applicable" && gate.freshness == "Fresh")
    {
        return;
    }
    let summary = if gate.failure_reasons.is_empty() {
        format!(
            "The Mac Control ideal-state gate is {} with {} evidence; a fresh passing gate is required before Pronto can claim the 4.0/4.0 maturity ideal.",
            gate.status, gate.freshness
        )
    } else {
        format!(
            "The Mac Control ideal-state gate is {} with {} evidence: {}",
            gate.status,
            gate.freshness,
            gate.failure_reasons.join("; ")
        )
    };
    seeds.push(ActionSeed {
        stable_key: format!("maturity:gate:{}", mac_control_maturity::MAC_CONTROL_GATE_ID),
        domain: "maturity".to_string(),
        title: "Bring the Mac Control ideal-state gate to a fresh pass".to_string(),
        summary: summary.clone(),
        severity: "maturity".to_string(),
        priority: if gate.status == "Blocked" || gate.status == "Not configured" {
            "P1".to_string()
        } else {
            "P2".to_string()
        },
        weight: 3,
        acceptance_criteria: vec![
            "The canonical report accounts for every repository in Pronto's current maturity scope; no fixed repository count is assumed.".to_string(),
            "Every applicable supported task exposes a stable target, direct semantic action, observable postcondition, meaningful hierarchy, required state observations, explicit change states, and a measured eligible route.".to_string(),
            "The report is fresh and its observed commit matches each current repository commit before the 4.0/4.0 maturity ideal is claimed.".to_string(),
            "Do not add tab stops, Accessibility-only routes, or visual evidence solely to raise the maturity score; route choice must remain evidence-backed and task-appropriate.".to_string(),
            maturity_improvement_rule(),
            maturity_integrity_rule(),
        ],
        evidence: vec![evidence(
            "Mac Control",
            mac_control_maturity::MAC_CONTROL_GATE_LABEL,
            &gate.status,
            &gate.freshness,
            gate.observed_at.as_deref(),
            gate.report_path.as_deref(),
            &summary,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn maturity_seed(
    stable_key: &str,
    title: &str,
    summary: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    weight: u64,
) -> ActionSeed {
    ActionSeed {
        stable_key: stable_key.to_string(),
        domain: "maturity".to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        severity: "maturity".to_string(),
        priority: if weight >= 3 { "P1" } else { "P2" }.to_string(),
        weight,
        acceptance_criteria: vec![
            "Quality Runner maturity evidence is refreshed after the relevant work.".to_string(),
            format!(
                "The resulting maturity evidence is fresh and at or above {MATURITY_CLOSURE_TARGET:.1}/4 where applicable."
            ),
            maturity_improvement_rule(),
            maturity_integrity_rule(),
        ],
        evidence: vec![evidence_with_provenance(
            "Quality Runner maturity evidence",
            "Repository maturity",
            status,
            freshness,
            observed_at,
            report_path,
            summary,
            scanned_branch,
            scanned_commit,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    }
}

fn materialize_action(
    repository: &RepositorySnapshot,
    seed: ActionSeed,
    previous: &HashMap<&str, &RemediationAction>,
    generated_at: &str,
) -> RemediationAction {
    let legacy_key = legacy_action_key_for_current(&seed.stable_key);
    let previous_action = previous.get(seed.stable_key.as_str()).copied().or_else(|| {
        legacy_key
            .as_deref()
            .and_then(|key| previous.get(key).copied())
    });
    let preserved_status = previous_action
        .filter(|action| {
            matches!(
                action.status.as_str(),
                "in_progress" | "blocked" | "deferred"
            ) || (action.status == "verified"
                && !action_was_resolved_by_refresh(action)
                && seed
                    .evidence
                    .iter()
                    .any(|item| item.freshness.eq_ignore_ascii_case("fresh")))
        })
        .map(|action| action.status.clone())
        .unwrap_or_else(|| "open".to_string());
    let preserved_status = if seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY {
        "blocked".to_string()
    } else {
        preserved_status
    };
    let notes = previous_action.and_then(|action| action.notes.clone());
    RemediationAction {
        id: stable_id(
            &format!("{}:{}", repository.id, seed.stable_key),
            "remediation-action",
        ),
        stable_key: seed.stable_key,
        repository_id: repository.id.clone(),
        domain: seed.domain,
        title: seed.title,
        summary: seed.summary,
        severity: seed.severity,
        priority: seed.priority,
        weight: seed.weight,
        status: preserved_status,
        acceptance_criteria: seed.acceptance_criteria,
        evidence: seed.evidence,
        related_finding_ids: seed.related_finding_ids,
        source_run_id: seed.source_run_id,
        updated_at: generated_at.to_string(),
        completed_at: previous_action.and_then(|action| action.completed_at.clone()),
        notes,
    }
}

fn action_was_resolved_by_refresh(action: &RemediationAction) -> bool {
    action
        .evidence
        .iter()
        .any(|item| item.label == RESOLVED_BY_REFRESH_LABEL)
}

fn normalized_retained_weight(action: &RemediationAction) -> u64 {
    if action.stable_key.starts_with("qr_findings:group:")
        || action.stable_key == "qr_findings:aggregate-report"
    {
        severity_weight(&action.severity)
    } else {
        action.weight
    }
}

fn is_fleet_maturity_qr_action_key(stable_key: &str) -> bool {
    stable_key.starts_with("qr_findings:group:")
        && stable_key.contains(&format!("|{FLEET_MATURITY_FINDING_PACK_PREFIX}"))
}

fn legacy_action_key_for_current(stable_key: &str) -> Option<String> {
    stable_key
        .strip_prefix(DEBLOAT_GROUP_KEY_PREFIX)
        .map(|pack| format!("{LEGACY_DEBLOAT_GROUP_KEY_PREFIX}{pack}"))
}

fn retain_resolved_action_history(
    actions: &mut Vec<RemediationAction>,
    previous: Option<&RemediationPlan>,
    generated_at: &str,
) {
    let Some(previous) = previous else {
        return;
    };
    let current_keys = actions
        .iter()
        .map(|action| action.stable_key.clone())
        .collect::<HashSet<_>>();
    let superseded_legacy_keys = current_keys
        .iter()
        .filter_map(|stable_key| legacy_action_key_for_current(stable_key))
        .collect::<HashSet<_>>();
    let compass_items_are_grouped = current_keys.contains(PROJECT_COMPASS_OPEN_ITEMS_KEY);
    let mut resolved = previous
        .actions
        .iter()
        .filter(|action| {
            action.stable_key != VERIFICATION_ACTION_KEY
                && !current_keys.contains(&action.stable_key)
                && !superseded_legacy_keys.contains(&action.stable_key)
                && !is_fleet_maturity_qr_action_key(&action.stable_key)
                && !(compass_items_are_grouped
                    && LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS.contains(&action.stable_key.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();

    for action in &mut resolved {
        action.weight = normalized_retained_weight(action);
        action.status = "verified".to_string();
        action.updated_at = generated_at.to_string();
        action.completed_at = Some(generated_at.to_string());
        if !action_was_resolved_by_refresh(action) {
            action.evidence.push(evidence(
                "Pronto remediation",
                RESOLVED_BY_REFRESH_LABEL,
                "Resolved",
                "Fresh",
                Some(generated_at),
                None,
                "The refreshed projection no longer emits this action. It is retained as verified history while other remediation remains active.",
            ));
        }
    }
    actions.extend(resolved);
}
