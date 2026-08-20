#[test]
    fn ci_maturity_summary_excludes_unscored_repositories() {
        let root = fixture_root();
        let mut scored = fixture_repository(&root.join("scored"));
        scored.quality.ci_readiness.score = Some(2.0);
        scored.quality.ci_readiness.profile_source = "repository_contract".to_string();
        let mut unscored = fixture_repository(&root.join("unscored"));
        unscored.quality.ci_readiness.profile_source = "invalid_repository_contract".to_string();
        let repositories = vec![scored, unscored];
        let mut portfolio = QualityPortfolioSnapshot::default();

        update_ci_readiness_summary(&mut portfolio, &repositories);

        assert_eq!(portfolio.ci_readiness_score, Some(2.0));
        assert_eq!(portfolio.ci_readiness_repository_count, 1);
        assert_eq!(portfolio.ci_readiness_unscored_repository_count, 1);
        assert_eq!(portfolio.ci_profile_repository_contract_count, 1);
        assert_eq!(portfolio.ci_profile_invalid_count, 1);
        assert_eq!(portfolio.ci_profile_compatibility_count, 0);
        assert_eq!(portfolio.ci_profile_unavailable_count, 0);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn conflicting_evidence_is_blocked() {
        let items = vec![
            evidence(
                QualitySource::Ci,
                QualityGateStatus::Passed,
                QualityFreshness::Fresh,
            ),
            evidence(
                QualitySource::Local,
                QualityGateStatus::Failed,
                QualityFreshness::Fresh,
            ),
        ];
        assert_eq!(
            aggregate_gate_status(&items),
            (QualityGateStatus::Blocked, QualityFreshness::Conflicted)
        );
    }

#[test]
    fn freshness_requires_current_commit_and_seven_day_window() {
        let now = DateTime::parse_from_rfc3339("2026-07-26T12:00:00Z")
            .expect("now should parse")
            .with_timezone(&Utc);
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("abc"),
                Some("main"),
                Some("abc"),
                Some("main"),
                now,
            ),
            QualityFreshness::Fresh
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-10T12:00:00Z"),
                Some("abc"),
                Some("main"),
                Some("abc"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("old"),
                Some("main"),
                Some("new"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("old"),
                Some("feature"),
                Some("new"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                None,
                Some("main"),
                None,
                Some("main"),
                now,
            ),
            QualityFreshness::Unknown
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                None,
                Some("feature"),
                None,
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
    }

#[test]
    fn target_freshness_is_commit_based_instead_of_time_based() {
        assert_eq!(
            evaluate_target_freshness(Some("dev"), Some("target-commit"), "dev", "target-commit"),
            QualityFreshness::Fresh
        );
        assert_eq!(
            evaluate_target_freshness(Some("dev"), Some("old-commit"), "dev", "target-commit"),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_target_freshness(Some("dev"), None, "dev", "target-commit"),
            QualityFreshness::Unknown
        );
        assert_eq!(
            evaluate_target_freshness(None, None, "dev", "target-commit"),
            QualityFreshness::Unknown
        );
    }

#[test]
    fn target_projection_keeps_exact_branch_failure_and_drops_mismatched_evidence() {
        let observed_at = (Utc::now() - Duration::days(MAX_EVIDENCE_AGE_DAYS + 1)).to_rfc3339();
        let mut snapshot = QualitySnapshot::default();
        snapshot.ci_readiness.applicable_gate_ids = vec!["build".to_string(), "tests".to_string()];
        let build = snapshot
            .gates
            .iter_mut()
            .find(|gate| gate.id == "build")
            .expect("build gate should exist");
        build.evidence.push(QualityEvidence {
            id: "build".to_string(),
            source: QualitySource::Qr,
            status: QualityGateStatus::Failed,
            freshness: QualityFreshness::Stale,
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            command: Some("pnpm build".to_string()),
            source_label: "target QR".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: "fixture failure".to_string(),
            verification_level: QualityVerificationLevel::SourceInferred,
            target_kind: Some("source".to_string()),
            target_url: None,
            target_provider: None,
            deployment_id: None,
        });
        let lint = snapshot
            .gates
            .iter_mut()
            .find(|gate| gate.id == "lint")
            .expect("lint gate should exist");
        lint.evidence.push(QualityEvidence {
            id: "lint".to_string(),
            source: QualitySource::Qr,
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("old-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            command: Some("pnpm lint".to_string()),
            source_label: "old QR".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: "mismatched fixture".to_string(),
            verification_level: QualityVerificationLevel::SourceInferred,
            target_kind: Some("source".to_string()),
            target_url: None,
            target_provider: None,
            deployment_id: None,
        });
        snapshot.findings = QualityFindings {
            total: 3,
            source: Some(QualitySource::Qr),
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            freshness: QualityFreshness::Stale,
            ..QualityFindings::default()
        };
        snapshot.maturity = QualityMaturity {
            score: Some(2.5),
            score_display: Some("2.500".to_string()),
            audit_id: Some("target-audit".to_string()),
            observed_at: Some(observed_at),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            freshness: QualityFreshness::Stale,
            ..QualityMaturity::default()
        };

        project_quality_snapshot_for_target(&mut snapshot, "dev", "target-commit");

        let build = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "build")
            .expect("build gate should remain configured");
        assert_eq!(build.status, QualityGateStatus::Failed);
        assert_eq!(build.evidence.len(), 1);
        assert_eq!(build.evidence[0].freshness, QualityFreshness::Fresh);
        assert_eq!(build.freshness, QualityFreshness::Unknown);
        let lint = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "lint")
            .expect("lint gate should remain in the canonical list");
        assert!(lint.evidence.is_empty());
        assert_eq!(lint.status, QualityGateStatus::NotConfigured);
        assert_eq!(snapshot.findings.total, 3);
        assert_eq!(snapshot.findings.freshness, QualityFreshness::Fresh);
        assert_eq!(snapshot.maturity.score, Some(2.5));
        assert_eq!(snapshot.maturity.freshness, QualityFreshness::Fresh);
        assert_eq!(snapshot.ingestion_status, "Available");
        assert!(snapshot.ingestion_message.is_none());
        assert!(snapshot
            .ci_readiness
            .failed_gate_ids
            .contains(&"build".to_string()));
        assert!(!snapshot
            .ci_readiness
            .configured_gate_ids
            .contains(&"lint".to_string()));
    }

#[test]
    fn ingests_qr_gates_and_severity_breakdown_without_running_commands() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let observed_at = Utc::now().to_rfc3339();
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-1");
        fs::create_dir_all(&run).expect("QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "created_at": observed_at,
                "git": { "branch": "main", "head_sha": "abc" }
            })
            .to_string(),
        )
        .expect("run manifest should be writable");
        fs::write(
            run.join("gate-verification.json"),
            serde_json::json!({
                "gates": [
                    {
                        "id": "runtime_smoke",
                        "status": "passed",
                        "capability_kind": "local_command",
                        "command": "pnpm smoke",
                        "completed_at": observed_at
                    },
                    {
                        "id": "security scan",
                        "status": "failed",
                        "capability_kind": "qr",
                        "failure_type": "command-failed",
                        "reason": "finding threshold exceeded"
                    },
                    {
                        "id": "review",
                        "status": "skipped",
                        "skip_type": "missing evidence"
                    }
                ]
            })
            .to_string(),
        )
        .expect("gate verification should be writable");
        fs::write(
            run.join("quality-audit.json"),
            r#"{"findings":[
                {"fingerprint":"audit-1","severity":"critical"},
                {"fingerprint":"audit-2","severity":"high"},
                {"fingerprint":"audit-3","severity":"warning"},
                {"fingerprint":"audit-4","severity":"low"}
            ]}"#,
        )
        .expect("QR report should be writable");
        fs::write(
            run.join("code-quality-scan.json"),
            r#"{"summary":{"findings_by_category":{"debloat":1,"simplify":1}},"findings":[
                {"fingerprint":"stable-1","category":"debloat","severity":"warning"},
                {"fingerprint":"stable-2","category":"simplify","severity":"observation"}
            ]}"#,
        )
        .expect("fingerprinted QR report should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        let smoke = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "runtime_smoke")
            .expect("smoke gate should be imported");
        assert_eq!(smoke.status, QualityGateStatus::Passed);
        assert_eq!(smoke.evidence[0].source, QualitySource::Local);
        assert_eq!(smoke.evidence[0].command.as_deref(), Some("pnpm smoke"));
        assert_eq!(smoke.freshness, QualityFreshness::Fresh);
        let security = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "custom:security_scan")
            .expect("security gate should be imported");
        assert_eq!(security.label, "Security Scan");
        assert_eq!(security.status, QualityGateStatus::Failed);
        assert_eq!(security.evidence[0].detail, "command-failed");
        assert!(snapshot
            .gates
            .iter()
            .any(|gate| gate.id == "custom:review" && gate.status == QualityGateStatus::Blocked));
        assert_eq!(snapshot.findings.total, 6);
        assert_eq!(snapshot.findings.category_counts.get("debloat"), Some(&1));
        let debloat = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "debloat")
            .expect("debloat review gate should be imported");
        assert_eq!(debloat.status, QualityGateStatus::Blocked);
        assert_eq!(debloat.freshness, QualityFreshness::Unknown);
        assert!(debloat.evidence[0]
            .detail
            .contains("no signal authorizes deletion"));
        assert_eq!(snapshot.findings.high_severity_total, 2);
        assert_eq!(snapshot.findings.severity_counts.get("critical"), Some(&1));
        assert_eq!(snapshot.findings.severity_counts.get("high"), Some(&1));
        assert_eq!(snapshot.findings.severity_counts.get("medium"), Some(&2));
        assert_eq!(snapshot.findings.report_paths.len(), 2);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("code-quality-scan.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn reads_execution_plan_and_run_summary_when_completed_artifacts_are_partial() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-partial");
        fs::create_dir_all(&run).expect("QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            r#"{"created_at":"2026-07-26T11:00:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("run manifest should be writable");
        fs::write(
            run.join("gate-execution-plan.json"),
            r#"[{"id":"formatter","command":"pnpm format:check","capability_kind":"local_command","source":"package.json","local_execution_status":"consent-required"}]"#,
        )
        .expect("execution plan should be writable");
        fs::write(
            run.join("run-summary.json"),
            r#"{"finding_counts":{"total":3}}"#,
        )
        .expect("run summary should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        let formatter = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "formatter")
            .expect("formatter gate should be imported from the plan");
        assert_eq!(formatter.status, QualityGateStatus::Blocked);
        assert_eq!(formatter.evidence[0].source, QualitySource::Local);
        assert_eq!(
            formatter.evidence[0].command.as_deref(),
            Some("pnpm format:check")
        );
        assert_eq!(snapshot.findings.total, 3);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("run-summary.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn fleet_detector_run_uses_sibling_inspect_findings_with_latest_verify_evidence() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let runs = repository_path.join(".quality-runner").join("runs");
        let inspect = runs.join("fleet-detector-transaction-repo-inspect");
        let verify = runs.join("fleet-detector-transaction-repo-verify");
        fs::create_dir_all(&inspect).expect("inspect run should be writable");
        fs::create_dir_all(&verify).expect("verify run should be writable");
        let publication = r#"{"schema":"quality-runner-fleet-detector-publication/v1"}"#;
        fs::write(inspect.join("fleet-detector-publication.json"), publication)
            .expect("inspect publication should be writable");
        fs::write(verify.join("fleet-detector-publication.json"), publication)
            .expect("verify publication should be writable");
        fs::write(
            inspect.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:00:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("inspect manifest should be writable");
        fs::write(
            inspect.join("code-quality-scan.json"),
            r#"{"findings":[{"severity":"high"},{"severity":"medium"}]}"#,
        )
        .expect("detector report should be writable");
        fs::write(
            verify.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:02:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("verify manifest should be writable");
        fs::write(
            verify.join("run-summary.json"),
            r#"{"finding_counts":{"total":0}}"#,
        )
        .expect("verify summary should be writable");
        fs::write(
            verify.join("gate-verification.json"),
            r#"{"gates":[{"id":"lint","status":"passed","command":"pnpm lint"}]}"#,
        )
        .expect("verification should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        assert_eq!(snapshot.findings.total, 2);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("-inspect/code-quality-scan.json")));
        assert!(snapshot.gates.iter().any(|gate| gate.id == "lint"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
