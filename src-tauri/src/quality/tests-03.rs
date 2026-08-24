#[test]
    fn rejects_custom_gate_audit_with_mismatched_target_provenance() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let mut feed = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        feed["repositories"][0]["ci_gate_audit"]["repository"]["head_sha"] =
            Value::String("different-head".to_string());
        let mut audit = feed["repositories"][0]["ci_gate_audit"].clone();
        audit["provenance_hash"] =
            Value::String(maturity_feed_hash(&audit).expect("candidate audit should rehash"));
        feed["repositories"][0]["ci_gate_audit"] = audit;
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("feed should rehash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&feed).expect("feed should serialize"),
        )
        .expect("feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        let audit = imported.maturities[&repository.id]
            .ci_gate_audit
            .as_ref()
            .expect("invalid audit should remain visible");
        assert_eq!(audit.status, "invalid");
        assert_eq!(audit.candidate_count, 0);
        assert!(audit
            .error
            .as_deref()
            .is_some_and(|error| error.contains("target branch and commit")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn rejects_invalid_and_stale_maturity_feeds() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let stale_as_of = (Utc::now() - Duration::days(MAX_EVIDENCE_AGE_DAYS + 1)).to_rfc3339();
        fs::write(
            &feed_path,
            serde_json::to_string(&fixture_maturity_feed(&repository, &stale_as_of))
                .expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let stale = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(stale.portfolio.audit_status, "Stale");
        assert_eq!(
            stale.maturities[&repository.id].freshness,
            QualityFreshness::Stale
        );

        let mut invalid_feed = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        invalid_feed["replay"]["status"] = Value::String("failed".to_string());
        fs::write(
            &feed_path,
            serde_json::to_string(&invalid_feed).expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let invalid = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(invalid.portfolio.audit_status, "Unavailable");
        assert!(invalid.maturities.is_empty());

        let mut contradictory = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        contradictory["measurement_confidence"]["limitations"] =
            serde_json::json!(["dynamic_verification_disabled"]);
        contradictory["provenance_hash"] =
            Value::String(maturity_feed_hash(&contradictory).expect("fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&contradictory).expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let contradictory =
            maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(contradictory.portfolio.audit_status, "Unavailable");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn normalizes_qr_repository_identity_inputs() {
        assert_eq!(
            normalized_origin("git@github.com:Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
        assert_eq!(
            normalized_origin("https://github.com/Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
        assert_eq!(
            normalized_origin("ssh://git@github.com/Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
    }

#[test]
    fn normalizes_qr_aliases_and_discovers_custom_gates() {
        assert_eq!(normalize_gate_id("runtime_smoke"), "runtime_smoke");
        assert_eq!(normalize_gate_id("smoke-test"), "runtime_smoke");
        assert_eq!(normalize_gate_id("dead_code"), "dead_code");
        assert_eq!(normalize_gate_id("unit_tests_vitest"), "tests");
        assert_eq!(
            normalize_gate_id("secret_scanning_gitleaks"),
            "secrets_scan"
        );
        assert_eq!(
            normalize_gate_id("security_dependency_audit"),
            "dependency_audit"
        );
        assert_eq!(normalize_gate_id("verify_and_build"), "build");
        assert_eq!(normalize_gate_id("security scan"), "custom:security_scan");
        assert_eq!(
            normalize_gate_id("custom:restore_drill"),
            "custom:restore_drill"
        );
        assert_eq!(
            normalize_declared_gate_id("custom:restore_drill"),
            Ok("custom:restore_drill".to_string())
        );
        assert!(normalize_declared_gate_id("restore_drill").is_err());
        assert!(normalize_declared_gate_id("custom:restore-drill").is_err());
        assert_eq!(
            default_quality_gates()
                .iter()
                .map(|gate| gate.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "build",
                "runtime_smoke",
                "tests",
                "lint",
                "formatter",
                "typecheck",
                "dead_code",
                "secrets_scan"
            ]
        );
    }

#[test]
    fn recommendation_matrix_provides_repository_specific_ideal_gates() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        repository.name = "BIP-Console".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(vec![
                "build".to_string(),
                "tests".to_string(),
                "runtime_smoke".to_string(),
                "lint".to_string(),
                "formatter".to_string(),
                "typecheck".to_string(),
                "dead_code".to_string(),
                "secrets_scan".to_string(),
                "dependency_audit".to_string(),
            ])
        );

        repository.name = "dotfiles".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(vec!["secrets_scan".to_string()])
        );

        repository.name = "Book-documents-github".to_string();
        assert_eq!(ideal_gate_ids_for_repository(&repository), None);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn repository_ci_gate_profile_classifies_standard_and_custom_gates() {
        let root = fixture_root();
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("profile directory should be writable");
        fs::write(
            contract_dir.join("ci-gate-profile.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": CI_GATE_PROFILE_SCHEMA,
                "reason": "This service must build and prove that production backups can be restored.",
                "gates": [
                    {"id": "build", "classification": "required", "reason": "The service ships a compiled artifact."},
                    {"id": "tests", "classification": "optional", "reason": "Tests inform changes but are not yet a merge blocker."},
                    {"id": "runtime_smoke", "classification": "not_applicable", "reason": "There is no independently runnable process."},
                    {"id": "lint", "classification": "not_applicable", "reason": "No supported linter is used."},
                    {"id": "formatter", "classification": "not_applicable", "reason": "Formatting is not enforced in CI."},
                    {"id": "typecheck", "classification": "not_applicable", "reason": "The compiler provides the relevant type checks through build."},
                    {"id": "dead_code", "classification": "not_applicable", "reason": "No supported dead-code analyzer is used."},
                    {"id": "secrets_scan", "classification": "not_applicable", "reason": "Secret scanning is enforced outside this repository."},
                    {"id": "dependency_audit", "classification": "not_applicable", "reason": "Dependency review is enforced outside this repository."},
                    {"id": "custom:restore_drill", "label": "Restore drill", "classification": "required", "reason": "A backup is useful only when a current restore succeeds."},
                    {"id": "custom:diagnostic_report", "label": "Diagnostic report", "classification": "optional", "reason": "The report helps operators but does not block a merge."}
                ]
            }))
            .expect("profile fixture should encode"),
        )
        .expect("profile fixture should be writable");
        let repository = fixture_repository(&root);

        let profile = ci_gate_profile_for_repository(&repository);

        assert_eq!(profile.source, "repository_contract");
        assert_eq!(
            profile.required_gate_ids,
            vec!["build".to_string(), "custom:restore_drill".to_string()]
        );
        assert!(profile.optional_gate_ids.contains(&"tests".to_string()));
        assert!(profile
            .optional_gate_ids
            .contains(&"custom:diagnostic_report".to_string()));
        assert!(profile
            .not_applicable_gate_ids
            .contains(&"runtime_smoke".to_string()));
        assert_eq!(
            profile
                .gate_labels
                .get("custom:restore_drill")
                .map(String::as_str),
            Some("Restore drill")
        );

        let snapshot = ingest_repository_quality(&repository, None, None, Some(&profile));
        assert_eq!(snapshot.ci_readiness.profile_source, "repository_contract");
        assert_eq!(snapshot.ci_readiness.applicable_gate_ids.len(), 2);
        assert_eq!(snapshot.ci_readiness.configuration_score, Some(0.0));
        assert_eq!(
            snapshot
                .gates
                .iter()
                .find(|gate| gate.id == "custom:restore_drill")
                .map(|gate| gate.label.as_str()),
            Some("Restore drill")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn invalid_repository_ci_gate_profile_is_unscored_with_a_descriptive_error() {
        let root = fixture_root();
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("profile directory should be writable");
        fs::write(
            contract_dir.join("ci-gate-profile.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": CI_GATE_PROFILE_SCHEMA,
                "reason": "Incomplete profile fixture.",
                "gates": [
                    {"id": "build", "classification": "required", "reason": "Build is required."}
                ]
            }))
            .expect("profile fixture should encode"),
        )
        .expect("profile fixture should be writable");
        let repository = fixture_repository(&root);

        let profile = ci_gate_profile_for_repository(&repository);
        let snapshot = ingest_repository_quality(&repository, None, None, Some(&profile));

        assert_eq!(profile.source, "invalid_repository_contract");
        assert!(profile
            .error
            .as_deref()
            .is_some_and(|message| message.contains("missing: tests")));
        assert_eq!(snapshot.ci_readiness.score, None);
        assert_eq!(
            snapshot.ci_readiness.profile_source,
            "invalid_repository_contract"
        );
        assert!(snapshot.ci_readiness.profile_error.is_some());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

fn readiness_gate(
        id: &str,
        status: QualityGateStatus,
        freshness: QualityFreshness,
        with_evidence: bool,
    ) -> QualityGate {
        QualityGate {
            id: id.to_string(),
            label: gate_label(id),
            status: status.clone(),
            freshness: freshness.clone(),
            evidence: with_evidence
                .then(|| QualityEvidence {
                    id: id.to_string(),
                    source: QualitySource::Ci,
                    status,
                    freshness,
                    observed_at: None,
                    scanned_commit: None,
                    scanned_branch: None,
                    command: None,
                    source_label: "fixture".to_string(),
                    report_path: None,
                    report_url: None,
                    report_kind: None,
                    detail: "fixture".to_string(),
                    verification_level: QualityVerificationLevel::SourceInferred,
                    target_kind: Some("source".to_string()),
                    target_url: None,
                    target_provider: None,
                    deployment_id: None,
                })
                .into_iter()
                .collect(),
        }
    }

#[test]
    fn ci_readiness_reaches_four_only_for_fresh_baseline_and_applicable_gates() {
        let baseline = [
            "build",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "secrets_scan",
        ]
        .into_iter()
        .map(|id| readiness_gate(id, QualityGateStatus::Passed, QualityFreshness::Fresh, true))
        .collect::<Vec<_>>();
        let complete = evaluate_ci_readiness(&baseline);
        assert_eq!(complete.score, Some(4.0));
        assert_eq!(complete.score_display.as_deref(), Some("4.0"));
        assert_eq!(complete.applicable_gate_ids.len(), 6);
        assert!(complete.missing_gate_ids.is_empty());

        let mut incomplete = baseline;
        let tests = incomplete
            .iter_mut()
            .find(|gate| gate.id == "tests")
            .unwrap();
        tests.status = QualityGateStatus::NotConfigured;
        tests.evidence.clear();
        let lint = incomplete
            .iter_mut()
            .find(|gate| gate.id == "lint")
            .unwrap();
        lint.freshness = QualityFreshness::Stale;
        lint.evidence[0].freshness = QualityFreshness::Stale;
        incomplete.push(readiness_gate(
            "dependency_audit",
            QualityGateStatus::Passed,
            QualityFreshness::Fresh,
            true,
        ));
        let result = evaluate_ci_readiness(&incomplete);
        assert_eq!(result.score, Some(2.86));
        assert_eq!(result.missing_gate_ids, vec!["tests"]);
        assert_eq!(result.stale_gate_ids, vec!["lint"]);
        assert!(result
            .applicable_gate_ids
            .contains(&"dependency_audit".to_string()));
    }

#[test]
    fn ci_maturity_scores_ideal_gate_coverage() {
        let ideal_gate_ids = [
            "build",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "secrets_scan",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let gates = ideal_gate_ids
            .iter()
            .enumerate()
            .map(|(index, gate_id)| {
                if index < 4 {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::Passed,
                        QualityFreshness::Fresh,
                        true,
                    )
                } else if index == 4 {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::Blocked,
                        QualityFreshness::Unknown,
                        true,
                    )
                } else {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::NotConfigured,
                        QualityFreshness::Unknown,
                        false,
                    )
                }
            })
            .collect::<Vec<_>>();

        let readiness = evaluate_ci_readiness_for_ideal(&gates, &ideal_gate_ids);
        assert_eq!(readiness.score, Some(2.67));
        assert_eq!(readiness.evidence_coverage_score, Some(3.33));
        assert_eq!(readiness.applicable_gate_ids, ideal_gate_ids);
        assert_eq!(
            readiness.covered_gate_ids,
            vec!["build", "tests", "lint", "formatter", "typecheck"]
        );
        assert_eq!(readiness.blocked_gate_ids, vec!["typecheck"]);
        assert_eq!(readiness.missing_gate_ids, vec!["secrets_scan"]);
    }

#[test]
    fn ci_configuration_separates_discovered_gates_from_imported_results() {
        let ideal_gate_ids = [
            "build",
            "tests",
            "runtime_smoke",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
            "secrets_scan",
            "dependency_audit",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let configured_gate_ids = [
            "build",
            "tests",
            "runtime_smoke",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let readiness = evaluate_ci_readiness_for_ideal_with_configuration(
            &default_quality_gates(),
            &ideal_gate_ids,
            &configured_gate_ids,
        );

        assert_eq!(
            readiness.configuration_score_display.as_deref(),
            Some("3.11")
        );
        assert_eq!(readiness.configured_gate_ids, configured_gate_ids);
        assert_eq!(
            readiness.unconfigured_gate_ids,
            vec!["secrets_scan", "dependency_audit"]
        );
        assert_eq!(readiness.score, Some(0.0));
        assert!(readiness.covered_gate_ids.is_empty());
        assert!(readiness.fresh_passing_gate_ids.is_empty());
    }
