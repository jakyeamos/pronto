pub fn derive_foundation_readiness(quality: &QualitySnapshot) -> FoundationReadinessGate {
    let mut gate = FoundationReadinessGate {
        freshness: quality.maturity.freshness.clone(),
        observed_at: quality.maturity.observed_at.clone(),
        scanned_commit: quality.maturity.scanned_commit.clone(),
        scanned_branch: quality.maturity.scanned_branch.clone(),
        unknowns: Vec::new(),
        ..FoundationReadinessGate::default()
    };
    let Some(model) = quality.maturity.repository_maturity.as_ref() else {
        return gate;
    };

    if model.evidence.applicable_pillar_count == 0
        && model.evidence.unknown_applicability.is_empty()
    {
        gate.disposition = "not_applicable".to_string();
        gate.confidence = "high".to_string();
        gate.summary = "No repository maturity pillars are applicable to this target.".to_string();
        gate.next_step = "Continue with the task-specific decision in chat; no repository modernization gate applies.".to_string();
        return gate;
    }

    let maintainability = model
        .pillars
        .iter()
        .find(|pillar| pillar.id == "maintainability_change_safety");
    let actionable_debloat = quality
        .findings
        .actionable_category_counts
        .get("debloat")
        .copied()
        .unwrap_or_default();
    let detected_debloat = quality
        .findings
        .category_counts
        .get("debloat")
        .copied()
        .unwrap_or_default();
    let findings_are_relevant = detected_debloat > 0 || quality.findings.source.is_some();
    let maturity_is_current = quality.maturity.freshness == QualityFreshness::Fresh;
    let findings_are_current = !findings_are_relevant
        || (quality.findings.freshness == QualityFreshness::Fresh
            && !quality.findings.refresh_required);

    if model.score.is_none() || model.evidence.evidence_coverage == 0.0 {
        gate.disposition = "unknown".to_string();
        gate.confidence = "low".to_string();
        gate.unknowns.push("repository_maturity_score".to_string());
        gate.summary = "Modernization readiness is unknown because the repository foundation has not been measured.".to_string();
        return gate;
    }

    if model.critical_cap.applied && maturity_is_current {
        gate.disposition = "modernize_first".to_string();
        gate.confidence = "high".to_string();
        gate.reasons.push(FoundationReadinessReason {
            id: "critical_maturity_blocker".to_string(),
            severity: "blocking".to_string(),
            summary:
                "Current correctness, security, or operability evidence caps repository maturity."
                    .to_string(),
            evidence: model.critical_cap.reasons.clone(),
        });
        gate.summary =
            "Stabilize the repository foundation before adding ordinary feature work.".to_string();
        gate.next_step = "In chat, propose the smallest foundation repair or modernization slice that clears the critical blocker; urgent containment fixes remain allowed.".to_string();
        return gate;
    }

    if !maturity_is_current || !findings_are_current {
        gate.disposition = "review_required".to_string();
        gate.confidence = "low".to_string();
        if !maturity_is_current {
            gate.unknowns.push("current_maturity_evidence".to_string());
        }
        if !findings_are_current {
            gate.unknowns
                .push("current_structural_findings".to_string());
        }
        gate.summary = "Modernization readiness needs review because foundation evidence is stale, conflicted, or refresh-required.".to_string();
        gate.next_step = "Refresh the repository evidence, then make the task-specific extend-versus-modernize recommendation in chat.".to_string();
        return gate;
    }

    if model.evidence.evidence_coverage < 0.6 || model.evidence.fresh_evidence_coverage < 0.6 {
        gate.disposition = "review_required".to_string();
        gate.confidence = "low".to_string();
        gate.unknowns
            .push("sufficient_foundation_coverage".to_string());
        gate.summary = "Modernization readiness needs review because foundation evidence coverage is incomplete.".to_string();
        gate.next_step = "Close the named maturity evidence gaps before treating the foundation as ready to extend.".to_string();
        return gate;
    }

    if actionable_debloat > 0 {
        gate.reasons.push(FoundationReadinessReason {
            id: "actionable_structural_signals".to_string(),
            severity: "modernize".to_string(),
            summary: format!(
                "Quality Runner reports {actionable_debloat} unresolved structural signal(s)."
            ),
            evidence: vec![format!(
                "debloat:{actionable_debloat}_actionable_of_{detected_debloat}_detected"
            )],
        });
    }
    if let Some(pillar) = maintainability {
        if pillar.score.is_some_and(|score| score < 3.0)
            || matches!(pillar.status.as_str(), "blocked" | "attention")
        {
            gate.reasons.push(FoundationReadinessReason {
                id: "maintainability_change_safety".to_string(),
                severity: if pillar.score.is_some_and(|score| score <= 1.0) {
                    "blocking"
                } else {
                    "modernize"
                }
                .to_string(),
                summary: format!(
                    "Maintainability and change safety are {} at {}.",
                    pillar.status,
                    pillar
                        .score
                        .map(|score| format!("{score:.2}/4.00"))
                        .unwrap_or_else(|| "an unknown score".to_string())
                ),
                evidence: pillar.dimension_scores.keys().cloned().collect(),
            });
        }
        if !pillar.missing_capabilities.is_empty() {
            gate.unknowns.extend(
                pillar
                    .missing_capabilities
                    .iter()
                    .map(|capability| format!("maintainability:{capability}")),
            );
        }
    }
    if quality.installed_runtime.applicability == "applicable"
        && quality.installed_runtime.status != "current"
    {
        gate.reasons.push(FoundationReadinessReason {
            id: "installed_runtime_drift".to_string(),
            severity: "modernize".to_string(),
            summary: quality.installed_runtime.summary.clone(),
            evidence: vec![format!(
                "installed_runtime:{}",
                quality.installed_runtime.status
            )],
        });
    }
    if quality.behavior_assurance.applicability == "applicable"
        && matches!(
            quality.behavior_assurance.state.as_str(),
            "missing_contract"
                | "legacy_v1"
                | "unprofiled"
                | "partially_verified"
                | "failed"
                | "blocked"
        )
    {
        gate.reasons.push(FoundationReadinessReason {
            id: "behavior_assurance_gap".to_string(),
            severity: "modernize".to_string(),
            summary: format!(
                "Behavior assurance is {} and does not yet provide a durable change-safety foundation.",
                quality.behavior_assurance.state
            ),
            evidence: vec![format!(
                "behavior_assurance:{}",
                quality.behavior_assurance.state
            )],
        });
    }

    let has_blocking_foundation_signal = gate
        .reasons
        .iter()
        .any(|reason| reason.severity == "blocking");
    let has_modernization_signal = !gate.reasons.is_empty();
    if has_blocking_foundation_signal {
        gate.disposition = "modernize_first".to_string();
        gate.confidence = "high".to_string();
        gate.summary = "Current evidence indicates that the repository foundation should be repaired before ordinary feature work.".to_string();
        gate.next_step = "In chat, propose a bounded foundation-first slice and explain how urgent fixes can proceed without deepening the debt.".to_string();
    } else if has_modernization_signal || !gate.unknowns.is_empty() {
        gate.disposition = "modernize_alongside".to_string();
        gate.confidence = if has_modernization_signal {
            "high"
        } else {
            "medium"
        }
        .to_string();
        gate.summary = "The repository can be extended, but the feature should carry a bounded modernization or evidence-closure slice.".to_string();
        gate.next_step = "In chat, compare the additive feature with a coupled modernization slice and recommend the smallest durable combination.".to_string();
    } else {
        gate.disposition = "ready_to_extend".to_string();
        gate.confidence = "high".to_string();
        gate.summary =
            "Current repository evidence supports extending the existing foundation.".to_string();
        gate.next_step = "Proceed with the task-specific design in chat while preserving current architecture and evidence contracts.".to_string();
    }
    gate
}

