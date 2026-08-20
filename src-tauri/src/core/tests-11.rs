#[test]
    fn workspace_role_map_builds_canonical_roles_without_mutation_fields() {
        let entry = serde_json::json!({
            "repository_role": "production_product",
            "release_ref": "master",
            "integration_ref": "dev"
        });
        let policy = workspace_policy_for_role(
            "example-repository",
            entry.as_object().expect("role entry should be an object"),
        )
        .expect("complete production role should validate");
        assert_eq!(policy["schema_version"], "workspace-policy/v1");
        assert_eq!(policy["repository_role"], "production_product");
        assert_eq!(
            policy["canonical_workspaces"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(
            policy["retention_exceptions"].as_array().map(Vec::len),
            Some(0)
        );
    }

#[test]
    fn workspace_fleet_manifest_requires_exact_registered_coverage() {
        let state = StoreState::default();
        let role_map = serde_json::json!({
            "schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
            "repositories": [{
                "repository_id": "not-registered",
                "repository_role": "role_unresolved"
            }]
        });
        let error = workspace_fleet_manifest(&state, &role_map)
            .expect_err("extra role-map entries must fail closed");
        assert!(error.contains("extra"));
    }

#[test]
    fn workspace_policy_generation_plans_writes_and_requires_explicit_replace() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let snapshot = scan_repository(&repository, None, &[]);
        let repository_id = snapshot.id.clone();
        let state = StoreState {
            repositories: vec![snapshot],
            ..StoreState::default()
        };
        let supporting_role_map = serde_json::json!({
            "schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
            "repositories": [{
                "repository_id": repository_id,
                "repository_role": "supporting_project",
                "working_ref": "main"
            }]
        });
        let policy_path = repository.join(crate::custody::WORKSPACE_POLICY_RELATIVE_PATH);

        let (dry_run, blocked) =
            workspace_policy_generation(&state, &supporting_role_map, None, false, false)
                .expect("dry-run generation should produce a plan");
        assert!(!blocked);
        assert_eq!(dry_run["status"], "ready");
        assert_eq!(dry_run["read_only"], true);
        assert_eq!(dry_run["implementation_allowed"], false);
        assert_eq!(dry_run["counts"]["would_create"], 1);
        assert!(!policy_path.exists());

        let (written, blocked) =
            workspace_policy_generation(&state, &supporting_role_map, None, true, false)
                .expect("explicit write should create the policy");
        assert!(!blocked);
        assert_eq!(written["status"], "written");
        assert_eq!(written["counts"]["created"], 1);
        let saved: Value = serde_json::from_str(
            &fs::read_to_string(&policy_path).expect("generated policy should be readable"),
        )
        .expect("generated policy should remain JSON");
        assert_eq!(saved["repository_role"], "supporting_project");

        let production_role_map = serde_json::json!({
            "schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
            "repositories": [{
                "repository_id": state.repositories[0].id,
                "repository_role": "production_product",
                "release_ref": "main",
                "integration_ref": "dev"
            }]
        });
        let (conflict, blocked) =
            workspace_policy_generation(&state, &production_role_map, None, true, false)
                .expect("conflicting policy should be reported");
        assert!(blocked);
        assert_eq!(conflict["status"], "blocked");
        assert_eq!(conflict["counts"]["conflict"], 1);
        let unchanged: Value = serde_json::from_str(
            &fs::read_to_string(&policy_path).expect("conflicting policy must be preserved"),
        )
        .expect("preserved policy should remain JSON");
        assert_eq!(unchanged["repository_role"], "supporting_project");

        let (replaced, blocked) =
            workspace_policy_generation(&state, &production_role_map, None, true, true)
                .expect("explicit replacement should update the policy");
        assert!(!blocked);
        assert_eq!(replaced["counts"]["replaced"], 1);
        let updated: Value = serde_json::from_str(
            &fs::read_to_string(&policy_path).expect("replaced policy should be readable"),
        )
        .expect("replaced policy should remain JSON");
        assert_eq!(updated["repository_role"], "production_product");

        fs::remove_dir_all(root).expect("policy generation fixture should be removable");
    }

#[test]
    fn workspace_policy_generation_blocks_symlinked_agents_directory() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let outside = root.join("outside");
        fs::create_dir(&outside).expect("outside fixture should be creatable");
        symlink(&outside, repository.join(".agents")).expect("agents symlink should be creatable");
        let snapshot = scan_repository(&repository, None, &[]);
        let role_map = serde_json::json!({
            "schema_version": WORKSPACE_ROLE_MAP_SCHEMA,
            "repositories": [{
                "repository_id": snapshot.id,
                "repository_role": "supporting_project",
                "working_ref": "main"
            }]
        });
        let state = StoreState {
            repositories: vec![snapshot],
            ..StoreState::default()
        };

        let (report, blocked) = workspace_policy_generation(&state, &role_map, None, true, false)
            .expect("unsafe target should be reported");
        assert!(blocked);
        assert_eq!(report["status"], "blocked");
        assert_eq!(report["counts"]["blocked"], 1);
        assert!(!outside.join("workspace-policy.json").exists());

        fs::remove_file(repository.join(".agents")).expect("agents symlink should be removable");
        fs::remove_dir_all(root).expect("symlink fixture should be removable");
    }
