#[test]
    fn plan_covers_every_repo_level_ui_surface_with_linked_gap_actions() {
        let mut repository = fixture_repository("ui-coverage");
        repository.workspace.dirty = true;
        repository.workspace.sync_state = "Ahead by 1".to_string();
        repository.workspace.remote_freshness = "Not fetched by Pronto".to_string();
        repository.workspace.operation = Some("rebase".to_string());
        repository.workspace.integration_state = "Integration eligible".to_string();
        repository.workspace.activity.state = "Active".to_string();
        repository.branches.push(BranchSummary {
            name: "feature/coverage".to_string(),
            role: "Feature".to_string(),
            role_confidence: "High".to_string(),
            target_branch: Some("dev".to_string()),
            target_confidence: "High".to_string(),
            ahead: 1,
            behind: 0,
            integration_state: "Integration eligible".to_string(),
            workspace_id: None,
            last_commit: Some("def".to_string()),
            last_commit_at: Some(Utc::now().to_rfc3339()),
        });
        repository.submodules.push(SubmoduleSummary {
            path: "vendor/example".to_string(),
            commit: Some("123".to_string()),
            status: "Modified commit".to_string(),
        });
        repository.conditions.push(Condition {
            id: "condition-custom".to_string(),
            kind: "new-ui-condition".to_string(),
            title: "New UI condition".to_string(),
            summary: "A newly tracked condition needs a remediation disposition.".to_string(),
            priority: 1,
            status: "Active".to_string(),
            fingerprint: "condition-custom".to_string(),
            rule: "fixture".to_string(),
            evidence: Vec::new(),
            missing: Vec::new(),
            confidence: Some("High".to_string()),
            freshness: Some("Fresh".to_string()),
        });

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-coverage"),
            "2026-07-29T12:00:00Z",
            None,
        );

        assert_eq!(
            plan.coverage
                .iter()
                .map(|entry| entry.surface.as_str())
                .collect::<Vec<_>>(),
            UI_TRACKED_SURFACE_IDS
        );
        assert!(plan
            .coverage
            .iter()
            .filter(|entry| matches!(entry.status.as_str(), "attention" | "blocked"))
            .all(|entry| !entry.action_ids.is_empty()));
        for stable_key in [
            "product_truth:project-compass",
            "branch_hygiene:activity:workspace-ui-coverage",
            "branch_hygiene:operation:workspace-ui-coverage",
            "branch_hygiene:remote-freshness:workspace-ui-coverage",
            "branch_hygiene:integrate:feature/coverage",
            "repository_health:submodule:vendor/example",
            "repository_health:condition:condition-custom",
        ] {
            assert!(
                plan.actions
                    .iter()
                    .any(|action| action.stable_key == stable_key),
                "missing action {stable_key}"
            );
        }
    }

#[test]
    fn dirty_workspace_without_an_owner_does_not_create_an_ownership_blocker() {
        let mut repository = fixture_repository("dirty-without-owner");
        repository.workspace.dirty = true;
        repository.workspace.activity = WorkspaceActivity {
            state: "Interrupted with dirty work".to_string(),
            confidence: "Medium".to_string(),
            signals: vec![ActivitySignal {
                source: "Process".to_string(),
                summary: "No associated process detected".to_string(),
                confidence: "Medium".to_string(),
                observed_at: Utc::now().to_rfc3339(),
                process_name: None,
                process_id: None,
                started_at: None,
                working_directory: None,
            }],
            manifest: None,
        };

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-dirty"),
            "2026-07-29T18:00:00Z",
            None,
        );

        assert!(plan.actions.iter().any(|action| {
            action.stable_key == "branch_hygiene:dirty:workspace-dirty-without-owner"
        }));
        assert!(!plan.actions.iter().any(|action| {
            action.stable_key == "branch_hygiene:activity:workspace-dirty-without-owner"
        }));
    }

#[test]
    fn verified_and_deferred_actions_form_a_terminal_deferred_plan() {
        let actions = vec![
            fixture_action("verified", "qr_findings", "P1", 3, "verified"),
            fixture_action("deferred", "maturity", "P2", 2, "deferred"),
        ];

        assert_eq!(plan_status(&actions), "deferred");
        assert_eq!(current_stage(&actions), "complete");
    }

