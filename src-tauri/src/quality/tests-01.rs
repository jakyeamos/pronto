use super::*;

use std::fs;

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture_root() -> PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("pronto-quality-{id}"));
        fs::create_dir_all(&path).expect("fixture root should be writable");
        path
    }

fn evidence(
        source: QualitySource,
        status: QualityGateStatus,
        freshness: QualityFreshness,
    ) -> QualityEvidence {
        QualityEvidence {
            id: "lint".to_string(),
            source,
            status,
            freshness,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            command: None,
            source_label: "fixture".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: String::new(),
            verification_level: QualityVerificationLevel::Unknown,
            target_kind: None,
            target_url: None,
            target_provider: None,
            deployment_id: None,
        }
    }

fn fixture_repository(path: &Path) -> RepositorySnapshot {
        serde_json::from_value(serde_json::json!({
            "id": "repo-1",
            "name": "repo",
            "path": path.to_string_lossy(),
            "locality": "Local",
            "lifecycle": "Active",
            "lifecycle_candidate": "Active",
            "provider_state": "Unknown",
            "branch": "main",
            "workspace": {
                "id": "w",
                "path": path.to_string_lossy(),
                "is_primary": true,
                "branch": "main",
                "dirty": false,
                "added": 0,
                "removed": 0,
                "line_totals_partial": false,
                "sync_state": "Synced",
                "remote_freshness": "Unknown",
                "ahead": 0,
                "behind": 0,
                "last_commit": "abc",
                "integration_state": "Unknown",
                "target_branch": null,
                "target_confidence": "Unknown",
                "role": "Primary",
                "role_confidence": "High",
                "activity": {"state": "Unknown", "confidence": "Low", "signals": []}
            },
            "workspaces": [],
            "branches": [],
            "submodules": [],
            "pull_requests": [],
            "releases": [],
            "conditions": [],
            "last_scan_at": "2026-07-26T11:00:00Z",
            "last_fetch_at": null,
            "last_activity_at": null
        }))
        .expect("repository fixture should decode")
    }