fn maturity_dimension_pillar(dimension: &str) -> Option<&'static str> {
    REPOSITORY_MATURITY_PILLARS
        .iter()
        .map(|(id, _, _, _)| *id)
        .find(|pillar| {
            maturity_pillar_patterns(pillar)
                .iter()
                .any(|pattern| dimension == *pattern || dimension.starts_with(pattern))
        })
}

fn maturity_dimension_matches(dimension: &str, patterns: &[&str]) -> bool {
    patterns
        .iter()
        .any(|pattern| dimension == *pattern || dimension.starts_with(pattern))
}

fn maturity_pillar_patterns(pillar: &str) -> &'static [&'static str] {
    match pillar {
        "correctness_reliability" => &[
            "behavior_assurance",
            "behavior_",
            "ci.",
            "dynamic_verification",
            "quality_commands",
            "reliability",
            "test_",
        ],
        "security_privacy_supply_chain" => &[
            "approval_gated_paths",
            "dependency_",
            "privacy",
            "security_",
            "secret_",
            "slsa",
            "supply_chain",
            "vulnerability",
        ],
        "maintainability_evolvability" => &[
            "architecture_boundaries",
            "change_surface_coverage",
            "coding_conventions",
            "maintainability",
            "strict_type_debt",
        ],
        "operability_release_safety" => &[
            "deployment_rollback",
            "diagnosability.",
            "failure_modes",
            "long_running_task_",
            "observability",
            "operability",
            "release_",
        ],
        "user_facing_quality" => &[
            "accessibility",
            "performance",
            "runtime_resource_efficiency",
            "user_journey",
            "web_readiness",
        ],
        "human_agent_usability" => &[
            "agent_usability.",
            "context_routing",
            "implementation_examples",
            "mac_control.",
            "skill_contract_quality",
        ],
        "governance_sustainability" => &[
            "cache_design",
            "contributor_",
            "definition_of_done",
            "governance",
            "license",
            "maintained",
            "matrix_maintenance",
            "ownership_",
        ],
        _ => &[],
    }
}

