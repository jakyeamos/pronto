use std::collections::BTreeMap;

use pronto_lib::quality::{
    derive_foundation_readiness, QualityFreshness, QualitySnapshot, QualitySource,
    RepositoryMaturityCriticalCap, RepositoryMaturityEvidence, RepositoryMaturityModel,
    RepositoryMaturityPillar,
};

fn measured_foundation(score: f64) -> QualitySnapshot {
    let mut quality = QualitySnapshot::default();
    quality.maturity.freshness = QualityFreshness::Fresh;
    quality.maturity.observed_at = Some("2026-08-17T12:00:00Z".to_string());
    quality.maturity.scanned_commit = Some("current-commit".to_string());
    quality.maturity.repository_maturity = Some(RepositoryMaturityModel {
        schema: "quality-runner-repository-maturity/v2".to_string(),
        score: Some(score),
        uncapped_score: Some(score),
        status: "measured".to_string(),
        pillars: vec![RepositoryMaturityPillar {
            id: "maintainability_change_safety".to_string(),
            label: "Maintainability and change safety".to_string(),
            weight: 0.16,
            applicability: "applicable".to_string(),
            status: if score == 4.0 {
                "maintained"
            } else {
                "attention"
            }
            .to_string(),
            score: Some(score),
            dimension_scores: BTreeMap::from([
                ("architecture_boundaries".to_string(), score),
                ("change_surface_coverage".to_string(), score),
                ("coding_conventions".to_string(), score),
            ]),
            missing_capabilities: Vec::new(),
            not_applicable_capabilities: Vec::new(),
            critical_dimensions: Vec::new(),
        }],
        evidence: RepositoryMaturityEvidence {
            applicable_pillar_count: 1,
            assessed_pillar_count: 1,
            applicable_dimension_count: 3,
            assessed_dimension_count: 3,
            applicable_weight: 0.16,
            assessed_weight: 0.16,
            evidence_coverage: 1.0,
            fresh_evidence_coverage: 1.0,
            unknown_applicability: Vec::new(),
            unmapped_dimensions: Vec::new(),
        },
        critical_cap: RepositoryMaturityCriticalCap::default(),
    });
    quality
}

#[test]
fn ready_to_extend_requires_current_complete_evidence() {
    let gate = derive_foundation_readiness(&measured_foundation(4.0));
    assert_eq!(gate.disposition, "ready_to_extend");
    assert_eq!(gate.confidence, "high");
    assert!(gate.advisory_only);
    assert!(!gate.execution_authority);
    assert!(!gate.blocks_urgent_fixes);
}

#[test]
fn actionable_structural_signals_modernize_alongside() {
    let mut quality = measured_foundation(4.0);
    quality.findings.source = Some(QualitySource::Qr);
    quality.findings.freshness = QualityFreshness::Fresh;
    quality
        .findings
        .category_counts
        .insert("debloat".to_string(), 3);
    quality
        .findings
        .actionable_category_counts
        .insert("debloat".to_string(), 2);
    let gate = derive_foundation_readiness(&quality);
    assert_eq!(gate.disposition, "modernize_alongside");
    assert!(gate
        .reasons
        .iter()
        .any(|reason| reason.id == "actionable_structural_signals"));
}

#[test]
fn accepted_intentional_signals_are_not_reopened() {
    let mut quality = measured_foundation(4.0);
    quality.findings.source = Some(QualitySource::Qr);
    quality.findings.freshness = QualityFreshness::Fresh;
    quality
        .findings
        .category_counts
        .insert("debloat".to_string(), 2);
    quality
        .findings
        .actionable_category_counts
        .insert("debloat".to_string(), 0);
    quality
        .findings
        .disposition_counts
        .insert("accepted_intentional".to_string(), 2);
    let gate = derive_foundation_readiness(&quality);
    assert_eq!(gate.disposition, "ready_to_extend");
    assert!(gate.reasons.is_empty());
}

#[test]
fn stale_or_incomplete_evidence_requires_review() {
    let mut stale = measured_foundation(4.0);
    stale.maturity.freshness = QualityFreshness::Stale;
    assert_eq!(
        derive_foundation_readiness(&stale).disposition,
        "review_required"
    );

    let mut incomplete = measured_foundation(4.0);
    incomplete
        .maturity
        .repository_maturity
        .as_mut()
        .expect("model should exist")
        .evidence
        .evidence_coverage = 0.5;
    assert_eq!(
        derive_foundation_readiness(&incomplete).disposition,
        "review_required"
    );
}

#[test]
fn missing_foundation_measurement_stays_unknown() {
    let gate = derive_foundation_readiness(&QualitySnapshot::default());
    assert_eq!(gate.disposition, "unknown");
    assert_eq!(gate.confidence, "low");
}

#[test]
fn current_critical_blockers_modernize_first_without_granting_authority() {
    let mut quality = measured_foundation(2.0);
    quality
        .maturity
        .repository_maturity
        .as_mut()
        .expect("model should exist")
        .critical_cap = RepositoryMaturityCriticalCap {
        applied: true,
        maximum_score: Some(2.0),
        reasons: vec!["correctness:behavior_regression".to_string()],
    };
    let gate = derive_foundation_readiness(&quality);
    assert_eq!(gate.disposition, "modernize_first");
    assert!(!gate.blocks_urgent_fixes);
    assert!(!gate.execution_authority);
}

#[test]
fn explicit_not_applicable_state_is_preserved() {
    let mut quality = measured_foundation(4.0);
    quality
        .maturity
        .repository_maturity
        .as_mut()
        .expect("model should exist")
        .evidence
        .applicable_pillar_count = 0;
    assert_eq!(
        derive_foundation_readiness(&quality).disposition,
        "not_applicable"
    );
}

#[test]
fn quality_json_remains_backward_compatible() {
    let mut value = serde_json::to_value(QualitySnapshot::default())
        .expect("quality snapshot should serialize");
    assert_eq!(
        value["foundation_readiness"]["schema"],
        "pronto-foundation-readiness/v1"
    );
    value
        .as_object_mut()
        .expect("snapshot should be an object")
        .remove("foundation_readiness");
    let restored: QualitySnapshot =
        serde_json::from_value(value).expect("legacy quality JSON should remain readable");
    assert_eq!(restored.foundation_readiness.disposition, "unknown");
    assert!(restored.foundation_readiness.advisory_only);
}