fn fixture_maturity_feed(repository: &RepositorySnapshot, as_of: &str) -> Value {
        let summary_hash = "b".repeat(64);
        let mut feed = serde_json::json!({
            "schema": MATURITY_FEED_SCHEMAS[0],
            "status": "completed",
            "feed_timestamp": as_of,
            "generated_at": as_of,
            "source": {
                "audit_id": "audit-fixture",
                "as_of": as_of,
                "projects_root": "/tmp/projects",
                "artifact_schema": "quality-runner-fleet-audit-v0.1",
                "summary_hash": summary_hash,
            },
            "replay": {
                "status": "passed",
                "deterministic": true,
                "source_summary_hash": "b".repeat(64),
                "replayed_summary_hash": "b".repeat(64),
            },
            "repository_count": 1,
            "checkout_count": 1,
            "mean_maturity": 3.5,
            "measurement_confidence": {
                "level": "high",
                "basis": ["population_complete", "dynamic_verification_conclusive"],
                "limitations": [],
                "population_coverage": {
                    "status": "complete",
                    "expected_repository_count": 1,
                    "observed_repository_count": 1,
                    "excluded_repository_count": 2,
                },
                "unresolved_measurement_gap_count": 0,
                "deterministic_replay": true,
            },
            "dimension_means": {
                "agent_usability.documentation_contract": 3.0,
                "agent_usability.growth_health": 4.0,
                "architecture_boundaries": 3.5
            },
            "maturity_certified_repository_count": 0,
            "maturity_status_counts": {"not_certified": 1},
            "quality_outcome_counts": {"healthy": 1},
            "quality_outcome_taxonomy": {
                "healthy": {
                    "label": "Quality healthy",
                    "meaning": "Verification passed or was safely reused and all applicable dimensions are maintained.",
                    "next_step": "Keep the evidence checkpoint current."
                }
            },
            "finding_counts": {},
            "unresolved_measurement_gaps": [],
            "repositories": [{
                "repo_id": repository_feed_id(repository),
                "display_name": repository.name,
                "target_branch": "dev",
                "target_branch_status": "ready",
                "target_head": "abc",
                "maturity_score": 3.5,
                "maturity_status": "not_certified",
                "dimension_scores": {
                    "agent_usability.documentation_contract": 3.0,
                    "agent_usability.growth_health": 4.0,
                    "architecture_boundaries": 3.5
                },
                "dimension_gaps": [{
                    "dimension": "change_surface_coverage",
                    "status": "missing",
                    "score": 0,
                    "message": "No repository-owned change-surface matrix was found."
                }],
                "quality_status": "healthy",
                "quality_outcome": {
                    "state": "healthy",
                    "label": "Quality healthy",
                    "disposition": "Applicable dimensions are maintained with current evidence.",
                    "next_step": "Keep the evidence checkpoint current."
                },
                "finding_count": 0,
                "blocker_count": 0,
                "dynamic_status": "reused"
            }],
            "privacy": {
                "private_local_feed": true,
                "raw_paths": false,
                "raw_prompts": false,
                "raw_code": false,
                "raw_diffs": false,
                "raw_transcripts": false,
                "credentials": false,
            },
            "provenance_hash": "",
        });
        feed["behavior_assurance"] = serde_json::json!({
            "schema": "quality-runner-behavior-assurance-summary/v1",
            "status": "gaps_present",
            "repository_count": 1,
            "ready_repository_count": 0,
            "applicability_counts": {"applicable": 1},
            "result_status_counts": {"unknown": 1},
            "required_scenario_count": 2,
            "passed_scenario_count": 1,
            "gap_count": 1
        });
        feed["repositories"][0]["behavior_assurance"] = serde_json::json!({
            "schema": "quality-runner-behavior-assurance/v1",
            "applicability": "applicable",
            "contract_status": "current",
            "result_status": "unknown",
            "freshness": "stale",
            "release_ready": false,
            "score": 2,
            "contract_path": ".pronto/behavior-assurance.json",
            "receipt_directory": ".quality-runner/behavior-assurance/receipts",
            "contract_digest": "contract-fixture",
            "target_branch": "dev",
            "target_commit": "abc",
            "observed_at": as_of,
            "required_scenario_count": 2,
            "passed_scenario_count": 1,
            "accepted_defect_count": 0,
            "receipt_count": 1,
            "verified": [],
            "gaps": [{
                "kind": "receipt_stale",
                "message": "One required scenario needs a current receipt.",
                "behavior_id": "save-state",
                "scenario_id": "reload-restores-value"
            }],
            "detail": "1/2 required Tier-0 scenarios have current trusted receipts.",
            "next_step": "Resolve the listed contract or receipt gaps, then rerun the Quality Runner fleet audit."
        });
        feed["repositories"][0]["agent_usability"] = serde_json::json!({
            "applicability": "applicable",
            "schema": "quality-runner-agent-usability/v1",
            "status": "attention",
            "manifest_status": "present",
            "manifest_path": ".agents/agent-usability.json",
            "applicable_lane_count": 4,
            "covered_lane_count": 3,
            "lanes": [{
                "id": "documentation_contract",
                "label": "Documentation contract",
                "applicable": true,
                "score": 3,
                "status": "maintained",
                "message": "Every declared tool has fresh, routed documentation."
            }],
            "growth_health": {
                "status": "healthy",
                "score": 4,
                "message": "Documentation and skill structure remains proportionate and routed.",
                "document_count": 12,
                "agent_document_count": 3,
                "routed_agent_document_count": 3,
                "unrouted_agent_document_count": 0,
                "oversized_document_count": 0,
                "skill_count": 4,
                "family_count": 2,
                "largest_family_size": 2,
                "unclassified_skill_count": 0,
                "oversized_skill_count": 0,
                "tool_count": 2,
                "documented_tool_count": 2,
                "skill_covered_tool_count": 2,
                "behavior_declared_tool_count": 0,
                "behavior_verified_tool_count": 0,
                "inventory_truncated": false
            }
        });
        let mut ci_gate_audit = serde_json::json!({
            "schema": CI_GATE_AUDIT_SCHEMA,
            "status": "complete",
            "generated_at": as_of,
            "repository": {
                "name": repository.name,
                "branch": "dev",
                "head_sha": "abc"
            },
            "policy": {
                "authority": "recommendation_only",
                "implementation_allowed": false,
                "promotion_requirement": "Repository-owned profile acceptance is required."
            },
            "inventory": {"file_count": 10, "text_file_count": 5, "truncated": false},
            "candidate_count": 1,
            "candidates": [{
                "id": "custom:migration_compatibility",
                "label": "Migration compatibility",
                "recommendation": "required_candidate",
                "confidence": "high",
                "invariant": "Schema changes remain rollback compatible.",
                "failure_mode": "A migration can strand an older application.",
                "evidence": [{
                    "kind": "path",
                    "path": "migrations/001.sql",
                    "reason": "migration surface"
                }],
                "suggested_trigger": {"event": "pull_request", "paths": ["**/migrations/**"]},
                "suggested_check_context": "custom / migration-compatibility",
                "existing_check": {"status": "not_found", "contexts": []},
                "negative_controls": [],
                "admission": {
                    "state": "proposal_only",
                    "blockers": ["Repository-owned policy has not accepted this candidate."]
                },
                "next_step": "Add a negative control and explicitly accept or reject the gate."
            }],
            "provenance_hash": ""
        });
        ci_gate_audit["provenance_hash"] =
            Value::String(maturity_feed_hash(&ci_gate_audit).expect("candidate audit should hash"));
        feed["repositories"][0]["ci_gate_audit"] = ci_gate_audit;
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("fixture feed should hash"));
        feed
    }

