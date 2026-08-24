const REPOSITORY_MATURITY_PILLARS: [(&str, &str, f64, bool); 7] = [
    (
        "correctness_reliability",
        "Correctness and reliability",
        0.22,
        true,
    ),
    (
        "security_privacy_supply_chain",
        "Security, privacy, and supply chain",
        0.22,
        true,
    ),
    (
        "maintainability_evolvability",
        "Maintainability and evolvability",
        0.16,
        true,
    ),
    (
        "operability_release_safety",
        "Operability and release safety",
        0.14,
        true,
    ),
    ("user_facing_quality", "User-facing quality", 0.10, false),
    (
        "human_agent_usability",
        "Human and agent usability",
        0.10,
        false,
    ),
    (
        "governance_sustainability",
        "Governance and sustainability",
        0.06,
        false,
    ),
];

pub fn update_composite_maturity_summary(
    portfolio: &mut QualityPortfolioSnapshot,
    repositories: &mut [RepositorySnapshot],
) {
    portfolio.source_maturity_score = portfolio.maturity_score;
    portfolio.source_maturity_score_display = portfolio.maturity_score_display.clone();
    portfolio.source_scored_dimension_count = portfolio.scored_dimension_count;

    let mut implementation_scores = Vec::new();
    let mut live_scores = Vec::new();
    for repository in repositories.iter_mut() {
        let maturity = &mut repository.quality.maturity;
        for dimension in LEGACY_COMPOSITE_MATURITY_DIMENSIONS {
            maturity.dimension_scores.remove(dimension);
        }
        maturity
            .gaps
            .retain(|gap| !LEGACY_COMPOSITE_MATURITY_DIMENSIONS.contains(&gap.dimension.as_str()));

        merge_composite_dimension(
            maturity,
            "ci.fresh_passing",
            repository.quality.ci_readiness.score,
            "Fresh passing CI evidence",
        );
        let mac_control = &repository.quality.mac_control_ideal_state;
        if mac_control.applicability != "Not applicable" {
            let implementation_score = mac_control_implementation_score(mac_control);
            let live_score = mac_control_live_score(mac_control);
            merge_composite_dimension(
                maturity,
                "mac_control.task_usability",
                average_quality_scores(&[implementation_score, live_score]),
                "Mac Control implementation and live task usability",
            );
            implementation_scores.push(implementation_score);
            live_scores.push(live_score);
        }

        merge_composite_dimension(
            maturity,
            "web_readiness.user_journey",
            web_readiness_maturity_score(&repository.quality.web_readiness),
            "User-facing route readiness",
        );

        let model = build_repository_maturity_model(maturity);
        maturity.score = model.score;
        maturity.score_display = model.score.map(|score| format!("{score:.3}"));
        maturity.scored_dimension_count = Some(model.evidence.assessed_pillar_count);
        maturity.repository_maturity = Some(model);
        maturity
            .gaps
            .sort_by(|left, right| left.dimension.cmp(&right.dimension));
        repository.quality.foundation_readiness = derive_foundation_readiness(&repository.quality);
    }

    portfolio.mac_control_ideal_state.implementation_score =
        average_quality_scores(&implementation_scores);
    portfolio
        .mac_control_ideal_state
        .implementation_score_display = portfolio
        .mac_control_ideal_state
        .implementation_score
        .map(format_quality_score);
    portfolio.mac_control_ideal_state.live_score = average_quality_scores(&live_scores);
    portfolio.mac_control_ideal_state.live_score_display = portfolio
        .mac_control_ideal_state
        .live_score
        .map(format_quality_score);

    let repository_scores = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .quality
                .maturity
                .repository_maturity
                .as_ref()
                .and_then(|model| model.score)
        })
        .collect::<Vec<_>>();
    portfolio.maturity_score = average_quality_scores(&repository_scores);
    portfolio.maturity_score_display = portfolio.maturity_score.map(|score| format!("{score:.3}"));
    portfolio.scored_dimension_count = Some(
        repositories
            .iter()
            .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
            .map(|model| model.evidence.assessed_pillar_count)
            .sum(),
    );
    portfolio.maturity_pillars = REPOSITORY_MATURITY_PILLARS
        .iter()
        .map(|(id, label, _, _)| {
            let scores = repositories
                .iter()
                .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
                .flat_map(|model| model.pillars.iter())
                .filter(|pillar| pillar.id == *id)
                .filter_map(|pillar| pillar.score)
                .collect::<Vec<_>>();
            PortfolioMaturityPillar {
                id: (*id).to_string(),
                label: (*label).to_string(),
                score: average_quality_scores(&scores),
                assessed_repository_count: scores.len(),
            }
        })
        .collect();
    let evidence_coverages = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .map(|model| model.evidence.evidence_coverage * 4.0)
        .collect::<Vec<_>>();
    portfolio.maturity_evidence_coverage =
        average_quality_scores(&evidence_coverages).map(|score| score / 4.0);
    let fresh_coverages = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .map(|model| model.evidence.fresh_evidence_coverage * 4.0)
        .collect::<Vec<_>>();
    portfolio.maturity_fresh_evidence_coverage =
        average_quality_scores(&fresh_coverages).map(|score| score / 4.0);
    portfolio.maturity_provisional_repository_count = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .filter(|model| model.status == "provisional")
        .count();
    portfolio.maturity_capped_repository_count = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .filter(|model| model.critical_cap.applied)
        .count();
}

