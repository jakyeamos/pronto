#[test]
    fn maturity_goals_expose_the_closure_floor_ideal_and_integrity_rule() {
        let goal = goal_definition("active_maintained").expect("maturity goal should be supported");
        let policy = goal
            .maturity_policy
            .as_ref()
            .expect("maturity-applicable goals should expose policy");

        assert_eq!(policy.minimum_closure_score, 3.0);
        assert_eq!(policy.ideal_score, 4.0);
        assert_eq!(
            policy.scoring_owner,
            "Quality Runner canonical maturity feed"
        );
        assert_eq!(
            policy.ideal_gate_ids,
            vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()]
        );
        assert_eq!(
            goal.maturity_gate_ids,
            vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()]
        );
        assert!(policy
            .improvement_rule
            .to_ascii_lowercase()
            .contains("continue material"));
        assert!(policy.integrity_rule.contains("superficial documentation"));

        let seed = maturity_seed(
            "maturity:score",
            "Raise maturity",
            "Below target",
            "Below target",
            "Fresh",
            Some("2026-07-29T12:00:00Z"),
            Some("/tmp/maturity.json"),
            None,
            None,
            3,
        );
        assert!(seed
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("4.0/4")));
        assert!(seed
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("solely to raise the score")));

        let repository = fixture_repository("mac-control-gate");
        let mut gate_seeds = Vec::new();
        add_maturity_gate_seeds(&repository, &goal, &mut gate_seeds);
        assert_eq!(gate_seeds.len(), 1);
        assert!(gate_seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("no fixed repository count")));
        assert!(gate_seeds[0].summary.contains("4.0/4.0 maturity ideal"));

        let mut passing_repository = repository;
        passing_repository.quality.mac_control_ideal_state.status = "Passed".to_string();
        passing_repository.quality.mac_control_ideal_state.freshness = "Fresh".to_string();
        let mut passing_gate_seeds = Vec::new();
        add_maturity_gate_seeds(&passing_repository, &goal, &mut passing_gate_seeds);
        assert!(passing_gate_seeds.is_empty());
    }

#[test]
    fn stale_evidence_contract_gets_one_generic_remediation_action() {
        let mut repository = fixture_repository("contract-audit");
        let contract = crate::evidence_contract::evaluate_repository_contract(
            mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_ID,
            mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_LABEL,
            mac_control_maturity::MAC_CONTROL_TASK_MANIFEST_SCHEMA,
            Some("mac-control-task-manifest/v2"),
            &repository.id,
            &repository.name,
        );
        repository.quality.evidence_contracts = vec![contract];
        let goal = goal_definition("active_maintained").expect("goal fixture");
        let mut seeds = Vec::new();

        add_evidence_contract_seeds(&repository, &mut seeds);
        add_maturity_gate_seeds(&repository, &goal, &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(
            seeds[0].stable_key,
            "evidence-contract:mac-control-task-manifest"
        );
        assert!(seeds[0]
            .title
            .contains(mac_control_maturity::MAC_CONTROL_TASK_MANIFEST_SCHEMA));
        assert!(seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("ambiguous evidence")));
    }

#[test]
    fn remediation_explanation_groups_work_into_ordered_phases_and_names_healthy_surfaces() {
        let mut preserve = fixture_action(
            "branch_hygiene:dirty:workspace",
            "branch_hygiene",
            "P1",
            2,
            "open",
        );
        preserve.title = "Preserve the dirty workspace".to_string();
        preserve.summary = "Review and preserve the current coherent slice.".to_string();
        preserve.acceptance_criteria = vec!["The workspace is intentional.".to_string()];
        let mut provider = fixture_action("provider:pull-request:14", "provider", "P2", 2, "open");
        provider.title = "Resolve pull request evidence".to_string();
        let mut maturity = fixture_action(
            "maturity:dimension:quality_commands",
            "maturity",
            "P2",
            2,
            "blocked",
        );
        maturity.title = "Improve quality command maturity".to_string();
        let verification = fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open");
        let verified_history = fixture_action(
            "product_truth:resolved",
            "product_truth",
            "P2",
            1,
            "verified",
        );
        let actions = vec![preserve, provider, maturity, verification, verified_history];
        let coverage = vec![
            RemediationCoverage {
                surface: "quality_evidence".to_string(),
                label: "Quality evidence".to_string(),
                status: "clear".to_string(),
                detail: "Required evidence is fresh.".to_string(),
                action_ids: Vec::new(),
            },
            RemediationCoverage {
                surface: "maturity".to_string(),
                label: "Repository maturity".to_string(),
                status: "blocked".to_string(),
                detail: "Score: 2.5/4.".to_string(),
                action_ids: vec!["maturity:dimension:quality_commands".to_string()],
            },
        ];
        let goal = goal_definition("active_maintained").expect("goal should be supported");

        let explanation = build_remediation_explanation(&goal, &actions, &coverage);

        assert_eq!(
            explanation
                .phases
                .iter()
                .map(|phase| phase.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "preserve_and_reconcile",
                "product_and_provider_truth",
                "quality_and_maturity",
                "verify_and_close"
            ]
        );
        assert_eq!(explanation.phases[0].steps.len(), 1);
        assert_eq!(
            explanation.phases[0].steps[0].title,
            "Preserve the dirty workspace"
        );
        assert_eq!(explanation.phases[2].status, "blocked");
        assert_eq!(
            explanation.summary,
            "4 ordered remediation phases remain across 4 active actions. Work from the first unresolved phase and verify each result before refreshing the queue."
        );
        assert!(!explanation
            .phases
            .iter()
            .flat_map(|phase| &phase.steps)
            .any(|step| step.action_id == "product_truth:resolved"));
        assert_eq!(explanation.healthy_surfaces.len(), 1);
        assert_eq!(explanation.healthy_surfaces[0].surface, "quality_evidence");
        assert!(explanation
            .closure_requirements
            .iter()
            .any(|requirement| requirement.contains("at least 3.0/4")));
        assert!(explanation
            .closure_requirements
            .iter()
            .any(|requirement| requirement.contains("configured maturity gates")));
        assert!(explanation.authority.contains("does not authorize Git"));
    }

