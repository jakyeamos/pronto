#[test]
    fn refresh_batch_scans_in_parallel_and_merges_deterministically() {
        let root = fixture_root();
        fixture_repository_named(&root, "alpha-repository");
        fixture_repository_named(&root, "beta-repository");
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
        .expect("batch refresh fixture should persist");

        let report = refresh_batch_at(&store, None, 2).expect("batch refresh should complete");
        assert_eq!(report.status, "Completed");
        assert_eq!(report.parallelism, 2);
        assert_eq!(report.repository_count, 2);
        assert_eq!(report.scan_phase, "Parallel read-only scan completed");
        assert_eq!(report.merge_phase, "Serialized locked merge committed");
        assert!(report
            .repositories
            .windows(2)
            .all(|window| window[0].repository_id <= window[1].repository_id));

        let persisted = load_store(&store).expect("batch refresh should persist the merged state");
        assert_eq!(persisted.repositories.len(), 2);
        assert_eq!(persisted.action_audits.len(), 1);
        assert!(!store_write_lock_path(&store).exists());

        fs::remove_dir_all(root).expect("batch refresh fixture should be removable");
    }

#[test]
    fn concurrent_batch_refreshes_preserve_all_scanned_repositories() {
        let root = fixture_root();
        fixture_repository_named(&root, "alpha-repository");
        fixture_repository_named(&root, "beta-repository");
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
        .expect("concurrent batch fixture should persist");

        let start = Arc::new(Barrier::new(3));
        let handles = (0..2)
            .map(|_| {
                let store = store.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    refresh_batch_at(&store, None, 2)
                })
            })
            .collect::<Vec<_>>();
        start.wait();

        let reports = handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .expect("concurrent batch worker should finish")
                    .expect("concurrent batch refresh should retry or commit")
            })
            .collect::<Vec<_>>();
        assert!(reports.iter().all(|report| report.status == "Completed"));
        let persisted = load_store(&store).expect("concurrent batch state should reload");
        assert_eq!(persisted.repositories.len(), 2);
        assert!(
            persisted.action_audits.len() >= 2,
            "each concurrent batch should retain its audit record"
        );
        assert!(!store_write_lock_path(&store).exists());

        fs::remove_dir_all(root).expect("concurrent batch fixture should be removable");
    }

#[test]
    fn maturity_checkpoint_reports_missing_and_stale_applicable_repositories() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_name = snapshot.repositories[0].name.clone();
        snapshot.repositories[0].lifecycle = "Active".to_string();
        snapshot.repositories[0].lifecycle_candidate = "Active".to_string();

        assert_eq!(
            maturity_coverage_gaps(&snapshot.repositories),
            vec![format!("{repository_name} (unknown)")]
        );

        snapshot.repositories[0].quality.maturity.score = Some(2.0);
        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Stale;
        assert_eq!(
            maturity_coverage_gaps(&snapshot.repositories),
            vec![format!("{repository_name} (stale)")]
        );

        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0]
            .quality
            .mac_control_ideal_state
            .status = "Not applicable".to_string();
        snapshot.repositories[0]
            .quality
            .mac_control_ideal_state
            .freshness = "Fresh".to_string();
        assert!(maturity_coverage_gaps(&snapshot.repositories).is_empty());

        fs::remove_dir_all(root).expect("maturity coverage fixture should be removable");
    }

#[test]
    fn cli_repeated_options_are_agent_friendly() {
        let arguments = vec![
            "group".to_string(),
            "create".to_string(),
            "Experiments".to_string(),
            "--repo".to_string(),
            "repository:one".to_string(),
            "--repo".to_string(),
            "repository:two".to_string(),
            "--json".to_string(),
        ];
        assert_eq!(
            cli_positionals(&arguments, &["--repo"]).expect("positionals should parse"),
            vec!["create", "Experiments"]
        );
        assert_eq!(
            cli_repeated_option(&arguments, "--repo").expect("repeated options should parse"),
            vec!["repository:one", "repository:two"]
        );
    }