#[test]
    fn markdown_export_separates_active_queue_from_resolved_action_history() {
        let mut run = empty_run();
        run.generated_at = "2026-07-29T13:00:00Z".to_string();
        run.plans = vec![fixture_plan(
            "active-repo",
            "open",
            vec![fixture_action(
                "preserve",
                "branch_hygiene",
                "P1",
                2,
                "open",
            )],
        )];
        run.closures = vec![closure_from_plan(
            &fixture_plan(
                "closed-repo",
                "complete",
                vec![fixture_action(
                    "verified",
                    "verification",
                    "P2",
                    1,
                    "verified",
                )],
            ),
            "2026-07-29T13:00:00Z",
            Some("refresh-1"),
        )];

        let markdown = render_active_queue_markdown(&run);

        assert!(markdown.contains(
            "| 1 | `active-repo` | Active maintained repository | repository_contract | open |"
        ));
        assert!(markdown.contains("| Remaining path |"));
        assert!(markdown.contains("Preserve and reconcile repository work"));
        assert!(markdown.contains("## Resolved action history"));
        assert!(markdown.contains("| Resolved at |"));
        assert!(markdown
            .contains("| `closed-repo` | active_maintained | repository_contract | verified |"));
        assert!(markdown.contains("GitHub-only candidates: **0**"));
        assert!(markdown.contains("## GitHub-only candidates"));
        assert!(!markdown.contains("## Closure ledger"));
        assert!(!markdown.contains("| 2 | `closed-repo` |"));
    }

#[test]
    fn remediation_export_writes_markdown_and_resolved_action_history_data() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let output_dir =
            std::env::temp_dir().join(format!("pronto-remediation-export-{fixture_id}"));
        let mut run = empty_run();
        run.id = "run-export".to_string();
        run.generated_at = "2026-07-29T13:00:00Z".to_string();
        run.plans = vec![fixture_plan(
            "active-repo",
            "open",
            vec![fixture_action(
                "preserve",
                "branch_hygiene",
                "P1",
                2,
                "open",
            )],
        )];
        run.closures = vec![closure_from_plan(
            &fixture_plan(
                "closed-repo",
                "complete",
                vec![fixture_action(
                    "verified",
                    "verification",
                    "P2",
                    1,
                    "verified",
                )],
            ),
            "2026-07-29T13:00:00Z",
            Some("refresh-1"),
        )];

        let exported = export_run(&run, &output_dir).expect("queue export should succeed");

        assert!(exported
            .files
            .iter()
            .any(|path| path.ends_with("repository-remediation-order.md")));
        assert!(exported
            .files
            .iter()
            .any(|path| path.ends_with("remediation-closures.json")));
        let markdown = fs::read_to_string(output_dir.join("repository-remediation-order.md"))
            .expect("markdown queue should be readable");
        assert!(markdown.contains("| 1 | `active-repo` | Active maintained repository |"));
        assert!(markdown
            .contains("| `closed-repo` | active_maintained | repository_contract | verified |"));

        fs::remove_dir_all(output_dir).expect("export fixture should be removable");
    }

#[test]
    fn progress_excludes_deferred_weight() {
        let actions = vec![
            RemediationAction {
                id: "a".to_string(),
                stable_key: "a".to_string(),
                repository_id: "repo".to_string(),
                domain: "qr_findings".to_string(),
                title: "A".to_string(),
                summary: String::new(),
                severity: "high".to_string(),
                priority: "P1".to_string(),
                weight: 3,
                status: "verified".to_string(),
                acceptance_criteria: Vec::new(),
                evidence: Vec::new(),
                related_finding_ids: Vec::new(),
                source_run_id: None,
                updated_at: String::new(),
                completed_at: None,
                notes: None,
            },
            RemediationAction {
                id: "b".to_string(),
                stable_key: "b".to_string(),
                repository_id: "repo".to_string(),
                domain: "maturity".to_string(),
                title: "B".to_string(),
                summary: String::new(),
                severity: "maturity".to_string(),
                priority: "P2".to_string(),
                weight: 4,
                status: "deferred".to_string(),
                acceptance_criteria: Vec::new(),
                evidence: Vec::new(),
                related_finding_ids: Vec::new(),
                source_run_id: None,
                updated_at: String::new(),
                completed_at: None,
                notes: None,
            },
        ];
        let progress = calculate_progress(&actions);
        assert_eq!(progress.total_weight, 3);
        assert_eq!(progress.deferred_weight, 4);
        assert_eq!(progress.percentage, 100.0);
    }

#[test]
    fn progress_reserves_one_hundred_percent_for_terminal_plans() {
        let actions = vec![
            fixture_action("verified", "qr_findings", "P1", 199, "verified"),
            fixture_action("remaining", "verification", "P2", 1, "open"),
        ];

        let progress = calculate_progress(&actions);

        assert_eq!(progress.verified_weight, 199);
        assert_eq!(progress.total_weight, 200);
        assert_eq!(progress.percentage, 99.0);
        assert_eq!(
            calculate_progress(&[fixture_action(
                "verified-only",
                "verification",
                "P2",
                1,
                "verified",
            )])
            .percentage,
            100.0
        );
    }

