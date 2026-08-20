#[test]
    fn release_recommendation_is_advisory_and_fail_closed() {
        let baseline = ReleaseSnapshot {
            id: "github:release-1".to_string(),
            provider: "github".to_string(),
            repository_id: "github:repo-1".to_string(),
            tag: "v1.2.3".to_string(),
            name: "v1.2.3".to_string(),
            target_commit: Some("baseline".to_string()),
            published_at: Some("2026-08-01T12:00:00Z".to_string()),
            draft: false,
            prerelease: false,
            last_refreshed_at: "2026-08-14T12:00:00Z".to_string(),
        };
        let feature_commit = ReleaseCommitSummary {
            sha: "abcdef1234567890".to_string(),
            subject: "feat: show release advice".to_string(),
            category: "Features".to_string(),
            bump: Some("minor".to_string()),
            committed_at: "2026-08-14T11:00:00Z".to_string(),
        };

        for (bump, version, disposition) in [
            ("patch", "v1.2.4", "release_patch"),
            ("minor", "v1.3.0", "release_minor"),
            ("major", "v2.0.0", "release_major"),
        ] {
            let ready = release_recommendation(
                Some(&baseline),
                std::slice::from_ref(&feature_commit),
                Some(bump),
                Some(version),
                Some(ReleaseRuleResult::Passed),
                false,
            );
            assert_eq!(ready.disposition, disposition);
            assert_eq!(ready.label, format!("Release {version} ({bump})"));
            assert_eq!(ready.suggested_version.as_deref(), Some(version));
            assert!(ready.advisory);
        }

        let unconfigured = release_recommendation(
            Some(&baseline),
            std::slice::from_ref(&feature_commit),
            Some("minor"),
            Some("v1.3.0"),
            None,
            false,
        );
        assert_eq!(unconfigured.disposition, "review_required");
        assert_eq!(unconfigured.label, "Review v1.3.0 (minor)");

        let blocked = release_recommendation(
            Some(&baseline),
            std::slice::from_ref(&feature_commit),
            Some("minor"),
            Some("v1.3.0"),
            Some(ReleaseRuleResult::Passed),
            true,
        );
        assert_eq!(blocked.disposition, "do_not_release_yet");
        assert_eq!(blocked.label, "Do not release yet");

        let no_changes = release_recommendation(
            Some(&baseline),
            &[],
            None,
            None,
            Some(ReleaseRuleResult::Passed),
            false,
        );
        assert_eq!(no_changes.disposition, "do_not_release_yet");
        assert!(no_changes
            .basis
            .contains("0 commits since last published tag v1.2.3"));

        let unclassified_commit = ReleaseCommitSummary {
            bump: None,
            category: "Other".to_string(),
            subject: "update release wording".to_string(),
            ..feature_commit
        };
        let ambiguous = release_recommendation(
            Some(&baseline),
            &[unclassified_commit],
            None,
            None,
            Some(ReleaseRuleResult::Passed),
            false,
        );
        assert_eq!(ambiguous.disposition, "review_required");
        assert_eq!(ambiguous.label, "Review release impact");
    }

#[test]
    fn rejects_ignore_patterns_that_escape_repository_scope() {
        assert!(normalize_ignore_patterns(vec!["../secrets".to_string()]).is_err());
        assert!(normalize_ignore_patterns(vec!["nested/cache".to_string()]).is_err());
        assert_eq!(
            normalize_ignore_patterns(vec![
                "/target/".to_string(),
                "target".to_string(),
                "*.tmp".to_string(),
            ])
            .expect("safe patterns should normalize"),
            vec!["*.tmp", "target"]
        );
    }

#[test]
    fn extracts_compact_analytics_metrics_and_keeps_quality_unavailable() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let repository = scan_repository(&repository_path, None, &[]);
        let observed_at = iso_now();
        let sample = analytics_repository_sample(&repository, &observed_at);

        assert_eq!(sample.repository_count, 1);
        assert_eq!(sample.workspace_count, 1);
        assert!(sample.branch_count >= 1);
        assert_eq!(sample.commits_last_30_days, Some(1));
        assert_eq!(sample.findings_total, None);
        assert_eq!(sample.high_severity_findings, None);

        let encoded = serde_json::to_string(&sample).expect("analytics sample should serialize");
        assert!(!encoded.contains(repository_path.to_string_lossy().as_ref()));
        assert!(!encoded.contains("tracked.txt"));

        let mut known_findings = repository.clone();
        known_findings.quality.findings.source = Some(quality::QualitySource::Qr);
        known_findings.quality.findings.observed_at = Some(observed_at.clone());
        let known_sample = analytics_repository_sample(&known_findings, &observed_at);
        assert_eq!(known_sample.findings_total, Some(0));
        assert_eq!(known_sample.high_severity_findings, Some(0));

        fs::remove_dir_all(root).expect("analytics metric fixture should be removable");
    }