#[test]
    fn remediation_timeout_option_requires_a_positive_integer() {
        let valid = vec![
            "remediation".to_string(),
            "refresh".to_string(),
            "--timeout-seconds".to_string(),
            "3600".to_string(),
        ];
        assert_eq!(
            cli_positive_u64_option(&valid, "--timeout-seconds")
                .expect("positive timeout should parse"),
            Some(3600)
        );

        for invalid_value in ["0", "-1", "not-a-number"] {
            let invalid = vec![
                "remediation".to_string(),
                "refresh".to_string(),
                "--timeout-seconds".to_string(),
                invalid_value.to_string(),
            ];
            assert_eq!(
                cli_positive_u64_option(&invalid, "--timeout-seconds")
                    .expect_err("invalid timeout should fail"),
                "--timeout-seconds must be a positive integer"
            );
        }
    }

#[test]
    fn qr_audit_runtime_arguments_apply_the_requested_timeout() {
        let mut arguments = vec!["fleet".to_string(), "audit".to_string()];
        append_qr_audit_runtime_arguments(&mut arguments, true, false, 3600);

        assert_eq!(
            arguments,
            vec![
                "fleet",
                "audit",
                "--dynamic",
                "--no-changed-only",
                "--timeout-seconds",
                "3600",
                "--json"
            ]
        );
    }

#[test]
    fn excludes_the_scanner_process_and_its_ancestors_from_activity_candidates() {
        let rows = parse_process_activity_rows(
            "10 9 pronto_cli Tue Jul 29 13:43:00 2026\n\
             9 8 cargo Tue Jul 29 13:42:59 2026\n\
             8 7 node Tue Jul 29 13:42:58 2026\n\
             7 1 zsh Tue Jul 29 13:42:57 2026\n\
             22 1 codex Tue Jul 29 12:00:00 2026\n",
        );

        let excluded = process_ancestor_ids(&rows, 10);

        assert_eq!(
            excluded,
            [10, 9, 8, 7, 1].into_iter().collect::<HashSet<_>>()
        );
        assert!(!excluded.contains(&22));
        assert_eq!(rows[0].process_name, "pronto_cli");
        assert_eq!(
            rows[0].started_at.as_deref(),
            Some("Tue Jul 29 13:43:00 2026")
        );
    }

#[test]
    fn ignores_idle_shells_as_agent_activity_candidates() {
        assert!(!process_name_is_activity_candidate("/bin/zsh"));
        assert!(!process_name_is_activity_candidate("bash"));
        assert!(process_name_is_activity_candidate("codex"));
        assert!(process_name_is_activity_candidate("claude-code"));
    }

#[test]
    fn observes_git_fetch_head_freshness() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        fs::write(repository.join(".git").join("FETCH_HEAD"), "fixture\n")
            .expect("fetch marker should be writable");

        assert!(observed_fetch_at(&repository).is_some());

        fs::remove_dir_all(root).expect("fetch fixture should be removable");
    }

#[test]
    fn appending_repository_ids_preserves_and_deduplicates_members() {
        let merged = merge_repository_ids(
            &["repository:one".to_string(), "repository:two".to_string()],
            vec!["repository:two".to_string(), "repository:three".to_string()],
        );
        let mut sorted = merged;
        sorted.sort();
        assert_eq!(
            sorted,
            vec![
                "repository:one".to_string(),
                "repository:three".to_string(),
                "repository:two".to_string()
            ]
        );
    }