#[test]
    fn remediation_explanation_supports_more_than_four_phases_and_covers_every_action_once() {
        let actions = vec![
            fixture_action("preserve", "branch_hygiene", "P1", 2, "open"),
            fixture_action("provider", "provider", "P2", 2, "open"),
            fixture_action("quality", "maturity", "P1", 3, "blocked"),
            fixture_action("deployment", "deployment_validation", "P2", 2, "open"),
            fixture_action("approval", "approval_rollout", "P2", 2, "in_progress"),
            fixture_action("unknown", "future_domain", "P2", 1, "open"),
            fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
            fixture_action("history", "future_history", "P3", 1, "verified"),
        ];
        let mut goal = goal_definition("active_maintained").expect("goal should be supported");
        goal.remediation_phases = vec![
            RemediationPhaseDefinition {
                id: "deployment_validation".to_string(),
                title: "Validate deployment".to_string(),
                summary: "Verify repository-specific deployment evidence.".to_string(),
                domains: vec!["deployment_validation".to_string()],
                completion_criterion: "Deployment evidence is current.".to_string(),
                after_phase_id: Some("quality_and_maturity".to_string()),
            },
            RemediationPhaseDefinition {
                id: "approval_rollout".to_string(),
                title: "Complete rollout approval".to_string(),
                summary: "Satisfy the repository-specific rollout approval.".to_string(),
                domains: vec!["approval_rollout".to_string()],
                completion_criterion: "Rollout approval is recorded.".to_string(),
                after_phase_id: Some("deployment_validation".to_string()),
            },
        ];

        let explanation = build_remediation_explanation(&goal, &actions, &[]);

        assert_eq!(
            explanation
                .phases
                .iter()
                .map(|phase| phase.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "preserve_and_reconcile",
                "product_and_provider_truth",
                "quality_and_maturity",
                "deployment_validation",
                "approval_rollout",
                "unclassified_remediation",
                "verify_and_close",
            ]
        );
        assert!(explanation
            .summary
            .starts_with("7 ordered remediation phases remain across 7 active actions."));
        let projected_action_ids = explanation
            .phases
            .iter()
            .flat_map(|phase| phase.steps.iter().map(|step| step.action_id.as_str()))
            .collect::<Vec<_>>();
        let active_action_ids = actions
            .iter()
            .filter(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"))
            .map(|action| action.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(projected_action_ids.len(), active_action_ids.len());
        for action_id in active_action_ids {
            assert_eq!(
                projected_action_ids
                    .iter()
                    .filter(|projected_id| **projected_id == action_id)
                    .count(),
                1,
                "active action {action_id} should appear exactly once"
            );
        }
        assert_eq!(
            explanation
                .phases
                .iter()
                .find(|phase| phase.id == "unclassified_remediation")
                .expect("unknown domains must stay visible")
                .steps[0]
                .action_id,
            "unknown"
        );
    }

#[test]
    fn clean_only_goal_does_not_inherit_universal_quality_or_maturity_work() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-clean-goal-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal fixture should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": REMEDIATION_GOAL_SCHEMA,
                "target_state": "clean_only",
                "reason": "This repository only needs clean preservation."
            }))
            .expect("goal fixture should encode"),
        )
        .expect("goal contract should be writable");
        let mut repository = fixture_repository("clean-only");
        repository.path = root.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-clean"),
            "2026-07-29T12:00:00Z",
            None,
        );

        assert_eq!(plan.goal.target_state, "clean_only");
        assert!(plan.goal.maturity_policy.is_none());
        assert!(plan.actions.iter().all(|action| !matches!(
            action.domain.as_str(),
            "evidence_refresh" | "ci_ideal" | "maturity"
        )));
        fs::remove_dir_all(root).expect("goal fixture should be removable");
    }

