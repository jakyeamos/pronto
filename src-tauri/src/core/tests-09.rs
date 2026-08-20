#[test]
    fn completes_refresh_action_audit_after_read_only_scan() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
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

        let snapshot = audited_scan_and_persist(&database, &mut state)
            .expect("audited refresh should scan local repositories");
        assert_eq!(snapshot.repositories.len(), 1);
        assert_eq!(snapshot.action_audits.len(), 1);
        assert_eq!(snapshot.action_audits[0].action, "refresh");
        assert_eq!(snapshot.action_audits[0].status, "Completed");
        assert!(snapshot.action_audits[0].completed_at.is_some());

        let persisted = load_store(&database).expect("completed audit should persist");
        assert_eq!(persisted.action_audits[0].status, "Completed");
        assert!(persisted.repositories[0].path.contains(
            repository
                .file_name()
                .and_then(|name| name.to_str())
                .expect("repository name should be valid UTF-8")
        ));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn migrates_schema_v1_for_action_audits() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let connection = SqliteConnection::open(&database).expect("schema fixture should open");
        connection
            .execute_batch(
                "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO metadata (key, value) VALUES ('schema_version', '1');
                 INSERT INTO metadata (key, value) VALUES ('store_version', '1');",
            )
            .expect("schema v1 fixture should be writable");
        drop(connection);

        let migrated = load_store(&database).expect("schema v1 should migrate");
        assert!(migrated.action_audits.is_empty());
        let connection = SqliteConnection::open(&database).expect("migrated database should open");
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("migrated schema version should be readable");
        assert_eq!(schema_version, SQLITE_SCHEMA_VERSION.to_string());
        let store_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'store_version'",
                [],
                |row| row.get(0),
            )
            .expect("migrated store version should be readable");
        assert_eq!(store_version, STORE_VERSION.to_string());
        let action_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'action_audits'",
                [],
                |row| row.get(0),
            )
            .expect("action audit table should exist after migration");
        assert_eq!(action_table, "action_audits");
        let provider_identity_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'provider_identities'",
                [],
                |row| row.get(0),
            )
            .expect("provider identity table should exist after migration");
        assert_eq!(provider_identity_table, "provider_identities");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn migrates_legacy_json_store_to_versioned_sqlite() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let legacy = root.join("registry.json");
        let state = StoreState {
            roots: vec![RootConfig {
                id: "root-1".to_string(),
                path: root.to_string_lossy().to_string(),
                label: "fixture".to_string(),
                ignore_patterns: vec!["target".to_string()],
                refresh_policy: default_refresh_policy(),
                background_monitoring: false,
                registered_at: iso_now(),
            }],
            ..StoreState::default()
        };
        let encoded = serde_json::to_string_pretty(&state).expect("legacy state should serialize");
        fs::write(&legacy, encoded).expect("legacy state should be writable");

        let migrated = load_store(&database).expect("legacy state should migrate");
        assert_eq!(migrated.roots.len(), 1);
        assert_eq!(migrated.version, STORE_VERSION);
        assert_eq!(migrated.roots[0].ignore_patterns, vec!["target"]);
        assert!(database.exists());
        assert!(legacy.exists());

        let connection = SqliteConnection::open(&database).expect("database should open");
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("schema version should be recorded");
        assert_eq!(schema_version, SQLITE_SCHEMA_VERSION.to_string());

        let analytics_table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'analytics_samples'",
                [],
                |row| row.get(0),
            )
            .expect("analytics table should be created");
        assert_eq!(analytics_table_count, 1);

        let analytics_views_table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'analytics_views'",
                [],
                |row| row.get(0),
            )
            .expect("analytics views table should be created");
        assert_eq!(analytics_views_table_count, 1);

        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn reloads_repositories_in_case_insensitive_name_order() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let repositories = ["alpha", "Beta", "charlie", "Delta"]
            .into_iter()
            .rev()
            .map(|name| fixture_repository_named(&root, name))
            .map(|path| scan_repository(&path, None, &[]))
            .collect();
        let state = StoreState {
            repositories,
            ..StoreState::default()
        };

        save_store(&database, &state).expect("mixed-case repository state should persist");
        let reloaded = load_store(&database).expect("mixed-case repository state should reload");
        let reloaded_names = reloaded
            .repositories
            .iter()
            .map(|repository| repository.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(reloaded_names, vec!["alpha", "Beta", "charlie", "Delta"]);

        let snapshot = snapshot_from_store(&database, &state);
        let snapshot_names = snapshot
            .repositories
            .iter()
            .map(|repository| repository.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(snapshot_names, vec!["alpha", "Beta", "charlie", "Delta"]);

        fs::remove_dir_all(root).expect("mixed-case fixture should be removable");
    }

#[test]
    fn persists_local_configuration_for_roots_products_groups_and_lifecycle() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
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
            roots: vec![root_config.clone()],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None).expect("fixture scan should persist");
        let repository_id = state.repositories[0].id.clone();

        update_root_settings_at(
            &database,
            &root_config.id,
            vec![
                "*.tmp".to_string(),
                "cache".to_string(),
                "cache".to_string(),
            ],
            "Manual",
            true,
        )
        .expect("root settings should persist");
        set_repository_lifecycle_at(&database, &repository_id, "Paused")
            .expect("lifecycle should persist");
        let product_snapshot = upsert_product_at(
            &database,
            None,
            "Public product",
            vec![repository_id.clone()],
            "Unified product version",
        )
        .expect("product should persist");
        let group_snapshot =
            upsert_group_at(&database, None, "Experiments", vec![repository_id.clone()])
                .expect("group should persist");
        set_retention_days_at(&database, 30).expect("retention should persist");

        let persisted = load_store(&database).expect("configured state should load");
        assert_eq!(persisted.roots[0].ignore_patterns, vec!["*.tmp", "cache"]);
        assert_eq!(persisted.roots[0].refresh_policy, "Manual");
        assert!(persisted.roots[0].background_monitoring);
        assert_eq!(persisted.repositories[0].lifecycle, "Paused");
        assert_eq!(persisted.products.len(), 1);
        assert_eq!(
            persisted.products[0].release_mode,
            "Unified product version"
        );
        assert_eq!(persisted.groups.len(), 1);
        assert_eq!(persisted.groups[0].name, "Experiments");
        assert_eq!(product_snapshot.products.len(), 1);
        assert_eq!(group_snapshot.groups.len(), 1);
        assert_eq!(persisted.retention_days, 30);

        let snapshot = snapshot_from_store(&database, &persisted);
        let product_status =
            filter_snapshot_by_collection(snapshot.clone(), Some("PUBLIC PRODUCT"), None)
                .expect("product status should resolve case-insensitively");
        assert_eq!(product_status.repositories.len(), 1);
        assert_eq!(product_status.products.len(), 1);
        assert!(product_status.groups.is_empty());
        let (target_ids, target_label) = resolve_refresh_target(&snapshot, "Experiments")
            .expect("group refresh target should resolve");
        assert_eq!(target_ids, [repository_id.clone()].into_iter().collect());
        assert_eq!(target_label, "Group Experiments");
        let mut refreshed_state = load_store(&database).expect("state should reload");
        let refreshed = audited_scan_and_persist_scoped(
            &database,
            &mut refreshed_state,
            Some(&target_ids),
            Some(&target_label),
        )
        .expect("targeted refresh should persist");
        assert_eq!(refreshed.repositories.len(), 1);
        assert!(refreshed.action_audits[0]
            .target_ids
            .contains(&repository_id));

        delete_product_at(&database, &persisted.products[0].id).expect("product should delete");
        delete_group_at(&database, &persisted.groups[0].id).expect("group should delete");
        assert!(load_store(&database)
            .expect("deleted configuration should load")
            .products
            .is_empty());
        assert!(load_store(&database)
            .expect("deleted configuration should load")
            .groups
            .is_empty());
        fs::remove_dir_all(root).expect("fixture root should be removable");
        let _ = repository;
    }

