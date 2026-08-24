#[test]
    fn doctor_warns_for_missing_clean_temporary_workspace_until_scoped_refresh() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let store = root.join("registry.db");
        let initial = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = initial.repositories[0].id.clone();
        let canonical_workspace = initial.repositories[0].workspace.clone();
        let missing_path = root.join("missing-temporary-workspace");
        assert!(!missing_path.exists());

        let mut state = load_store(&store).expect("fixture store should reload");
        let repository = state
            .repositories
            .iter_mut()
            .find(|repository| repository.id == repository_id)
            .expect("fixture repository should be persisted");
        let mut stale_workspace = canonical_workspace.clone();
        stale_workspace.id = path_id("workspace", &missing_path);
        stale_workspace.path = missing_path.to_string_lossy().to_string();
        stale_workspace.is_primary = false;
        stale_workspace.provenance = WorkspaceProvenance {
            kind: "temporary".to_string(),
            owner: Some("task-incident".to_string()),
            lease: Some("completed".to_string()),
            canonical_repository: repository.path.clone(),
            head: canonical_workspace.last_commit.clone(),
            preservation_evidence: Some(
                "Temporary workspace was inspected before its directory disappeared.".to_string(),
            ),
            cleanup_state: "completed".to_string(),
        };
        repository.workspaces.push(stale_workspace.clone());
        save_store(&store, &state).expect("stale workspace should be persisted");

        let persisted = load_store(&store).expect("stale store should reload");
        let snapshot = snapshot_from_store(&store, &persisted);
        let report = agent_doctor_report(&snapshot, &store, 60, "repository:fixture");

        assert!(report.ready);
        assert_eq!(report.status, "Ready with warnings");
        assert!(report.unavailable_paths.is_empty());
        assert_eq!(report.workspace_warnings.len(), 1);
        assert!(report.workspace_warnings[0].contains(missing_path.to_string_lossy().as_ref()));
        assert!(report.workspace_warnings[0].contains("HEAD"));
        assert!(report.workspace_warnings[0].contains("task-incident"));
        assert!(report.workspace_warnings[0].contains("pronto refresh"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "paths" && check.status == "Warning"));

        let repository_query = repository_path.to_string_lossy().to_string();
        let route_before_refresh = agent_route_report(
            &snapshot,
            &store,
            60,
            &format!("repository:{repository_query}"),
            Some(&repository_query),
            3,
        )
        .expect("scoped route should inspect the retained stale record");
        assert!(route_before_refresh.ready);
        let retained = load_store(&store).expect("route should leave the store readable");
        assert!(retained
            .repositories
            .iter()
            .find(|repository| repository.id == repository_id)
            .is_some_and(|repository| repository
                .workspaces
                .iter()
                .any(|workspace| workspace.path == stale_workspace.path)));

        let mut state = load_store(&store).expect("stale store should be available to refresh");
        let refreshed = audited_scan_and_persist_repository_path(
            &store,
            &mut state,
            Path::new(&repository_path),
        )
        .expect("scoped refresh should reconstruct workspaces from live Git");
        let refreshed_repository = refreshed
            .repositories
            .iter()
            .find(|repository| repository.id == repository_id)
            .expect("refreshed repository should remain registered");
        assert!(!refreshed_repository
            .workspaces
            .iter()
            .any(|workspace| workspace.path == stale_workspace.path));
        let route = agent_route_report(
            &refreshed,
            &store,
            60,
            &format!("repository:{repository_query}"),
            Some(&repository_query),
            3,
        )
        .expect("scoped route should read the refreshed snapshot");
        assert!(route.ready);
        assert!(route.repository.as_ref().is_some_and(|detail| !detail
            .repository
            .workspaces
            .iter()
            .any(|workspace| workspace.path == stale_workspace.path)));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }

