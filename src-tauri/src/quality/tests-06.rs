#[test]
    fn audit_import_matches_canonical_path_before_remote_identity() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit").join("audit-1");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-1","as_of":"2026-07-26T11:00:00Z","mean_maturity":1.933,"scored_dimension_count":10}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            format!(
                r#"{{"canonical_path":"{}","mean_maturity":4.0,"scored_dimension_count":10}}"#,
                repository_path.display()
            ),
        )
        .expect("finding should be writable");
        let repository = RepositorySnapshot {
            id: "repo-1".to_string(),
            name: "repo".to_string(),
            path: repository_path.to_string_lossy().to_string(),
            remote_url: Some("git@github.com:example/repo.git".to_string()),
            ..serde_json::from_value(serde_json::json!({
                "id":"repo-1","name":"repo","path":repository_path.to_string_lossy(),
                "locality":"Local","lifecycle":"Active","lifecycle_candidate":"Active",
                "provider_state":"Unknown","branch":"main","workspace":{
                    "id":"w","path":repository_path.to_string_lossy(),"is_primary":true,"branch":"main",
                    "dirty":false,"added":0,"removed":0,"line_totals_partial":false,"sync_state":"Synced",
                    "remote_freshness":"Unknown","ahead":0,"behind":0,"integration_state":"Unknown",
                    "target_branch":null,"target_confidence":"Unknown","role":"Primary","role_confidence":"High",
                    "activity":{"state":"Unknown","confidence":"Low","signals":[]}
                },"workspaces":[],"branches":[],"conditions":[],"last_scan_at":"2026-07-26T11:00:00Z",
                "last_fetch_at":null,"last_activity_at":null
            })).expect("repository fixture should decode")
        };
        let imported = audit_import(Some(&root.join("audit")), &[repository]);
        assert_eq!(
            imported.portfolio.maturity_score_display.as_deref(),
            Some("1.933")
        );
        assert_eq!(imported.portfolio.matched_repository_count, 1);
        assert_eq!(
            imported
                .maturities
                .values()
                .next()
                .and_then(|maturity| maturity.score_display.as_deref()),
            Some("4.0")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn audit_import_selects_latest_valid_run_and_does_not_guess_unmatched_repositories() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        let older = audit_root.join("older");
        let latest = audit_root.join("latest");
        let invalid = audit_root.join("invalid");
        for directory in [&older, &latest, &invalid] {
            fs::create_dir_all(directory.join("findings")).expect("audit should be writable");
        }
        fs::write(
            older.join("summary.json"),
            r#"{"audit_id":"older","as_of":"2026-07-20T11:00:00Z","mean_maturity":1.2}"#,
        )
        .expect("older summary should be writable");
        fs::write(
            latest.join("summary.json"),
            r#"{"audit_id":"latest","as_of":"2026-07-26T11:00:00Z","mean_maturity":1.933}"#,
        )
        .expect("latest summary should be writable");
        fs::write(
            invalid.join("summary.json"),
            r#"{"audit_id":"invalid","as_of":"not-a-timestamp","mean_maturity":4.0}"#,
        )
        .expect("invalid summary should be writable");
        fs::write(
            older.join("findings").join("repository.json"),
            format!(
                r#"{{"canonical_path":"{}","mean_maturity":1.2}}"#,
                repository_path.display()
            ),
        )
        .expect("older finding should be writable");

        let repository = fixture_repository(&repository_path);
        let imported = audit_import(Some(&audit_root), &[repository]);
        assert_eq!(
            imported.portfolio.latest_audit_id.as_deref(),
            Some("latest")
        );
        assert_eq!(
            imported.portfolio.maturity_score_display.as_deref(),
            Some("1.933")
        );
        assert_eq!(imported.portfolio.matched_repository_count, 0);
        assert!(imported.maturities.is_empty());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn composite_maturity_uses_pillars_and_excludes_product_progress() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        repository.quality.maturity.dimension_scores =
            BTreeMap::from([("source.one".to_string(), 4.0)]);
        repository.quality.ci_readiness.configuration_score = Some(4.0);
        repository.quality.ci_readiness.evidence_coverage_score = Some(2.0);
        repository.quality.ci_readiness.score = Some(0.0);
        repository.quality.mac_control_ideal_state.applicability = "Not applicable".to_string();
        let mut repositories = vec![repository];
        let mut portfolio = QualityPortfolioSnapshot {
            maturity_score: Some(2.0),
            maturity_score_display: Some("2.000".to_string()),
            scored_dimension_count: Some(10),
            ..QualityPortfolioSnapshot::default()
        };

        update_composite_maturity_summary(&mut portfolio, &mut repositories);

        assert_eq!(portfolio.source_maturity_score, Some(2.0));
        assert_eq!(portfolio.source_scored_dimension_count, Some(10));
        assert_eq!(portfolio.scored_dimension_count, Some(1));
        assert_eq!(portfolio.maturity_score, Some(0.0));
        assert_eq!(
            repositories[0]
                .quality
                .maturity
                .dimension_scores
                .get("ci.fresh_passing"),
            Some(&0.0)
        );
        assert!(!repositories[0]
            .quality
            .maturity
            .dimension_scores
            .contains_key("project_compass.mvp_progress"));
        let model = repositories[0]
            .quality
            .maturity
            .repository_maturity
            .as_ref()
            .expect("composite summary should expose the holistic model");
        assert_eq!(model.status, "blocked");
        assert!(model.critical_cap.applied);
        assert_eq!(model.evidence.assessed_pillar_count, 1);
        assert!(model
            .evidence
            .unmapped_dimensions
            .contains(&"source.one".to_string()));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn repository_maturity_caps_critical_blockers() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("quality_commands".to_string(), 4.0),
                ("security_constraints".to_string(), 4.0),
                ("architecture_boundaries".to_string(), 4.0),
                ("observability".to_string(), 4.0),
            ]),
            gaps: vec![QualityMaturityGap {
                dimension: "security_constraints".to_string(),
                status: "blocked".to_string(),
                score: Some(4.0),
                message: "Security verification is blocked.".to_string(),
            }],
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);

        assert_eq!(model.uncapped_score, Some(4.0));
        assert_eq!(model.score, Some(2.0));
        assert_eq!(model.status, "blocked");
        assert!(model.critical_cap.applied);
    }

