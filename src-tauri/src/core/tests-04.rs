#[test]
    fn remediation_execution_gate_enforces_workspace_ownership_coordination() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        let active = WorkspaceActivity {
            state: "Active".to_string(),
            confidence: "High".to_string(),
            signals: vec![ActivitySignal {
                source: "fixture".to_string(),
                summary: "Workspace owner is active".to_string(),
                confidence: "High".to_string(),
                observed_at: iso_now(),
                process_name: None,
                process_id: None,
                started_at: None,
                working_directory: Some(repository_path.to_string_lossy().to_string()),
            }],
            manifest: None,
        };
        repository.workspace.activity = active.clone();
        for workspace in &mut repository.workspaces {
            if workspace.id == repository.workspace.id {
                workspace.activity = active.clone();
            }
        }

        let handoff = remediation_handoff_check_for_repository(&repository, None)
            .expect("owned workspace should remain inspectable");
        assert!(!handoff.ready);
        assert!(handoff.ownership_coordination_required);

        let gate = remediation_execution_gate_for_repository(&repository, None, None, None)
            .expect("owned workspace should produce a structured execution gate");
        assert!(!gate.ready);
        assert_eq!(gate.status, "blocked");
        assert!(gate
            .blockers
            .iter()
            .any(|blocker| blocker.kind == "ownership_coordination_required"));
        assert!(gate
            .blocked_operations
            .contains(&"mutate_workspace".to_string()));

        fs::remove_dir_all(root).expect("ownership-gate fixture should be removable");
    }

#[test]
    fn remediation_execution_gate_distinguishes_unavailable_ownership_evidence() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        let uncertain = WorkspaceActivity {
            state: "Unknown".to_string(),
            confidence: "Low".to_string(),
            signals: vec![ActivitySignal {
                source: "Process".to_string(),
                summary: "Activity state uncertain".to_string(),
                confidence: "Low".to_string(),
                observed_at: iso_now(),
                process_name: None,
                process_id: None,
                started_at: None,
                working_directory: None,
            }],
            manifest: None,
        };
        repository.workspace.activity = uncertain.clone();
        for workspace in &mut repository.workspaces {
            if workspace.id == repository.workspace.id {
                workspace.activity = uncertain.clone();
            }
        }

        let gate = remediation_execution_gate_for_repository(&repository, None, None, None)
            .expect("uncertain ownership should produce a structured execution gate");

        assert!(!gate.ready);
        assert!(gate
            .blockers
            .iter()
            .any(|blocker| blocker.kind == "ownership_evidence_unavailable"));
        assert!(!gate
            .blockers
            .iter()
            .any(|blocker| blocker.kind == "ownership_coordination_required"));
        assert!(gate
            .workspace_checks
            .iter()
            .all(|check| check.ownership_status == "evidence_unavailable"));

        fs::remove_dir_all(root).expect("ownership-evidence fixture should be removable");
    }

#[test]
    fn remediation_execution_gate_reports_unavailable_secondary_workspace_as_partial() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.workspace.activity = WorkspaceActivity::default();
        for workspace in &mut repository.workspaces {
            workspace.activity = WorkspaceActivity::default();
        }
        if repository.workspaces.is_empty() {
            repository.workspaces.push(repository.workspace.clone());
        }
        let mut unavailable = repository.workspace.clone();
        unavailable.id = "workspace-unavailable".to_string();
        unavailable.path = root.join("missing-workspace").to_string_lossy().to_string();
        unavailable.branch = "feature/unavailable".to_string();
        repository.workspaces.push(unavailable);

        let gate = remediation_execution_gate_for_repository(&repository, None, None, None)
            .expect("unavailable path should be a structured blocker");

        assert!(!gate.ready);
        assert_eq!(
            gate.status, "partially_blocked",
            "unexpected blockers: {:#?}",
            gate.blockers
        );
        assert!(gate
            .blockers
            .iter()
            .any(|blocker| blocker.kind == "path_unavailable"));
        assert!(gate
            .blocked_operations
            .contains(&"inspect_workspace".to_string()));

        fs::remove_dir_all(root).expect("partial-gate fixture should be removable");
    }

