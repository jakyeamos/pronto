#[test]
    fn prepares_pull_request_and_release_evidence_deterministically() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let initial_commit =
            String::from_utf8_lossy(&git(&repository, &["rev-parse", "main"]).stdout)
                .trim()
                .to_string();
        git(&repository, &["switch", "-c", "feature/release-preview"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "feat: add release preview"]);
        fs::write(repository.join("feature.txt"), "feature\nfix\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(
            &repository,
            &["commit", "-m", "fix: correct release preview"],
        );
        fs::write(repository.join("breaking.txt"), "breaking\n")
            .expect("breaking file should be writable");
        git(&repository, &["add", "breaking.txt"]);
        git(
            &repository,
            &["commit", "-m", "feat!: change release contract"],
        );

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
            .expect("release fixture should persist");
        let mut persisted = load_store(&database).expect("release fixture should reload");
        let (repository_id, workspace_id) = {
            let stored_repository = persisted
                .repositories
                .first_mut()
                .expect("fixture repository should be registered");
            stored_repository.provider_state = "GitHub connected as github:fixture".to_string();
            stored_repository.quality.release_boundary.status = "Passed".to_string();
            stored_repository.quality.release_boundary.freshness = "Fresh".to_string();
            stored_repository
                .quality
                .release_boundary
                .blocking_check_ids
                .clear();
            stored_repository.quality.behavior_assurance.contract_status = "current".to_string();
            stored_repository.quality.behavior_assurance.result_status = "passed".to_string();
            stored_repository.quality.behavior_assurance.freshness = "current".to_string();
            stored_repository.quality.behavior_assurance.release_ready = true;
            stored_repository
                .quality
                .behavior_assurance
                .required_scenario_count = 1;
            stored_repository
                .quality
                .behavior_assurance
                .passed_scenario_count = 1;
            stored_repository.quality.behavior_assurance.gaps.clear();
            stored_repository.quality.release_boundary.checks = [
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
            stored_repository.releases = vec![ReleaseSnapshot {
                id: "github:release-1".to_string(),
                provider: "github".to_string(),
                repository_id: "github:repo-1".to_string(),
                tag: "v1.2.3".to_string(),
                name: "v1.2.3".to_string(),
                target_commit: Some(initial_commit),
                published_at: Some("2026-07-25T12:00:00Z".to_string()),
                draft: false,
                prerelease: false,
                last_refreshed_at: "2026-07-26T12:00:00Z".to_string(),
            }];
            stored_repository.pull_requests = vec![PullRequestSnapshot {
                id: "github:pr-1".to_string(),
                provider: "github".to_string(),
                repository_id: "github:repo-1".to_string(),
                number: 7,
                html_url: "https://github.com/fixture/repository/pull/7".to_string(),
                title: "Release preview".to_string(),
                head_branch: "feature/release-preview".to_string(),
                base_branch: "main".to_string(),
                state: "OPEN".to_string(),
                draft: true,
                checks_state: "Pending".to_string(),
                reviews_state: "Required review unavailable".to_string(),
                mergeability: "Unknown — provider snapshot unavailable".to_string(),
                checks: Vec::new(),
                last_refreshed_at: "2026-07-26T12:00:00Z".to_string(),
                head_commit: None,
            }];
            (
                stored_repository.id.clone(),
                stored_repository.workspace.id.clone(),
            )
        };
        persisted.provider_status = ProviderStatus {
            provider: "GitHub".to_string(),
            state: "Ready".to_string(),
            message: "fixture".to_string(),
            last_refresh_at: Some("2026-07-26T12:00:00Z".to_string()),
            identity_count: 1,
            repository_count: 1,
        };
        save_store(&database, &persisted).expect("release evidence should persist");

        let preparation = prepare_repository_at(&database, &repository_id, Some(&workspace_id))
            .expect("preparation should be deterministic");
        assert_eq!(preparation.pull_request.status, "Evidence ready");
        assert_eq!(
            preparation.pull_request.base_branch.as_deref(),
            Some("main")
        );
        assert_eq!(preparation.pull_request.commit_count, 3);
        assert_eq!(preparation.pull_request.checks_state, "Pending");
        assert_eq!(
            preparation
                .pull_request
                .existing_pull_request
                .as_ref()
                .map(|pull_request| pull_request.number),
            Some(7)
        );
        assert_eq!(
            preparation.release.baseline_status,
            "Published release baseline"
        );
        assert_eq!(preparation.release.commits_since_baseline.len(), 3);
        assert_eq!(preparation.release.candidate_bump.as_deref(), Some("major"));
        assert_eq!(
            preparation.release.candidate_version.as_deref(),
            Some("v2.0.0")
        );
        assert_eq!(
            preparation.release.rule_status,
            "Not configured — commits are shown without threshold evaluation"
        );
        assert_eq!(
            preparation
                .release
                .notes
                .iter()
                .map(|section| section.category.as_str())
                .collect::<Vec<_>>(),
            vec!["Breaking", "Features", "Fixes"]
        );
        let configured_snapshot = set_release_rule_at(
            &database,
            &repository_id,
            Some(ReleaseRuleConfig {
                name: "Two meaningful commits".to_string(),
                operator: "and".to_string(),
                min_commits: Some(2),
                min_elapsed_days: None,
                required_commit_types: vec!["FEAT".to_string()],
                allow_first_release: false,
                required_quality_gates: Vec::new(),
            }),
        )
        .expect("release rule should persist");
        assert_eq!(
            configured_snapshot.repositories[0]
                .release_rule
                .as_ref()
                .map(|rule| rule.operator.as_str()),
            Some("AND")
        );
        assert!(configured_snapshot.repositories[0]
            .conditions
            .iter()
            .any(|condition| condition.title == "Configured release threshold met"));
        let configured_preparation =
            prepare_repository_at(&database, &repository_id, Some(&workspace_id))
                .expect("configured preparation should be deterministic");
        assert_eq!(
            configured_preparation.release.rule_status,
            "Configured release threshold met"
        );
        assert_eq!(configured_preparation.release.rule_trace.len(), 3);
        assert!(configured_preparation
            .release
            .rule_trace
            .iter()
            .all(|trace| trace.status == "Passed"));
        assert_eq!(configured_preparation.recipe.status, "Blocked");
        assert_eq!(configured_preparation.recipe.steps[2].status, "Blocked");
        let mismatched_version =
            set_release_version_at(&database, &repository_id, Some("9.9.9".to_string()))
                .expect_err("stale version confirmations should be rejected");
        assert_eq!(
            mismatched_version,
            "Release version must match the current deterministic candidate"
        );
        let invalid_recipe = set_release_recipe_at(
            &database,
            &repository_id,
            Some(ReleaseRecipeConfig {
                name: "Unsafe recipe".to_string(),
                validation_commands: vec!["pnpm test\nrm -rf .".to_string()],
                release_commands: Vec::new(),
                generated_paths: vec!["../outside.txt".to_string()],
                commit_message: "chore(release): prepare {version}".to_string(),
            }),
        )
        .expect_err("unsafe recipe paths and commands should be rejected");
        assert!(invalid_recipe.contains("line breaks"));
        let confirmed_snapshot =
            set_release_version_at(&database, &repository_id, Some("2.0.0".to_string()))
                .expect("candidate version should require and accept explicit confirmation");
        assert_eq!(
            confirmed_snapshot.repositories[0]
                .confirmed_release_version
                .as_deref(),
            Some("v2.0.0")
        );
        set_release_recipe_at(
            &database,
            &repository_id,
            Some(ReleaseRecipeConfig {
                name: "Fixture release".to_string(),
                validation_commands: vec!["pnpm test".to_string()],
                release_commands: vec!["pnpm release:version".to_string()],
                generated_paths: vec!["CHANGELOG.md".to_string()],
                commit_message: "chore(release): prepare {version}".to_string(),
            }),
        )
        .expect("release recipe should persist");
        let ready_recipe_preparation =
            prepare_repository_at(&database, &repository_id, Some(&workspace_id))
                .expect("configured recipe should be readable");
        assert_eq!(
            ready_recipe_preparation.release.version_status,
            "Candidate version confirmed"
        );
        assert_eq!(
            ready_recipe_preparation.recipe.status,
            "Ready for user review"
        );
        assert!(!ready_recipe_preparation.recipe.actions_performed);
        assert_eq!(ready_recipe_preparation.recipe.steps.len(), 9);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn previews_only_allowed_committed_ai_payload_evidence() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/ai-preview"]);
        fs::write(repository.join("committed.txt"), "committed evidence\n")
            .expect("committed file should be writable");
        git(&repository, &["add", "committed.txt"]);
        git(
            &repository,
            &["commit", "-m", "feat: add committed evidence"],
        );
        fs::write(
            repository.join("uncommitted.txt"),
            "private uncommitted evidence\n",
        )
        .expect("uncommitted file should be writable");

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
            .expect("AI preview fixture should persist");
        let mut persisted = load_store(&database).expect("AI preview fixture should reload");
        let (repository_id, workspace_id) = {
            let stored_repository = persisted
                .repositories
                .first_mut()
                .expect("AI preview repository should be registered");
            stored_repository.workspace.dirty = true;
            (
                stored_repository.id.clone(),
                stored_repository.workspace.id.clone(),
            )
        };
        save_store(&database, &persisted).expect("AI preview state should persist");

        let disabled = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("disabled AI preview should be readable");
        assert_eq!(disabled.permission, "Disabled");
        assert_eq!(disabled.status, "AI disabled by repository policy");
        assert!(disabled.payload_text.is_empty());
        assert!(!disabled.request_performed);
        assert!(!disabled.uncommitted_included);

        set_ai_permission_at(&database, &repository_id, "Commit metadata only")
            .expect("metadata permission should persist");
        let metadata = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("metadata preview should be readable");
        assert_eq!(metadata.status, "Payload ready for user inspection");
        assert_eq!(metadata.source_references.len(), 1);
        assert_eq!(metadata.categories.len(), 1);
        assert!(metadata
            .payload_text
            .contains("feat: add committed evidence"));
        assert!(!metadata.payload_text.contains("uncommitted.txt"));
        assert!(metadata
            .reasons
            .iter()
            .any(|reason| reason.contains("Uncommitted changes are excluded")));

        set_ai_permission_at(&database, &repository_id, "Committed diff allowed")
            .expect("diff permission should persist");
        let diff = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("diff preview should be readable");
        assert_eq!(diff.categories.len(), 2);
        assert!(diff.payload_text.contains("committed.txt"));
        assert!(!diff.payload_text.contains("uncommitted.txt"));
        assert!(!diff.request_performed);
        assert!(!diff.uncommitted_included);
        fs::remove_dir_all(root).expect("AI preview fixture should be removable");
    }

#[test]
    fn parses_github_repository_pages_into_remote_snapshots() {
        let payload = serde_json::json!([
            [{
                "id": 42,
                "full_name": "Acme/Portfolio",
                "name": "Portfolio",
                "owner": {"login": "Acme"},
                "html_url": "https://github.com/Acme/Portfolio",
                "default_branch": "main",
                "archived": false
            }],
            [{
                "full_name": "Acme/Archive",
                "html_url": "https://github.com/Acme/Archive",
                "archived": true
            }]
        ]);
        let repositories =
            parse_github_repositories(&payload, "github:jakyeamos", "2026-07-25T12:00:00Z")
                .expect("GitHub pages should parse");

        assert_eq!(repositories.len(), 2);
        assert_eq!(repositories[0].id, "github:42");
        assert_eq!(repositories[0].full_name, "Acme/Portfolio");
        assert_eq!(repositories[0].owner, "Acme");
        assert_eq!(repositories[0].locality, "Remote only");
        assert_eq!(repositories[1].name, "Archive");
        assert!(repositories[1].archived);
    }
