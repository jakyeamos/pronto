import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

test("candidate packets preserve local evidence boundaries", async () => {
  const packets = await Promise.all(
    [
      [
        "quality-lens",
        "evidence/ql-w1-normalized-model.json",
        "candidate_local",
        "stale",
        "public-description.txt",
      ],
      [
        "debug-trail",
        "evidence/debug-trail-w1-receipt.json",
        "candidate_local",
        "fresh",
        "public-description.txt",
      ],
      [
        "quality-setup",
        "evidence/qs-w1-scenario-matrix.json",
        "candidate_local",
        "scenario-matrix",
        "public-description.txt",
      ],
      [
        "rule-lab",
        "evidence/rl-w2-fixture-comparison.json",
        "candidate_local",
        "fixture-comparison",
        "public-description.txt",
      ],
      [
        "evidence-replay",
        "evidence/er-w2-reader-matrix.json",
        "candidate_local",
        "reader-matrix",
        "public-description.txt",
      ],
      [
        "workflow-gateboard",
        "evidence/wg-w2-declared-gate-receipts.json",
        "candidate_local",
        "gate-receipts",
        "public-description.txt",
      ],
      [
        "failure-capsule",
        "evidence/fc-w2-redacted-failure.json",
        "candidate_local",
        "failure-capture",
        "public-description.txt",
      ],
      [
        "change-radius",
        "evidence/cr-w2-typescript-radius.json",
        "candidate_local",
        "radius-graph",
        "public-description.txt",
      ],
      [
        "behavior-coverage-atlas",
        "evidence/bca-w2-fixture-matrix.json",
        "candidate_local",
        "behavior-matrix",
        "public-description.txt",
      ],
      [
        "automation-flight-recorder",
        "evidence/afr-w3-gate-traces.json",
        "candidate_local",
        "flight-traces",
        "public-description.txt",
      ],
      [
        "remediation-canvas",
        "evidence/rc-w3-partial-stale-handoff.json",
        "candidate_local",
        "remediation-handoff",
        "public-description.txt",
      ],
      [
        "contract-watch",
        "evidence/cw-w3-openapi-handoff.json",
        "candidate_local",
        "contract-diff",
        "public-description.txt",
      ],
      [
        "review-attention-map",
        "evidence/ram-w3-two-source-overlay.json",
        "candidate_local",
        "two-source-overlay",
        "public-description.txt",
      ],
      [
        "review-sandbox",
        "evidence/rs-w4-scenario-matrix.json",
        "candidate_local",
        "sandbox-matrix",
        "public-description.txt",
      ],
      [
        "change-integration-simulator",
        "evidence/cis-w4-clean-conflict.json",
        "candidate_local",
        "integration-matrix",
        "public-description.txt",
      ],
      [
        "deletion-proof-workbench",
        "evidence/dpw-w4-bounded-deletion.json",
        "candidate_local",
        "deletion-proof",
        "public-description.txt",
      ],
      [
        "readiness-inspector",
        "evidence/ri-w5-upstream-projection.json",
        "candidate_local",
        "individual-states",
        "public-description.txt",
      ],
      [
        "fleet-radar",
        "evidence/fr-w5-readonly-refresh.json",
        "candidate_local",
        "fleet-refresh",
        "public-description.txt",
      ],
    ].map(
      async ([project, evidencePath, status, targetStatus, description]) => {
        const [caseStudy, claims, evidence, copy, page, preview] =
          await Promise.all([
            readFile(
              new URL(`showcase-materials/${project}/case-study.json`, root),
              "utf8",
            ).then(JSON.parse),
            readFile(
              new URL(`showcase-materials/${project}/claim-ledger.json`, root),
              "utf8",
            ).then(JSON.parse),
            readFile(
              new URL(`showcase-materials/${project}/${evidencePath}`, root),
              "utf8",
            ).then(JSON.parse),
            readFile(
              new URL(`showcase-materials/${project}/${description}`, root),
              "utf8",
            ),
            readFile(
              new URL(`showcase-materials/${project}/public/index.html`, root),
              "utf8",
            ),
            readFile(
              new URL(`showcase-materials/${project}/preview.html`, root),
              "utf8",
            ),
          ]);

        assert.equal(caseStudy.status, status);
        if (targetStatus === "scenario-matrix") {
          assert.equal(evidence.ecosystem, "node");
          assert.ok(
            evidence.scenarios?.every((scenario) => scenario.status),
            "Quality Setup scenario matrix must record every scenario status",
          );
          assert.equal(evidence.post_rollback?.status, "supported");
        } else if (targetStatus === "fixture-comparison") {
          assert.equal(evidence.status, "passed");
          assert.equal(evidence.receipt?.status, "fresh");
          assert.deepEqual(evidence.comparison?.gained_matches, []);
          assert.deepEqual(evidence.comparison?.lost_matches, []);
        } else if (targetStatus === "reader-matrix") {
          assert.equal(evidence.inspect?.freshness, "stale");
          assert.equal(evidence.inspect?.current_outcome, "not-run");
          assert.equal(evidence.rerun_preview?.status, "blocked");
          assert.equal(evidence.inspect?.opening_executes, false);
        } else if (targetStatus === "gate-receipts") {
          assert.equal(evidence.manifest_status, "ready");
          assert.equal(evidence.target?.repository_ref_unchanged, true);
          assert.ok(
            evidence.receipts?.every((receipt) => receipt.outcome === "passed"),
          );
          assert.equal(evidence.scenarios?.[0]?.status, "blocked");
        } else if (targetStatus === "failure-capture") {
          assert.equal(evidence.capture?.outcome, "failed");
          assert.match(evidence.capture?.output_excerpt ?? "", /REDACTED/);
          assert.equal(evidence.capture?.secret_value_present, false);
          assert.equal(evidence.inspection?.executes_on_open, false);
        } else if (targetStatus === "radius-graph") {
          assert.equal(evidence.status, "ready");
          assert.ok(evidence.edges?.some((edge) => edge.test));
          assert.ok(
            evidence.unknowns?.some(
              (unknown) => unknown.kind === "dynamic-import",
            ),
          );
          assert.ok(evidence.target?.revision);
        } else if (targetStatus === "behavior-matrix") {
          assert.equal(evidence.fresh_run?.status, "ready");
          assert.deepEqual(
            evidence.fresh_run?.entries?.map((entry) => entry.status),
            ["verified", "exercised_without_assertion", "failed", "unknown"],
          );
          assert.equal(evidence.stale_run?.evidence_class, "historical");
          assert.equal(
            evidence.fresh_run?.diagnostics?.[0]?.kind,
            "duplicate-link",
          );
        } else if (targetStatus === "flight-traces") {
          assert.equal(evidence.pass_trace?.status, "passed");
          assert.equal(evidence.failure_trace?.status, "failed");
          assert.equal(evidence.failure_trace?.steps?.[2]?.status, "not_run");
          assert.match(
            evidence.failure_trace?.steps?.[1]?.output ?? "",
            /REDACTED/,
          );
          assert.ok(
            evidence.pass_trace?.steps?.every((step) => step.parent_id),
          );
        } else if (targetStatus === "remediation-handoff") {
          assert.equal(evidence.fresh_handoff?.refresh_status, "fresh");
          assert.equal(evidence.stale_refresh?.status, "stale");
          assert.equal(evidence.stale_refresh?.partial_work_preserved, true);
          assert.equal(evidence.source?.authority_copied, false);
        } else if (targetStatus === "contract-diff") {
          assert.equal(evidence.status, "ready");
          assert.equal(evidence.changes?.length, 4);
          assert.ok(
            evidence.changes?.some(
              (change) =>
                change.kind === "removal" && change.certainty === "definite",
            ),
          );
          assert.ok(
            evidence.changes?.some(
              (change) =>
                change.kind === "response-change" &&
                change.certainty === "potential",
            ),
          );
          assert.equal(evidence.separation?.human_disposition_required, true);
          assert.equal(evidence.separation?.merge_verdict, null);
          assert.equal(evidence.separation?.release_verdict, null);
        } else if (targetStatus === "two-source-overlay") {
          assert.equal(evidence.overlay?.status, "ready");
          assert.equal(evidence.overlay?.source_count, 2);
          assert.equal(evidence.direct_navigation?.status, "updated");
          assert.equal(evidence.direct_navigation?.disposition, "reviewed");
          assert.equal(evidence.uncertainty?.unmatched?.length, 1);
        } else if (targetStatus === "sandbox-matrix") {
          assert.equal(evidence.preview?.status, "ready");
          assert.ok(
            evidence.scenarios?.some(
              (scenario) =>
                scenario.id === "clean" && scenario.cleanup === "removed",
            ),
          );
          assert.ok(
            evidence.scenarios?.some(
              (scenario) =>
                scenario.id === "conflict" && scenario.status === "blocked",
            ),
          );
          assert.ok(
            evidence.scenarios?.some(
              (scenario) =>
                scenario.id === "retained-dirty" &&
                scenario.cleanup === "retained",
            ),
          );
          assert.equal(evidence.primary_checkout?.ref_mutation, false);
          assert.match(
            evidence.scenarios?.find(
              (scenario) => scenario.id === "cancelled-launch",
            )?.classification ?? "",
            /not a distinct/,
          );
        } else if (targetStatus === "integration-matrix") {
          assert.equal(evidence.simulations?.clean?.status, "mergeable");
          assert.equal(evidence.simulations?.conflict?.status, "conflict");
          assert.equal(
            evidence.simulations?.conflict?.conflict_file,
            "shared.txt",
          );
          assert.equal(
            evidence.gate_probe?.status,
            "passed_local_unintegrated",
          );
          assert.equal(evidence.safety?.refs_before_after_same, true);
        } else if (targetStatus === "deletion-proof") {
          assert.equal(evidence.candidate?.decision, "applied");
          assert.equal(evidence.candidate?.static_references?.length, 0);
          assert.equal(evidence.refusal?.status, "blocked");
          assert.ok(evidence.unknowns?.includes("dynamic imports"));
          assert.equal(evidence.stale_probe?.status, "open");
        } else if (targetStatus === "individual-states") {
          assert.equal(evidence.status, "candidate_local");
          assert.deepEqual(
            evidence.individual_checks?.map((check) => check.outcome),
            ["passed", "failed", "blocked", "ready"],
          );
          assert.equal(
            evidence.state_boundary?.unsupported,
            "missing profile remains explicit",
          );
          assert.equal(
            evidence.continuity?.quality_setup_receipt,
            "not_projected",
          );
          assert.equal(evidence.verification?.fixture_refs_unchanged, true);
        } else if (targetStatus === "fleet-refresh") {
          assert.equal(evidence.status, "candidate_local");
          assert.equal(evidence.snapshot?.status, "ready");
          assert.deepEqual(
            evidence.snapshot?.repositories?.map(
              (repo) => `${repo.state}/${repo.freshness}`,
            ),
            ["ready/fresh", "failed/stale"],
          );
          assert.deepEqual(evidence.snapshot?.attention, ["beta"]);
          assert.equal(evidence.refresh_safety?.alpha_ref_unchanged, true);
          assert.equal(evidence.refresh_safety?.beta_ref_unchanged, true);
        } else {
          assert.equal(
            evidence.target?.status ?? evidence.freshness?.status,
            targetStatus,
          );
        }
        assert.ok(caseStudy.open_gates.length >= 3);
        assert.ok(
          claims.levels.some((level) => level.status === "not_verified"),
        );
        assert.ok(copy.trim().length <= 500);
        assert.match(page, /data-material-status|Candidate local material/);
        assert.match(preview, /data-material-status="candidate-local"/);
        return project;
      },
    ),
  );

  assert.deepEqual(packets.sort(), [
    "automation-flight-recorder",
    "behavior-coverage-atlas",
    "change-integration-simulator",
    "change-radius",
    "contract-watch",
    "debug-trail",
    "deletion-proof-workbench",
    "evidence-replay",
    "failure-capsule",
    "fleet-radar",
    "quality-lens",
    "quality-setup",
    "readiness-inspector",
    "remediation-canvas",
    "review-attention-map",
    "review-sandbox",
    "rule-lab",
    "workflow-gateboard",
  ]);
});