#[test]
    fn refresh_retains_disappeared_actions_as_verified_progress() {
        let previous = fixture_plan(
            "retained-progress",
            "open",
            vec![
                fixture_action("resolved", "product_truth", "P1", 3, "open"),
                fixture_action("remaining", "maturity", "P2", 2, "open"),
                fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
            ],
        );
        let mut actions = vec![
            fixture_action("remaining", "maturity", "P2", 2, "open"),
            fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
        ];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-07-30T12:00:00Z");

        let resolved = actions
            .iter()
            .find(|action| action.stable_key == "resolved")
            .expect("the disappeared action should remain in the plan");
        assert_eq!(resolved.status, "verified");
        assert_eq!(
            resolved.completed_at.as_deref(),
            Some("2026-07-30T12:00:00Z")
        );
        assert!(action_was_resolved_by_refresh(resolved));
        assert_eq!(
            actions
                .iter()
                .filter(|action| action.stable_key == VERIFICATION_ACTION_KEY)
                .count(),
            1
        );
        let progress = calculate_progress(&actions);
        assert_eq!(progress.verified_weight, 3);
        assert_eq!(progress.total_weight, 6);
        assert_eq!(progress.percentage, 50.0);
    }

#[test]
    fn grouped_project_compass_action_replaces_legacy_duplicate_actions() {
        let previous = fixture_plan(
            "compass-migration",
            "blocked",
            vec![
                fixture_action(
                    LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS[0],
                    "product_truth",
                    "P1",
                    2,
                    "open",
                ),
                fixture_action(
                    LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS[1],
                    "product_truth",
                    "P1",
                    2,
                    "open",
                ),
            ],
        );
        let mut actions = vec![fixture_action(
            PROJECT_COMPASS_OPEN_ITEMS_KEY,
            "product_truth",
            "P1",
            severity_weight("warning"),
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-03T18:04:36Z");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].stable_key, PROJECT_COMPASS_OPEN_ITEMS_KEY);
        assert!(!actions.iter().any(|action| {
            LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS.contains(&action.stable_key.as_str())
        }));
    }

#[test]
    fn debloat_group_migration_preserves_state_without_legacy_history_churn() {
        let repository = fixture_repository("debloat-key-migration");
        let legacy_key = format!("{LEGACY_DEBLOAT_GROUP_KEY_PREFIX}unknown");
        let current_key = format!("{DEBLOAT_GROUP_KEY_PREFIX}unknown");
        let mut legacy_action = fixture_action(
            &legacy_key,
            "qr_findings",
            "P2",
            severity_weight("warning"),
            "in_progress",
        );
        legacy_action.notes = Some("Owner review has started.".to_string());
        let previous = fixture_plan("debloat-key-migration", "open", vec![legacy_action.clone()]);
        let previous_actions = HashMap::from([(legacy_action.stable_key.as_str(), &legacy_action)]);
        let seed = ActionSeed {
            stable_key: current_key.clone(),
            domain: "qr_findings".to_string(),
            title: "Review oversized source files".to_string(),
            summary: "Review the current debloat candidates.".to_string(),
            severity: "warning".to_string(),
            priority: "P2".to_string(),
            weight: severity_weight("warning"),
            acceptance_criteria: Vec::new(),
            evidence: Vec::new(),
            related_finding_ids: vec!["finding-debloat".to_string()],
            source_run_id: Some("qr-run".to_string()),
        };
        let mut actions = vec![materialize_action(
            &repository,
            seed,
            &previous_actions,
            "2026-08-04T03:00:00Z",
        )];

        assert_eq!(actions[0].stable_key, current_key);
        assert_eq!(actions[0].status, "in_progress");
        assert_eq!(
            actions[0].notes.as_deref(),
            Some("Owner review has started.")
        );

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-04T03:00:00Z");

        assert_eq!(actions.len(), 1);
        assert!(!actions.iter().any(|action| action.stable_key == legacy_key));
    }

#[test]
    fn maturity_actions_replace_legacy_fleet_maturity_qr_history() {
        let legacy_key = format!(
            "qr_findings:group:quality_commands|quality_commands|{FLEET_MATURITY_FINDING_PACK_PREFIX}v0.1"
        );
        let previous = fixture_plan(
            "fleet-maturity-migration",
            "blocked",
            vec![fixture_action(&legacy_key, "qr_findings", "P2", 2, "open")],
        );
        let mut actions = vec![fixture_action(
            "maturity:dimension:quality_commands",
            "maturity",
            "P2",
            2,
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-03T18:04:36Z");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].stable_key, "maturity:dimension:quality_commands");
        assert!(!actions.iter().any(|action| action.stable_key == legacy_key));
    }

#[test]
    fn refresh_normalizes_legacy_unbounded_qr_history_weight() {
        let mut legacy = fixture_action(
            "qr_findings:group:developer-experience:setup-path",
            "qr_findings",
            "P3",
            14_831,
            "open",
        );
        legacy.severity = "observation".to_string();
        let previous = fixture_plan("legacy-qr-history", "open", vec![legacy]);
        let mut actions = vec![fixture_action(
            VERIFICATION_ACTION_KEY,
            "verification",
            "P2",
            1,
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-07-30T12:00:00Z");

        let retained = actions
            .iter()
            .find(|action| action.stable_key.starts_with("qr_findings:group:"))
            .expect("the grouped finding should remain as history");
        assert_eq!(retained.status, "verified");
        assert_eq!(retained.weight, severity_weight("observation"));
        assert_eq!(calculate_progress(&actions).percentage, 50.0);
    }
