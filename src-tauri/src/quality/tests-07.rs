#[test]
fn test_portfolio_runs_roll_up_independently_without_becoming_gates() {
    let root = fixture_root();
    let repository_path = root.join("repo");
    let runs = repository_path.join(".quality-runner").join("runs");
    let audit_run = runs.join("portfolio-1");
    let proof_run = runs.join("removal-1");
    let unrelated_run = runs.join("later-general-run");
    fs::create_dir_all(&audit_run).expect("audit run should be writable");
    fs::create_dir_all(&proof_run).expect("proof run should be writable");
    fs::create_dir_all(&unrelated_run).expect("general run should be writable");
    let audit_time = (Utc::now() - Duration::minutes(3)).to_rfc3339();
    let proof_time = (Utc::now() - Duration::minutes(2)).to_rfc3339();
    let general_time = (Utc::now() - Duration::minutes(1)).to_rfc3339();
    let manifest = |run_id: &str, mode: &str, created_at: &str| {
        serde_json::json!({
            "schema": QR_RUN_MANIFEST_SCHEMA,
            "run_id": run_id,
            "mode": mode,
            "repo_root": repository_path.to_string_lossy(),
            "created_at": created_at,
            "git": {"head_sha": "abc", "branch": "main", "dirty": false}
        })
    };
    fs::write(
        audit_run.join("run-manifest.json"),
        serde_json::to_string(&manifest(
            "portfolio-1",
            "test-portfolio-audit",
            &audit_time,
        ))
        .expect("audit manifest should encode"),
    )
    .expect("audit manifest should be writable");
    fs::write(
        audit_run.join("test-portfolio-audit.json"),
        serde_json::to_string(&serde_json::json!({
            "schema": TEST_PORTFOLIO_AUDIT_SCHEMA,
            "status": "review-required",
            "implementation_allowed": false,
            "revision": {"head": "abc"},
            "audit_hash": "audit-hash",
            "summary": {
                "removal_candidates": 1,
                "dispositions": {"insufficient_evidence": 2}
            }
        }))
        .expect("audit should encode"),
    )
    .expect("audit should be writable");
    fs::write(
        proof_run.join("run-manifest.json"),
        serde_json::to_string(&manifest(
            "removal-1",
            "test-removal-proof",
            &proof_time,
        ))
        .expect("proof manifest should encode"),
    )
    .expect("proof manifest should be writable");
    fs::write(
        proof_run.join("test-removal-proof.json"),
        serde_json::to_string(&serde_json::json!({
            "schema": TEST_REMOVAL_PROOF_SCHEMA,
            "status": "blocked",
            "implementation_allowed": false,
            "revision": {"head": "abc"},
            "portfolio_audit_hash": "audit-hash",
            "proof_hash": "proof-hash",
            "blockers": ["mutation-preservation"],
            "summary": {"removed_test_count": 1}
        }))
        .expect("proof should encode"),
    )
    .expect("proof should be writable");
    fs::write(
        unrelated_run.join("run-manifest.json"),
        serde_json::to_string(&manifest("later-general-run", "inspect", &general_time))
            .expect("general manifest should encode"),
    )
    .expect("general manifest should be writable");

    let repository = fixture_repository(&repository_path);
    let mut snapshot = ingest_repository_quality(&repository, None, None, None);

    assert_eq!(snapshot.test_portfolio.status, "Blocked");
    assert_eq!(snapshot.qr_command_runs.total_count, 3);
    assert_eq!(snapshot.qr_command_runs.visible_count, 3);
    assert_eq!(snapshot.qr_command_runs.invalid_count, 0);
    assert_eq!(snapshot.qr_command_runs.mode_counts.get("inspect"), Some(&1));
    assert_eq!(snapshot.qr_command_runs.runs[0].run_id, "later-general-run");
    let audit = snapshot
        .test_portfolio
        .audit
        .as_ref()
        .expect("audit should be imported");
    assert_eq!(audit.run_id, "portfolio-1");
    assert_eq!(audit.removal_candidate_count, 1);
    assert_eq!(audit.insufficient_evidence_count, 2);
    let proof = snapshot
        .test_portfolio
        .removal_proof
        .as_ref()
        .expect("proof should be imported");
    assert_eq!(proof.run_id, "removal-1");
    assert_eq!(proof.blocker_ids, vec!["mutation-preservation"]);
    assert!(snapshot
        .gates
        .iter()
        .all(|gate| gate.id != "test_portfolio"));

    project_quality_snapshot_for_target(&mut snapshot, "dev", "different-head");
    assert_eq!(
        snapshot
            .test_portfolio
            .audit
            .as_ref()
            .expect("audit should remain visible")
            .freshness,
        QualityFreshness::Stale
    );
    fs::remove_dir_all(root).expect("fixture root should be removable");
}
