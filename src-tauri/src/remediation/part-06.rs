fn add_ci_ideal_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let readiness = &repository.quality.ci_readiness;
    let mut gate_ids = readiness.unconfigured_gate_ids.clone();
    gate_ids.extend(readiness.missing_gate_ids.iter().cloned());
    gate_ids.extend(readiness.stale_gate_ids.iter().cloned());
    gate_ids.extend(readiness.failed_gate_ids.iter().cloned());
    gate_ids.extend(readiness.blocked_gate_ids.iter().cloned());
    gate_ids.sort();
    gate_ids.dedup();
    gate_ids.retain(|gate_id| goal.required_gate_ids.contains(gate_id));
    for gate_id in gate_ids {
        let status = if readiness.failed_gate_ids.contains(&gate_id) {
            "Failed"
        } else if readiness.blocked_gate_ids.contains(&gate_id) {
            "Blocked"
        } else if readiness.stale_gate_ids.contains(&gate_id) {
            "Stale"
        } else if readiness.unconfigured_gate_ids.contains(&gate_id) {
            "Not configured"
        } else {
            "Missing"
        };
        let label = readiness
            .gate_labels
            .get(&gate_id)
            .cloned()
            .unwrap_or_else(|| quality::gate_label(&gate_id));
        let gate_evidence = repository
            .quality
            .gates
            .iter()
            .find(|gate| gate.id == gate_id)
            .and_then(|gate| gate.evidence.first());
        let freshness = repository
            .quality
            .gates
            .iter()
            .find(|gate| gate.id == gate_id)
            .map(|gate| gate.freshness.as_str())
            .unwrap_or_else(|| {
                if readiness.stale_gate_ids.contains(&gate_id) {
                    "Stale"
                } else {
                    "Unknown"
                }
            });
        seeds.push(ActionSeed {
            stable_key: format!("ci_ideal:gate:{gate_id}"),
            domain: "ci_ideal".to_string(),
            title: format!("Bring the {label} gate to the ideal state"),
            summary: format!(
                "The '{}' remediation goal requires {label}; its current state is {status}.",
                goal.label
            ),
            severity: if matches!(status, "Failed" | "Blocked") {
                "high".to_string()
            } else {
                "quality".to_string()
            },
            priority: if matches!(status, "Failed" | "Blocked") {
                "P1".to_string()
            } else {
                "P2".to_string()
            },
            weight: 2,
            acceptance_criteria: vec![
                format!(
                    "The {label} gate is configured according to the repository ideal profile."
                ),
                format!("A fresh passing {label} result is recorded where the gate is executable."),
                "The gate is represented in Pronto with its source, status, and freshness."
                    .to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Pronto quality",
                &format!("Ideal CI gate · {label}"),
                status,
                freshness,
                gate_evidence.and_then(|item| item.observed_at.as_deref()),
                gate_evidence.and_then(|item| item.report_path.as_deref()),
                &format!(
                    "Required by the repository's '{}' remediation goal.",
                    goal.target_state
                ),
                gate_evidence.and_then(|item| item.scanned_branch.as_deref()),
                gate_evidence.and_then(|item| item.scanned_commit.as_deref()),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_qr_finding_seeds(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(run) = qr_run else {
        return;
    };
    let non_actionable = if repository.quality.findings.disposition_status == "Ready" {
        quality::non_actionable_finding_fingerprints(Path::new(&repository.path))
            .unwrap_or_default()
    } else {
        Default::default()
    };
    let mut groups = BTreeMap::<String, Vec<&ParsedFinding>>::new();
    for finding in run.findings.iter().filter(|finding| {
        let maturity_projection_owns_action = is_fleet_maturity_finding(finding)
            && (!goal_requires_maturity(goal)
                || repository
                    .quality
                    .maturity
                    .dimension_scores
                    .contains_key(&finding.category));
        !maturity_projection_owns_action
            && !finding
                .fingerprint
                .as_ref()
                .is_some_and(|fingerprint| non_actionable.contains(fingerprint))
    }) {
        groups
            .entry(finding.group_key.clone())
            .or_default()
            .push(finding);
    }
    for (group_key, findings) in groups {
        let severity = findings
            .iter()
            .max_by_key(|finding| severity_weight(&finding.severity))
            .map(|finding| finding.severity.clone())
            .unwrap_or_else(|| "warning".to_string());
        let max_weight = findings
            .iter()
            .map(|finding| severity_weight(&finding.severity))
            .max()
            .unwrap_or(2);
        let count = findings.len() as u64;
        let first = findings[0];
        let locations = findings
            .iter()
            .filter_map(|finding| {
                finding.file.as_ref().map(|file| match finding.line {
                    Some(line) => format!("{file}:{line}"),
                    None => file.clone(),
                })
            })
            .take(3)
            .collect::<Vec<_>>();
        let detail = if locations.is_empty() {
            format!("{} finding(s) in the current QR report.", findings.len())
        } else {
            format!(
                "{} finding(s); examples: {}",
                findings.len(),
                locations.join(", ")
            )
        };
        let source_path = first.report_path.clone();
        let source_label = first
            .pack
            .as_deref()
            .map(|pack| format!("QR · {pack} · {}", first.category))
            .unwrap_or_else(|| format!("QR · {}", first.category));
        seeds.push(ActionSeed {
            stable_key: format!("qr_findings:group:{group_key}"),
            domain: "qr_findings".to_string(),
            title: if first.title.is_empty() {
                format!("Resolve {} QR findings", first.category)
            } else {
                first.title.clone()
            },
            summary: format!("{} {}", first.summary, detail),
            severity: severity.clone(),
            priority: priority_for_weight(max_weight),
            // Repeated instances describe the scope of one remediation action; they must
            // not make that action dominate the entire plan's completion percentage.
            weight: max_weight.max(1),
            acceptance_criteria: findings
                .iter()
                .find_map(|finding| finding.verification.clone())
                .map(|verification| vec![verification])
                .unwrap_or_else(|| {
                    vec![
                        "The grouped QR findings are resolved in source or configuration."
                            .to_string(),
                        "The relevant verification command or evidence is rerun.".to_string(),
                        "A fresh QR report no longer includes these finding identities."
                            .to_string(),
                    ]
                }),
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                &source_label,
                &format!("{count} finding(s)"),
                &freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days),
                run.observed_at.as_deref(),
                Some(&source_path),
                &detail,
                run.scanned_branch.as_deref(),
                run.scanned_commit.as_deref(),
            )],
            related_finding_ids: findings.iter().map(|finding| finding.id.clone()).collect(),
            source_run_id: Some(run.id.clone()),
        });
    }
    if run.findings.is_empty() && repository.quality.findings.actionable_total > 0 {
        seeds.push(ActionSeed {
            stable_key: "qr_findings:aggregate-report".to_string(),
            domain: "qr_findings".to_string(),
            title: "Resolve the findings in the QR report".to_string(),
            summary: format!(
                "Pronto imported {} actionable QR findings, but the current artifact does not expose leaf identities for grouping.",
                repository.quality.findings.actionable_total
            ),
            severity: if repository.quality.findings.high_severity_total > 0 {
                "high".to_string()
            } else {
                "warning".to_string()
            },
            priority: "P1".to_string(),
            weight: if repository.quality.findings.high_severity_total > 0 {
                severity_weight("high")
            } else {
                severity_weight("warning")
            },
            acceptance_criteria: vec![
                "The current QR report is reviewed and its findings are addressed.".to_string(),
                "A fresh QR report is rerun to verify the result.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                "QR aggregate report",
                &repository.quality.findings.actionable_total.to_string(),
                repository.quality.findings.freshness.as_str(),
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "Leaf finding identities were not available in the current report.",
                repository.quality.findings.scanned_branch.as_deref(),
                repository.quality.findings.scanned_commit.as_deref(),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
}

fn add_debloat_gate_seed(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(gate) = repository
        .quality
        .gates
        .iter()
        .find(|gate| gate.id == "debloat")
    else {
        return;
    };
    if gate.status == quality::QualityGateStatus::Passed
        || seeds.iter().any(|seed| {
            seed.stable_key.starts_with(DEBLOAT_GROUP_CATEGORY_PREFIX)
                || seed.stable_key == "qr_findings:aggregate-report"
        })
    {
        return;
    }

    let detected = repository
        .quality
        .findings
        .category_counts
        .get("debloat")
        .copied()
        .unwrap_or_default();
    let actionable = repository
        .quality
        .findings
        .actionable_category_counts
        .get("debloat")
        .copied()
        .unwrap_or(detected);
    let gate_evidence = gate.evidence.first();
    seeds.push(ActionSeed {
        stable_key: DEBLOAT_GATE_ACTION_KEY.to_string(),
        domain: "qr_findings".to_string(),
        title: "Review the repository debloat signals".to_string(),
        summary: format!(
            "The debloat review gate is {} with {detected} detected structural signal(s) and {actionable} unresolved signal(s). No leaf remediation action remains after finding dispositions, so this action preserves an explicit path to review or refresh the signals without claiming architectural maturity.",
            gate.status.as_str()
        ),
        severity: "warning".to_string(),
        priority: "P2".to_string(),
        weight: severity_weight("warning"),
        acceptance_criteria: vec![
            "Each unresolved structural signal receives an ownership-pressure audit that checks duplicated engines, reusable helpers, parallel or legacy surfaces, and obsolete ownership paths."
                .to_string(),
            "Each audit finding records confidence independently from implementation or deletion readiness."
                .to_string(),
            "A fresh QR scan reports no unresolved structural debloat signals before the review gate passes."
                .to_string(),
            "Any deletion or structural rewrite is separately authorized and behavior-verified."
                .to_string(),
        ],
        evidence: vec![evidence_with_provenance(
            "Pronto quality",
            "Repository debloat review",
            gate.status.as_str(),
            gate.freshness.as_str(),
            gate_evidence.and_then(|item| item.observed_at.as_deref()),
            gate_evidence.and_then(|item| item.report_path.as_deref()),
            gate_evidence
                .map(|item| item.detail.as_str())
                .unwrap_or("The debloat review gate has no leaf evidence detail."),
            gate_evidence.and_then(|item| item.scanned_branch.as_deref()),
            gate_evidence.and_then(|item| item.scanned_commit.as_deref()),
        )],
        related_finding_ids: Vec::new(),
        source_run_id: qr_run.map(|run| run.id.clone()),
    });
}

fn is_fleet_maturity_finding(finding: &ParsedFinding) -> bool {
    finding
        .pack
        .as_deref()
        .is_some_and(|pack| pack.starts_with(quality::FLEET_MATURITY_FINDING_SCHEMA_PREFIX))
}
