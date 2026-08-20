#[test]
    fn scoped_refresh_admits_unregistered_repository_path_under_registered_root() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: vec!["portfolio-repository".to_string()],
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        save_store(&store, &state).expect("unregistered fixture state should persist");

        let before = snapshot_from_store(&store, &state);
        let target = resolve_local_refresh_target(&before, &state, &repository.to_string_lossy())
            .expect("repository path should resolve under the registered root");
        let repository_path = match target {
            LocalRefreshTarget::RepositoryPath(path) => path,
            LocalRefreshTarget::Registered { .. } => {
                panic!("an unregistered repository path should not resolve as registered")
            }
        };
        let refreshed =
            audited_scan_and_persist_repository_path(&store, &mut state, &repository_path)
                .expect("scoped repository-path refresh should admit the repository");

        assert_eq!(refreshed.repositories.len(), 1);
        assert_eq!(
            refreshed.repositories[0].path,
            repository_path.to_string_lossy()
        );
        assert_eq!(refreshed.action_audits.len(), 1);
        assert_eq!(refreshed.action_audits[0].status, "Completed");
        assert_eq!(
            refreshed.action_audits[0].target_ids,
            vec![path_id("repository", &repository_path)]
        );
        assert_eq!(refreshed.roots.len(), 1);

        fs::remove_dir_all(root).expect("scoped refresh fixture should be removable");
    }

#[test]
    fn scoped_refresh_rejects_repository_path_outside_registered_root() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let other_root = fixture_root();
        let other_repository = fixture_repository(&other_root);
        let store = root.join("registry.db");
        let state = StoreState {
            roots: vec![RootConfig {
                id: path_id("root", &root),
                path: root.to_string_lossy().to_string(),
                label: "fixture".to_string(),
                ignore_patterns: Vec::new(),
                refresh_policy: default_refresh_policy(),
                background_monitoring: false,
                registered_at: iso_now(),
            }],
            ..StoreState::default()
        };
        save_store(&store, &state).expect("root-only fixture state should persist");
        let snapshot = snapshot_from_store(&store, &state);

        let error =
            resolve_local_refresh_target(&snapshot, &state, &other_repository.to_string_lossy())
                .expect_err("a path outside the registered root should be rejected");
        assert!(error.contains("not covered by a registered discovery root"));

        fs::remove_dir_all(root).expect("root fixture should be removable");
        fs::remove_dir_all(other_root).expect("outside fixture should be removable");
        let _ = repository;
    }

#[test]
    fn ordinary_refresh_retains_persisted_scoped_maturity_evidence() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = &snapshot.repositories[0];
        let audit_root = root.join("qr-audit");
        let findings_root = audit_root.join("findings");
        fs::create_dir_all(&findings_root).expect("findings root should be creatable");
        let observed_at = iso_now();
        fs::write(
            findings_root.join("repository.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-scoped",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path,
                    "checkouts": [{
                        "path": repository.workspace.path,
                        "head": repository.workspace.last_commit,
                        "branch": repository.branch
                    }]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands need work.",
                    "priority": "P1",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }]
            }))
            .expect("scoped QR finding should encode"),
        )
        .expect("scoped QR finding should be writable");

        let mut state = load_store(&store).expect("fixture store should reload");
        state.remediation.refresh_steps = vec![remediation::RemediationRefreshStep {
            id: "qr_fleet_run".to_string(),
            label: "Fresh Quality Runner fleet run".to_string(),
            status: "completed".to_string(),
            evidence_path: Some(audit_root.to_string_lossy().to_string()),
            ..remediation::RemediationRefreshStep::default()
        }];
        apply_quality_evidence_scoped(&mut state, None, None);
        assert_eq!(state.repositories[0].quality.maturity.score, Some(1.375));
        assert_eq!(
            state.repositories[0]
                .quality
                .maturity
                .scored_dimension_count,
            Some(2)
        );
        assert_eq!(
            state.repositories[0].quality.maturity.freshness,
            QualityFreshness::Fresh
        );
        save_store(&store, &state).expect("scoped maturity state should persist");

        let mut persisted = load_store(&store).expect("scoped maturity state should reload");
        let refreshed = scan_and_persist_scoped(&store, &mut persisted, None)
            .expect("ordinary refresh should succeed");
        assert_eq!(
            refreshed.repositories[0].quality.maturity.score,
            Some(1.375)
        );
        assert_eq!(
            refreshed.repositories[0].quality.maturity.freshness,
            QualityFreshness::Fresh
        );

        fs::remove_dir_all(root).expect("scoped maturity fixture should be removable");
    }

