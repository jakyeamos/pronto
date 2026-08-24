#[test]
    fn parses_ci_runs_with_failure_context_forks_artifacts_and_stable_signatures() {
        let payload = serde_json::json!({
            "workflow_runs": [
                {
                    "id": 7001,
                    "name": "Quality",
                    "path": ".github/workflows/quality.yml",
                    "display_title": "Add tracker",
                    "run_number": 42,
                    "run_attempt": 2,
                    "event": "pull_request",
                    "status": "completed",
                    "conclusion": "failure",
                    "head_branch": "feature/tracker",
                    "head_sha": "abc123",
                    "html_url": "https://github.com/acme/project/actions/runs/7001",
                    "created_at": "2026-08-14T12:00:00Z",
                    "updated_at": "2026-08-14T12:05:00Z",
                    "repository": {"full_name": "acme/project"},
                    "head_repository": {"full_name": "jakyeamos/project"},
                    "pull_requests": [{"number": 17}]
                },
                {
                    "id": 7002,
                    "name": "Quality",
                    "run_number": 43,
                    "run_attempt": 1,
                    "event": "push",
                    "status": "completed",
                    "conclusion": "success",
                    "head_sha": "def456",
                    "repository": {"full_name": "acme/project"},
                    "head_repository": {"full_name": "acme/project"}
                }
            ]
        });
        let mut runs = parse_github_workflow_runs(&payload, "acme/project", "2026-08-14T12:10:00Z")
            .expect("workflow runs should parse");
        assert_eq!(runs.len(), 2);
        assert!(runs[0].is_fork);
        assert_eq!(runs[0].pull_request_number, Some(17));
        assert_eq!(runs[0].run_attempt, 2);
        assert!(!runs[1].is_fork);

        let jobs = parse_github_jobs(&serde_json::json!({
            "jobs": [{
                "id": 9001,
                "name": "macOS",
                "status": "completed",
                "conclusion": "failure",
                "html_url": "https://github.com/acme/project/actions/runs/7001/job/9001",
                "steps": [
                    {"name": "Install", "conclusion": "success"},
                    {"name": "Run tests", "conclusion": "failure"},
                    {"name": "Upload logs", "conclusion": "skipped"}
                ]
            }]
        }))
        .expect("jobs should parse");
        runs[0].jobs = jobs;
        runs[0].failure_summary = summarize_ci_failure(&runs[0]);
        runs[0].failure_signature = ci_failure_signature(&runs[0]);
        assert_eq!(runs[0].failure_summary.as_deref(), Some("macOS: Run tests"));
        let first_signature = runs[0].failure_signature.clone();
        runs[0].failure_signature = ci_failure_signature(&runs[0]);
        assert_eq!(runs[0].failure_signature, first_signature);

        let artifact = parse_ci_prompt_artifact(
            &serde_json::json!({
                "artifacts": [{"id": 55, "name": "codex-ci-prompt-7001-2", "expired": false}]
            }),
            7001,
            2,
        )
        .expect("current-attempt artifact should parse");
        assert_eq!(artifact.name, "codex-ci-prompt-7001-2");
        assert!(parse_ci_prompt_artifact(
            &serde_json::json!({
                "artifacts": [{"id": 56, "name": "codex-ci-prompt-7001-1", "expired": true}]
            }),
            7001,
            1,
        )
        .is_none());
        assert!(runs[1].failure_summary.is_none());
        assert!(!ci_conclusion_is_failure(Some("success")));
        assert!(ci_conclusion_is_failure(Some("cancelled")));
    }

#[test]
    fn parses_github_pull_requests_and_published_release_snapshots() {
        let pull_requests = parse_github_pull_requests(
            &serde_json::json!([
                [{
                    "number": 12,
                    "html_url": "https://github.com/Acme/Portfolio/pull/12",
                    "title": "Release preview",
                    "state": "open",
                    "draft": true,
                    "head": {"ref": "feature/release"},
                    "base": {"ref": "main"},
                    "mergeable_state": "unknown"
                }]
            ]),
            "github:42",
            "2026-07-26T12:00:00Z",
        )
        .expect("pull-request pages should parse");
        assert_eq!(pull_requests.len(), 1);
        assert_eq!(pull_requests[0].number, 12);
        assert_eq!(pull_requests[0].head_branch, "feature/release");
        assert_eq!(
            pull_requests[0].checks_state,
            "Unknown — provider snapshot unavailable"
        );

        let releases = parse_github_releases(
            &serde_json::json!([
                {
                    "id": 9,
                    "tag_name": "v1.4.0",
                    "name": "Version 1.4.0",
                    "target_commitish": "abc123",
                    "published_at": "2026-07-25T12:00:00Z",
                    "draft": false,
                    "prerelease": false
                },
                {
                    "id": 10,
                    "tag_name": "v1.5.0-rc.1",
                    "published_at": "2026-07-26T12:00:00Z",
                    "draft": false,
                    "prerelease": true
                }
            ]),
            "github:42",
            "2026-07-26T12:00:00Z",
        )
        .expect("release response should parse");
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag, "v1.4.0");
        assert!(!releases[0].prerelease);
        assert!(releases[1].prerelease);
    }

