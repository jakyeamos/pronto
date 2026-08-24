#[test]
    fn active_queue_ranks_preservation_before_later_quality_work() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "quality",
                "open",
                vec![fixture_action("quality", "qr_findings", "P1", 8, "open")],
            ),
            fixture_plan(
                "preserve",
                "open",
                vec![fixture_action(
                    "preserve",
                    "branch_hygiene",
                    "P2",
                    2,
                    "open",
                )],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "preserve");
        assert_eq!(run.plans[1].repository_name, "quality");
    }

#[test]
    fn active_queue_uses_fleet_leverage_before_raw_action_weight() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "ordinary-heavy-repo",
                "open",
                vec![fixture_action(
                    "many-findings",
                    "scope",
                    "P1",
                    10_000,
                    "open",
                )],
            ),
            fixture_plan(
                "quality-runner",
                "open",
                vec![fixture_action(
                    "evidence-provider",
                    "scope",
                    "P1",
                    3,
                    "open",
                )],
            ),
            fixture_plan(
                "AIOS",
                "open",
                vec![fixture_action("coordination", "scope", "P1", 2, "open")],
            ),
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(
            run.plans
                .iter()
                .map(|plan| plan.repository_name.as_str())
                .collect::<Vec<_>>(),
            vec!["pronto", "AIOS", "quality-runner", "ordinary-heavy-repo"]
        );
    }

#[test]
    fn active_queue_keeps_blocked_work_ahead_of_fleet_leverage() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 100, "open")],
            ),
            fixture_plan(
                "ordinary-blocker",
                "blocked",
                vec![fixture_action("blocker", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "ordinary-blocker");
        assert_eq!(run.plans[1].repository_name, "pronto");
    }

#[test]
    fn active_queue_uses_fleet_leverage_before_repository_goal() {
        let mut release = fixture_plan(
            "ordinary-public-release",
            "open",
            vec![fixture_action("release", "scope", "P1", 100, "open")],
        );
        release.goal =
            goal_definition("public_release").expect("public release goal should be supported");
        let mut run = empty_run();
        run.plans = vec![
            release,
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "pronto");
        assert_eq!(run.plans[1].repository_name, "ordinary-public-release");
    }

#[test]
    fn active_queue_uses_goal_priority_within_the_same_safety_stage() {
        let mut release = fixture_plan(
            "release",
            "open",
            vec![fixture_action(
                "release-provider",
                "provider",
                "P1",
                2,
                "open",
            )],
        );
        release.goal = goal_definition("public_release").expect("release goal should be supported");
        let mut clean = fixture_plan(
            "clean",
            "open",
            vec![fixture_action(
                "clean-provider",
                "provider",
                "P1",
                2,
                "open",
            )],
        );
        clean.goal = goal_definition("clean_only").expect("clean goal should be supported");
        let mut run = empty_run();
        run.plans = vec![clean, release];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "release");
        assert_eq!(run.plans[1].repository_name, "clean");
    }

#[test]
    fn public_release_goal_requires_an_explicit_distribution_boundary() {
        let repository = fixture_repository("public-distribution-boundary");
        let mut goal = goal_definition("public_release").expect("public release goal");
        goal.source = "repository_contract".to_string();
        let mut seeds = Vec::new();

        add_goal_seeds(&repository, &goal, &mut seeds);

        let boundary = seeds
            .iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("public release should require a distribution boundary action");
        assert_eq!(boundary.domain, "release_boundary");
        assert!(boundary
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("public_core")));
        assert!(boundary
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("isolated temporary home")));
        assert_eq!(
            seeds
                .iter()
                .filter(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
                .count(),
            1
        );
    }

#[test]
    fn passing_release_boundary_removes_the_remediation_action() {
        let mut repository = fixture_repository("public-boundary-passed");
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
        let goal = goal_definition("public_release").expect("public release goal");
        let mut seeds = Vec::new();

        add_goal_seeds(&repository, &goal, &mut seeds);

        assert!(seeds
            .iter()
            .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY));
    }