#[test]
    fn repository_maturity_maps_long_running_task_dimensions_to_operability() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("long_running_task_observability".to_string(), 2.0),
                ("long_running_task_optimization".to_string(), 0.0),
            ]),
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);
        let operability = model
            .pillars
            .iter()
            .find(|pillar| pillar.id == "operability_release_safety")
            .expect("long-running task dimensions should map to operability");

        assert!(model.evidence.unmapped_dimensions.is_empty());
        assert_eq!(
            operability
                .dimension_scores
                .get("long_running_task_observability"),
            Some(&2.0)
        );
        assert_eq!(
            operability
                .dimension_scores
                .get("long_running_task_optimization"),
            Some(&0.0)
        );
    }

#[test]
    fn repository_maturity_keeps_conditional_applicability_explicit() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("quality_commands".to_string(), 4.0),
                ("security_constraints".to_string(), 4.0),
                ("architecture_boundaries".to_string(), 4.0),
                ("observability".to_string(), 4.0),
            ]),
            agent_usability: Some(AgentUsabilityMaturity {
                applicability: "not_applicable".to_string(),
                ..AgentUsabilityMaturity::default()
            }),
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);

        assert_eq!(model.score, Some(4.0));
        assert_eq!(model.status, "provisional");
        assert_eq!(model.pillars[4].applicability, "unknown");
        assert_eq!(model.pillars[5].applicability, "not_applicable");
        assert_eq!(model.pillars[6].applicability, "unknown");
        assert_eq!(model.evidence.assessed_pillar_count, 4);
        assert_eq!(model.evidence.assessed_weight, 0.228);
        assert_eq!(model.evidence.evidence_coverage, 0.309);
    }

