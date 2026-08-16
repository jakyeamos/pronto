import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { pathToFileURL, URL } from "node:url";

const root = new URL("../", import.meta.url);

test("Terrace TR-1 uses a real, attributable regression case", async () => {
  const [fixture, failure, receipt, route] = await Promise.all([
    readFile(
      new URL("showcase-materials/terrace/case-fixture.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/terrace/expected-failure.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/tr-1-regression-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/terrace/route-plan.md", root), "utf8"),
  ]);

  assert.equal(fixture.source_kind, "historical_product_regression");
  assert.equal(fixture.owning_stage, "workflow_routing_validation");
  assert.equal(failure.command_exit_code, 1);
  assert.equal(failure.attribution.single_stage, true);
  assert.notEqual(
    failure.expected.command,
    failure.observed.command,
    "the historical fixture must preserve the observed routing mismatch",
  );
  assert.equal(receipt.acceptance.tr_1_closed, true);
  assert.equal(receipt.failing_run.exit_code, 1);
  assert.equal(receipt.corrected_run.exit_code, 0);
  assert.match(route, /\*\*Current closures:\*\* TR-1 through TR-6/);
});

test("Terrace TR-2 records restart and replay proof on the integrated revision", async () => {
  const [receipt, integration, route] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/tr-2-stage-state-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/integration-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/terrace/route-plan.md", root), "utf8"),
  ]);

  assert.deepEqual(receipt.architecture.stage_order, [
    "plan",
    "execute",
    "validate",
    "review",
    "complete",
  ]);
  assert.deepEqual(receipt.architecture.statuses, [
    "pending",
    "active",
    "passed",
    "failed",
    "blocked",
  ]);
  assert.equal(receipt.verification.full_ci.exit_code, 0);
  assert.equal(receipt.verification.cli_restart_proof.same_run_id, true);
  assert.equal(receipt.acceptance.replay_consistent, true);
  assert.equal(receipt.acceptance.tr_2_source_candidate_verified, true);
  assert.equal(receipt.acceptance.tr_2_integrated_into_dev, true);
  assert.equal(receipt.acceptance.tr_2_closed, true);
  assert.equal(integration.target.branch, "dev");
  assert.equal(
    integration.target.after_commit,
    "9ac360b0f246aa91c85a15183cf9de10c94330b0",
  );
  assert.equal(integration.verification.ci.exit_code, 0);
  assert.match(route, /\*\*Integrated commit:\*\*/);
});

test("Terrace TR-3 records an actionable durable stop packet on the integrated revision", async () => {
  const [receipt, route] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/tr-3-stop-packet-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/terrace/route-plan.md", root), "utf8"),
  ]);

  assert.deepEqual(receipt.contract.required_fields, [
    "command",
    "evidence_refs",
    "owner",
    "safe_next_step",
    "forbidden_bypass",
  ]);
  assert.equal(receipt.verification.full_ci.exit_code, 0);
  assert.equal(
    receipt.verification.separate_process_resume_proof.same_stop_packet,
    true,
  );
  assert.equal(
    receipt.verification.negative_path_proof.later_stage_rejected_while_blocked,
    true,
  );
  assert.equal(receipt.acceptance.tr_3_source_candidate_verified, true);
  assert.equal(receipt.acceptance.tr_3_integrated_into_dev, true);
  assert.equal(receipt.acceptance.tr_3_closed, true);
  assert.match(route, /TR-4 adds/);
});

test("Terrace TR-4 records bounded correction and checkpoint resume on the integrated revision", async () => {
  const [receipt, route] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/tr-4-bounded-resume-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/terrace/route-plan.md", root), "utf8"),
  ]);

  assert.equal(receipt.verification.full_ci.exit_code, 0);
  assert.equal(receipt.verification.direct_cli_trace.blocked_revision, 4);
  assert.equal(receipt.verification.direct_cli_trace.resumed_revision, 12);
  assert.equal(
    receipt.verification.direct_cli_trace
      .passed_plan_attempts_before_correction,
    1,
  );
  assert.equal(
    receipt.verification.direct_cli_trace.passed_plan_attempts_after_resume,
    1,
  );
  assert.equal(
    receipt.verification.direct_cli_trace.passed_execute_attempts_after_resume,
    2,
  );
  assert.deepEqual(receipt.verification.direct_cli_trace.successor_attempts, {
    validate: 1,
    review: 1,
    complete: 1,
  });
  assert.equal(receipt.acceptance.prior_evidence_preserved, true);
  assert.equal(receipt.acceptance.tr_4_source_candidate_verified, true);
  assert.equal(receipt.acceptance.tr_4_integrated_into_dev, true);
  assert.equal(receipt.acceptance.tr_4_closed, true);
  assert.match(
    route,
    /TR-5 proves that replay repairs a forged terminal snapshot/,
  );
});