fn maturity_pillar_capabilities(
    pillar: &str,
) -> &'static [(&'static str, &'static [&'static str])] {
    match pillar {
        "correctness_reliability" => &[
            (
                "automated_quality_gates",
                &["quality_commands", "dynamic_verification", "ci."],
            ),
            ("behavior_outcomes", &["behavior_assurance", "behavior_"]),
            ("reliability_evidence", &["reliability", "test_"]),
        ],
        "security_privacy_supply_chain" => &[
            (
                "security_constraints",
                &["security_constraints", "approval_gated_paths"],
            ),
            (
                "dependency_and_vulnerability_risk",
                &["dependency_", "vulnerability"],
            ),
            ("secret_and_privacy_controls", &["secret_", "privacy"]),
            ("artifact_provenance", &["slsa", "supply_chain"]),
        ],
        "maintainability_evolvability" => &[
            ("architecture_boundaries", &["architecture_boundaries"]),
            ("change_impact_contract", &["change_surface_coverage"]),
            (
                "coding_and_type_health",
                &["coding_conventions", "strict_type_debt"],
            ),
        ],
        "operability_release_safety" => &[
            (
                "failure_and_recovery_contract",
                &["failure_modes", "deployment_rollback"],
            ),
            ("diagnosability", &["diagnosability."]),
            (
                "operational_observability",
                &[
                    "observability",
                    "operability",
                    "long_running_task_observability",
                    "long_running_task_optimization",
                ],
            ),
        ],
        "user_facing_quality" => &[
            ("accessible_experience", &["accessibility"]),
            ("performance_evidence", &["performance"]),
            (
                "runtime_resource_efficiency",
                &["runtime_resource_efficiency"],
            ),
            ("user_journey_evidence", &["user_journey", "web_readiness"]),
        ],
        "human_agent_usability" => &[
            (
                "documentation_contract",
                &["agent_usability.documentation_contract"],
            ),
            (
                "tool_skill_coverage",
                &["agent_usability.tool_skill_coverage"],
            ),
            ("behavior_evidence", &["agent_usability.behavior_evidence"]),
            (
                "routing_and_examples",
                &["context_routing", "implementation_examples"],
            ),
        ],
        "governance_sustainability" => &[
            ("ownership_and_governance", &["ownership_", "governance"]),
            ("maintenance_continuity", &["maintained", "contributor_"]),
            ("license_and_contribution", &["license"]),
            ("cache_lifecycle", &["cache_design"]),
            (
                "completion_and_matrix_discipline",
                &["definition_of_done", "matrix_maintenance"],
            ),
        ],
        _ => &[],
    }
}

fn is_critical_maturity_pillar(pillar: &str) -> bool {
    matches!(
        pillar,
        "correctness_reliability" | "security_privacy_supply_chain" | "operability_release_safety"
    )
}

fn maturity_pillar_capability_coverage(pillar: &RepositoryMaturityPillar) -> f64 {
    let capability_count = maturity_pillar_capabilities(&pillar.id)
        .len()
        .saturating_sub(pillar.not_applicable_capabilities.len());
    if capability_count == 0 {
        return 1.0;
    }
    (capability_count.saturating_sub(pillar.missing_capabilities.len())) as f64
        / capability_count as f64
}

fn round_quality_score(score: f64) -> f64 {
    (score * 1000.0).round() / 1000.0
}

fn round_ratio(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn merge_composite_dimension(
    maturity: &mut QualityMaturity,
    dimension: &str,
    score: Option<f64>,
    label: &str,
) {
    let Some(score) = score.filter(|score| score.is_finite() && (0.0..=4.0).contains(score)) else {
        return;
    };
    maturity
        .dimension_scores
        .insert(dimension.to_string(), score);
    if score < 4.0 {
        maturity.gaps.push(QualityMaturityGap {
            dimension: dimension.to_string(),
            status: if score == 0.0 { "blocked" } else { "attention" }.to_string(),
            score: Some(score),
            message: format!("{label} is {score:.2}/4.00 and remains below the fleet ideal."),
        });
    }
}

fn mac_control_implementation_score(state: &MacControlRepositoryState) -> f64 {
    if state.implementation_status == "Blocked" || state.implementation_criteria_total == 0 {
        return 0.0;
    }
    let score = (state.implementation_criteria_passed_count as f64
        / state.implementation_criteria_total as f64)
        * 4.0;
    cap_stale_maturity_score(score, &state.freshness)
}

fn mac_control_live_score(state: &MacControlRepositoryState) -> f64 {
    if state.live_status == "Blocked" || state.live_task_count == 0 {
        return 0.0;
    }
    let score = (state.measured_route_count as f64 / state.live_task_count as f64) * 4.0;
    cap_stale_maturity_score(score, &state.freshness)
}

fn cap_stale_maturity_score(score: f64, freshness: &str) -> f64 {
    let score = if freshness == "Fresh" {
        score
    } else {
        score.min(3.0)
    };
    (score * 100.0).round() / 100.0
}

fn average_quality_scores(scores: &[f64]) -> Option<f64> {
    (!scores.is_empty()).then(|| {
        let score = scores.iter().sum::<f64>() / scores.len() as f64;
        (score * 100.0).round() / 100.0
    })
}

fn format_quality_score(score: f64) -> String {
    if score.fract() == 0.0 {
        format!("{score:.1}")
    } else {
        format!("{score:.2}")
    }
}