#[test]
    fn counts_only_local_commits_in_the_trailing_analytics_window() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        fs::write(repository_path.join("tracked.txt"), "one\ntwo\n")
            .expect("tracked file should be updated");
        git(&repository_path, &["add", "tracked.txt"]);
        git(&repository_path, &["commit", "-m", "Second fixture"]);

        let repository = scan_repository(&repository_path, None, &[]);
        let sample = analytics_repository_sample(&repository, &iso_now());
        assert_eq!(sample.commits_last_30_days, Some(2));

        fs::remove_dir_all(root).expect("commit metric fixture should be removable");
    }

#[test]
    fn deduplicates_unchanged_samples_and_prunes_by_retention() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let database = root.join("registry.db");
        let mut state = StoreState::default();
        state.repositories = vec![scan_repository(&repository_path, None, &[])];
        save_store(&database, &state).expect("analytics fixture state should persist");

        let base = Utc::now() - chrono::Duration::minutes(20);
        let first = base.to_rfc3339();
        let second = (base + chrono::Duration::minutes(5)).to_rfc3339();
        let third = (base + chrono::Duration::minutes(16)).to_rfc3339();
        record_analytics_samples_at(&database, &state, &first)
            .expect("first analytics sample should persist");
        record_analytics_samples_at(&database, &state, &second)
            .expect("unchanged analytics sample should deduplicate");

        let connection = open_store(&database).expect("analytics database should open");
        let fleet_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE repository_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("fleet sample count should be readable");
        assert_eq!(fleet_count, 1);
        let repository_id = state.repositories[0].id.clone();
        let repository_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE repository_id = ?1",
                params![analytics_scope_id(&repository_id)],
                |row| row.get(0),
            )
            .expect("repository sample count should be readable");
        assert_eq!(repository_count, 1);
        let stored_scope: String = connection
            .query_row(
                "SELECT repository_id FROM analytics_samples WHERE repository_id IS NOT NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("repository analytics scope should be readable");
        assert!(!stored_scope.contains(root.to_string_lossy().as_ref()));
        assert_ne!(stored_scope, repository_id);
        drop(connection);

        record_analytics_samples_at(&database, &state, &third)
            .expect("sample outside the deduplication window should persist");
        let analytics = load_analytics_at(&database, None).expect("analytics should load");
        assert_eq!(analytics.portfolio_samples.len(), 2);
        assert_eq!(analytics.repositories[0].samples.len(), 2);

        let old_observed_at = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        let old_payload = serde_json::to_string(&analytics_portfolio_sample(
            &state.repositories,
            &state.remediation,
            &old_observed_at,
        ))
        .expect("old analytics sample should serialize");
        let connection = open_store(&database).expect("analytics database should reopen");
        connection
            .execute(
                "INSERT INTO analytics_samples (id, repository_id, observed_at, payload_json)
                 VALUES (?1, NULL, ?2, ?3)",
                params!["old-analytics-sample", old_observed_at, old_payload],
            )
            .expect("old analytics sample should insert");
        drop(connection);

        prune_analytics_samples(&database, 1).expect("retention pruning should succeed");
        let connection = open_store(&database).expect("pruned analytics database should open");
        let old_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE id = 'old-analytics-sample'",
                [],
                |row| row.get(0),
            )
            .expect("old analytics row count should be readable");
        assert_eq!(old_count, 0);

        fs::remove_dir_all(root).expect("analytics retention fixture should be removable");
    }