#[test]
    fn repository_maturity_governs_runtime_resource_efficiency_as_user_facing_evidence() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("quality_commands".to_string(), 4.0),
                ("security_constraints".to_string(), 4.0),
                ("architecture_boundaries".to_string(), 4.0),
                ("observability".to_string(), 4.0),
                ("runtime_resource_efficiency".to_string(), 3.0),
            ]),
            gaps: vec![QualityMaturityGap {
                dimension: "runtime_resource_efficiency".to_string(),
                status: "attention".to_string(),
                score: Some(3.0),
                message: "Runtime outcomes pass, but regression enforcement is incomplete."
                    .to_string(),
            }],
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);
        let user_facing = model
            .pillars
            .iter()
            .find(|pillar| pillar.id == "user_facing_quality")
            .expect("runtime efficiency should make user-facing quality applicable");

        assert_eq!(user_facing.applicability, "applicable");
        assert_eq!(user_facing.status, "attention");
        assert_eq!(user_facing.score, Some(3.0));
        assert_eq!(
            user_facing
                .dimension_scores
                .get("runtime_resource_efficiency"),
            Some(&3.0)
        );
        assert!(!user_facing
            .missing_capabilities
            .contains(&"runtime_resource_efficiency".to_string()));
        assert!(!model.critical_cap.applied);
        assert!(!model
            .evidence
            .unmapped_dimensions
            .contains(&"runtime_resource_efficiency".to_string()));
    }

#[test]
    fn repository_maturity_reports_missing_runtime_resource_evidence() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([("performance".to_string(), 4.0)]),
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);
        let user_facing = model
            .pillars
            .iter()
            .find(|pillar| pillar.id == "user_facing_quality")
            .expect("performance should make user-facing quality applicable");

        assert_eq!(user_facing.score, Some(4.0));
        assert!(user_facing
            .missing_capabilities
            .contains(&"runtime_resource_efficiency".to_string()));
        assert_eq!(maturity_pillar_capability_coverage(user_facing), 0.25);
        assert_eq!(model.status, "provisional");
    }

#[test]
    fn repository_maturity_excludes_explicitly_inapplicable_runtime_evidence() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([("performance".to_string(), 4.0)]),
            gaps: vec![QualityMaturityGap {
                dimension: "runtime_resource_efficiency".to_string(),
                status: "not_applicable".to_string(),
                score: None,
                message: "This repository does not distribute or run an artifact.".to_string(),
            }],
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);
        let user_facing = model
            .pillars
            .iter()
            .find(|pillar| pillar.id == "user_facing_quality")
            .expect("performance should make user-facing quality applicable");

        assert!(user_facing
            .not_applicable_capabilities
            .contains(&"runtime_resource_efficiency".to_string()));
        assert!(!user_facing
            .missing_capabilities
            .contains(&"runtime_resource_efficiency".to_string()));
        assert_eq!(maturity_pillar_capability_coverage(user_facing), 1.0 / 3.0);
    }

#[test]
    fn safe_report_paths_reject_escape() {
        let root = fixture_root();
        let allowed = root.join("allowed");
        fs::create_dir_all(&allowed).expect("allowed root should be writable");
        let report = allowed.join("report.json");
        fs::write(&report, "{}").expect("report should be writable");
        assert!(safe_report_path(&report, std::slice::from_ref(&allowed)).is_ok());
        let sibling = root.join("allowed-escape");
        fs::create_dir_all(&sibling).expect("sibling should be writable");
        let sibling_report = sibling.join("report.json");
        fs::write(&sibling_report, "{}").expect("sibling report should be writable");
        assert!(safe_report_path(&sibling_report, std::slice::from_ref(&allowed)).is_err());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
