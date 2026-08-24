#[test]
    fn unmarked_similarly_named_runs_do_not_share_finding_reports() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let runs = repository_path.join(".quality-runner").join("runs");
        let inspect = runs.join("ordinary-run-inspect");
        let verify = runs.join("ordinary-run-verify");
        fs::create_dir_all(&inspect).expect("inspect run should be writable");
        fs::create_dir_all(&verify).expect("verify run should be writable");
        fs::write(
            inspect.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:00:00Z"}"#,
        )
        .expect("inspect manifest should be writable");
        fs::write(
            inspect.join("code-quality-scan.json"),
            r#"{"findings":[{"severity":"high"}]}"#,
        )
        .expect("detector report should be writable");
        fs::write(
            verify.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:02:00Z"}"#,
        )
        .expect("verify manifest should be writable");
        fs::write(
            verify.join("run-summary.json"),
            r#"{"finding_counts":{"total":0}}"#,
        )
        .expect("verify summary should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        assert_eq!(snapshot.findings.total, 0);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("-verify/run-summary.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn source_requirements_do_not_fall_back_to_another_source() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let mut repository = repository;
        repository.quality.gates = vec![QualityGate {
            id: "lint".to_string(),
            label: "Lint".to_string(),
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            evidence: vec![evidence(
                QualitySource::Local,
                QualityGateStatus::Passed,
                QualityFreshness::Fresh,
            )],
        }];
        let (status, freshness, detail) = evaluate_requirement(
            &repository,
            &QualityGateRequirement {
                gate_id: "lint".to_string(),
                source: QualitySource::Ci,
                minimum_verification_level: None,
                policy: QualityRequirementPolicy::Block,
            },
        );
        assert_eq!(status, QualityGateStatus::NotConfigured);
        assert_eq!(freshness, QualityFreshness::Unknown);
        assert!(detail.contains("CI evidence"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn verification_level_requirement_rejects_weaker_passing_evidence() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        let mut source = evidence(
            QualitySource::Qr,
            QualityGateStatus::Passed,
            QualityFreshness::Fresh,
        );
        source.id = "web_readiness".to_string();
        source.verification_level = QualityVerificationLevel::SourceInferred;
        repository.quality.gates = vec![QualityGate {
            id: "web_readiness".to_string(),
            label: "Web readiness".to_string(),
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            evidence: vec![source],
        }];
        let requirement = QualityGateRequirement {
            gate_id: "web_readiness".to_string(),
            source: QualitySource::Qr,
            minimum_verification_level: Some(QualityVerificationLevel::DeploymentVerified),
            policy: QualityRequirementPolicy::Block,
        };

        let (status, freshness, detail) = evaluate_requirement(&repository, &requirement);
        assert_eq!(status, QualityGateStatus::Blocked);
        assert_eq!(freshness, QualityFreshness::Unknown);
        assert!(detail.contains("deployment_verified"));

        let mut deployment = repository.quality.gates[0].evidence[0].clone();
        deployment.verification_level = QualityVerificationLevel::DeploymentVerified;
        deployment.target_kind = Some("deployment".to_string());
        repository.quality.gates[0].evidence.push(deployment);
        let (status, freshness, _) = evaluate_requirement(&repository, &requirement);
        assert_eq!(status, QualityGateStatus::Passed);
        assert_eq!(freshness, QualityFreshness::Fresh);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn web_readiness_import_preserves_target_identity_and_categorical_status() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(repository_path.join(".quality-runner"))
            .expect("web-readiness directory should be writable");
        let repository = fixture_repository(&repository_path);
        fs::write(
            repository_path.join(WEB_READINESS_RELATIVE_PATH),
            serde_json::to_string(&serde_json::json!({
                "schema": WEB_READINESS_SCHEMA,
                "status": "warnings",
                "generated_at": Utc::now().to_rfc3339(),
                "repository": {"path": repository.path, "branch": "main", "head_sha": "abc"},
                "applicability": {"status": "public_web", "reason": "Public product"},
                "summary": {"passed": 2, "failed": 1, "blocked": 0, "unknown": 0},
                "target": {
                    "kind": "deployment",
                    "commit": "abc",
                    "url": "https://preview.example.test",
                    "provider": "fixture",
                    "deployment_id": "dep-123"
                },
                "checks": [
                    {
                        "id": "route_titles", "label": "Route titles", "category": "baseline",
                        "policy": "block", "status": "passed", "verification_level": "deployment_verified",
                        "detail": "Unique titles", "evidence": [{"route": "/"}]
                    },
                    {
                        "id": "social_image", "label": "Social image", "category": "polish",
                        "policy": "warn", "status": "failed", "verification_level": "deployment_verified",
                        "detail": "Missing image", "evidence": [{"route": "/"}]
                    }
                ]
            }))
            .expect("web-readiness fixture should encode"),
        )
        .expect("web-readiness fixture should be writable");

        let ci_gate_profile = ci_gate_profile_for_repository(&repository);
        let snapshot = ingest_repository_quality(&repository, None, None, Some(&ci_gate_profile));
        assert_eq!(snapshot.web_readiness.status, "Warnings");
        assert_eq!(snapshot.web_readiness.applicability, "public_web");
        assert_eq!(
            snapshot.web_readiness.target.deployment_id.as_deref(),
            Some("dep-123")
        );
        assert_eq!(snapshot.web_readiness.warning_count, 1);
        assert!(snapshot
            .ci_readiness
            .applicable_gate_ids
            .contains(&"web_readiness".to_string()));
        let evidence = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "web_readiness")
            .and_then(|gate| gate.evidence.first())
            .expect("web-readiness evidence should be projected");
        assert_eq!(evidence.status, QualityGateStatus::Passed);
        assert_eq!(
            evidence.verification_level,
            QualityVerificationLevel::DeploymentVerified
        );
        assert_eq!(
            evidence.target_url.as_deref(),
            Some("https://preview.example.test")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn fleet_import_keeps_maturity_rows_out_of_quality_findings() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-fleet","as_of":"2026-08-03T20:05:55Z"}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet-repo",
                "as_of": "2026-08-03T20:05:55Z",
                "repository": {
                    "primary_path": repository_path.to_string_lossy(),
                    "checkouts": [{"path": repository_path.to_string_lossy(), "head": "abc", "branch": "main"}]
                },
                "detector_evidence": [{
                    "detector": "anti-slop",
                    "status": "passed",
                    "applicable": true,
                    "enabled_rules": ["anti-slop/no-known-value-widening", "anti-slop/no-widen-then-assert"],
                    "producer": {"version": "0.8.0", "source_sha": "producer-sha"},
                    "ruleset_hash": "ruleset-sha",
                    "configuration_hash": "configuration-sha",
                    "qr_version": "0.7.0",
                    "target_sha": "abc",
                    "scan_time": "2026-08-03T20:05:54Z"
                }],
                "agent_usability": {
                    "applicability": "applicable",
                    "schema": "quality-runner-agent-usability/v1",
                    "status": "attention",
                    "manifest_status": "present",
                    "manifest_path": ".agents/agent-usability.json",
                    "applicable_lane_count": 4,
                    "covered_lane_count": 3,
                    "lanes": [
                        {
                            "id": "documentation_contract",
                            "label": "Documentation contract",
                            "applicable": true,
                            "score": 3,
                            "status": "validated",
                            "message": "Documentation exists."
                        },
                        {
                            "id": "tool_skill_coverage",
                            "label": "Tool-to-skill coverage",
                            "applicable": true,
                            "score": 3,
                            "status": "static_validated",
                            "message": "Skill mapping exists."
                        },
                        {
                            "id": "behavior_evidence",
                            "label": "Behavior evidence",
                            "applicable": true,
                            "score": 1,
                            "status": "missing",
                            "message": "Behavior evidence remains unverified."
                        },
                        {
                            "id": "freshness_portability",
                            "label": "Freshness and portability",
                            "applicable": true,
                            "score": 3,
                            "status": "static_validated",
                            "message": "References are portable."
                        }
                    ],
                    "growth_health": {
                        "status": "healthy",
                        "score": 4,
                        "message": "Documentation and skill structure remains proportionate and routed.",
                        "document_count": 4,
                        "agent_document_count": 2,
                        "routed_agent_document_count": 2,
                        "skill_count": 1,
                        "family_count": 1,
                        "tool_count": 1
                    }
                },
                "findings": [
                    {
                        "applicable": true,
                        "dimension": "quality_commands",
                        "score": 4,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "validated"
                    },
                    {
                        "applicable": true,
                        "dimension": "security_constraints",
                        "score": 4,
                        "pack": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "maintained"
                    },
                    {
                        "applicable": true,
                        "dimension": "matrix_maintenance",
                        "score": 2,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "incomplete"
                    },
                    {
                        "finding_id": "finding-real",
                        "schema": "quality-runner-code-finding-v1",
                        "severity": "high"
                    }
                ]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let imported =
            fleet_audit_import(Some(&audit_root), &[fixture_repository(&repository_path)]);
        let evidence = imported
            .evidence
            .get("repo-1")
            .expect("repository evidence should be imported");

        assert_eq!(evidence.maturity.score_display.as_deref(), Some("3.000"));
        assert_eq!(evidence.maturity.dimension_scores.len(), 8);
        assert_eq!(
            evidence.maturity.dimension_scores.get("matrix_maintenance"),
            Some(&2.0)
        );
        assert_eq!(
            evidence
                .maturity
                .dimension_scores
                .get("agent_usability.behavior_evidence"),
            Some(&1.0)
        );
        let agent_usability = evidence
            .maturity
            .agent_usability
            .as_ref()
            .expect("per-repository audits should retain agent-usability evidence");
        assert_eq!(agent_usability.covered_lane_count, 3);
        assert_eq!(agent_usability.lanes[2].id, "behavior_evidence");
        assert_eq!(evidence.findings.total, 1);
        assert_eq!(evidence.findings.detector_findings_total, 1);
        assert_eq!(evidence.findings.enabled_detector_count, 1);
        assert_eq!(evidence.findings.enabled_rule_count, 2);
        assert_eq!(
            evidence.findings.producer_versions.get("anti-slop"),
            Some(&"0.8.0".to_string())
        );
        assert_eq!(
            evidence.findings.ruleset_fingerprints.get("anti-slop"),
            Some(&"ruleset-sha".to_string())
        );
        assert_eq!(evidence.findings.target_sha.as_deref(), Some("abc"));
        assert_eq!(evidence.findings.qr_version.as_deref(), Some("0.7.0"));
        assert_eq!(evidence.findings.high_severity_total, 1);
        assert_eq!(evidence.findings.severity_counts.get("high"), Some(&1));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn fleet_import_uses_the_selected_target_checkout_for_branch_provenance() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-target","as_of":"2026-08-08T12:00:00Z"}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-target-repo",
                "as_of": "2026-08-08T12:00:00Z",
                "repository": {
                    "primary_path": repository_path.to_string_lossy(),
                    "target_branch": {"branch": "dev"},
                    "checkouts": [
                        {"path": repository_path.to_string_lossy(), "head": "stale-main", "branch": "main"},
                        {"path": "/private/tmp/target-worktree", "head": "target-dev", "branch": "dev"}
                    ]
                },
                "findings": []
            }))
            .expect("target fleet finding should encode"),
        )
        .expect("target fleet finding should be writable");

        let imported =
            fleet_audit_import(Some(&audit_root), &[fixture_repository(&repository_path)]);
        let evidence = imported
            .evidence
            .get("repo-1")
            .expect("repository evidence should be imported");
        assert_eq!(evidence.maturity.scanned_branch.as_deref(), Some("dev"));
        assert_eq!(
            evidence.maturity.scanned_commit.as_deref(),
            Some("target-dev")
        );
        assert_eq!(evidence.findings.scanned_branch.as_deref(), Some("dev"));
        assert_eq!(
            evidence.findings.scanned_commit.as_deref(),
            Some("target-dev")
        );
        fs::remove_dir_all(root).expect("target fleet fixture should be removable");
    }