#[test]
    fn quality_refresh_records_changed_analytics_and_deduplicates_unchanged_imports() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let database = root.join("registry.db");
        let mut state = StoreState::default();
        state.repositories = vec![scan_repository(&repository_path, None, &[])];
        save_store(&database, &state).expect("quality refresh fixture should persist");

        let apply_import = |state: &mut StoreState, audit_id: &str, score: f64| {
            state.quality.latest_audit_id = Some(audit_id.to_string());
            state.quality.latest_audit_at = Some(iso_now());
            state.quality.audit_status = "Ready".to_string();
            state.repositories[0].quality.maturity.audit_id = Some(audit_id.to_string());
            state.repositories[0].quality.maturity.observed_at = Some(iso_now());
            state.repositories[0].quality.maturity.score = Some(score);
            state.repositories[0].quality.maturity.freshness = QualityFreshness::Fresh;
        };

        refresh_quality_at_with(&database, |state| {
            state.quality.audit_status = "Unavailable".to_string();
            state.quality.latest_audit_id = None;
        })
        .expect("an unavailable import should remain a successful cached refresh");
        let connection = open_store(&database).expect("analytics database should open");
        let unavailable_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE repository_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("unavailable import sample count should be readable");
        assert_eq!(
            unavailable_count, 0,
            "rejected evidence must not create history"
        );
        drop(connection);

        refresh_quality_at_with(&database, |state| apply_import(state, "audit-one", 1.0))
            .expect("accepted quality import should persist");
        refresh_quality_at_with(&database, |state| apply_import(state, "audit-one", 1.0))
            .expect("repeated accepted quality import should persist");
        let analytics = load_analytics_at(&database, None).expect("analytics should load");
        assert_eq!(analytics.portfolio_samples.len(), 1);
        assert_eq!(analytics.repositories[0].samples.len(), 1);

        refresh_quality_at_with(&database, |state| apply_import(state, "audit-two", 2.0))
            .expect("changed accepted quality import should persist");
        let analytics = load_analytics_at(&database, None).expect("changed analytics should load");
        assert_eq!(analytics.portfolio_samples.len(), 2);
        assert_eq!(analytics.repositories[0].samples.len(), 2);
        assert_eq!(
            analytics.portfolio_samples[1].maturity_score,
            Some(2.0),
            "the new audit should be represented by the latest observation"
        );

        fs::remove_dir_all(root).expect("quality refresh analytics fixture should be removable");
    }

#[test]
    fn material_detector_evidence_change_creates_a_new_analytics_observation() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let database = root.join("registry.db");
        let mut state = StoreState::default();
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality.findings.source = Some(quality::QualitySource::Qr);
        repository.quality.findings.observed_at = Some("2026-08-16T03:00:00Z".to_string());
        repository.quality.findings.detector_findings_total = 1;
        repository.quality.findings.detector_actionable_total = 1;
        repository.quality.findings.detector_unreviewed_total = 1;
        repository.quality.findings.enabled_detector_count = 1;
        repository.quality.findings.enabled_rule_count = 3;
        repository
            .quality
            .findings
            .ruleset_fingerprints
            .insert("anti-slop".to_string(), "ruleset-before".to_string());
        state.repositories = vec![repository];
        save_store(&database, &state).expect("analytics fixture state should persist");

        let base = Utc::now() - chrono::Duration::minutes(20);
        let first = base.to_rfc3339();
        let second = (base + chrono::Duration::minutes(5)).to_rfc3339();
        let rerun = (base + chrono::Duration::minutes(6)).to_rfc3339();
        record_analytics_samples_at(&database, &state, &first)
            .expect("initial detector evidence should persist");

        let findings = &mut state.repositories[0].quality.findings;
        findings.detector_findings_total = 2;
        findings.detector_actionable_total = 2;
        findings
            .ruleset_fingerprints
            .insert("anti-slop".to_string(), "ruleset-after".to_string());
        record_analytics_samples_at(&database, &state, &second)
            .expect("material detector evidence change should persist");
        record_analytics_samples_at(&database, &state, &rerun)
            .expect("unchanged changed-evidence rerun should deduplicate");

        let analytics = load_analytics_at(&database, None).expect("analytics should load");
        assert_eq!(analytics.portfolio_samples.len(), 2);
        assert_eq!(analytics.repositories[0].samples.len(), 2);
        assert_eq!(
            analytics.repositories[0].samples[0].detector_findings_total,
            Some(1)
        );
        assert_eq!(
            analytics.repositories[0].samples[1].detector_findings_total,
            Some(2)
        );
        assert_ne!(
            analytics.repositories[0].samples[0].quality_evidence_fingerprint,
            analytics.repositories[0].samples[1].quality_evidence_fingerprint
        );

        fs::remove_dir_all(root).expect("analytics evidence fixture should be removable");
    }