fn fixture_sha256(path: &Path) -> String {
        Sha256::digest(fs::read(path).expect("fixture component should be readable"))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

#[test]
    fn coordinated_maturity_checkpoint_import_binds_both_evidence_lanes() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let as_of = "2026-07-26T11:00:00Z";
        let bundle = root.join("current/checkpoints/checkpoint-fixture");
        fs::create_dir_all(&bundle).expect("checkpoint bundle should be writable");
        let feed_path = bundle.join("maturity.json");
        fs::write(
            &feed_path,
            serde_json::to_vec(&fixture_maturity_feed(&repository, as_of))
                .expect("fixture feed should serialize"),
        )
        .expect("fixture feed should be writable");
        let mac_control_path = bundle.join("mac-control-ideal-state.json");
        fs::write(
            &mac_control_path,
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "pronto-mac-control-ideal-state/v1",
                "producer": "mac-control",
                "scope": "quality_runner_fleet",
                "run_id": "mac-audit-fixture",
                "observed_at": as_of,
                "repositories": [{
                    "repository_id": repository_feed_id(&repository),
                    "repository_name": repository.name.clone(),
                    "applicability": "not_applicable",
                    "applicability_reason": "The fixture has no supported task surface.",
                    "observed_at": as_of,
                    "observed_commit": "abc",
                    "manifest_schema": "",
                    "supported_tasks": [],
                    "criteria": {},
                    "evidence": [],
                    "implementation_contract": {},
                    "live_task_evidence": {}
                }]
            }))
            .expect("fixture report should serialize"),
        )
        .expect("fixture report should be writable");
        let checkpoint_path = root.join("current/maturity-checkpoint.json");
        let repository_id = repository_feed_id(&repository);
        fs::write(
            &checkpoint_path,
            serde_json::to_vec(&serde_json::json!({
                "schema": MATURITY_CHECKPOINT_SCHEMA,
                "status": "complete",
                "publication_status": "ready",
                "quality_status": "ready_with_blockers",
                "checkpoint_id": "checkpoint-fixture",
                "observed_at": as_of,
                "target": {
                    "repository_count": 1,
                    "repositories": [{
                        "repo_id": repository_id,
                        "observed_commit": "abc"
                    }]
                },
                "components": {
                    "qr_maturity": {
                        "audit_id": "audit-fixture",
                        "as_of": as_of,
                        "path": "current/checkpoints/checkpoint-fixture/maturity.json",
                        "sha256": fixture_sha256(&feed_path)
                    },
                    "mac_control": {
                        "audit_id": "mac-audit-fixture",
                        "as_of": as_of,
                        "path": "current/checkpoints/checkpoint-fixture/mac-control-ideal-state.json",
                        "sha256": fixture_sha256(&mac_control_path)
                    }
                }
            }))
            .expect("checkpoint should serialize"),
        )
        .expect("checkpoint should be writable");

        let imported =
            maturity_checkpoint_import(Some(&checkpoint_path), std::slice::from_ref(&repository))
                .expect("checkpoint should be attempted");

        assert_eq!(imported.checkpoint.status, "Coordinated");
        assert_eq!(
            imported.checkpoint.qr_audit_id.as_deref(),
            Some("audit-fixture")
        );
        assert_eq!(
            imported.checkpoint.mac_control_audit_id.as_deref(),
            Some("mac-audit-fixture")
        );
        assert_eq!(
            imported.audit.portfolio.latest_audit_id.as_deref(),
            Some("audit-fixture")
        );
        assert_eq!(
            imported.mac_control.portfolio.run_id.as_deref(),
            Some("mac-audit-fixture")
        );
    }

#[test]
    fn invalid_maturity_checkpoint_does_not_fallback_to_separate_feeds() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let checkpoint_path = root.join("current/maturity-checkpoint.json");
        fs::create_dir_all(
            checkpoint_path
                .parent()
                .expect("checkpoint should have a parent"),
        )
        .expect("checkpoint directory should be writable");
        fs::write(&checkpoint_path, b"{\"schema\":\"wrong\"}")
            .expect("invalid checkpoint should be writable");

        let imported =
            maturity_checkpoint_import(Some(&checkpoint_path), std::slice::from_ref(&repository))
                .expect("existing checkpoint should be attempted");

        assert_eq!(imported.checkpoint.status, "Blocked");
        assert_eq!(imported.audit.portfolio.audit_status, "Blocked");
        assert_eq!(imported.mac_control.portfolio.status, "Blocked");
    }