#[test]
    fn doctor_blocks_missing_temporary_workspace_with_dirty_or_unknown_last_state() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let initial = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = initial.repositories[0].id.clone();
        let canonical_workspace = initial.repositories[0].workspace.clone();
        let mut state = load_store(&store).expect("fixture store should reload");
        let repository = state
            .repositories
            .iter_mut()
            .find(|repository| repository.id == repository_id)
            .expect("fixture repository should be persisted");

        for (name, status_available, dirty) in [
            ("missing-dirty-temporary", true, true),
            ("missing-unknown-temporary", false, false),
        ] {
            let missing_path = root.join(name);
            let mut stale_workspace = canonical_workspace.clone();
            stale_workspace.id = path_id("workspace", &missing_path);
            stale_workspace.path = missing_path.to_string_lossy().to_string();
            stale_workspace.is_primary = false;
            stale_workspace.status_available = status_available;
            stale_workspace.status_error = (!status_available)
                .then(|| "Git status was unavailable before the workspace disappeared".to_string());
            stale_workspace.dirty = dirty;
            stale_workspace.provenance = WorkspaceProvenance {
                kind: "temporary".to_string(),
                owner: Some("task-preserve".to_string()),
                lease: Some("completed".to_string()),
                canonical_repository: repository.path.clone(),
                head: canonical_workspace.last_commit.clone(),
                preservation_evidence: Some(
                    "Preserve until the owner confirms recovery.".to_string(),
                ),
                cleanup_state: "blocked".to_string(),
            };
            repository.workspaces.push(stale_workspace);
        }
        save_store(&store, &state).expect("blocked stale workspaces should be persisted");

        let persisted = load_store(&store).expect("blocked store should reload");
        let snapshot = snapshot_from_store(&store, &persisted);
        let report = agent_doctor_report(&snapshot, &store, 60, "repository:fixture");

        assert!(!report.ready);
        assert_eq!(report.status, "Blocked");
        assert!(report.workspace_warnings.is_empty());
        assert!(report
            .unavailable_paths
            .iter()
            .any(|path| path.ends_with("missing-dirty-temporary")));
        assert!(report
            .unavailable_paths
            .iter()
            .any(|path| path.ends_with("missing-unknown-temporary")));
        let paths_check = report
            .checks
            .iter()
            .find(|check| check.id == "paths")
            .expect("paths check should be present");
        assert_eq!(paths_check.status, "Blocked");
        assert!(paths_check
            .evidence
            .iter()
            .any(|evidence| evidence.contains("task-preserve")));
        assert!(paths_check
            .evidence
            .iter()
            .any(|evidence| evidence.contains("Preserve until the owner confirms recovery")));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }

#[test]
    fn doctor_default_freshness_window_is_two_days() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");

        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(47)).to_rfc3339();
        let fresh = agent_doctor_report(
            &snapshot,
            &store,
            DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES,
            "repository:fixture",
        );

        assert_eq!(fresh.max_age_minutes, 2_880);
        assert!(fresh.stale_repository_ids.is_empty());
        assert!(fresh
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Passed"));

        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(49)).to_rfc3339();
        let stale = agent_doctor_report(
            &snapshot,
            &store,
            DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES,
            "repository:fixture",
        );

        assert_eq!(stale.stale_repository_ids.len(), 1);
        assert!(stale
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Blocked"));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }

#[test]
    fn scoped_doctor_ignores_unrelated_stale_repositories() {
        let root = fixture_root();
        let first_repository = fixture_repository(&root);
        let second_root = root.join("second");
        fs::create_dir_all(&second_root).expect("second repository parent should be creatable");
        fixture_repository(&second_root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        assert_eq!(snapshot.repositories.len(), 2);
        let first_repository_path = canonical_path(&first_repository)
            .expect("first repository should canonicalize")
            .to_string_lossy()
            .to_string();
        let selected_id = snapshot
            .repositories
            .iter()
            .find(|repository| repository.path == first_repository_path)
            .expect("first repository should be registered")
            .id
            .clone();
        let unrelated = snapshot
            .repositories
            .iter_mut()
            .find(|repository| repository.id != selected_id)
            .expect("second repository should be registered");
        unrelated.last_scan_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        unrelated.workspaces[0].path = root
            .join("missing-unrelated-workspace")
            .to_string_lossy()
            .to_string();
        let mut scoped_snapshot = snapshot;
        scoped_snapshot
            .repositories
            .retain(|repository| repository.id == selected_id);
        let report = agent_doctor_report(
            &scoped_snapshot,
            &store,
            60,
            "current_repository:/Users/jakyeamos/Documents/pronto",
        );

        assert!(report.ready);
        assert!(matches!(
            report.status.as_str(),
            "Ready" | "Ready with warnings"
        ));
        assert_eq!(report.repository_count, 1);
        assert_eq!(report.root_count, 1);
        assert!(report.stale_repository_ids.is_empty());
        assert!(report.unavailable_paths.is_empty());

        fs::remove_dir_all(root).expect("scoped doctor fixture should be removable");
    }

#[test]
    fn doctor_read_only_load_does_not_create_missing_store() {
        let root = fixture_root();
        let store = root.join("missing.db");

        assert!(load_store_read_only(&store).is_err());
        assert!(!store.exists());

        fs::remove_dir_all(root).expect("doctor read-only fixture should be removable");
    }

#[test]
    fn route_exposes_bounded_projections_after_a_ready_doctor_gate() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        snapshot.repositories[0]
            .quality
            .maturity
            .dimension_scores
            .insert("developer_legibility".to_string(), 3.0);
        let repository_path = snapshot.repositories[0].path.clone();
        let scope = format!("repository:{repository_path}");

        let report = agent_route_report(&snapshot, &store, 60, &scope, Some(&repository_path), 3)
            .expect("route report should build");

        assert_eq!(report.schema_version, AGENT_ROUTE_SCHEMA);
        assert!(report.ready);
        assert!(report.next.is_some());
        assert!(report.repository.is_some());
        assert!(report.fold_preview.is_some());
        assert!(report
            .fold_preview
            .as_ref()
            .is_some_and(|preview| preview.live_verification_required));
        assert_eq!(
            report
                .quality
                .as_ref()
                .map(|quality| quality.scope.as_str()),
            Some(scope.as_str())
        );
        assert_eq!(report.doctor.scope, scope);
        assert_eq!(
            report
                .developer_legibility
                .as_ref()
                .map(|gate| gate.status.as_str()),
            Some("enforced")
        );
        assert!(report
            .developer_legibility
            .as_ref()
            .is_some_and(|gate| gate.recommended_inspection.contains("developer-legibility")));
        assert!(report.authorization.contains("Inspection only"));

        fs::remove_dir_all(root).expect("route fixture should be removable");
    }

#[test]
    fn route_projection_does_not_run_live_merge_checks() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/route"]);
        fs::write(repository.join("route.txt"), "route\n")
            .expect("route fixture file should be writable");
        git(&repository, &["add", "route.txt"]);
        git(&repository, &["commit", "-m", "Route projection"]);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("route fixture portfolio should scan");
        let repository_path = snapshot.repositories[0].path.clone();
        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            &format!("repository:{repository_path}"),
            Some(&repository_path),
            3,
        )
        .expect("route report should build without live merge checks");
        let fold_preview = report
            .fold_preview
            .expect("route should include a fold projection");
        assert!(
            fold_preview
                .candidates
                .iter()
                .all(|candidate| candidate.merge_preview.is_none()),
            "route must keep live merge verification out of the response path"
        );
        assert!(fold_preview.live_verification_required);

        fs::remove_dir_all(root).expect("route merge-check fixture should be removable");
    }