#[test]
    fn read_only_route_load_recovers_persisted_scoped_maturity_evidence() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = &snapshot.repositories[0];
        let audit_root = root.join("qr-audit");
        let findings_root = audit_root.join("findings");
        fs::create_dir_all(&findings_root).expect("findings root should be creatable");
        fs::write(
            findings_root.join("repository.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-route-scoped",
                "as_of": iso_now(),
                "repository": {
                    "primary_path": repository.path,
                    "checkouts": [{
                        "path": repository.workspace.path,
                        "head": repository.workspace.last_commit,
                        "branch": repository.branch
                    }]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-route-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands need work.",
                    "priority": "P1",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }]
            }))
            .expect("scoped QR finding should encode"),
        )
        .expect("scoped QR finding should be writable");

        let mut state = load_store(&store).expect("fixture store should reload");
        state.remediation.refresh_steps = vec![remediation::RemediationRefreshStep {
            id: "qr_fleet_run".to_string(),
            label: "Fresh Quality Runner fleet run".to_string(),
            status: "completed".to_string(),
            evidence_path: Some(audit_root.to_string_lossy().to_string()),
            ..remediation::RemediationRefreshStep::default()
        }];
        save_store(&store, &state).expect("scoped audit provenance should persist");

        let state = load_store_read_only_with_quality(&store)
            .expect("route should hydrate quality without opening the store for writes");
        let snapshot = snapshot_from_store(&store, &state);
        let repository_path = snapshot.repositories[0].path.clone();
        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            &format!("repository:{repository_path}"),
            Some(&repository_path),
            3,
        )
        .expect("route report should build");

        assert_eq!(
            report
                .repository
                .as_ref()
                .and_then(|detail| detail.repository.quality.maturity.score),
            Some(1.375)
        );
        assert_eq!(
            report
                .repository
                .as_ref()
                .map(|detail| &detail.repository.quality.maturity.freshness),
            Some(&QualityFreshness::Fresh)
        );

        fs::remove_dir_all(root).expect("route maturity fixture should be removable");
    }

#[test]
    fn cached_read_does_not_ingest_quality_artifacts_until_explicit_refresh() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        assert_eq!(
            snapshot.repositories[0].quality.ingestion_status,
            "No evidence"
        );

        let observed_at = iso_now();
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-after-scan");
        fs::create_dir_all(&run).expect("quality run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "created_at": observed_at,
                "git": {
                    "branch": snapshot.repositories[0].branch,
                    "head_sha": snapshot.repositories[0].workspace.last_commit
                }
            })
            .to_string(),
        )
        .expect("quality run manifest should be writable");
        fs::write(
            run.join("gate-verification.json"),
            serde_json::json!({
                "gates": [{
                    "id": "runtime_smoke",
                    "status": "passed",
                    "capability_kind": "local_command",
                    "command": "pnpm smoke",
                    "completed_at": observed_at
                }]
            })
            .to_string(),
        )
        .expect("quality gate evidence should be writable");

        let cached = load_store_read_only(&store).expect("cached store should load");
        assert_eq!(
            cached.repositories[0].quality.ingestion_status, "No evidence",
            "read projections must not rescan quality artifacts"
        );
        let fresh = load_store_read_only_with_quality(&store)
            .expect("explicit fresh quality projection should load");
        assert_eq!(
            fresh.repositories[0].quality.ingestion_status, "Available",
            "the explicit fresh path should see newly-created quality artifacts"
        );

        fs::remove_dir_all(root).expect("cached-read fixture should be removable");
    }

#[test]
    fn refresh_write_lock_is_single_flight() {
        let root = fixture_root();
        let store = root.join("registry.db");
        let first = acquire_store_write_lock(&store).expect("first write lock should succeed");
        let second = acquire_store_write_lock_with_timeout(&store, StdDuration::from_millis(20));
        assert!(
            second.is_err(),
            "a concurrent writer must not enter the refresh critical section"
        );
        drop(first);
        let third = acquire_store_write_lock_with_timeout(&store, StdDuration::from_millis(20));
        assert!(
            third.is_ok(),
            "the lock should be reusable after the writer exits"
        );
        drop(third);

        fs::remove_dir_all(root).expect("write-lock fixture should be removable");
    }

#[test]
    fn store_write_state_reloads_after_previous_writer_commits() {
        let root = fixture_root();
        let store = root.join("registry.db");
        save_store(&store, &StoreState::default()).expect("initial store should persist");

        let first_entered = Arc::new(Barrier::new(2));
        let first_release = Arc::new(Barrier::new(2));
        let first_store = store.clone();
        let first_entered_thread = Arc::clone(&first_entered);
        let first_release_thread = Arc::clone(&first_release);
        let first = thread::spawn(move || {
            with_store_write_state(&first_store, |state| {
                state.retention_days = 31;
                first_entered_thread.wait();
                first_release_thread.wait();
                save_store(&first_store, state)
            })
        });

        first_entered.wait();
        let second_store = store.clone();
        let second = thread::spawn(move || {
            with_store_write_state(&second_store, |state| {
                assert_eq!(
                    state.retention_days, 31,
                    "a writer must load the snapshot committed by the previous writer"
                );
                Ok(())
            })
        });
        first_release.wait();

        first
            .join()
            .expect("first writer thread should finish")
            .expect("first writer should persist");
        second
            .join()
            .expect("second writer thread should finish")
            .expect("second writer should observe the committed state");
        assert!(
            !store_write_lock_path(&store).exists(),
            "the write lock must be released before the operation returns"
        );

        fs::remove_dir_all(root).expect("write-state fixture should be removable");
    }

#[test]
    fn refresh_releases_write_lock_before_returning() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        save_store(
            &store,
            &StoreState {
                roots: vec![RootConfig {
                    id: path_id("root", &root),
                    path: root.to_string_lossy().to_string(),
                    label: "fixture".to_string(),
                    ignore_patterns: Vec::new(),
                    refresh_policy: default_refresh_policy(),
                    background_monitoring: false,
                    registered_at: iso_now(),
                }],
                ..StoreState::default()
            },
        )
        .expect("refresh fixture should persist");

        let snapshot = refresh_at(&store).expect("refresh should complete");
        assert_eq!(snapshot.repositories.len(), 1);
        assert!(
            !store_write_lock_path(&store).exists(),
            "refresh must release the write lock before returning its snapshot"
        );

        fs::remove_dir_all(root).expect("refresh fixture should be removable");
    }

#[test]
    fn concurrent_refreshes_serialize_without_stale_lock_failure() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        save_store(
            &store,
            &StoreState {
                roots: vec![RootConfig {
                    id: path_id("root", &root),
                    path: root.to_string_lossy().to_string(),
                    label: "fixture".to_string(),
                    ignore_patterns: Vec::new(),
                    refresh_policy: default_refresh_policy(),
                    background_monitoring: false,
                    registered_at: iso_now(),
                }],
                ..StoreState::default()
            },
        )
        .expect("concurrent refresh fixture should persist");

        let start = Arc::new(Barrier::new(3));
        let handles = (0..2)
            .map(|_| {
                let store = store.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    refresh_at(&store)
                })
            })
            .collect::<Vec<_>>();
        start.wait();

        for handle in handles {
            handle
                .join()
                .expect("concurrent refresh thread should finish")
                .expect("concurrent refresh should wait for the prior writer");
        }
        let persisted = load_store(&store).expect("serialized refreshes should persist");
        assert!(
            persisted.action_audits.len() >= 2,
            "each serialized refresh should retain its audit record"
        );
        assert!(
            !store_write_lock_path(&store).exists(),
            "serialized refreshes must leave no write lock behind"
        );

        fs::remove_dir_all(root).expect("concurrent refresh fixture should be removable");
    }