#[test]
    fn stable_detector_reports_are_distinguished_from_aggregate_reports() {
        assert!(is_stable_detector_report(Some(
            "/tmp/pronto/.quality-runner/runs/run-1/code-quality-scan.json"
        )));
        assert!(!is_stable_detector_report(Some(
            "/tmp/pronto/.quality-runner/fleet-audit/findings/repo.json"
        )));
        assert!(!is_stable_detector_report(None));
    }

#[test]
    fn blocked_detector_refresh_retains_prior_evidence_and_suppresses_delta() {
        let mut prior = QualityFindings {
            total: 2,
            detector_findings_total: 2,
            source: Some(QualitySource::Qr),
            target_sha: Some("target-sha".to_string()),
            qr_version: Some("0.7.0".to_string()),
            producer_versions: BTreeMap::from([("anti-slop".to_string(), "0.8.0".to_string())]),
            producer_source_shas: BTreeMap::from([(
                "anti-slop".to_string(),
                "producer-sha".to_string(),
            )]),
            ruleset_fingerprints: BTreeMap::from([(
                "anti-slop".to_string(),
                "ruleset-sha".to_string(),
            )]),
            configuration_fingerprints: BTreeMap::from([(
                "anti-slop".to_string(),
                "configuration-sha".to_string(),
            )]),
            ..QualityFindings::default()
        };
        prior.delta_total = Some(1);
        let mut blocked = QualityFindings {
            source: Some(QualitySource::Qr),
            target_sha: Some("target-sha".to_string()),
            qr_version: Some("0.7.0".to_string()),
            refresh_required: true,
            refresh_required_reason: Some("Detector execution failed".to_string()),
            detector_status: Some("blocked".to_string()),
            report_path: Some("/tmp/anti-slop-detector.json".to_string()),
            ..QualityFindings::default()
        };

        preserve_detector_evidence_on_refresh_failure(&prior, &mut blocked);
        assert_eq!(blocked.detector_findings_total, 2);
        assert_eq!(blocked.total, 2);
        assert_eq!(blocked.ruleset_fingerprints, prior.ruleset_fingerprints);
        assert!(blocked.refresh_required);
        assert_eq!(blocked.detector_status.as_deref(), Some("blocked"));

        update_detector_delta(&prior, &mut blocked);
        assert_eq!(blocked.delta_total, None);

        let mut comparable = prior.clone();
        comparable.total = 3;
        comparable.detector_findings_total = 3;
        update_detector_delta(&prior, &mut comparable);
        assert_eq!(comparable.delta_total, Some(1));
    }