#[test]
    fn fresh_route_timeout_names_the_cached_fallback() {
        let root = fixture_root();
        let store = root.join("registry.db");
        let report = agent_route_error_report(
            &store,
            60,
            "fleet",
            "storage",
            "Fresh quality projection exceeded the 60 second deadline; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.".to_string(),
        );

        assert!(report
            .next_safe_step
            .contains("cached snapshot or run `pronto quality refresh`"));
        assert!(report.doctor.checks[0]
            .next_safe_step
            .contains("cached snapshot or run `pronto quality refresh`"));

        fs::remove_dir_all(root).expect("route timeout fixture should be removable");
    }

#[test]
    fn route_withholds_follow_up_projections_when_doctor_is_blocked() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();

        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            "repository:fixture",
            Some(&snapshot.repositories[0].path.clone()),
            3,
        )
        .expect("blocked route report should still build");

        assert_eq!(report.schema_version, AGENT_ROUTE_SCHEMA);
        assert!(!report.ready);
        assert_eq!(report.status, "Blocked");
        assert!(report.next.is_none());
        assert!(report.repository.is_none());
        assert!(report.quality.is_none());
        assert!(report.fold_preview.is_none());
        assert!(report.next_safe_step.contains("refresh"));

        fs::remove_dir_all(root).expect("blocked route fixture should be removable");
    }

#[test]
    fn parses_numstat_and_marks_binary_totals_partial() {
        let totals = parse_numstat("4\t2\ttext.txt\n-\t-\timage.png\n");
        assert_eq!(totals.added, 4);
        assert_eq!(totals.removed, 2);
        assert!(totals.partial);
    }

#[test]
    fn normalizes_github_remote_names_without_exposing_credentials() {
        assert_eq!(
            normalize_remote_name("git@github.com:Acme/Portfolio.git").as_deref(),
            Some("acme/portfolio")
        );
        assert_eq!(
            normalize_remote_name("https://github.com/Acme/Portfolio/").as_deref(),
            Some("acme/portfolio")
        );
        assert_eq!(normalize_remote_name("  ").as_deref(), None);
    }