#[test]
    fn installed_runtime_drift_creates_one_causal_reconciliation_action() {
        let mut repository = fixture_repository("runtime-drift");
        repository.quality.installed_runtime = crate::installed_runtime::InstalledRuntimeSnapshot {
            applicability: "applicable".to_string(),
            status: "attention_required".to_string(),
            summary: "One runtime target needs a restart.".to_string(),
            config_path: Some(".pronto/installed-runtime-parity.json".to_string()),
            targets: vec![crate::installed_runtime::InstalledRuntimeTargetSnapshot {
                id: "daemon".to_string(),
                label: "Daemon".to_string(),
                status: "restart_required".to_string(),
                source_revision: Some(String::from("a").repeat(40)),
                build_revision: Some(String::from("a").repeat(40)),
                process_id: Some(42),
                observed_at: Some("2026-08-14T05:00:00Z".to_string()),
                issues: vec![crate::installed_runtime::InstalledRuntimeIssue {
                    stage: "runtime".to_string(),
                    status: "restart_required".to_string(),
                    message: "Running process differs from install.".to_string(),
                }],
            }],
            ..crate::installed_runtime::InstalledRuntimeSnapshot::default()
        };
        let mut seeds = Vec::new();

        add_installed_runtime_seed(&repository, &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(
            seeds[0].stable_key,
            "repository_health:installed-runtime-parity"
        );
        assert!(seeds[0].evidence[0].detail.contains("restart_required"));
    }

#[test]
    fn missing_goal_contract_stays_visible_as_confirmation_work() {
        let repository = fixture_repository("inferred-goal");

        let run = rebuild_run(&[repository], &empty_run(), Some("refresh-goal"));
        let plan = &run.plans[0];

        assert_eq!(plan.goal.source, "inferred");
        assert!(plan
            .actions
            .iter()
            .any(|action| action.stable_key == "scope:confirm-remediation-goal"));
    }

#[test]
    fn github_only_candidates_are_counted_with_a_terminal_task_label() {
        let mut run = empty_run();
        let remote_repositories = vec![
            RemoteRepositorySnapshot {
                id: "github:42".to_string(),
                provider: "github".to_string(),
                full_name: "example/kept-online".to_string(),
                name: "kept-online".to_string(),
                owner: "example".to_string(),
                html_url: "https://github.com/example/kept-online".to_string(),
                default_branch: Some("main".to_string()),
                archived: false,
                locality: GITHUB_ONLY_LOCALITY.to_string(),
                identity_id: "github:jakyeamos".to_string(),
                last_refreshed_at: "2026-08-04T12:00:00Z".to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
                ci_runs: Vec::new(),
            },
            RemoteRepositorySnapshot {
                id: "github:43".to_string(),
                provider: "github".to_string(),
                full_name: "example/local-match".to_string(),
                name: "local-match".to_string(),
                owner: "example".to_string(),
                html_url: "https://github.com/example/local-match".to_string(),
                default_branch: Some("main".to_string()),
                archived: false,
                locality: "Local and remote".to_string(),
                identity_id: "github:jakyeamos".to_string(),
                last_refreshed_at: "2026-08-04T12:00:00Z".to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
                ci_runs: Vec::new(),
            },
        ];

        sync_github_only_candidates(&mut run, &remote_repositories);

        assert_eq!(run.github_only_candidates.len(), 1);
        assert_eq!(
            run.github_only_candidates[0].full_name,
            "example/kept-online"
        );
        assert_eq!(run.github_only_candidates[0].label, GITHUB_ONLY_LOCALITY);
        assert_eq!(
            run.github_only_candidates[0].last_remediation_task,
            GITHUB_ONLY_REMEDIATION_TASK
        );
        assert_eq!(run.github_only_candidates[0].status, "candidate");
    }

#[test]
    fn github_only_goal_ends_the_local_plan_with_the_github_only_task() {
        let mut repository = fixture_repository("kept-online");
        repository.lifecycle = GITHUB_ONLY_LOCALITY.to_string();
        repository.lifecycle_candidate = GITHUB_ONLY_LOCALITY.to_string();

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-github-only"),
            "2026-08-04T12:00:00Z",
            None,
        );

        assert_eq!(plan.goal.target_state, "github_only");
        assert_eq!(
            plan.actions.last().map(|action| action.title.as_str()),
            Some(GITHUB_ONLY_REMEDIATION_TASK)
        );
        assert_eq!(
            plan.actions.last().map(|action| action.stable_key.as_str()),
            Some(GITHUB_ONLY_VERIFICATION_ACTION_KEY)
        );
        assert_eq!(
            plan.actions.last().map(|action| action.domain.as_str()),
            Some("verification")
        );
    }