#[test]
    fn receipt_blockers_drive_remediation_and_manual_status_cannot_bypass_them() {
        let mut repository = fixture_repository("public-boundary-blocked");
        repository.quality.release_boundary.status = "Blocked".to_string();
        repository.quality.release_boundary.freshness = "Stale".to_string();
        repository.quality.release_boundary.blocking_check_ids = vec![
            "artifact_digest_mismatch".to_string(),
            "matrix_digest_mismatch".to_string(),
        ];
        repository.quality.release_boundary.detail =
            "The receipt no longer matches the release inputs.".to_string();
        let goal = goal_definition("public_release").expect("public release goal");
        let mut seeds = Vec::new();
        add_goal_seeds(&repository, &goal, &mut seeds);
        let seed = seeds
            .into_iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("blocked receipt should produce remediation");
        assert!(seed.summary.contains("artifact_digest_mismatch"));
        assert!(seed.summary.contains("matrix_digest_mismatch"));

        let mut previous_action = fixture_action(
            PUBLIC_RELEASE_BOUNDARY_ACTION_KEY,
            "release_boundary",
            "P1",
            3,
            "verified",
        );
        previous_action.stable_key = PUBLIC_RELEASE_BOUNDARY_ACTION_KEY.to_string();
        let previous = HashMap::from([(previous_action.stable_key.as_str(), &previous_action)]);
        let action = materialize_action(&repository, seed, &previous, "2026-08-11T12:00:00Z");
        assert_eq!(action.status, "blocked");
    }

#[test]
    fn non_public_and_ambiguous_goals_do_not_inherit_the_distribution_boundary() {
        let repository = fixture_repository("non-public-distribution-boundary");
        for target in [
            "deployed_product",
            "active_maintained",
            "clean_only",
            "prototype",
            "archived",
            "github_only",
        ] {
            let goal = goal_definition(target).expect("supported remediation goal");
            let mut seeds = Vec::new();
            add_goal_seeds(&repository, &goal, &mut seeds);
            assert!(
                seeds
                    .iter()
                    .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY),
                "{target} should not inherit public distribution work"
            );
        }

        let inferred = inferred_goal_profile(&repository, None);
        assert_eq!(inferred.target_state, "active_maintained");
        let mut inferred_seeds = Vec::new();
        add_goal_seeds(&repository, &inferred, &mut inferred_seeds);
        assert!(inferred_seeds
            .iter()
            .any(|seed| seed.stable_key == "scope:confirm-remediation-goal"));
        assert!(inferred_seeds
            .iter()
            .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY));
    }

#[test]
    fn public_distribution_boundary_is_a_release_preparation_phase_and_surface() {
        let repository = fixture_repository("public-boundary-coverage");
        let mut goal = goal_definition("public_release").expect("public release goal");
        goal.source = "repository_contract".to_string();
        let mut seeds = Vec::new();
        add_goal_seeds(&repository, &goal, &mut seeds);
        let boundary_seed = seeds
            .into_iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("public release boundary seed");
        let action = materialize_action(
            &repository,
            boundary_seed,
            &HashMap::new(),
            "2026-08-11T12:00:00Z",
        );

        let coverage = build_ui_coverage(&repository, &goal, std::slice::from_ref(&action));
        let release_preparation = coverage
            .iter()
            .find(|entry| entry.surface == "release_preparation")
            .expect("release preparation coverage");
        assert_eq!(release_preparation.status, "blocked");
        assert_eq!(release_preparation.action_ids, vec![action.id.clone()]);

        let explanation = build_remediation_explanation(&goal, &[action], &coverage);
        assert_eq!(explanation.phases.len(), 1);
        assert_eq!(explanation.phases[0].id, "public_distribution_boundary");
    }

#[test]
    fn repository_goal_contract_controls_required_gates_and_freshness() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-goal-contract-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal fixture should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": REMEDIATION_GOAL_SCHEMA,
                "target_state": "clean_only",
                "reason": "This utility only needs a clean, preserved canonical branch.",
                "additional_required_gate_ids": ["tests"],
                "optional_gate_ids": ["secrets_scan"],
                "evidence_max_age_days": 21,
                "remediation_phases": [
                    {
                        "id": "deployment_validation",
                        "title": "Validate the local deployment",
                        "summary": "Verify repository-specific deployment behavior.",
                        "domains": ["deployment_validation"],
                        "completion_criterion": "The local deployment evidence is current.",
                        "after_phase_id": "quality_and_maturity"
                    }
                ]
            }))
            .expect("goal fixture should encode"),
        )
        .expect("goal contract should be writable");
        let mut repository = fixture_repository("goal-contract");
        repository.path = root.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();

        let goal = resolve_goal_profile(&repository);

        assert_eq!(goal.target_state, "clean_only");
        assert_eq!(goal.source, "repository_contract");
        assert_eq!(goal.evidence_max_age_days, 21);
        assert_eq!(goal.required_gate_ids, vec!["tests"]);
        assert!(goal.optional_gate_ids.contains(&"secrets_scan".to_string()));
        assert_eq!(goal.remediation_phases.len(), 1);
        assert_eq!(goal.remediation_phases[0].id, "deployment_validation");
        assert_eq!(
            goal.remediation_phases[0].after_phase_id.as_deref(),
            Some("quality_and_maturity")
        );
        fs::remove_dir_all(root).expect("goal fixture should be removable");
    }

