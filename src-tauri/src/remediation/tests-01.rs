use super::*;

use crate::core::{
        ActivitySignal, BranchSummary, Condition, RemoteRepositorySnapshot, SubmoduleSummary,
        WorkspaceActivity, WorkspaceProvenance, WorkspaceSummary,
    };

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture_repository(name: &str) -> RepositorySnapshot {
        RepositorySnapshot {
            id: format!("repo-{name}"),
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            locality: "Local".to_string(),
            lifecycle: "Active".to_string(),
            lifecycle_candidate: "Active".to_string(),
            remote_url: Some(format!("https://github.com/example/{name}")),
            provider_state: "GitHub connected".to_string(),
            branch: "dev".to_string(),
            default_branch: Some("dev".to_string()),
            target_branch: Some("dev".to_string()),
            target_branch_configured: false,
            workspace: WorkspaceSummary {
                id: format!("workspace-{name}"),
                path: format!("/tmp/{name}"),
                is_primary: true,
                branch: "dev".to_string(),
                status_available: true,
                status_error: None,
                dirty: false,
                added: 0,
                removed: 0,
                line_totals_partial: false,
                sync_state: "Synced".to_string(),
                remote_freshness: "Fresh".to_string(),
                ahead: 0,
                behind: 0,
                upstream: Some("origin/dev".to_string()),
                operation: None,
                last_commit: Some("abc".to_string()),
                last_commit_at: Some(Utc::now().to_rfc3339()),
                last_activity_at: None,
                integration_state: "Ready".to_string(),
                target_branch: Some("dev".to_string()),
                target_confidence: "High".to_string(),
                role: "Primary".to_string(),
                role_confidence: "High".to_string(),
                activity: WorkspaceActivity::default(),
                provenance: WorkspaceProvenance::default(),
                sync_detail: None,
            },
            workspaces: Vec::new(),
            branches: Vec::<BranchSummary>::new(),
            branch_lifecycle: Default::default(),
            submodules: Vec::new(),
            pull_requests: Vec::new(),
            releases: Vec::new(),
            quality: Default::default(),
            project_compass: Default::default(),
            custody: Default::default(),
            release_rule: None,
            release_recipe: None,
            confirmed_release_version: None,
            ai_permission: "Blocked".to_string(),
            conditions: Vec::new(),
            last_scan_at: Utc::now().to_rfc3339(),
            last_fetch_at: Some(Utc::now().to_rfc3339()),
            last_activity_at: None,
        }
    }

