#[test]
    fn action_reopens_when_a_refresh_resolved_key_reappears() {
        let repository = fixture_repository("recurrence");
        let mut previous_action = fixture_action(
            "project_compass:blockers",
            "product_truth",
            "P1",
            3,
            "verified",
        );
        previous_action.evidence.push(evidence(
            "Pronto remediation",
            RESOLVED_BY_REFRESH_LABEL,
            "Resolved",
            "Fresh",
            Some("2026-07-30T12:00:00Z"),
            None,
            "Previously absent from the refreshed projection.",
        ));
        let previous_actions =
            HashMap::from([(previous_action.stable_key.as_str(), &previous_action)]);
        let seed = ActionSeed {
            stable_key: previous_action.stable_key.clone(),
            domain: previous_action.domain.clone(),
            title: previous_action.title.clone(),
            summary: previous_action.summary.clone(),
            severity: previous_action.severity.clone(),
            priority: previous_action.priority.clone(),
            weight: previous_action.weight,
            acceptance_criteria: previous_action.acceptance_criteria.clone(),
            evidence: vec![evidence(
                "Project Compass",
                "Open blockers",
                "Blocked",
                "Fresh",
                Some("2026-07-31T12:00:00Z"),
                None,
                "The blocker is present again.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        };

        let action =
            materialize_action(&repository, seed, &previous_actions, "2026-07-31T12:00:00Z");

        assert_eq!(action.status, "open");
    }

#[test]
    fn integration_only_requires_no_other_active_or_blocked_remediation() {
        let integration = fixture_action(
            "branch_hygiene:integrate:feature",
            "branch_hygiene",
            "P2",
            2,
            "open",
        );
        let verification = fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open");
        assert!(integration_only_remaining(&[
            integration.clone(),
            verification.clone(),
        ]));

        let maturity = fixture_action("maturity:minimum", "maturity", "P2", 2, "open");
        assert!(!integration_only_remaining(&[
            integration.clone(),
            verification.clone(),
            maturity,
        ]));

        let mut blocked_integration = integration;
        blocked_integration.status = "blocked".to_string();
        assert!(!integration_only_remaining(&[
            blocked_integration,
            verification,
        ]));
    }

#[test]
    fn severity_weights_are_deterministic() {
        assert_eq!(severity_weight("critical"), 4);
        assert_eq!(severity_weight("error"), 3);
        assert_eq!(severity_weight("warning"), 2);
        assert_eq!(severity_weight("observation"), 1);
    }

#[test]
    fn fleet_qr_artifact_supplies_findings_but_not_private_maturity_to_plan() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-remediation-fleet-{fixture_id}"));
        let repository_path = root.join("repo");
        let findings_dir = root.join("findings");
        fs::create_dir_all(&repository_path).expect("repository fixture should be writable");
        fs::create_dir_all(&findings_dir).expect("findings fixture should be writable");
        let observed_at = Utc::now().to_rfc3339();
        let mut repository = fixture_repository("fleet-repo");
        repository.path = repository_path.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();
        repository.workspace.branch = "dev".to_string();
        repository.branch = "dev".to_string();
        repository.workspace.last_commit = Some("abc".to_string());
        let finding_path = findings_dir.join("repo.json");
        fs::write(
            &finding_path,
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path.clone(),
                    "checkouts": [{"path": repository.workspace.path.clone(), "head": "abc", "branch": "dev"}]
                },
                "findings": [
                    {
                        "applicable": true,
                        "dimension": "quality_commands",
                        "finding_id": "finding-quality-commands",
                        "label": "quality commands",
                        "message": "Quality commands are not fully legible.",
                        "priority": "P1",
                        "score": 2,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "validation_commands": ["pnpm test"]
                    },
                    {
                        "applicable": false,
                        "dimension": "deployment_rollback",
                        "finding_id": "finding-deployment",
                        "score": null,
                        "severity": "observation"
                    },
                    {
                        "applicable": true,
                        "dimension": "context_routing",
                        "finding_id": "finding-context-routing",
                        "score": 3,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "validated"
                    },
                    {
                        "applicable": true,
                        "dimension": "change_surface_coverage",
                        "finding_id": "finding-change-surface-coverage",
                        "score": 4,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "maintained"
                    }
                ]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let run = rebuild_run_with_fleet_root(
            &[repository],
            &empty_run(),
            Some("refresh-fleet"),
            Some(&root),
        );
        let plan = &run.plans[0];
        assert!(plan
            .actions
            .iter()
            .any(|action| action.stable_key.starts_with("qr_findings:group:")));
        assert!(plan.actions.iter().any(|action| {
            action.stable_key == "maturity:score"
                && action
                    .summary
                    .contains("No repository maturity score is available")
        }));
        assert!(!plan
            .actions
            .iter()
            .any(|action| action.stable_key.starts_with("maturity:dimension:")));
        assert!(!plan
            .actions
            .iter()
            .any(|action| action.stable_key == "evidence_refresh:qr-run"));
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.domain == "qr_findings")
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("fleet fixture should be removable");
    }

#[test]
    fn canonical_maturity_projection_owns_below_target_fleet_dimension_action() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-remediation-fleet-maturity-owner-{fixture_id}"
        ));
        let repository_path = root.join("repo");
        let findings_dir = root.join("findings");
        fs::create_dir_all(&repository_path).expect("repository fixture should be writable");
        fs::create_dir_all(&findings_dir).expect("findings fixture should be writable");
        let observed_at = Utc::now().to_rfc3339();
        let mut repository = fixture_repository("fleet-maturity-owner");
        repository.path = repository_path.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();
        repository.workspace.last_commit = Some("abc".to_string());
        repository.quality.maturity.score = Some(2.5);
        repository.quality.maturity.score_display = Some("2.5/4".to_string());
        repository.quality.maturity.freshness = crate::quality::QualityFreshness::Fresh;
        repository.quality.maturity.observed_at = Some(observed_at.clone());
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("quality_commands".to_string(), 2.0);
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("matrix_maintenance".to_string(), 0.0);
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("long_running_task_observability".to_string(), 2.0);
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("long_running_task_optimization".to_string(), 0.0);
        fs::write(
            findings_dir.join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet-maturity-owner",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path.clone(),
                    "checkouts": [{"path": repository.workspace.path.clone(), "head": "abc", "branch": "dev"}]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands are not fully legible.",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }, {
                    "applicable": true,
                    "dimension": "matrix_maintenance",
                    "finding_id": "finding-matrix-maintenance",
                    "label": "change-matrix maintenance",
                    "message": "The repository matrix does not require same-change updates.",
                    "score": 0,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation",
                    "status": "missing"
                }]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let run = rebuild_run_with_fleet_root(
            &[repository],
            &empty_run(),
            Some("refresh-fleet-maturity-owner"),
            Some(&root),
        );
        let plan = &run.plans[0];

        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.domain == "qr_findings")
                .count(),
            0
        );
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.stable_key == "maturity:dimension:quality_commands")
                .count(),
            1
        );
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.stable_key == "maturity:dimension:matrix_maintenance")
                .count(),
            1
        );
        let observability = plan
            .actions
            .iter()
            .find(|action| {
                action.stable_key == "maturity:dimension:long_running_task_observability"
            })
            .expect("observability gap should become deferred maturity work");
        assert_eq!(
            observability.title,
            "Add agent-readable progress to long-running tasks"
        );
        assert_eq!(observability.priority, "P2");
        assert_eq!(observability.severity, "maturity");
        let optimization = plan
            .actions
            .iter()
            .find(|action| action.stable_key == "maturity:dimension:long_running_task_optimization")
            .expect("optimization gap should become deferred maturity work");
        assert_eq!(
            optimization.title,
            "Review long-running tasks for avoidable repeated work"
        );
        assert_eq!(optimization.priority, "P2");
        assert_eq!(optimization.severity, "maturity");
        fs::remove_dir_all(root).expect("fleet fixture should be removable");
    }
