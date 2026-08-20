#[test]
    fn reconciles_current_finding_dispositions_without_hiding_raw_detector_totals() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"summary":{"findings_by_category":{"debloat":3,"simplify":1}},"findings":[{"fingerprint":"fp-1","category":"debloat"},{"fingerprint":"fp-1","category":"debloat"},{"fingerprint":"fp-2","category":"debloat"},{"fingerprint":"fp-3","category":"simplify"}]}"#,
        )
        .expect("finding report should be writable");
        let contract_path = root.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
        fs::create_dir_all(
            contract_path
                .parent()
                .expect("contract should have a parent"),
        )
        .expect("contract directory should be writable");
        fs::write(
            &contract_path,
            format!(
                r#"{{
  "schema_version": "{FINDING_DISPOSITIONS_SCHEMA}",
  "updated_at": "2026-07-29T18:00:00Z",
  "dispositions": [
    {{
      "fingerprint": "fp-1",
      "status": "false_positive",
      "reason": "The symbol has a verified caller.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z",
      "evidence": ["src/example.ts:10"]
    }},
    {{
      "fingerprint": "fp-2",
      "status": "confirmed",
      "reason": "The finding reproduces.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z"
    }},
    {{
      "fingerprint": "no-longer-detected",
      "status": "fixed",
      "reason": "The finding disappeared after the fix.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z"
    }}
  ]
}}"#
            ),
        )
        .expect("disposition contract should be writable");
        let mut findings = QualityFindings {
            total: 4,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(findings.total, 4);
        assert_eq!(findings.reviewed_total, 3);
        assert_eq!(findings.unreviewed_total, 1);
        assert_eq!(findings.actionable_total, 2);
        assert_eq!(findings.category_counts.get("debloat"), Some(&3));
        assert_eq!(findings.actionable_category_counts.get("debloat"), Some(&1));
        assert_eq!(
            findings.actionable_category_counts.get("simplify"),
            Some(&1)
        );
        assert_eq!(findings.disposition_counts.get("false_positive"), Some(&2));
        assert_eq!(findings.disposition_counts.get("confirmed"), Some(&1));
        assert_eq!(findings.stale_disposition_total, 1);
        assert_eq!(findings.disposition_status, "Ready");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn preserves_open_finding_categories_and_reconciles_accepted_risk() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"summary":{"findings_by_category":{"maintenance-surface":0,"speed":1,"skill:future-review":1,"future-category":1}},"findings":[{"fingerprint":"surface-1","category":"maintenance-surface"},{"fingerprint":"speed-1","category":"speed"},{"fingerprint":"skill-1","category":"skill:future-review"},{"fingerprint":"future-1","category":"future-category"}]}"#,
        )
        .expect("finding report should be writable");
        let contract_path = root.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
        fs::create_dir_all(
            contract_path
                .parent()
                .expect("contract should have a parent"),
        )
        .expect("contract directory should be writable");
        fs::write(
            &contract_path,
            format!(
                r#"{{
  "schema_version": "{FINDING_DISPOSITIONS_SCHEMA}",
  "updated_at": "2026-08-11T13:00:00Z",
  "dispositions": [
    {{
      "fingerprint": "surface-1",
      "status": "accepted_risk",
      "reason": "The maintenance burden is documented and explicitly accepted.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-08-11T13:00:00Z"
    }}
  ]
}}"#
            ),
        )
        .expect("disposition contract should be writable");
        let mut findings = QualityFindings {
            total: 4,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(
            findings.category_counts.get("maintenance-surface"),
            Some(&1)
        );
        assert_eq!(findings.category_counts.get("speed"), Some(&1));
        assert_eq!(
            findings.category_counts.get("skill:future-review"),
            Some(&1)
        );
        assert_eq!(findings.category_counts.get("future-category"), Some(&1));
        assert_eq!(
            findings
                .actionable_category_counts
                .get("maintenance-surface"),
            Some(&0)
        );
        assert_eq!(findings.actionable_total, 3);
        assert_eq!(findings.disposition_counts.get("accepted_risk"), Some(&1));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn debloat_review_gate_requires_explicit_fresh_category_evidence() {
        let mut findings = QualityFindings::default();
        assert!(debloat_maturity_evidence(&findings).is_none());

        findings.category_counts.insert("debloat".to_string(), 0);
        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 0);
        findings.freshness = QualityFreshness::Fresh;
        let clear = debloat_maturity_evidence(&findings)
            .expect("declared debloat coverage should create maturity evidence");
        assert_eq!(clear.status, QualityGateStatus::Passed);
        assert_eq!(clear.freshness, QualityFreshness::Fresh);

        findings.category_counts.insert("debloat".to_string(), 1);
        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 1);
        let unresolved = debloat_maturity_evidence(&findings)
            .expect("an unresolved candidate should create maturity evidence");
        assert_eq!(unresolved.status, QualityGateStatus::Blocked);
        assert!(unresolved.detail.contains("ownership-pressure audit"));

        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 0);
        findings.freshness = QualityFreshness::Stale;
        let stale =
            debloat_maturity_evidence(&findings).expect("stale coverage should remain visible");
        assert_eq!(stale.status, QualityGateStatus::Blocked);
        assert_eq!(stale.freshness, QualityFreshness::Stale);
    }

