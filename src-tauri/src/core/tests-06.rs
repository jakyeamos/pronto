#[test]
    fn reads_optional_agent_manifest_with_explicit_activity_language() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "agent/manifest-task"]);
        let manifest_directory = repository.join(".pronto");
        fs::create_dir_all(&manifest_directory).expect("manifest directory should be creatable");
        fs::write(
            manifest_directory.join("agent.json"),
            serde_json::to_vec(&serde_json::json!({
                "task_id": "task-42",
                "title": "Document workspace recovery",
                "target_branch": "main",
                "agent_type": "codex",
                "start_time": "2026-07-26T12:00:00Z",
                "status": "active",
                "source_session_id": "session-42"
            }))
            .expect("manifest should encode"),
        )
        .expect("manifest should be writable");

        let workspace = scan_workspace(&repository, true, Some("main"), None, "Medium", None);
        assert_eq!(workspace.activity.state, "Active");
        assert_eq!(workspace.activity.confidence, "High");
        assert_eq!(
            workspace
                .activity
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.task_id.as_deref()),
            Some("task-42")
        );
        assert_eq!(workspace.provenance.kind, "canonical");
        assert_eq!(workspace.provenance.owner.as_deref(), Some("task-42"));
        assert_eq!(workspace.provenance.lease.as_deref(), Some("active"));
        assert_eq!(
            workspace.provenance.canonical_repository,
            canonical_path(&repository)
                .expect("repository should canonicalize")
                .to_string_lossy()
        );
        assert_eq!(workspace.provenance.cleanup_state, "not_applicable");
        assert_eq!(workspace.target_branch.as_deref(), Some("main"));
        assert_eq!(workspace.target_confidence, "High");
        assert_eq!(workspace.role, "Agent task");
        assert!(workspace
            .activity
            .signals
            .iter()
            .any(|signal| signal.source == "Manifest"));

        let encoded = serde_json::to_string(&workspace).expect("workspace should serialize");
        assert!(!encoded.contains("terminal contents"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn classifies_clean_inspected_workspaces_as_idle_instead_of_unknown() {
        assert_eq!(
            workspace_activity_state(false, false, false, true, false, false, 0),
            "Idle"
        );
        assert_eq!(
            workspace_activity_state(false, false, false, false, true, false, 0),
            "Unknown"
        );
        assert_eq!(
            workspace_activity_state(false, false, false, true, false, true, 0),
            "Interrupted with dirty work"
        );
        assert_eq!(
            workspace_activity_state(false, false, false, true, false, false, 1),
            "Interrupted with unpushed commits"
        );
    }

#[test]
    fn active_workspace_activity_blocks_integration_eligibility() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/active"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "Feature commit"]);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.activity.state = "Active".to_string();
        assert_eq!(
            branch_integration_state(
                &repository,
                &workspace.branch,
                Some("main"),
                Some(&workspace),
            ),
            "Blocked"
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn stale_quality_evidence_blocks_release_rule_evaluation() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality = QualitySnapshot {
            gates: vec![quality::QualityGate {
                id: "lint".to_string(),
                label: "Lint".to_string(),
                status: QualityGateStatus::Passed,
                freshness: QualityFreshness::Stale,
                evidence: vec![quality::QualityEvidence {
                    id: "lint".to_string(),
                    source: quality::QualitySource::Ci,
                    status: QualityGateStatus::Passed,
                    freshness: QualityFreshness::Stale,
                    observed_at: Some("2026-07-10T12:00:00Z".to_string()),
                    scanned_commit: Some("old-commit".to_string()),
                    scanned_branch: Some("main".to_string()),
                    command: None,
                    source_label: "GitHub check · lint".to_string(),
                    report_path: None,
                    report_url: None,
                    report_kind: Some("GitHub check run".to_string()),
                    detail: "success".to_string(),
                    verification_level: quality::QualityVerificationLevel::SourceInferred,
                    target_kind: Some("source".to_string()),
                    target_url: None,
                    target_provider: Some("github".to_string()),
                    deployment_id: None,
                }],
            }],
            ..QualitySnapshot::default()
        };
        let rule = ReleaseRuleConfig {
            name: "Fresh lint required".to_string(),
            operator: "AND".to_string(),
            min_commits: None,
            min_elapsed_days: None,
            required_commit_types: Vec::new(),
            allow_first_release: false,
            required_quality_gates: vec![QualityGateRequirement {
                gate_id: "lint".to_string(),
                source: quality::QualitySource::Ci,
                minimum_verification_level: None,
                policy: quality::QualityRequirementPolicy::Block,
            }],
        };

        let (result, trace) = evaluate_release_rule_with_quality(&repository, &rule, None, &[]);
        assert_eq!(result, ReleaseRuleResult::Blocked);
        assert!(trace.iter().any(|item| {
            item.label == "Quality gate · Lint · CI · Block" && item.status == "Passed · Stale"
        }));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn warning_quality_requirement_is_visible_without_blocking_release() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality = QualitySnapshot {
            gates: vec![quality::QualityGate {
                id: "web_readiness".to_string(),
                label: "Web readiness".to_string(),
                status: QualityGateStatus::Failed,
                freshness: QualityFreshness::Fresh,
                evidence: vec![quality::QualityEvidence {
                    id: "web_readiness".to_string(),
                    source: quality::QualitySource::Qr,
                    status: QualityGateStatus::Failed,
                    freshness: QualityFreshness::Fresh,
                    observed_at: Some(iso_now()),
                    scanned_commit: repository.workspace.last_commit.clone(),
                    scanned_branch: Some(repository.branch.clone()),
                    command: None,
                    source_label: "Quality Runner web readiness".to_string(),
                    report_path: None,
                    report_url: None,
                    report_kind: Some("Quality Runner web readiness".to_string()),
                    detail: "Polish warning".to_string(),
                    verification_level: quality::QualityVerificationLevel::DeploymentVerified,
                    target_kind: Some("deployment".to_string()),
                    target_url: Some("https://preview.example.test".to_string()),
                    target_provider: Some("fixture".to_string()),
                    deployment_id: Some("dep-1".to_string()),
                }],
            }],
            ..QualitySnapshot::default()
        };
        let rule = ReleaseRuleConfig {
            name: "Web polish visibility".to_string(),
            operator: "AND".to_string(),
            min_commits: None,
            min_elapsed_days: None,
            required_commit_types: Vec::new(),
            allow_first_release: false,
            required_quality_gates: vec![QualityGateRequirement {
                gate_id: "web_readiness".to_string(),
                source: quality::QualitySource::Qr,
                minimum_verification_level: Some(
                    quality::QualityVerificationLevel::DeploymentVerified,
                ),
                policy: quality::QualityRequirementPolicy::Warn,
            }],
        };

        let (result, trace) = evaluate_release_rule_with_quality(&repository, &rule, None, &[]);
        assert_eq!(result, ReleaseRuleResult::Passed);
        assert!(trace.iter().any(|item| {
            item.label == "Quality gate · Web readiness · QR · Warn"
                && item.status == "Failed · Fresh"
        }));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn public_release_preview_is_blocked_by_missing_boundary_evidence() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let contract_dir = repository_path.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal contract directory should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::json!({
                "schema_version": "pronto-remediation-goal/v1",
                "target_state": "public_release",
                "reason": "Fixture exercises public release preparation."
            })
            .to_string(),
        )
        .expect("goal contract should be writable");
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.provider_state = "GitHub connected as github:fixture".to_string();
        repository.last_fetch_at = Some(iso_now());

        let preparation = prepare_release(&repository, &repository.workspace, true);

        assert_eq!(
            preparation.release_boundary_status.as_deref(),
            Some("Missing · Unknown")
        );
        assert!(preparation.reasons.iter().any(|reason| {
            reason.contains("Public-release boundary evidence is missing and unknown")
        }));
        assert!(preparation.evidence.iter().any(|item| {
            item.label == "Public-release boundary"
                && item.value.contains("blocking: receipt_missing")
        }));
        assert_eq!(preparation.status, "Blocked");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn public_release_preview_accepts_only_a_fresh_passing_boundary() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let contract_dir = repository_path.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal contract directory should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::json!({
                "schema_version": "pronto-remediation-goal/v1",
                "target_state": "public_release",
                "reason": "Fixture exercises public release preparation."
            })
            .to_string(),
        )
        .expect("goal contract should be writable");
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.provider_state = "GitHub connected as github:fixture".to_string();
        repository.last_fetch_at = Some(iso_now());
        repository.quality.release_boundary.status = "Passed".to_string();
        repository.quality.release_boundary.freshness = "Fresh".to_string();
        repository
            .quality
            .release_boundary
            .blocking_check_ids
            .clear();
        repository.quality.release_boundary.checks = [
            "source_provenance",
            "surface_classification",
            "tracked_public_content",
            "public_adapter_fixtures",
            "distribution_archives",
            "clean_room_install",
        ]
        .into_iter()
        .map(|id| crate::release_boundary::ReleaseBoundaryCheck {
            id: id.to_string(),
            status: "passed".to_string(),
            reason: None,
        })
        .collect();

        let preparation = prepare_release(&repository, &repository.workspace, true);

        assert_eq!(
            preparation.release_boundary_status.as_deref(),
            Some("Passed · Fresh")
        );
        assert!(!preparation
            .reasons
            .iter()
            .any(|reason| reason.contains("Public-release boundary evidence")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn non_public_release_preview_does_not_require_boundary_evidence() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let repository = scan_repository(&repository_path, None, &[]);

        let preparation = prepare_release(&repository, &repository.workspace, false);

        assert_eq!(preparation.release_boundary_status, None);
        assert!(!preparation
            .reasons
            .iter()
            .any(|reason| reason.contains("Public-release boundary evidence")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn release_preview_is_blocked_when_behavior_assurance_is_not_ready() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.provider_state = "GitHub connected as github:fixture".to_string();
        repository.last_fetch_at = Some(iso_now());

        let preparation = prepare_release(&repository, &repository.workspace, true);

        assert!(preparation.reasons.iter().any(|reason| {
            reason.contains("Behavior assurance is missing · unknown · unknown")
        }));
        assert!(preparation.evidence.iter().any(|item| {
            item.label == "Behavior assurance" && item.value.contains("missing · unknown · unknown")
        }));
        assert_eq!(preparation.status, "Blocked");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn release_preview_accepts_current_passing_behavior_assurance() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.provider_state = "GitHub connected as github:fixture".to_string();
        repository.last_fetch_at = Some(iso_now());
        repository.quality.behavior_assurance.contract_status = "current".to_string();
        repository.quality.behavior_assurance.result_status = "passed".to_string();
        repository.quality.behavior_assurance.freshness = "current".to_string();
        repository.quality.behavior_assurance.release_ready = true;
        repository
            .quality
            .behavior_assurance
            .required_scenario_count = 4;
        repository.quality.behavior_assurance.passed_scenario_count = 4;
        repository.quality.behavior_assurance.gaps.clear();

        let preparation = prepare_release(&repository, &repository.workspace, true);

        assert!(!preparation
            .reasons
            .iter()
            .any(|reason| reason.contains("Behavior assurance is")));
        assert!(preparation.evidence.iter().any(|item| {
            item.label == "Behavior assurance"
                && item.value == "current · passed · current · 4/4 required scenarios"
        }));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn bounded_release_git_command_times_out_instead_of_hanging() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let started_at = Instant::now();
        let result = run_git_bounded(
            &repository,
            ["-c", "alias.pronto-slow=!sleep 2", "pronto-slow"],
            StdDuration::from_millis(50),
        );

        assert!(result.is_err_and(|error| error.contains("release-preview deadline")));
        assert!(started_at.elapsed() < StdDuration::from_secs(1));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn external_handoff_requires_exact_workspace_and_supported_tool() {
        let root = fixture_root();
        let _repository = fixture_repository(&root);
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None)
            .expect("workspace fixture should persist");
        let persisted = load_store(&database).expect("workspace fixture should reload");
        let stored_repository = persisted
            .repositories
            .first()
            .expect("fixture repository should be registered");
        let workspace_id = stored_repository.workspace.id.clone();

        let missing_workspace = open_workspace_at(
            &database,
            &stored_repository.id,
            "workspace-that-is-not-registered",
            "file_browser",
        )
        .expect_err("handoff must reject an unknown workspace");
        assert_eq!(
            missing_workspace,
            "Workspace is not registered for this repository"
        );

        let unsupported_tool = open_workspace_at(
            &database,
            &stored_repository.id,
            &workspace_id,
            "unsupported",
        )
        .expect_err("handoff must reject an unknown tool");
        assert_eq!(unsupported_tool, "Choose a supported external handoff tool");
        assert_eq!(
            load_store(&database)
                .expect("handoff fixture should remain readable")
                .repositories
                .len(),
            1
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