fn web_readiness_maturity_score(readiness: &WebReadinessSnapshot) -> Option<f64> {
    if readiness.applicability == "not_applicable" {
        return None;
    }
    let total = readiness.passed_count
        + readiness.failed_count
        + readiness.blocked_count
        + readiness.unknown_count;
    (total > 0)
        .then(|| ((readiness.passed_count as f64 / total as f64) * 4.0 * 1000.0).round() / 1000.0)
}

fn build_repository_maturity_model(maturity: &QualityMaturity) -> RepositoryMaturityModel {
    let prior_critical_dimensions = maturity
        .repository_maturity
        .as_ref()
        .map(|model| {
            model
                .pillars
                .iter()
                .flat_map(|pillar| pillar.critical_dimensions.iter().cloned())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let statuses = maturity
        .gaps
        .iter()
        .map(|gap| (gap.dimension.as_str(), gap.status.as_str()))
        .collect::<BTreeMap<_, _>>();
    let agent_applicability = maturity
        .agent_usability
        .as_ref()
        .map(|assessment| assessment.applicability.as_str())
        .unwrap_or("unknown");
    let mut assigned = BTreeSet::new();
    let mut critical_reasons = Vec::new();
    let mut pillars = Vec::new();

    for (id, label, weight, required) in REPOSITORY_MATURITY_PILLARS {
        let dimension_scores = maturity
            .dimension_scores
            .iter()
            .filter(|(dimension, _)| maturity_dimension_pillar(dimension) == Some(id))
            .map(|(dimension, score)| {
                assigned.insert(dimension.clone());
                (dimension.clone(), *score)
            })
            .collect::<BTreeMap<_, _>>();
        let applicability = if required {
            "applicable"
        } else if id == "human_agent_usability" && agent_applicability == "not_applicable" {
            "not_applicable"
        } else if !dimension_scores.is_empty()
            || (id == "human_agent_usability" && agent_applicability == "applicable")
        {
            "applicable"
        } else {
            "unknown"
        };
        let score = (applicability == "applicable")
            .then(|| {
                average_quality_scores(&dimension_scores.values().copied().collect::<Vec<_>>())
            })
            .flatten();
        let critical_dimensions = dimension_scores
            .keys()
            .filter(|dimension| {
                is_critical_maturity_pillar(id)
                    && (prior_critical_dimensions.contains(*dimension)
                        || statuses.get(dimension.as_str()) == Some(&"blocked"))
            })
            .cloned()
            .collect::<Vec<_>>();
        let not_applicable_capabilities = maturity_pillar_capabilities(id)
            .iter()
            .filter(|(_, patterns)| {
                maturity.gaps.iter().any(|gap| {
                    gap.status == "not_applicable"
                        && maturity_dimension_matches(&gap.dimension, patterns)
                })
            })
            .map(|(capability, _)| (*capability).to_string())
            .collect::<Vec<_>>();
        critical_reasons.extend(
            critical_dimensions
                .iter()
                .map(|dimension| format!("{id}:{dimension}")),
        );
        let missing_capabilities = maturity_pillar_capabilities(id)
            .iter()
            .filter(|(_, patterns)| {
                !dimension_scores
                    .keys()
                    .any(|dimension| maturity_dimension_matches(dimension, patterns))
            })
            .filter(|(capability, _)| {
                !not_applicable_capabilities
                    .iter()
                    .any(|excluded| excluded.as_str() == *capability)
            })
            .map(|(capability, _)| (*capability).to_string())
            .collect::<Vec<_>>();
        let pillar_status = if applicability == "not_applicable" {
            "not_applicable".to_string()
        } else if applicability == "unknown" || score.is_none() {
            "unknown".to_string()
        } else if !critical_dimensions.is_empty() {
            "blocked".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"blocked"))
        {
            "blocked".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"stale"))
        {
            "stale".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"unknown"))
        {
            "unknown".to_string()
        } else if score == Some(4.0) {
            "maintained".to_string()
        } else {
            "attention".to_string()
        };
        pillars.push(RepositoryMaturityPillar {
            id: id.to_string(),
            label: label.to_string(),
            weight,
            applicability: applicability.to_string(),
            status: pillar_status,
            score,
            dimension_scores,
            missing_capabilities,
            not_applicable_capabilities,
            critical_dimensions,
        });
    }

    let applicable = pillars
        .iter()
        .filter(|pillar| pillar.applicability == "applicable")
        .collect::<Vec<_>>();
    let assessed = applicable
        .iter()
        .copied()
        .filter(|pillar| pillar.score.is_some())
        .collect::<Vec<_>>();
    let applicable_weight = applicable.iter().map(|pillar| pillar.weight).sum::<f64>();
    let scored_weight = assessed.iter().map(|pillar| pillar.weight).sum::<f64>();
    let assessed_weight = applicable
        .iter()
        .map(|pillar| pillar.weight * maturity_pillar_capability_coverage(pillar))
        .sum::<f64>();
    let uncapped_score = (scored_weight > 0.0).then(|| {
        round_quality_score(
            assessed
                .iter()
                .map(|pillar| pillar.score.unwrap_or_default() * pillar.weight)
                .sum::<f64>()
                / scored_weight,
        )
    });
    let maximum_score = (!critical_reasons.is_empty()).then_some(2.0);
    let score = uncapped_score.map(|score| maximum_score.map_or(score, |cap| score.min(cap)));
    let unknown_applicability = pillars
        .iter()
        .filter(|pillar| pillar.applicability == "unknown")
        .map(|pillar| pillar.id.clone())
        .collect::<Vec<_>>();
    let evidence_coverage = if applicable_weight > 0.0 {
        round_ratio(assessed_weight / applicable_weight)
    } else {
        0.0
    };
    let fresh_weight = assessed
        .iter()
        .filter(|pillar| !matches!(pillar.status.as_str(), "blocked" | "stale" | "unknown"))
        .map(|pillar| pillar.weight * maturity_pillar_capability_coverage(pillar))
        .sum::<f64>();
    let fresh_evidence_coverage = if applicable_weight > 0.0 {
        round_ratio(fresh_weight / applicable_weight)
    } else {
        0.0
    };
    let certified = score == Some(4.0)
        && evidence_coverage == 1.0
        && critical_reasons.is_empty()
        && unknown_applicability.is_empty()
        && applicable
            .iter()
            .all(|pillar| pillar.status == "maintained");
    let status = if score.is_none() {
        "unknown"
    } else if !critical_reasons.is_empty() {
        "blocked"
    } else if certified {
        "certified"
    } else if evidence_coverage < 1.0 || !unknown_applicability.is_empty() {
        "provisional"
    } else {
        "measured"
    };
    let applicable_pillar_count = applicable.len() as u64;
    let assessed_pillar_count = assessed.len() as u64;
    let assessed_dimension_count = maturity.dimension_scores.len() as u64;
    let applicable_dimensions = maturity
        .dimension_scores
        .keys()
        .cloned()
        .chain(
            maturity
                .gaps
                .iter()
                .filter(|gap| gap.status != "not_applicable")
                .map(|gap| gap.dimension.clone()),
        )
        .collect::<BTreeSet<_>>();
    let applicable_dimension_count = applicable_dimensions.len() as u64;

    critical_reasons.sort();
    RepositoryMaturityModel {
        schema: "quality-runner-repository-maturity/v2".to_string(),
        score,
        uncapped_score,
        status: status.to_string(),
        pillars,
        evidence: RepositoryMaturityEvidence {
            applicable_pillar_count,
            assessed_pillar_count,
            applicable_dimension_count,
            assessed_dimension_count,
            applicable_weight: round_ratio(applicable_weight),
            assessed_weight: round_ratio(assessed_weight),
            evidence_coverage,
            fresh_evidence_coverage,
            unknown_applicability,
            unmapped_dimensions: maturity
                .dimension_scores
                .keys()
                .filter(|dimension| !assigned.contains(*dimension))
                .cloned()
                .collect(),
        },
        critical_cap: RepositoryMaturityCriticalCap {
            applied: maximum_score.is_some(),
            maximum_score,
            reasons: critical_reasons,
        },
    }
}