#[test]
    fn repository_target_branch_override_persists_and_recomputes_tracking() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["branch", "develop"]);
        let database = root.join("registry.db");
        let snapshot = register_root_and_scan(&database, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();

        assert_eq!(
            snapshot.repositories[0].default_branch.as_deref(),
            Some("main")
        );
        assert_eq!(
            snapshot.repositories[0].target_branch.as_deref(),
            Some("main")
        );
        assert!(!snapshot.repositories[0].target_branch_configured);

        let updated = set_repository_target_branch_at(&database, &repository_id, "develop")
            .expect("a local branch should be configurable as the repository target");
        let configured = &updated.repositories[0];
        assert_eq!(configured.default_branch.as_deref(), Some("main"));
        assert_eq!(configured.target_branch.as_deref(), Some("develop"));
        assert!(configured.target_branch_configured);
        assert_eq!(
            configured.workspace.target_branch.as_deref(),
            Some("develop")
        );
        assert_eq!(configured.workspace.target_confidence, "High");
        assert_eq!(updated.events[0].kind, "state-transition");
        assert!(updated.events[0].fingerprint.contains("|develop|true|"));

        let fold_target = agent_fold_target(configured, None);
        assert_eq!(fold_target.0.as_deref(), Some("develop"));
        assert_eq!(fold_target.1, "Pronto configured repository target");

        let mut persisted = load_store(&database).expect("configured state should reload");
        let refreshed = audited_scan_and_persist(&database, &mut persisted)
            .expect("ordinary refresh should preserve the configured target");
        assert_eq!(
            refreshed.repositories[0].target_branch.as_deref(),
            Some("develop")
        );
        assert!(refreshed.repositories[0].target_branch_configured);
        assert!(set_repository_target_branch_at(&database, &repository_id, "missing").is_err());

        fs::remove_dir_all(root).expect("target branch fixture should be removable");
    }