test("Terrace TR-5 proves bypass resistance without overstating its threat model", async () => {
  const [receipt, route] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/terrace/evidence/tr-5-bypass-resistance-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/terrace/route-plan.md", root), "utf8"),
  ]);

  assert.equal(receipt.verification.full_ci.exit_code, 0);
  assert.equal(receipt.verification.full_ci.tests_passed, 314);
  assert.equal(
    receipt.verification.negative_paths
      .forged_terminal_snapshot_repaired_from_events,
    true,
  );
  assert.equal(
    receipt.verification.negative_paths
      .injected_complete_pending_to_passed_event_rejected,
    true,
  );
  assert.match(
    receipt.threat_boundary.not_claimed,
    /not protection against an attacker/i,
  );
  assert.equal(receipt.acceptance.tr_5_source_candidate_verified, true);
  assert.equal(receipt.acceptance.tr_5_integrated_into_dev, true);
  assert.equal(receipt.acceptance.tr_5_closed, true);
  assert.match(route, /local `dev` revision now contains/);
});

test("Terrace TR-6 records a reviewed responsive visual package with explicit claim boundaries", async () => {
  const [receipt, review, preview, desktop, mobile, previewCapture] =
    await Promise.all([
      readFile(
        new URL(
          "showcase-materials/terrace/evidence/tr-6-visual-review-receipt.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/terrace/comprehension-review.md", root),
        "utf8",
      ),
      readFile(
        new URL("showcase-materials/terrace/workflow-preview.html", root),
        "utf8",
      ),
      readFile(
        new URL("showcase-materials/terrace/evidence/tr-6-desktop.png", root),
      ),
      readFile(
        new URL("showcase-materials/terrace/evidence/tr-6-mobile.png", root),
      ),
      readFile(new URL("showcase-materials/terrace/preview-16x9.png", root)),
    ]);

  assert.equal(receipt.review.desktop_horizontal_overflow, false);
  assert.equal(receipt.review.mobile_horizontal_overflow, false);
  assert.equal(receipt.review.console_warnings_or_errors, 0);
  assert.equal(receipt.artifact.capture_state, "pre_integration_local_dev");
  assert.equal(receipt.acceptance.tr_6_local_visual_package_reviewed, true);
  assert.equal(receipt.acceptance.tr_6_closed, true);
  assert.match(review, /Status: passed local rendered review/);
  assert.match(review, /pre-integration/);
  assert.match(preview, /Source candidate/);
  assert.match(preview, /Integration pending/);
  assert.match(preview, /Synthetic reproducibility appendix/);
  assert.match(preview, /tr-4-bounded-resume-receipt\.json/);
  assert.match(preview, /tr-5-bypass-resistance-receipt\.json/);

  const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  assert.deepEqual(desktop.subarray(0, 8), pngSignature);
  assert.deepEqual(mobile.subarray(0, 8), pngSignature);
  assert.deepEqual(previewCapture.subarray(0, 8), pngSignature);
});

test("Context Compiler Contract CC-1 uses a real AIOS result and exact validator mutations", async () => {
  const [fixture, expected, comparison, preview, claims, receipt, route] =
    await Promise.all([
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/case-fixture.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/expected-results.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/comparison.html",
          root,
        ),
        "utf8",
      ),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/preview.svg",
          root,
        ),
        "utf8",
      ),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/evidence/claim-ledger.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/evidence/validation-receipt.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/context-compiler-contract/route-plan.md",
          root,
        ),
        "utf8",
      ),
    ]);

  const validator = await import(
    pathToFileURL(`${fixture.contract.validator_repository}/index.mjs`).href
  );
  const baseline = structuredClone(fixture.valid_result);
  const missingReason = structuredClone(baseline);
  delete missingReason.context_routing_manifest.context_sources_loaded[0]
    .reason;
  const routeMismatch = structuredClone(baseline);
  routeMismatch.packet_contract.route_compatible = false;

  assert.equal(fixture.source_kind, "real_aios_compile_output");
  assert.equal(
    fixture.source_commit,
    "a9fafaca502b3b8a5845c6f320c14d74df3de831",
  );
  assert.equal(expected.cases[0].expected.passed, true);
  assert.deepEqual(
    validator.validateCompiledContextResult(baseline),
    expected.cases[0].expected,
  );
  assert.deepEqual(
    validator.validateCompiledContextResult(missingReason),
    expected.cases[1].expected,
  );
  assert.deepEqual(
    validator.validateCompiledContextResult(routeMismatch),
    expected.cases[2].expected,
  );
  assert.deepEqual(
    validator.validateCompiledContextResult(baseline),
    expected.cases[3].expected,
  );
  assert.equal(receipt.equivalent_runs, true);
  assert.equal(
    receipt.runs[0].cases["valid-baseline"].digest,
    receipt.runs[1].cases["valid-baseline"].digest,
  );
  assert.equal(
    receipt.runs[0].cases["missing-source-reason"].digest,
    receipt.runs[1].cases["missing-source-reason"].digest,
  );
  assert.equal(
    receipt.runs[0].cases["route-boundary-mismatch"].digest,
    receipt.runs[1].cases["route-boundary-mismatch"].digest,
  );
  assert.equal(claims.review_status, "ready_for_local_package_review");
  assert.ok(claims.not_claimed.includes("general context-scope enforcement"));
  assert.match(comparison, /Provenance goes missing/);
  assert.match(comparison, /Route boundary breaks/);
  assert.match(comparison, /Validation here\. Execution elsewhere\./);
  assert.match(preview, /Context Compiler Contract: invalid to valid/);
  assert.match(preview, /a9fafac/);
  assert.match(route, /CC-1 through CC-5 closed/);
});