#[test]
    fn excludes_named_folders_case_insensitively_and_prunes_existing_repositories() {
        let root = fixture_root();
        let included = fixture_repository(&root);
        let excluded_parent = root.join("Not-mine");
        fs::create_dir_all(&excluded_parent).expect("excluded folder should be creatable");
        let excluded = fixture_repository(&excluded_parent);
        let store = root.join("registry.db");

        let first = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("initial scan should discover both repositories");
        assert_eq!(first.repositories.len(), 2);
        assert!(matches_ignore("Not-mine", &["not-mine".to_string()]));

        let filtered = exclude_root_patterns_at(
            &store,
            &root.to_string_lossy(),
            vec!["not-mine".to_string(), "test-fixtures".to_string()],
        )
        .expect("root exclusions should rescan the root");
        assert_eq!(filtered.repositories.len(), 1);
        assert_eq!(
            filtered.repositories[0].path,
            canonical_path(&included)
                .expect("included repository should canonicalize")
                .to_string_lossy()
        );
        assert!(!filtered.repositories.iter().any(|repository| {
            repository.path
                == canonical_path(&excluded)
                    .expect("excluded repository should canonicalize")
                    .to_string_lossy()
        }));

        let persisted = load_store(&store).expect("root exclusions should persist");
        assert_eq!(
            persisted.roots[0].ignore_patterns,
            vec!["not-mine", "test-fixtures"]
        );

        fs::remove_dir_all(root).expect("exclusion fixture should be removable");
    }

#[test]
    fn prunes_deleted_repositories_on_full_refresh() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");

        let first = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("initial scan should discover the repository");
        assert_eq!(first.repositories.len(), 1);

        fs::remove_dir_all(&repository).expect("repository should be removable");
        let mut state = load_store(&store).expect("initial store should be readable");
        let refreshed =
            scan_and_persist_scoped(&store, &mut state, None).expect("full refresh should succeed");

        assert!(refreshed.repositories.is_empty());
        assert!(load_store(&store)
            .expect("refreshed store should be readable")
            .repositories
            .is_empty());

        fs::remove_dir_all(root).expect("deleted repository fixture should be removable");
    }

#[test]
    fn prunes_existing_non_git_namespace_records_on_full_refresh() {
        let root = fixture_root();
        let namespace = root.join("BBDSE");
        let nested_repository = fixture_repository_named(&namespace, "nested-repository");
        let store = root.join("registry.db");

        let first = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("initial scan should discover nested Git repositories");
        assert_eq!(first.repositories.len(), 1);
        assert_eq!(
            first.repositories[0].path,
            canonical_path(&nested_repository)
                .expect("nested repository should canonicalize")
                .to_string_lossy()
        );

        let mut state = load_store(&store).expect("initial store should be readable");
        let stale_namespace = scan_repository(&namespace, None, &state.expected_conditions);
        state.repositories.push(stale_namespace);
        save_store(&store, &state).expect("stale namespace should persist for the fixture");

        let mut state = load_store(&store).expect("state with stale namespace should reload");
        let refreshed = scan_and_persist_scoped(&store, &mut state, None)
            .expect("full refresh should prune the non-Git namespace");

        assert_eq!(refreshed.repositories.len(), 1);
        assert_eq!(
            refreshed.repositories[0].path,
            canonical_path(&nested_repository)
                .expect("nested repository should canonicalize")
                .to_string_lossy()
        );
        assert!(!refreshed.repositories.iter().any(|repository| {
            repository.path
                == canonical_path(&namespace)
                    .expect("namespace should canonicalize")
                    .to_string_lossy()
        }));

        fs::remove_dir_all(root).expect("namespace fixture should be removable");
    }

#[test]
    fn parses_porcelain_branch_and_dirty_state() {
        let parsed = parse_status(
            "# branch.oid abc\n# branch.head feature/test\n# branch.upstream origin/feature/test\n# branch.ab +3 -2\n1 .M N... 100644 100644 100644 abc def tracked.txt\n",
        );
        assert_eq!(parsed.branch, "feature/test");
        assert_eq!(parsed.upstream.as_deref(), Some("origin/feature/test"));
        assert_eq!(parsed.ahead, 3);
        assert_eq!(parsed.behind, 2);
        assert!(parsed.dirty);
    }