fn fixture_action(
        id: &str,
        domain: &str,
        priority: &str,
        weight: u64,
        status: &str,
    ) -> RemediationAction {
        RemediationAction {
            id: id.to_string(),
            stable_key: id.to_string(),
            repository_id: "repo".to_string(),
            domain: domain.to_string(),
            title: format!("Action {id}"),
            summary: String::new(),
            severity: "test".to_string(),
            priority: priority.to_string(),
            weight,
            status: status.to_string(),
            acceptance_criteria: Vec::new(),
            evidence: vec![evidence(
                "Pronto",
                "Fixture evidence",
                "Passed",
                "Fresh",
                Some("2026-07-29T12:00:00Z"),
                None,
                "Fixture evidence is current.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some("refresh-1".to_string()),
            updated_at: "2026-07-29T12:00:00Z".to_string(),
            completed_at: (status == "verified").then(|| "2026-07-29T12:00:00Z".to_string()),
            notes: None,
        }
    }

fn fixture_plan(
        repository_name: &str,
        status: &str,
        actions: Vec<RemediationAction>,
    ) -> RemediationPlan {
        let repository_id = format!("repo-{repository_name}");
        let mut goal =
            goal_definition("active_maintained").expect("fixture goal should be supported");
        goal.source = "repository_contract".to_string();
        goal.confidence = "High".to_string();
        goal.reason = "Fixture repository is actively maintained.".to_string();
        let coverage = Vec::new();
        let explanation = build_remediation_explanation(&goal, &actions, &coverage);
        RemediationPlan {
            schema_version: REMEDIATION_SCHEMA.to_string(),
            id: format!("plan-{repository_name}"),
            repository_id,
            repository_name: repository_name.to_string(),
            repository_path: format!("/tmp/{repository_name}"),
            generated_at: "2026-07-29T12:00:00Z".to_string(),
            source_refresh_id: Some("refresh-1".to_string()),
            goal,
            current_stage: actions
                .first()
                .map(|action| action.domain.clone())
                .unwrap_or_else(|| "complete".to_string()),
            status: status.to_string(),
            integration_only_remaining: integration_only_remaining(&actions),
            progress: calculate_progress(&actions),
            coverage,
            explanation,
            tracks: build_tracks(&actions),
            actions,
        }
    }

#[test]
    fn project_compass_blockers_and_drift_share_one_remediation_action() {
        let mut repository = fixture_repository("compass-grouping");
        repository.project_compass.status = "Ready".to_string();
        repository.project_compass.contract_path =
            "/tmp/compass-grouping/.project-compass/contract.json".to_string();
        repository.project_compass.updated_at = Some("2026-08-03T18:04:36Z".to_string());
        repository.project_compass.open_blockers = 1;
        repository.project_compass.open_drift = 1;
        let mut seeds = Vec::new();

        add_project_compass_seeds(&repository, &mut seeds);

        assert_eq!(seeds.len(), 1);
        let seed = &seeds[0];
        assert_eq!(seed.stable_key, PROJECT_COMPASS_OPEN_ITEMS_KEY);
        assert_eq!(seed.weight, severity_weight("warning"));
        assert_eq!(seed.evidence.len(), 2);
        assert!(seed.summary.contains("1 open blocker(s)"));
        assert!(seed.summary.contains("1 open drift item(s)"));
    }

#[test]
    fn does_not_exclude_repositories_without_policy_entries() {
        assert!(!is_excluded_repository(&fixture_repository(
            "soundscape-app"
        )));
        assert!(!is_excluded_repository(&fixture_repository("tenure")));
        assert!(!is_excluded_repository(&fixture_repository("pronto")));
    }

#[test]
    fn reviewed_non_actionable_findings_do_not_seed_remediation_actions() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-finding-disposition-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("disposition fixture should be writable");
        fs::write(
            contract_dir.join("quality-finding-dispositions.json"),
            r#"{
  "schema_version": "pronto-quality-finding-dispositions/v1",
  "updated_at": "2026-07-29T12:00:00Z",
  "dispositions": [
    {
      "fingerprint": "fp-reviewed",
      "status": "false_positive",
      "reason": "The symbol is referenced by the renderer contract.",
      "reviewer": "fixture-reviewer",
      "reviewed_at": "2026-07-29T12:00:00Z",
      "evidence": ["src/renderer/src/types.ts:1"]
    }
  ]
}"#,
        )
        .expect("disposition contract should be writable");

        let mut repository = fixture_repository("disposition-filter");
        repository.path = root.to_string_lossy().to_string();
        repository.quality.findings.disposition_status = "Ready".to_string();
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: root.join(".quality-runner/runs/qr-run"),
            observed_at: Some("2026-07-29T12:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-reviewed".to_string(),
                fingerprint: Some("fp-reviewed".to_string()),
                group_key: "dead-code:reviewed".to_string(),
                category: "dead-code".to_string(),
                pack: Some("maintainability".to_string()),
                severity: "warning".to_string(),
                title: "Unused export".to_string(),
                summary: "A detector reported an unused export.".to_string(),
                file: Some("src/renderer/src/types.ts".to_string()),
                line: Some(1),
                verification: None,
                report_path: root
                    .join(".quality-runner/runs/qr-run/findings.json")
                    .to_string_lossy()
                    .to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);

        assert!(seeds.is_empty());

        repository.quality.findings.disposition_status = "Unreconcilable".to_string();
        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].related_finding_ids, vec!["finding-reviewed"]);
        let _ = fs::remove_dir_all(root);
    }

#[test]
    fn accepted_risk_debloat_gate_keeps_a_remediation_link() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-debloat-risk-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("disposition fixture should be writable");
        fs::write(
            contract_dir.join("quality-finding-dispositions.json"),
            r#"{
  "schema_version": "pronto-quality-finding-dispositions/v1",
  "updated_at": "2026-08-04T02:00:00Z",
  "dispositions": [
    {
      "fingerprint": "fp-debloat-risk",
      "status": "accepted_risk",
      "reason": "The owner accepts the current size pending a later redesign.",
      "reviewer": "fixture-reviewer",
      "reviewed_at": "2026-08-04T02:00:00Z",
      "evidence": ["src/router.rs:1"]
    }
  ]
}"#,
        )
        .expect("disposition contract should be writable");

        let mut repository = fixture_repository("debloat-risk");
        repository.path = root.to_string_lossy().to_string();
        repository.quality.findings.disposition_status = "Ready".to_string();
        repository
            .quality
            .findings
            .category_counts
            .insert("debloat".to_string(), 1);
        repository
            .quality
            .findings
            .actionable_category_counts
            .insert("debloat".to_string(), 1);
        repository.quality.gates.push(quality::QualityGate {
            id: "debloat".to_string(),
            label: "Repository debloat review".to_string(),
            status: quality::QualityGateStatus::Blocked,
            freshness: quality::QualityFreshness::Fresh,
            evidence: Vec::new(),
        });
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: root.join(".quality-runner/runs/qr-run"),
            observed_at: Some("2026-08-04T02:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-debloat-risk".to_string(),
                fingerprint: Some("fp-debloat-risk".to_string()),
                group_key: "debloat|debloat candidate review|unknown".to_string(),
                category: "debloat".to_string(),
                pack: None,
                severity: "warning".to_string(),
                title: "Review oversized source files".to_string(),
                summary: "A source file exceeds the debloat review threshold.".to_string(),
                file: Some("src/router.rs".to_string()),
                line: Some(1),
                verification: None,
                report_path: root
                    .join(".quality-runner/runs/qr-run/findings.json")
                    .to_string_lossy()
                    .to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        assert!(
            seeds.is_empty(),
            "accepted risk should not remain a leaf action"
        );
        add_debloat_gate_seed(&repository, Some(&run), &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].stable_key, DEBLOAT_GATE_ACTION_KEY);
        assert!(seeds[0].summary.contains("1 unresolved signal(s)"));
        assert!(seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("separately authorized")));
        let _ = fs::remove_dir_all(root);
    }