#[test]
    fn adapts_v1_samples_into_governed_v2_metrics() {
        let payload = r#"{"observed_at":"2026-08-13T00:00:00Z","repository_count":1,"workspace_count":1,"branch_count":1,"active_condition_count":2,"dirty_workspace_count":1,"unsynced_workspace_count":0,"active_workspace_count":1,"interrupted_workspace_count":0,"idle_workspace_count":0,"unknown_workspace_count":0,"ahead_commit_count":75,"behind_commit_count":3,"commits_last_30_days":8000,"ci_readiness_score":3.0,"maturity_score":2.5,"findings_total":4,"high_severity_findings":1,"ci_readiness_scored_repository_count":1,"maturity_scored_repository_count":1,"findings_repository_count":1,"release_rule_repository_count":1,"release_ready_repository_count":0,"quality_freshness":"Fresh"}"#;
        let legacy: AnalyticsMetricSample =
            serde_json::from_str(payload).expect("v1 sample should decode");
        let adapted = adapt_analytics_sample(legacy);
        assert_eq!(
            adapted.metrics["git.commits.trailing_30_days"],
            Some(8000.0)
        );
        assert_eq!(adapted.metrics["git.ahead_commits"], Some(75.0));
        assert_eq!(adapted.metrics["quality.maturity_score"], Some(2.5));
        assert_eq!(adapted.metrics["workspaces.activity.active"], Some(1.0));
        assert_eq!(adapted.metrics["remediation.actions.open"], None);
        assert_eq!(adapted.metrics["remediation.progress_percent"], None);
    }

#[test]
    fn analytics_metric_catalog_rejects_semantic_axis_mismatches() {
        let catalog = analytics_metric_catalog();
        assert!(!metric_axis_compatible(
            &catalog,
            &[
                "git.commits.trailing_30_days".to_string(),
                "git.ahead_commits".to_string()
            ]
        ));
        assert!(metric_axis_compatible(
            &catalog,
            &[
                "workspaces.dirty".to_string(),
                "workspaces.unsynced".to_string()
            ]
        ));
        assert!(metric_axis_compatible(
            &catalog,
            &[
                "workspaces.activity.active".to_string(),
                "workspaces.activity.interrupted".to_string(),
                "workspaces.activity.idle".to_string(),
                "workspaces.activity.unknown".to_string()
            ]
        ));
        assert!(metric_axis_compatible(
            &catalog,
            &[
                "remediation.actions.open".to_string(),
                "remediation.actions.blocked".to_string(),
                "remediation.actions.verified".to_string()
            ]
        ));
    }

#[test]
    fn analytics_range_caps_to_retention_and_rejects_zero() {
        assert_eq!(
            validated_analytics_range(Some(90), 30).expect("range should cap"),
            30
        );
        assert!(validated_analytics_range(Some(0), 90).is_err());
    }

#[test]
    fn analytics_views_are_local_validated_and_curated_is_protected() {
        let root = fixture_root();
        let database = root.join("registry.db");
        save_store(&database, &StoreState::default())
            .expect("analytics view store should initialize");
        let now = iso_now();
        let view = AnalyticsView {
            schema_version: "pronto-analytics-view/v1".to_string(),
            id: "quality-view".to_string(),
            name: "Quality view".to_string(),
            builtin: false,
            is_default: true,
            filters: AnalyticsViewFilters {
                range_days: 30,
                repository_ids: vec![],
                group_ids: vec![],
                product_ids: vec![],
                freshness: "all".to_string(),
            },
            widgets: vec![AnalyticsWidgetConfig {
                id: "quality".to_string(),
                title: "Quality".to_string(),
                metric_ids: vec![
                    "quality.maturity_score".to_string(),
                    "quality.evidence_score".to_string(),
                ],
                chart_type: "scatter".to_string(),
                grouping: "repository".to_string(),
                width: 2,
                height: 1,
                order: 0,
            }],
            created_at: now.clone(),
            updated_at: now,
        };
        let views =
            save_analytics_view_at(&database, view).expect("valid analytics view should save");
        assert_eq!(
            views
                .iter()
                .find(|candidate| candidate.is_default)
                .map(|candidate| candidate.id.as_str()),
            Some("quality-view")
        );
        assert!(validate_analytics_view(&builtin_analytics_view(30), 90).is_err());
        assert!(set_default_analytics_view_at(&database, "curated")
            .expect("curated may become default")
            .iter()
            .any(|candidate| candidate.id == "curated" && candidate.is_default));
        fs::remove_dir_all(root).expect("analytics view fixture should be removable");
    }

#[test]
    fn workspace_role_map_requires_explicit_production_refs() {
        let entry = serde_json::json!({
            "repository_role": "production_product"
        });
        let error = workspace_policy_for_role(
            "example-repository",
            entry.as_object().expect("role entry should be an object"),
        )
        .expect_err("production role without refs must fail closed");
        assert!(error.contains("release_ref"));
    }