#[test]
    fn remediation_goal_accepts_explicit_stable_custom_gate_ids() {
        assert_eq!(
            normalized_gate_ids(&["custom:restore_drill".to_string()]),
            Ok(vec!["custom:restore_drill".to_string()])
        );
        assert!(normalized_gate_ids(&["restore_drill".to_string()]).is_err());
        assert!(normalized_gate_ids(&["custom:restore-drill".to_string()]).is_err());
    }

#[test]
    fn repository_required_custom_ci_gates_enter_active_remediation() {
        let mut repository = fixture_repository("custom-ci-gate");
        repository.quality.ci_readiness.profile_source = "repository_contract".to_string();
        repository.quality.ci_readiness.applicable_gate_ids =
            vec!["build".to_string(), "custom:restore_drill".to_string()];
        repository.quality.ci_readiness.unconfigured_gate_ids =
            vec!["custom:restore_drill".to_string()];
        repository.quality.ci_readiness.missing_gate_ids = vec!["custom:restore_drill".to_string()];
        repository.quality.ci_readiness.gate_labels.insert(
            "custom:restore_drill".to_string(),
            "Restore drill".to_string(),
        );

        let goal = resolve_goal_profile(&repository);
        let mut seeds = Vec::new();
        add_ci_ideal_seeds(&repository, &goal, &mut seeds);

        assert_eq!(goal.target_state, "active_maintained");
        assert!(goal
            .required_gate_ids
            .contains(&"custom:restore_drill".to_string()));
        assert!(seeds.iter().any(|seed| {
            seed.stable_key == "ci_ideal:gate:custom:restore_drill"
                && seed.title == "Bring the Restore drill gate to the ideal state"
        }));
    }

#[test]
    fn discovered_custom_evidence_does_not_become_a_required_gate() {
        let mut repository = fixture_repository("discovered-custom-gate");
        repository.quality.ci_readiness.profile_source = "recommendation_matrix".to_string();
        repository.quality.ci_readiness.applicable_gate_ids =
            vec!["build".to_string(), "custom:restore_drill".to_string()];

        let goal = resolve_goal_profile(&repository);

        assert!(!goal
            .required_gate_ids
            .contains(&"custom:restore_drill".to_string()));
    }

#[test]
    fn repository_phase_contract_rejects_duplicate_domain_ownership() {
        let phases = vec![
            RemediationPhaseDefinition {
                id: "first".to_string(),
                title: "First".to_string(),
                summary: "First phase.".to_string(),
                domains: vec!["deployment".to_string()],
                completion_criterion: "First phase is complete.".to_string(),
                after_phase_id: None,
            },
            RemediationPhaseDefinition {
                id: "second".to_string(),
                title: "Second".to_string(),
                summary: "Second phase.".to_string(),
                domains: vec!["deployment".to_string()],
                completion_criterion: "Second phase is complete.".to_string(),
                after_phase_id: Some("first".to_string()),
            },
        ];

        let error = normalized_remediation_phases(&phases)
            .expect_err("one action domain cannot belong to two repository phases");

        assert!(error.contains("claimed by more than one repository phase"));
    }

#[test]
    fn repository_phase_contract_reserves_the_unclassified_fallback_id() {
        let phases = vec![RemediationPhaseDefinition {
            id: UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string(),
            title: "Custom fallback".to_string(),
            summary: "This would collide with the planner fallback.".to_string(),
            domains: vec!["future_domain".to_string()],
            completion_criterion: "The phase is complete.".to_string(),
            after_phase_id: None,
        }];

        let error = normalized_remediation_phases(&phases)
            .expect_err("the planner fallback id cannot be redefined by a repository");

        assert!(error.contains("duplicated or reserved"));
    }