#[test]
    fn debloat_gate_does_not_duplicate_a_leaf_finding_action() {
        let mut repository = fixture_repository("debloat-leaf");
        repository.quality.findings.disposition_status = "Missing".to_string();
        repository.quality.gates.push(quality::QualityGate {
            id: "debloat".to_string(),
            label: "Repository debloat review".to_string(),
            status: quality::QualityGateStatus::Blocked,
            freshness: quality::QualityFreshness::Fresh,
            evidence: Vec::new(),
        });
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: PathBuf::from("/tmp/qr-run"),
            observed_at: Some("2026-08-04T02:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-debloat".to_string(),
                fingerprint: None,
                group_key: "debloat|debloat candidate review|unknown".to_string(),
                category: "debloat".to_string(),
                pack: None,
                severity: "warning".to_string(),
                title: "Review oversized source files".to_string(),
                summary: "A source file exceeds the debloat review threshold.".to_string(),
                file: Some("src/router.rs".to_string()),
                line: Some(1),
                verification: None,
                report_path: "/tmp/findings.json".to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        add_debloat_gate_seed(&repository, Some(&run), &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert!(seeds[0]
            .stable_key
            .starts_with(DEBLOAT_GROUP_CATEGORY_PREFIX));
    }

#[test]
    fn grouped_qr_occurrences_do_not_multiply_plan_weight() {
        let mut repository = fixture_repository("grouped-findings");
        repository.quality.findings.disposition_status = "Missing".to_string();
        let finding = ParsedFinding {
            id: "finding-one".to_string(),
            fingerprint: None,
            group_key: "developer-experience:setup-path".to_string(),
            category: "developer-experience".to_string(),
            pack: Some("maintainability".to_string()),
            severity: "high".to_string(),
            title: "Remove machine-specific setup paths".to_string(),
            summary: "A setup path is machine-specific.".to_string(),
            file: Some("README.md".to_string()),
            line: Some(10),
            verification: None,
            report_path: "/tmp/findings.json".to_string(),
        };
        let mut second = finding.clone();
        second.id = "finding-two".to_string();
        second.line = Some(20);
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: PathBuf::from("/tmp/qr-run"),
            observed_at: Some("2026-07-29T12:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![finding, second],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].weight, severity_weight("high"));
        assert_eq!(seeds[0].related_finding_ids.len(), 2);
        assert!(seeds[0].summary.contains("2 finding(s)"));
    }

#[test]
    fn resolved_actions_leave_the_active_queue_and_retain_history() {
        let mut run = empty_run();
        run.plans = vec![fixture_plan(
            "pronto",
            "complete",
            vec![fixture_action(
                "verified",
                "verification",
                "P2",
                1,
                "verified",
            )],
        )];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert!(run.plans.is_empty());
        assert_eq!(run.closures.len(), 1);
        assert_eq!(run.closures[0].repository_name, "pronto");
        assert_eq!(run.closures[0].disposition, "verified");
        assert_eq!(run.closures[0].resolved_action_count, 1);
        let retained_policy = run.closures[0]
            .maturity_policy
            .as_ref()
            .expect("resolved history should retain maturity policy");
        assert_eq!(retained_policy.minimum_closure_score, 3.0);
        assert_eq!(retained_policy.ideal_score, 4.0);
        assert_eq!(
            run.closures[0].last_evidence_at.as_deref(),
            Some("2026-07-29T12:00:00Z")
        );
    }

#[test]
    fn resolved_history_stays_out_of_queue_until_new_evidence_arrives() {
        let mut repository = fixture_repository("closure-retention");
        repository.last_scan_at = "2026-08-03T12:00:00Z".to_string();
        repository.last_fetch_at = Some("2026-08-03T12:00:00Z".to_string());
        repository.workspace.last_commit_at = Some("2026-08-03T12:00:00Z".to_string());
        let current = build_plan(
            &repository,
            None,
            Some("refresh-1"),
            "2026-08-03T12:30:00Z",
            None,
        );
        let mut previous = empty_run();
        previous.closures = vec![closure_from_plan(
            &current,
            "2026-08-03T13:00:00Z",
            Some("refresh-1"),
        )];

        let retained = rebuild_run(&[repository.clone()], &previous, Some("refresh-1"));

        assert!(retained.plans.is_empty());
        assert_eq!(retained.closures.len(), 1);

        repository.last_scan_at = "2026-08-03T14:00:00Z".to_string();
        let reopened = rebuild_run(&[repository], &retained, Some("refresh-1"));

        assert_eq!(reopened.plans.len(), 1);
        assert_eq!(reopened.plans[0].repository_name, "closure-retention");
    }