#[test]
    fn applies_provider_refresh_and_marks_local_matches() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(
            &repository,
            &[
                "remote",
                "add",
                "origin",
                "git@github.com:Acme/Portfolio-Repository.git",
            ],
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
            .expect("local fixture should persist before provider refresh");

        let refresh = ProviderRefresh {
            identities: vec![ProviderIdentity {
                id: "github:jakyeamos".to_string(),
                provider: "github".to_string(),
                login: "jakyeamos".to_string(),
                display_name: Some("Jakye Amos".to_string()),
                organizations: Vec::new(),
                credential_state: "Authenticated".to_string(),
                updated_at: "2026-07-25T12:00:00Z".to_string(),
            }],
            repositories: vec![
                RemoteRepositorySnapshot {
                    id: "github:42".to_string(),
                    provider: "github".to_string(),
                    full_name: "acme/portfolio-repository".to_string(),
                    name: "portfolio-repository".to_string(),
                    owner: "acme".to_string(),
                    html_url: "https://github.com/acme/portfolio-repository".to_string(),
                    default_branch: Some("main".to_string()),
                    archived: false,
                    locality: "Remote only".to_string(),
                    identity_id: "github:jakyeamos".to_string(),
                    last_refreshed_at: "2026-07-25T12:00:00Z".to_string(),
                    pull_requests: Vec::new(),
                    releases: Vec::new(),
                    ci_checks: Vec::new(),
                    ci_branch: None,
                    ci_commit: None,
                    ci_runs: Vec::new(),
                },
                RemoteRepositorySnapshot {
                    id: "github:99".to_string(),
                    provider: "github".to_string(),
                    full_name: "acme/remote-only".to_string(),
                    name: "remote-only".to_string(),
                    owner: "acme".to_string(),
                    html_url: "https://github.com/acme/remote-only".to_string(),
                    default_branch: Some("main".to_string()),
                    archived: false,
                    locality: "Remote only".to_string(),
                    identity_id: "github:jakyeamos".to_string(),
                    last_refreshed_at: "2026-07-25T12:00:00Z".to_string(),
                    pull_requests: Vec::new(),
                    releases: Vec::new(),
                    ci_checks: Vec::new(),
                    ci_branch: None,
                    ci_commit: None,
                    ci_runs: Vec::new(),
                },
            ],
            pull_requests: Vec::new(),
            releases: Vec::new(),
            refreshed_at: "2026-07-25T12:00:00Z".to_string(),
        };
        let snapshot = apply_provider_refresh_at(&database, refresh, None)
            .expect("provider refresh should persist locally");

        assert_eq!(snapshot.provider_status.state, "Ready");
        assert_eq!(snapshot.provider_identities.len(), 1);
        assert_eq!(snapshot.remote_repositories.len(), 2);
        assert_eq!(
            snapshot.remote_repositories[0].full_name,
            "acme/portfolio-repository"
        );
        assert_eq!(snapshot.remote_repositories[0].locality, "Local and remote");
        assert_eq!(
            snapshot.remote_repositories[1].full_name,
            "acme/remote-only"
        );
        assert_eq!(
            snapshot.remote_repositories[1].locality,
            remediation::GITHUB_ONLY_LOCALITY
        );
        assert_eq!(snapshot.remediation.github_only_candidates.len(), 1);
        assert_eq!(
            snapshot.remediation.github_only_candidates[0].last_remediation_task,
            remediation::GITHUB_ONLY_REMEDIATION_TASK
        );
        assert_eq!(snapshot.repositories[0].locality, "Local and remote");
        assert_eq!(
            snapshot.repositories[0].provider_state,
            "GitHub connected as github:jakyeamos"
        );

        let mut persisted = load_store(&database).expect("provider snapshot should reload");
        assert_eq!(persisted.provider_status.state, "Ready");
        assert_eq!(persisted.remote_repositories.len(), 2);
        assert_eq!(persisted.remediation.github_only_candidates.len(), 1);
        let rescanned = scan_and_persist_scoped(&database, &mut persisted, None)
            .expect("local rescan should preserve provider evidence");
        assert_eq!(rescanned.repositories[0].locality, "Local and remote");
        assert_eq!(
            rescanned.repositories[0].provider_state,
            "GitHub connected as github:jakyeamos"
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn discovers_canonical_repository_and_attaches_linked_worktree() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let linked = root.join("linked-worktree");
        git(
            &repository,
            &[
                "worktree",
                "add",
                "-b",
                "agent/test",
                linked.to_str().expect("linked path should be valid UTF-8"),
                "main",
            ],
        );
        git(&repository, &["switch", "-c", "feature/test"]);
        fs::write(repository.join("tracked.txt"), "one\nupdated\n")
            .expect("tracked file should be writable");
        fs::write(repository.join("notes.md"), "first\nsecond\n")
            .expect("untracked file should be writable");

        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let discovered = discover_repositories(&root_config);
        assert_eq!(
            discovered,
            vec![canonical_path(&repository).expect("repository should canonicalize")]
        );

        let snapshot = scan_repository(&discovered[0], None, &[]);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert!(snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.is_primary));
        assert!(snapshot
            .workspaces
            .iter()
            .any(|workspace| !workspace.is_primary));
        assert!(snapshot.workspace.dirty);
        assert_eq!(snapshot.workspace.added, 3);
        assert_eq!(snapshot.workspace.removed, 0);
        assert!(!snapshot.workspace.line_totals_partial);
        assert_eq!(
            snapshot
                .conditions
                .first()
                .map(|condition| condition.kind.as_str()),
            Some("dirty-workspace")
        );
        assert!(snapshot
            .conditions
            .windows(2)
            .all(|window| window[0].priority <= window[1].priority));
        let encoded = serde_json::to_string(&snapshot).expect("snapshot should serialize");
        assert!(!encoded.contains("tracked.txt"));
        assert!(!encoded.contains("notes.md"));

        let dirty = snapshot
            .conditions
            .iter()
            .find(|condition| condition.kind == "dirty-workspace")
            .expect("dirty condition should exist");
        let expected = [ExpectedCondition {
            repository_id: snapshot.id.clone(),
            condition_id: dirty.id.clone(),
            fingerprint: dirty.fingerprint.clone(),
            marked_at: iso_now(),
        }];
        let expected_snapshot = scan_repository(&discovered[0], Some(&snapshot), &expected);
        assert_eq!(
            expected_snapshot
                .conditions
                .iter()
                .find(|condition| condition.kind == "dirty-workspace")
                .map(|condition| condition.status.as_str()),
            Some("Expected")
        );
        assert_eq!(
            transition_fingerprint(&snapshot),
            transition_fingerprint(&expected_snapshot)
        );

        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn detects_interrupted_merge_marker() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        fs::write(repository.join(".git/MERGE_HEAD"), "abc\n")
            .expect("merge marker should be writable");
        assert_eq!(
            interrupted_operation(&repository).as_deref(),
            Some("Merge in progress")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn persists_transition_events_without_duplicate_scans() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
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

        let first =
            scan_and_persist_scoped(&store, &mut state, None).expect("first scan should persist");
        assert_eq!(first.repositories.len(), 1);
        assert_eq!(first.events.len(), 1);

        let second =
            scan_and_persist_scoped(&store, &mut state, None).expect("second scan should persist");
        assert_eq!(second.events.len(), 1);

        fs::write(repository.join("tracked.txt"), "one\nupdated\n")
            .expect("tracked file should be writable");
        let third =
            scan_and_persist_scoped(&store, &mut state, None).expect("changed scan should persist");
        assert_eq!(third.events.len(), 2);
        assert!(third
            .events
            .iter()
            .any(|event| event.kind == "state-transition"));

        let persisted = load_store(&store).expect("persisted state should be readable");
        assert_eq!(persisted.repositories.len(), 1);
        assert_eq!(persisted.events.len(), 2);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn records_allowed_and_rejected_action_preflights() {
        let root = fixture_root();
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
        let state = StoreState {
            roots: vec![root_config.clone()],
            ..StoreState::default()
        };
        save_store(&database, &state).expect("preflight store should be writable");

        let allowed = preflight_action_at(&database, "refresh", None)
            .expect("refresh preflight should be recorded");
        assert!(allowed.allowed);
        assert_eq!(allowed.audit.risk, "read-only");
        assert_eq!(allowed.audit.status, "Preflighted");
        assert_eq!(allowed.audit.target_ids, vec![root_config.id]);

        let rejected = preflight_action_at(&database, "push", None)
            .expect("blocked action should be recorded");
        assert!(!rejected.allowed);
        assert_eq!(rejected.audit.risk, "blocked");
        assert_eq!(rejected.audit.status, "Rejected");
        assert!(rejected
            .audit
            .summary
            .contains("Git mutation and provider writes remain blocked"));

        let persisted = load_store(&database).expect("action audits should persist");
        assert_eq!(persisted.action_audits.len(), 2);
        assert!(persisted
            .action_audits
            .iter()
            .any(|audit| audit.id == allowed.audit.id && audit.status == "Preflighted"));
        assert!(persisted
            .action_audits
            .iter()
            .any(|audit| audit.id == rejected.audit.id && audit.status == "Rejected"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