#[test]
    fn agent_sync_attention_matches_renderer_synced_state() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.sync_state = "Synced".to_string();
        assert!(!workspace_requires_sync_attention(&workspace));
        workspace.sync_state = "Ahead by 1".to_string();
        assert!(workspace_requires_sync_attention(&workspace));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn workspace_sync_detail_exposes_expiry_reason_and_scoped_refresh() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.sync_state = "Behind by 1".to_string();
        workspace.upstream = Some("origin/main".to_string());
        workspace.ahead = 0;
        workspace.behind = 1;

        let detail = workspace_sync_detail(
            &workspace,
            "/tmp/pronto with spaces",
            "2026-07-30T12:00:00Z",
        )
        .expect("unsynced workspace should have detail");

        assert_eq!(
            detail.evidence_observed_at.as_deref(),
            Some("2026-07-30T12:00:00Z")
        );
        assert_eq!(
            detail.evidence_expires_at.as_deref(),
            Some("2026-08-01T12:00:00Z")
        );
        assert!(detail.reason.contains("behind by 1 commit"));
        assert_eq!(
            detail.scoped_refresh_command,
            "pronto refresh '/tmp/pronto with spaces' --json"
        );
        assert!(detail
            .authorization
            .contains("does not pull, push, merge, rebase"));

        workspace.sync_detail = Some(detail.clone());
        let evidence = agent_workspace_sync_evidence(&workspace);
        assert!(evidence.iter().any(|item| {
            item.label == "Evidence expires"
                && item.value.as_deref() == Some("2026-08-01T12:00:00Z")
        }));
        let action = agent_next_action(&AgentAttentionItem {
            id: "workspace-sync".to_string(),
            repository_id: "repository:pronto".to_string(),
            repository_name: "pronto".to_string(),
            repository_path: "/tmp/pronto with spaces".to_string(),
            workspace_id: Some(workspace.id.clone()),
            workspace_path: Some(workspace.path.clone()),
            category: "synchronization".to_string(),
            severity: "warning".to_string(),
            status: workspace.sync_state.clone(),
            freshness: None,
            summary: "Workspace is unsynced".to_string(),
            evidence,
        });
        assert!(action
            .next_safe_step
            .contains("pronto refresh '/tmp/pronto with spaces' --json"));
        assert!(action.authorization.contains("read-only local Git scan"));

        workspace.sync_state = "Synced".to_string();
        assert!(workspace_sync_detail(&workspace, "/tmp/pronto", "2026-07-30T12:00:00Z").is_none());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

#[test]
    fn next_report_is_bounded_ranked_and_repository_aware() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();
        let repository_path = snapshot.repositories[0].path.clone();
        snapshot.repositories[0].quality.gates.clear();
        snapshot.repositories[0].quality.findings.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0].quality.ingestion_status = "Available".to_string();
        snapshot.repositories[0].conditions = vec![
            Condition {
                id: "routine-condition".to_string(),
                kind: "branch".to_string(),
                title: "Routine condition".to_string(),
                summary: "Routine evidence needs review".to_string(),
                priority: 4,
                status: "Active".to_string(),
                fingerprint: "routine".to_string(),
                rule: "fixture".to_string(),
                evidence: Vec::new(),
                missing: Vec::new(),
                confidence: Some("High".to_string()),
                freshness: Some("Fresh".to_string()),
            },
            Condition {
                id: "urgent-condition".to_string(),
                kind: "branch".to_string(),
                title: "Urgent condition".to_string(),
                summary: "Urgent evidence needs review".to_string(),
                priority: 1,
                status: "Active".to_string(),
                fingerprint: "urgent".to_string(),
                rule: "fixture".to_string(),
                evidence: Vec::new(),
                missing: Vec::new(),
                confidence: Some("High".to_string()),
                freshness: Some("Fresh".to_string()),
            },
        ];

        let report = agent_next_report(&snapshot, Some(&repository_path), "fleet", 1)
            .expect("next report should resolve the repository");

        assert_eq!(report.schema_version, AGENT_NEXT_SCHEMA);
        assert_eq!(report.summary.repository_count, 1);
        assert_eq!(
            report.current_repository.as_ref().map(|item| &item.id),
            Some(&repository_id)
        );
        assert!(report.attention_total >= 2);
        assert_eq!(report.attention.len(), 1);
        assert_eq!(
            report.attention[0].id,
            format!("{repository_id}:condition:urgent-condition")
        );
        assert_eq!(report.actions.len(), 1);
        assert_eq!(report.actions[0].recommended_projection, "repo");
        assert!(report.actions[0]
            .authorization
            .contains("explicit authorization"));

        fs::remove_dir_all(root).expect("next report fixture should be removable");
    }

#[test]
    fn fold_preview_preserves_unpublished_branch_and_requires_live_verification() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "dev"]);
        git(&repository, &["switch", "-c", "feature/fold-preview"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "Feature preview"]);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        snapshot.repositories[0].workspaces[0].activity.state =
            "Interrupted with unpushed commits".to_string();
        snapshot.repositories[0].workspaces[0].activity.confidence = "Medium".to_string();
        let repository_path = snapshot.repositories[0].path.clone();

        let report = agent_fold_preview_report_with_cursor_and_merge_preview(
            &snapshot,
            Some(&repository_path),
            Some("dev"),
            "repository:fixture",
            10,
            None,
            true,
        )
        .expect("fold preview should resolve the repository");

        assert_eq!(report.schema_version, AGENT_FOLD_PREVIEW_SCHEMA);
        assert_eq!(report.repository_count, 1);
        assert_eq!(report.branch_total, 3);
        assert_eq!(report.candidate_total, 2);
        assert_eq!(report.returned_count, 2);
        assert!(!report.has_more);
        assert!(report.next_cursor.is_none());
        assert_eq!(report.candidates.len(), 2);
        let candidate = report
            .candidates
            .iter()
            .find(|candidate| candidate.source_branch == "feature/fold-preview")
            .expect("feature branch should be in fold preview");
        assert_eq!(candidate.target_branch.as_deref(), Some("dev"));
        assert_eq!(candidate.decision, "preserve_unpublished");
        assert_eq!(candidate.integration_state, "Integration eligible");
        assert_eq!(candidate.dirty, Some(false));
        assert!(candidate.merge_preview.is_some());
        assert!(report.live_verification_required);
        assert!(candidate.authorization.contains("Preview only"));

        fs::remove_dir_all(root).expect("fold preview fixture should be removable");
    }