#[test]
    fn failed_git_status_is_projected_as_unavailable_not_clean_detached() {
        let root = fixture_root();
        let workspace = scan_workspace(&root, true, Some("main"), Some("main"), "Medium", None);

        assert!(!workspace.status_available);
        assert_eq!(workspace.branch, "Unknown");
        assert!(!workspace.dirty);
        assert_eq!(workspace.sync_state, "Git status unavailable");
        assert_eq!(workspace.integration_state, "Unknown");
        assert!(!workspace_is_unsynced(&workspace));
        assert!(workspace_requires_sync_attention(&workspace));
        assert!(workspace
            .status_error
            .as_deref()
            .is_some_and(|error| error.contains("Git status failed")));

        let conditions = build_conditions(
            "repository:test",
            &workspace,
            Some("main"),
            &[],
            "2026-08-08T00:00:00Z",
        );
        assert!(conditions
            .iter()
            .any(|condition| condition.kind == "git-status-unavailable"));
        assert!(!conditions
            .iter()
            .any(|condition| condition.kind == "no-upstream"));

        fs::remove_dir_all(root).expect("failed-status fixture should be removable");
    }

#[test]
    fn remediation_handoff_requires_a_checkpoint_and_fresh_snapshot() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut clean_snapshot = scan_repository(&repository_path, None, &[]);
        clean_snapshot.workspace.activity = WorkspaceActivity::default();
        let clean_check = remediation_handoff_check_for_repository(&clean_snapshot, None)
            .expect("clean fixture should be checkable");
        assert!(clean_check.ready);
        assert!(!clean_check.checkpoint_required);

        fs::write(repository_path.join("tracked.txt"), "uncommitted\n")
            .expect("dirty fixture should be writable");
        let mut dirty_snapshot = scan_repository(&repository_path, Some(&clean_snapshot), &[]);
        dirty_snapshot.workspace.activity = WorkspaceActivity::default();
        let dirty_check = remediation_handoff_check_for_repository(&dirty_snapshot, None)
            .expect("dirty fixture should be checkable");
        assert!(!dirty_check.ready);
        assert!(dirty_check.checkpoint_required);
        assert!(dirty_check.workspace_dirty);
        assert!(dirty_check
            .reasons
            .iter()
            .any(|reason| reason.contains("checkpoint commit")));

        git(&repository_path, &["add", "tracked.txt"]);
        git(&repository_path, &["commit", "-m", "Checkpoint fixture"]);
        let stale_check = remediation_handoff_check_for_repository(&dirty_snapshot, None)
            .expect("stale fixture should still be checkable");
        assert!(!stale_check.ready);
        assert!(!stale_check.workspace_dirty);
        assert!(stale_check
            .reasons
            .iter()
            .any(|reason| reason.contains("persisted Pronto snapshot")));

        let mut refreshed_snapshot = scan_repository(&repository_path, Some(&dirty_snapshot), &[]);
        refreshed_snapshot.workspace.activity = WorkspaceActivity::default();
        let refreshed_check = remediation_handoff_check_for_repository(&refreshed_snapshot, None)
            .expect("refreshed fixture should be checkable");
        assert!(refreshed_check.ready);
        assert!(!refreshed_check.checkpoint_required);

        fs::remove_dir_all(root).expect("handoff fixture should be removable");
    }

#[test]
    fn remediation_execution_gate_separates_execution_from_closure_blockers() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.workspace.activity = WorkspaceActivity::default();
        for workspace in &mut repository.workspaces {
            workspace.activity = WorkspaceActivity::default();
        }
        let mut remediation_run =
            remediation::rebuild_run(&[repository.clone()], &remediation::empty_run(), None);
        let plan = remediation_run
            .plans
            .first_mut()
            .expect("fixture repository should produce a remediation plan");
        plan.actions[0].status = "blocked".to_string();
        remediation::recompute_plan_derived(plan);

        let gate = remediation_execution_gate_for_repository(&repository, Some(plan), None, None)
            .expect("clean repository should produce an execution gate");

        assert!(gate.ready, "unexpected blockers: {:#?}", gate.blockers);
        assert_eq!(gate.status, "ready");
        assert!(gate.blockers.is_empty());
        assert_eq!(gate.closure_gate.status, "blocked");
        assert_eq!(gate.closure_gate.blocked_action_count, 1);
        assert!(gate
            .closure_gate
            .detail
            .contains("does not by itself block remediation execution"));

        fs::remove_dir_all(root).expect("execution-gate fixture should be removable");
    }