#[test]
    fn target_evidence_reuse_requires_matching_branch_head_and_artifacts() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let target_commit = git_static(&repository_path, &["rev-parse", "refs/heads/main"])
            .expect("fixture target head should resolve");
        let target_root = root.join("target-fleet-audit");
        fs::create_dir_all(&target_root).expect("target evidence root should be creatable");

        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality.target_fleet_audit_root =
            Some(target_root.to_string_lossy().to_string());
        repository.quality.findings.scanned_branch = Some("main".to_string());
        repository.quality.findings.scanned_commit = Some(target_commit.clone());
        repository.quality.findings.observed_at = Some("2000-01-01T00:00:00Z".to_string());
        repository.quality.findings.freshness = quality::QualityFreshness::Stale;

        assert!(target_evidence_is_reusable(
            &repository,
            "main",
            &target_commit
        ));
        assert!(!target_evidence_is_reusable(
            &repository,
            "main",
            "different-head"
        ));
        assert!(!target_evidence_is_reusable(
            &repository,
            "develop",
            &target_commit
        ));

        repository.quality.target_fleet_audit_root = Some(
            root.join("missing-target-fleet-audit")
                .to_string_lossy()
                .to_string(),
        );
        assert!(!target_evidence_is_reusable(
            &repository,
            "main",
            &target_commit
        ));

        fs::remove_dir_all(root).expect("target evidence fixture should be removable");
    }

#[test]
    fn repository_target_branch_override_respects_store_write_lock() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["branch", "develop"]);
        let database = root.join("registry.db");
        let snapshot = register_root_and_scan(&database, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();
        let _lock = acquire_store_write_lock(&database).expect("fixture lock should succeed");

        let error = set_repository_target_branch_at_with_lock_timeout(
            &database,
            &repository_id,
            "develop",
            StdDuration::from_millis(20),
        )
        .expect_err("target branch writes must not bypass an active store writer");
        assert!(error.contains("Another Pronto write is already in progress"));

        fs::remove_dir_all(root).expect("target branch lock fixture should be removable");
    }