#[test]
    fn fold_preview_paginates_all_candidates_without_duplicates() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "dev"]);
        for index in 0..5 {
            let branch = format!("feature/fold-pagination-{index}");
            git(&repository, &["switch", "-c", branch.as_str()]);
            fs::write(
                repository.join(format!("feature-{index}.txt")),
                format!("feature {index}\n"),
            )
            .expect("pagination feature file should be writable");
            let file = format!("feature-{index}.txt");
            git(&repository, &["add", file.as_str()]);
            git(
                &repository,
                &[
                    "commit",
                    "-m",
                    format!("Pagination feature {index}").as_str(),
                ],
            );
            git(&repository, &["switch", "dev"]);
        }
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("pagination fixture portfolio should scan");
        let repository_path = snapshot.repositories[0].path.clone();
        let mut cursor = None;
        let mut first_cursor = None;
        let mut candidate_keys = Vec::new();
        let mut page_count = 0;
        let mut candidate_total = None;

        loop {
            let report = agent_fold_preview_report_with_cursor_and_merge_preview(
                &snapshot,
                Some(&repository_path),
                Some("dev"),
                "repository:pagination-fixture",
                2,
                cursor.as_deref(),
                false,
            )
            .expect("paginated fold preview should resolve");
            page_count += 1;
            candidate_total.get_or_insert(report.candidate_total);
            assert_eq!(Some(report.candidate_total), candidate_total);
            assert_eq!(report.returned_count, report.candidates.len());
            assert!(report.returned_count <= 2);
            candidate_keys.extend(report.candidates.iter().map(|candidate| {
                format!("{}::{}", candidate.repository_name, candidate.source_branch)
            }));
            if !report.has_more {
                assert!(report.next_cursor.is_none());
                assert_eq!(candidate_keys.len(), report.candidate_total);
                break;
            }
            let next_cursor = report
                .next_cursor
                .clone()
                .expect("a non-terminal page should return a cursor");
            if first_cursor.is_none() {
                first_cursor = Some(next_cursor.clone());
            }
            cursor = Some(next_cursor);
            assert!(page_count < 10, "pagination should terminate");
        }

        let unique_keys = candidate_keys.iter().collect::<BTreeSet<_>>();
        assert_eq!(candidate_keys.len(), unique_keys.len());
        assert!(page_count >= 2);
        let first_cursor = first_cursor.expect("the first page should return a cursor");
        let mut reread_snapshot = snapshot.clone();
        reread_snapshot.generated_at = "different-presentation-time".to_string();
        let reread = agent_fold_preview_report_with_cursor_and_merge_preview(
            &reread_snapshot,
            Some(&repository_path),
            Some("dev"),
            "repository:pagination-fixture",
            2,
            Some(first_cursor.as_str()),
            false,
        )
        .expect("a cursor should survive a new presentation timestamp");
        assert_eq!(reread.returned_count, 2);
        let mismatch = agent_fold_preview_report_with_cursor_and_merge_preview(
            &snapshot,
            Some(&repository_path),
            Some("main"),
            "repository:pagination-fixture",
            2,
            Some(first_cursor.as_str()),
            false,
        )
        .expect_err("a cursor must not cross target scopes");
        assert!(mismatch.contains("does not match this snapshot or scope"));

        fs::remove_dir_all(root).expect("pagination fixture should be removable");
    }

#[test]
    fn doctor_report_blocks_stale_and_unavailable_snapshot() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = snapshot
            .repositories
            .first_mut()
            .expect("fixture repository should be present");
        repository.last_scan_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        repository.workspaces[0].path =
            root.join("missing-workspace").to_string_lossy().to_string();

        let report = agent_doctor_report(&snapshot, &store, 60, "repository:fixture");

        assert_eq!(report.schema_version, AGENT_DOCTOR_SCHEMA);
        assert!(!report.ready);
        assert_eq!(report.status, "Blocked");
        assert_eq!(report.stale_repository_ids.len(), 1);
        assert!(report.invalid_scan_repository_ids.is_empty());
        assert!(report
            .unavailable_paths
            .iter()
            .any(|path| path.ends_with("missing-workspace")));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Blocked"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "paths" && check.status == "Blocked"));
        assert!(report.authorization.contains("does not refresh"));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }
