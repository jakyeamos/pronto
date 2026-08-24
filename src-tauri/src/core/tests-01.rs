use super::*;

use std::fs;

use std::os::unix::fs::symlink;

use std::os::unix::fs::PermissionsExt;

use std::process::Output;

use std::sync::atomic::{AtomicU64, Ordering};

use std::sync::{Arc, Barrier};

use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

fn git(path: &Path, arguments: &[&str]) -> Output {
        let output = git_process(path)
            .args(arguments)
            .output()
            .expect("git should be installed for Pronto core tests");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

fn fixture_root() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-core-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("test root should be creatable");
        root
    }

fn fixture_repository(root: &Path) -> PathBuf {
        fixture_repository_named(root, "portfolio-repository")
    }

fn fixture_repository_named(root: &Path, name: &str) -> PathBuf {
        let repository = root.join(name);
        fs::create_dir_all(&repository).expect("fixture repository should be creatable");
        git(&repository, &["init", "-b", "main"]);
        git(
            &repository,
            &["config", "user.email", "pronto-tests@example.com"],
        );
        git(&repository, &["config", "user.name", "Pronto Tests"]);
        fs::write(repository.join("tracked.txt"), "one\n")
            .expect("tracked file should be writable");
        git(&repository, &["add", "tracked.txt"]);
        git(&repository, &["commit", "-m", "Initial fixture"]);
        repository
    }

#[test]
    fn temporary_worktree_cleanup_requires_fresh_clean_status_and_postconditions() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let worktree = root.join("temporary-worktree");
        let head = String::from_utf8_lossy(&git(&repository, &["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_string();
        git(
            &repository,
            &[
                "worktree",
                "add",
                "-b",
                "temporary-cleanup",
                worktree.to_str().expect("worktree path should be UTF-8"),
                "HEAD",
            ],
        );
        fs::write(worktree.join("dirty.txt"), "preserve me\n")
            .expect("temporary worktree should accept a dirty file");
        fs::write(worktree.join("tracked.txt"), "modified tracked content\n")
            .expect("temporary worktree should accept a tracked-file modification");

        let blocked = remove_temporary_worktree_transactionally(&repository, &worktree, &head)
            .expect_err("dirty temporary worktree cleanup should be blocked");
        assert!(blocked.contains(&worktree.to_string_lossy().to_string()));
        assert!(blocked.contains("dirty.txt"));
        assert!(blocked.contains("tracked.txt"));
        assert!(worktree.exists());
        assert_eq!(live_worktree_contains(&repository, &worktree), Some(true));

        fs::remove_file(worktree.join("dirty.txt")).expect("dirty fixture should be removable");
        fs::write(worktree.join("tracked.txt"), "one\n")
            .expect("tracked fixture should be restored before cleanup");
        remove_temporary_worktree_transactionally(&repository, &worktree, &head)
            .expect("clean temporary worktree cleanup should pass");
        assert!(!worktree.exists());
        assert_eq!(live_worktree_contains(&repository, &worktree), Some(false));
        assert_eq!(git_head_reachable(&repository, &head), Some(true));

        fs::remove_dir_all(root).expect("transactional cleanup fixture should be removable");
    }

#[test]
    fn target_qr_detached_head_provenance_is_rewritten_to_selected_branch() {
        let root = fixture_root();
        let run = root.join("run");
        fs::create_dir_all(&run).expect("target QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "git": { "branch": "HEAD", "ref": "refs/heads/HEAD" },
                "provenance": { "branch": "HEAD" }
            })
            .to_string(),
        )
        .expect("target QR manifest should be writable");

        assert_eq!(
            rewrite_target_qr_branch_provenance(&run, "dev")
                .expect("target QR provenance should be rewritten"),
            1
        );
        let payload: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(run.join("run-manifest.json"))
                .expect("rewritten target QR manifest should be readable"),
        )
        .expect("rewritten target QR manifest should remain JSON");
        assert_eq!(payload["git"]["branch"], "dev");
        assert_eq!(payload["git"]["ref"], "refs/heads/dev");
        assert_eq!(payload["provenance"]["branch"], "dev");

        fs::remove_dir_all(root).expect("target QR fixture should be removable");
    }

#[test]
    fn quality_detector_refresh_runs_registered_fleet_then_reimports_quality() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let registered = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        set_repository_target_branch_at(&store, &registered.repositories[0].id, "main")
            .expect("fixture target should configure");
        fs::write(repository.join("tracked.txt"), "two\n")
            .expect("target change should be writable");
        git(&repository, &["add", "tracked.txt"]);
        git(&repository, &["commit", "-m", "Advance configured target"]);
        let target_head =
            git_static(&repository, &["rev-parse", "main"]).expect("target commit should resolve");
        git(&repository, &["switch", "-c", "feature"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature change should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "Create divergent workspace"]);
        let run_dir = repository
            .join(".quality-runner")
            .join("runs")
            .join("fleet-detector-fixture-verify");
        fs::create_dir_all(&run_dir).expect("published QR run should be creatable");
        fs::write(
            run_dir.join("run-manifest.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "created_at": iso_now(),
                "git": {"branch": "main", "head_sha": target_head},
            }))
            .expect("manifest should encode"),
        )
        .expect("manifest should persist");
        fs::write(
            run_dir.join("code-quality-scan.json"),
            r#"{"findings":[{"id":"fixture"}]}"#,
        )
        .expect("findings should persist");
        let arguments_path = root.join("qr-arguments.txt");
        let fake_qr = root.join("fake-qr");
        let qr_payload = serde_json::json!({
            "schema": "quality-runner-fleet-detector-refresh/v1",
            "status": "completed",
            "counts": {"published": 1, "blocked": 0, "unsupported": 0},
            "results": [{
                "primary_path": repository,
                "status": "published",
                "target": {"branch": "main", "head": target_head},
                "published_paths": [run_dir],
                "finding_count": 1,
            }],
        });
        fs::write(
            &fake_qr,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\nprintf '%s\\n' '{}'\n",
                arguments_path.display(),
                serde_json::to_string(&qr_payload).expect("QR payload should encode")
            ),
        )
        .expect("fake QR should be writable");
        let mut permissions = fs::metadata(&fake_qr)
            .expect("fake QR metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_qr, permissions).expect("fake QR should be executable");

        let report =
            refresh_quality_detectors_at(&store, Some(&fake_qr.to_string_lossy()), 321, "off")
                .expect("combined detector refresh should complete");

        assert_eq!(report.schema_version, "pronto-quality-detector-refresh/v1");
        assert_eq!(
            report.status, "Completed",
            "reconciliation: {:?}",
            report.reconciliation
        );
        assert_eq!(report.provenance_refreshes, 2);
        assert_eq!(report.published_repositories, 1);
        assert_eq!(report.ingested_published_repositories, 1);
        assert_eq!(report.rejected_published_repositories, 0);
        assert_eq!(report.tracked_repositories, 1);
        assert_eq!(report.detector_applicable_repositories, 1);
        assert_eq!(report.detector_excluded_repositories, 0);
        assert_eq!(report.findings_evidence_repositories, 1);
        assert_eq!(report.applicable_findings_evidence_repositories, 1);
        assert_eq!(report.missing_findings_evidence_repositories, 0);
        assert_eq!(report.reconciliation[0].status, "ingested");
        assert_eq!(
            report.snapshot.repositories[0].quality.findings.freshness,
            QualityFreshness::Fresh
        );
        assert_eq!(
            report.snapshot.repositories[0]
                .quality
                .findings
                .scanned_commit
                .as_deref(),
            Some(target_head.as_str())
        );
        assert_eq!(report.snapshot.repositories.len(), 1);
        let arguments = fs::read_to_string(arguments_path).expect("QR arguments should persist");
        assert!(arguments.contains("fleet\ndetector\nrefresh\n"));
        assert!(arguments.contains("--projects-root\n"));
        assert!(arguments.contains("--repo-path\n"));
        assert!(arguments.contains(&repository.to_string_lossy().to_string()));
        assert!(arguments.contains("--target-path-override\n"));
        assert!(arguments.contains(&format!("{}\nmain\n", repository.to_string_lossy())));
        assert!(arguments.contains("--timeout-seconds\n321\n"));
        assert!(arguments.contains("--agent-review-mode\noff\n"));

        fs::remove_dir_all(root).expect("detector fixture should be removable");
    }

#[test]
    fn quality_detector_refresh_excludes_qr_unsupported_repositories_from_coverage() {
        let root = fixture_root();
        let active = fixture_repository_named(&root, "active-repository");
        let archive = fixture_repository_named(&root, "archival-repository");
        let store = root.join("registry.db");
        let registered = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        assert_eq!(registered.repositories.len(), 2);

        let qr_payload = serde_json::json!({
            "schema": "quality-runner-fleet-detector-refresh/v1",
            "status": "partial",
            "counts": {"published": 0, "blocked": 1, "unsupported": 1},
            "results": [
                {
                    "primary_path": active,
                    "status": "blocked",
                    "reason": "fixture detector blocker",
                },
                {
                    "primary_path": archive,
                    "status": "unsupported",
                    "reason": "repository documentation declares an archival generated snapshot",
                },
            ],
        });
        let fake_qr = root.join("fake-qr-lifecycle-coverage");
        fs::write(
            &fake_qr,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                serde_json::to_string(&qr_payload).expect("QR payload should encode")
            ),
        )
        .expect("fake QR should be writable");
        let mut permissions = fs::metadata(&fake_qr)
            .expect("fake QR metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_qr, permissions).expect("fake QR should be executable");

        let report =
            refresh_quality_detectors_at(&store, Some(&fake_qr.to_string_lossy()), 60, "off")
                .expect("lifecycle-aware report should be produced");

        assert_eq!(report.status, "Partial");
        assert_eq!(report.tracked_repositories, 2);
        assert_eq!(report.detector_applicable_repositories, 1);
        assert_eq!(report.detector_excluded_repositories, 1);
        assert_eq!(report.findings_evidence_repositories, 0);
        assert_eq!(report.applicable_findings_evidence_repositories, 0);
        assert_eq!(report.missing_findings_evidence_repositories, 1);

        fs::remove_dir_all(root).expect("detector fixture should be removable");
    }

#[test]
    fn quality_detector_refresh_rejects_published_evidence_that_was_not_ingested() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let registered = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        set_repository_target_branch_at(&store, &registered.repositories[0].id, "main")
            .expect("fixture target should configure");
        let target_head =
            git_static(&repository, &["rev-parse", "main"]).expect("target commit should resolve");
        let missing_run = repository
            .join(".quality-runner")
            .join("runs")
            .join("missing-published-run");
        let qr_payload = serde_json::json!({
            "schema": "quality-runner-fleet-detector-refresh/v1",
            "status": "completed",
            "counts": {"published": 1, "blocked": 0, "unsupported": 0},
            "results": [{
                "primary_path": repository,
                "status": "published",
                "target": {"branch": "main", "head": target_head},
                "published_paths": [missing_run],
                "finding_count": 0,
            }],
        });
        let fake_qr = root.join("fake-qr-missing-publication");
        fs::write(
            &fake_qr,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                serde_json::to_string(&qr_payload).expect("QR payload should encode")
            ),
        )
        .expect("fake QR should be writable");
        let mut permissions = fs::metadata(&fake_qr)
            .expect("fake QR metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_qr, permissions).expect("fake QR should be executable");

        let report =
            refresh_quality_detectors_at(&store, Some(&fake_qr.to_string_lossy()), 60, "off")
                .expect("reconciliation should return a structured partial report");

        assert_eq!(report.status, "Partial");
        assert_eq!(report.published_repositories, 1);
        assert_eq!(report.ingested_published_repositories, 0);
        assert_eq!(report.rejected_published_repositories, 1);
        assert_eq!(report.reconciliation[0].status, "rejected");
        assert!(report.reconciliation[0]
            .reason
            .contains("did not import QR findings evidence"));

        fs::remove_dir_all(root).expect("detector fixture should be removable");
    }

#[test]
    fn quality_detector_refresh_rejects_unknown_agent_review_mode() {
        let root = fixture_root();
        let error =
            refresh_quality_detectors_at(&root.join("registry.db"), Some("qr"), 60, "invented")
                .expect_err("unknown agent review modes must fail before state or process access");
        assert!(error.contains("must be off, auto, parallel, or required"));
        fs::remove_dir_all(root).expect("detector fixture should be removable");
    }

#[test]
    fn verification_accepts_intentionally_deferred_terminal_dependencies() {
        assert!(remediation_dependencies_are_terminal(
            ["verified", "deferred"].into_iter()
        ));
        assert!(!remediation_dependencies_are_terminal(
            ["verified", "open"].into_iter()
        ));
        assert!(!remediation_dependencies_are_terminal(
            ["verified", "blocked"].into_iter()
        ));
    }

#[test]
    fn registers_root_and_scans_from_cli_path() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("cli root registration should scan the folder");

        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(
            snapshot.roots[0].path,
            canonical_path(&root)
                .expect("fixture root should be canonical")
                .to_string_lossy()
        );
        assert_eq!(snapshot.repositories.len(), 1);
        assert_eq!(
            snapshot.repositories[0].path,
            canonical_path(&repository)
                .expect("fixture repository should be canonical")
                .to_string_lossy()
        );

        fs::remove_dir_all(root).expect("cli root fixture should be removable");
    }

#[test]
    fn registering_new_repository_enters_the_normal_fleet_without_mutating_showcase() {
        let root = fixture_root();
        let contract_repository = fixture_repository_named(&root, "pronto");
        let _new_repository = fixture_repository_named(&root, "new-project");
        let contract = serde_json::json!({
            "schema_version": "pronto-showcase-goal/v2",
            "target_publishable_demo_count": 2,
            "reviewed_at": "2026-08-12T00:00:00Z",
            "quality_bar_source": "Authenticated Handshake AI Showcase audit",
            "scoring": {
                "product_weight": 0.6,
                "materials_weight": 0.4,
                "priority_career_weight": 0.5,
                "priority_product_weight": 0.3,
                "priority_materials_gap_weight": 0.2,
                "publishable_product_minimum": 3.5,
                "publishable_materials_minimum": 4.0
            },
            "projects": [{
                "repository_name": "pronto",
                "display_name": "pronto",
                "public_eligibility": "not_applicable",
                "disposition_source": "test fixture",
                "product_readiness": {
                    "status": "not_applicable",
                    "evidence": "support repository"
                },
                "demo_materials": {
                    "status": "not_applicable",
                    "evidence": "support repository"
                },
                "career_signal": {
                    "status": "not_applicable",
                    "evidence": "support repository"
                },
                "blockers": [],
                "missing_materials": [],
                "next_step": "No showcase work."
            }],
            "public_release_target_policy": {
                "matrix_path": "showcase-materials/public-release-targets.json"
            }
        });
        fs::create_dir_all(contract_repository.join(".pronto"))
            .expect("showcase contract directory should be creatable");
        fs::write(
            contract_repository.join(".pronto/showcase-goal.json"),
            serde_json::to_vec_pretty(&contract).expect("contract JSON"),
        )
        .expect("showcase contract should be writable");

        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("registration should persist the normal fleet repository");
        assert_eq!(snapshot.repositories.len(), 2);

        let persisted: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(contract_repository.join(".pronto/showcase-goal.json"))
                .expect("showcase contract should remain readable"),
        )
        .expect("showcase contract should remain valid JSON");
        let projects = persisted["projects"].as_array().expect("projects array");
        assert!(!projects
            .iter()
            .any(|project| project["repository_name"] == "new-project"));
        assert!(persisted["public_release_target_policy"].is_object());

        fs::remove_dir_all(root).expect("registration fixture should be removable");
    }