#[test]
    fn missing_disposition_contract_keeps_every_finding_actionable() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"findings":[{"fingerprint":"fp-1"},{"fingerprint":"fp-2"}]}"#,
        )
        .expect("finding report should be writable");
        let mut findings = QualityFindings {
            total: 2,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(findings.actionable_total, 2);
        assert_eq!(findings.unreviewed_total, 2);
        assert_eq!(findings.reviewed_total, 0);
        assert_eq!(findings.disposition_status, "Missing");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn writes_auditable_finding_dispositions_and_replaces_prior_review() {
        let root = fixture_root();
        let first = set_finding_disposition(
            &root,
            "fp-1",
            "confirmed",
            "The finding reproduces.",
            "test-reviewer",
            vec!["test:quality".to_string()],
            None,
        )
        .expect("first disposition should be written");
        assert_eq!(first.dispositions.len(), 1);
        assert_eq!(first.dispositions[0].status, "confirmed");

        let replaced = set_finding_disposition(
            &root,
            "fp-1",
            "false-positive",
            "A caller was verified.",
            "second-reviewer",
            vec!["src/example.ts:10".to_string()],
            Some("2099-01-01T00:00:00Z".to_string()),
        )
        .expect("replacement disposition should be written");

        assert_eq!(replaced.dispositions.len(), 1);
        assert_eq!(replaced.dispositions[0].status, "false_positive");
        assert_eq!(replaced.dispositions[0].reviewer, "second-reviewer");
        assert_eq!(replaced.dispositions[0].evidence, vec!["src/example.ts:10"]);
        assert!(root.join(FINDING_DISPOSITIONS_RELATIVE_PATH).is_file());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn imports_canonical_maturity_feed_by_stable_repository_id() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let as_of = Utc::now().to_rfc3339();
        fs::write(
            &feed_path,
            serde_json::to_string(&fixture_maturity_feed(&repository, &as_of))
                .expect("feed should serialize"),
        )
        .expect("feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(imported.portfolio.audit_status, "Ready");
        assert_eq!(
            imported.portfolio.latest_audit_id.as_deref(),
            Some("audit-fixture")
        );
        let expected_provenance = imported
            .portfolio
            .provenance_hash
            .clone()
            .expect("feed should expose provenance");
        assert_eq!(
            imported.portfolio.provenance_hash.as_deref(),
            Some(expected_provenance.as_str())
        );
        assert_eq!(imported.portfolio.matched_repository_count, 1);
        let confidence = imported
            .portfolio
            .measurement_confidence
            .as_ref()
            .expect("feed should expose measurement confidence");
        assert_eq!(confidence.level, "high");
        assert_eq!(confidence.expected_repository_count, 1);
        assert_eq!(confidence.observed_repository_count, 1);
        assert_eq!(confidence.excluded_repository_count, 2);
        assert_eq!(confidence.unresolved_measurement_gap_count, 0);
        assert_eq!(imported.portfolio.behavior_assurance.status, "gaps_present");
        assert_eq!(
            imported.portfolio.behavior_assurance.ready_repository_count,
            0
        );
        assert_eq!(
            imported.portfolio.quality_outcome_counts.get("healthy"),
            Some(&1)
        );
        assert_eq!(
            imported.portfolio.quality_outcome_taxonomy["healthy"].label,
            "Quality healthy"
        );
        assert_eq!(
            imported.maturities[&repository.id]
                .quality_outcome
                .as_ref()
                .and_then(|outcome| outcome.disposition.as_deref()),
            Some("Applicable dimensions are maintained with current evidence.")
        );
        assert_eq!(imported.maturities[&repository.id].score, Some(3.5));
        assert!(imported.maturities[&repository.id].cache_design.is_none());
        assert_eq!(
            imported.maturities[&repository.id]
                .dimension_scores
                .get("agent_usability.growth_health"),
            Some(&4.0)
        );
        assert_eq!(imported.maturities[&repository.id].gaps.len(), 1);
        assert_eq!(
            imported.maturities[&repository.id].gaps[0].dimension,
            "change_surface_coverage"
        );
        let agent_usability = imported.maturities[&repository.id]
            .agent_usability
            .as_ref()
            .expect("feed should preserve agent-usability evidence");
        assert_eq!(agent_usability.covered_lane_count, 3);
        assert_eq!(agent_usability.lanes[0].id, "documentation_contract");
        assert_eq!(agent_usability.growth_health.skill_count, 4);
        assert_eq!(agent_usability.growth_health.family_count, 2);
        let behavior_assurance = &imported.behavior_assurance[&repository.id];
        assert_eq!(behavior_assurance.contract_status, "current");
        assert_eq!(behavior_assurance.state, "legacy_v1");
        assert_eq!(behavior_assurance.result_status, "unknown");
        assert!(!behavior_assurance.release_ready);
        assert_eq!(behavior_assurance.gaps[0].kind, "receipt_stale");
        assert_eq!(
            imported.maturities[&repository.id].freshness,
            QualityFreshness::Fresh
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn imports_legacy_v1_maturity_feed() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let mut feed = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        feed["schema"] = Value::String(MATURITY_FEED_SCHEMAS[0].to_string());
        feed.as_object_mut()
            .expect("fixture feed should be an object")
            .remove("measurement_confidence");
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&feed).expect("feed should serialize"),
        )
        .expect("feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), &[repository]);
        assert_eq!(imported.portfolio.audit_status, "Ready");
        assert_eq!(
            imported.portfolio.feed_schema.as_deref(),
            Some(MATURITY_FEED_SCHEMAS[0])
        );
        assert!(imported.portfolio.measurement_confidence.is_none());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn imports_holistic_v2_maturity_projection() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity-v2.json");
        let as_of = Utc::now().to_rfc3339();
        let mut feed = fixture_maturity_feed(&repository, &as_of);
        feed["schema"] = Value::String("quality-runner-maturity-feed/v2".to_string());
        feed["repositories"][0]["repository_maturity"] = serde_json::json!({
            "schema": "quality-runner-repository-maturity/v2",
            "score": 3.5,
            "uncapped_score": 3.5,
            "status": "provisional",
            "pillars": [
                {"id": "correctness_reliability", "label": "Correctness and reliability", "weight": 0.22, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "security_privacy_supply_chain", "label": "Security, privacy, and supply chain", "weight": 0.22, "applicability": "applicable", "status": "unknown", "score": null},
                {"id": "maintainability_evolvability", "label": "Maintainability and evolvability", "weight": 0.16, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "operability_release_safety", "label": "Operability and release safety", "weight": 0.14, "applicability": "applicable", "status": "unknown", "score": null},
                {"id": "user_facing_quality", "label": "User-facing quality", "weight": 0.10, "applicability": "unknown", "status": "unknown", "score": null},
                {"id": "human_agent_usability", "label": "Human and agent usability", "weight": 0.10, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "governance_sustainability", "label": "Governance and sustainability", "weight": 0.06, "applicability": "unknown", "status": "unknown", "score": null}
            ],
            "evidence": {
                "applicable_pillar_count": 5,
                "assessed_pillar_count": 3,
                "applicable_weight": 0.84,
                "assessed_weight": 0.48,
                "evidence_coverage": 0.571,
                "fresh_evidence_coverage": 0.571,
                "unknown_applicability": ["user_facing_quality", "governance_sustainability"],
                "unmapped_dimensions": []
            },
            "critical_cap": {"applied": false, "maximum_score": null, "reasons": []}
        });
        feed["repositories"][0]["cache_design"] = serde_json::json!({
            "schema": "quality-runner-cache-design-assessment-v1",
            "status": "maintained",
            "score": 4,
            "measurement_complete": true,
            "totals": {
                "logical_bytes": 2097152,
                "allocated_bytes": 1048576,
                "exclusive_allocated_bytes": 1048576,
                "shared_allocated_bytes": 0,
                "file_count": 12,
                "shared_file_count": 0
            },
            "categories": {
                "tool_cache": {"allocated_bytes": 1048576, "file_count": 12}
            },
            "risk_flags": [],
            "growth": {"snapshot_count": 2, "within_policy": true}
        });
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("v2 fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&feed).expect("v2 feed should serialize"),
        )
        .expect("v2 feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));

        assert_eq!(imported.portfolio.audit_status, "Ready");
        let agent_usability = imported.maturities[&repository.id]
            .agent_usability
            .as_ref()
            .expect("feed should preserve agent-usability evidence");
        assert_eq!(agent_usability.covered_lane_count, 3);
        assert_eq!(agent_usability.lanes[0].id, "documentation_contract");
        assert_eq!(agent_usability.growth_health.skill_count, 4);
        assert_eq!(agent_usability.growth_health.family_count, 2);
        let model = imported.maturities[&repository.id]
            .repository_maturity
            .as_ref()
            .expect("v2 feed should preserve the repository model");
        assert_eq!(model.status, "provisional");
        assert_eq!(model.pillars.len(), 7);
        assert_eq!(model.evidence.evidence_coverage, 0.571);
        let cache_design = imported.maturities[&repository.id]
            .cache_design
            .as_ref()
            .expect("v2 feed should preserve cache-design evidence");
        assert_eq!(cache_design.status, "maintained");
        assert_eq!(cache_design.score, Some(4));
        assert_eq!(cache_design.totals.allocated_bytes, 1_048_576);
        assert_eq!(cache_design.categories["tool_cache"].file_count, 12);
        assert_eq!(cache_design.risk_flags, Vec::<String>::new());
        let ci_gate_audit = imported.maturities[&repository.id]
            .ci_gate_audit
            .as_ref()
            .expect("feed should preserve custom-gate recommendations");
        assert_eq!(ci_gate_audit.status, "complete");
        assert_eq!(ci_gate_audit.candidate_count, 1);
        assert_eq!(
            ci_gate_audit.candidates[0].id,
            "custom:migration_compatibility"
        );
        assert_eq!(
            imported.maturities[&repository.id].freshness,
            QualityFreshness::Fresh
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
