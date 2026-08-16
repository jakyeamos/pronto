import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

test("Quality Runner's actual Tenure finding drivers preserve configuration and provenance", async () => {
  const packet = JSON.parse(
    await readFile(
      new URL("showcase-materials/quality-runner/finding-drivers.json", root),
      "utf8",
    ),
  );
  assert.equal(
    packet.historical_run.run_id,
    "qr060-tenure-20260717-capabilities-verify",
  );
  assert.equal(packet.historical_run.dirty, true);
  assert.equal(packet.selection.selected_pack_count, 8);
  assert.equal(packet.scan_result.total_findings, 4022);
  assert.equal(packet.scan_result.coverage_entries, 50);
  assert.equal(
    packet.scan_result.quality_skill_findings["ui-foundations"],
    192,
  );
  assert.match(
    packet.customization_model.owner_control,
    /active packs.*local skills.*scope.*thresholds/i,
  );
  assert.deepEqual(
    packet.value_execution_loop.stages.map((stage) => stage.id),
    ["standard", "negative", "contract", "finding", "plan"],
  );
  assert.match(
    packet.value_execution_loop.customer_promise,
    /standards.*guide agent output.*inspect what those agents produce/i,
  );
  assert.match(
    packet.value_execution_loop.boundary,
    /does not execute arbitrary Skill code/i,
  );
  assert.equal(
    packet.value_execution_loop.tenure_example.find(
      (item) => item.label === "Measure",
    ).value,
    "192 UI-foundations + 179 UI-specificity findings",
  );
});

test("Quality Runner's burndown distinguishes raw output from open actionable work", async () => {
  const packet = JSON.parse(
    await readFile(
      new URL("showcase-materials/quality-runner/burndown.json", root),
      "utf8",
    ),
  );
  assert.equal(packet.baseline.raw_code_quality_findings, 4022);
  assert.equal(packet.terminal_checkpoint.raw_code_quality_findings, 537);
  assert.equal(packet.terminal_checkpoint.open_code_quality_findings, 0);
  assert.match(
    packet.interpretation.forbidden_claim,
    /detector output to zero/i,
  );
  assert.match(packet.evidence_boundary, /not retained/i);
});

test("Quality Runner's change ledger binds semantic lanes to regression proof", async () => {
  const ledger = JSON.parse(
    await readFile(
      new URL("showcase-materials/quality-runner/change-ledger.json", root),
      "utf8",
    ),
  );
  assert.equal(ledger.schema_version, "pronto-showcase-change-ledger/v1");
  assert.equal(ledger.project, "quality-runner");
  assert.equal(ledger.case_id, "tenure-semantic-reconciliation-2026-08-10");
  assert.ok(ledger.lanes.length >= 5, "expected coherent change lanes");
  for (const lane of ledger.lanes) {
    assert.ok(lane.commits.length > 0, `${lane.id} has no reviewed commits`);
    assert.ok(
      lane.regression_proof.length > 0,
      `${lane.id} has no regression proof`,
    );
  }
  assert.equal(ledger.comparison.raw_finding_delta_is_outcome_proof, false);
});

test("Quality Runner's provenance receipt separates source, branch, and deployment", async () => {
  const receipt = JSON.parse(
    await readFile(
      new URL(
        "showcase-materials/quality-runner/provenance-receipt.json",
        root,
      ),
      "utf8",
    ),
  );
  assert.equal(receipt.schema_version, "pronto-showcase-provenance-receipt/v1");
  assert.equal(receipt.scan_source.mode, "inspect");
  assert.equal(receipt.scan_source.implementation_allowed, false);
  assert.equal(receipt.recorded_branch.contains_candidate, true);
  assert.equal(receipt.recorded_branch.provider_readback_status, "verified");
  assert.notEqual(
    receipt.recorded_branch.revision,
    receipt.scan_source.revision,
  );
  assert.equal(receipt.deployment.status, "not_verified");
  assert.equal(receipt.deployment.claim_allowed, false);
});

test("Quality Runner's corrected decision trail agrees with the historical burndown receipt", async () => {
  const [packet, burndown] = await Promise.all(
    [
      "showcase-materials/quality-runner/case-study.json",
      "showcase-materials/quality-runner/burndown.json",
    ].map(async (path) =>
      JSON.parse(await readFile(new URL(path, root), "utf8")),
    ),
  );
  const terminalStage = packet.stages.find((stage) => stage.id === "terminal");
  assert.equal(
    packet.headline_case.terminal_evidence_commit,
    burndown.terminal_checkpoint.evidence_commit,
  );
  assert.match(terminalStage.fact, /537 raw.*0 open actionable/i);
  assert.match(
    terminalStage.interpretation,
    /not a cosmetically empty scanner/i,
  );
  assert.match(burndown.evidence_boundary, /historical receipt/i);
});

test("Quality Runner's responsive surface review closes the corrected local surface", async () => {
  const review = await readFile(
    new URL("showcase-materials/quality-runner/surface-review.md", root),
    "utf8",
  );
  assert.match(review, /QR-5 closed/);
  assert.match(review, /4,022/);
  assert.match(review, /537 raw/);
  assert.match(review, /0 open\s+actionable/);
  assert.match(review, /Deployment remains `not_verified`/);
  assert.match(review, /no horizontal overflow/i);
  assert.match(review, /0 application errors/i);
});

test("Quality Runner's claim ledger negatively preserves every evidence level", async () => {
  const ledger = JSON.parse(
    await readFile(
      new URL("showcase-materials/quality-runner/claim-ledger.json", root),
      "utf8",
    ),
  );
  assert.equal(ledger.schema_version, "pronto-showcase-claim-ledger/v1");
  assert.deepEqual(
    ledger.levels.map((level) => level.id),
    [
      "candidate",
      "finding",
      "local_gate",
      "branch_promotion",
      "browser",
      "deployment",
    ],
  );

  for (const level of ledger.levels) {
    assert.ok(level.allowed_claim?.trim(), `${level.id} has no allowed claim`);
    assert.ok(
      level.forbidden_inference?.trim(),
      `${level.id} has no forbidden inference`,
    );
    assert.ok(
      level.evidence_refs.length > 0,
      `${level.id} has no evidence reference`,
    );
  }

  const byId = Object.fromEntries(
    ledger.levels.map((level) => [level.id, level]),
  );
  assert.equal(byId.candidate.status, "historical_scoped_signal");
  assert.match(byId.candidate.forbidden_inference, /confirmed defects/i);
  assert.equal(byId.finding.status, "historical_exact_comparison");
  assert.match(
    byId.finding.evidence_summary,
    /537 raw.*every open actionable/i,
  );
  assert.equal(byId.local_gate.status, "blocked");
  assert.match(byId.local_gate.forbidden_inference, /fresh passing gate/i);
  assert.equal(byId.branch_promotion.status, "historical_commit");
  assert.match(
    byId.branch_promotion.forbidden_inference,
    /current branch or deployment/i,
  );
  assert.equal(byId.browser.status, "local_browser_rendered");
  assert.match(byId.browser.forbidden_inference, /public hosting/i);
  assert.equal(byId.deployment.status, "not_verified");
  assert.equal(byId.deployment.public_claim_allowed, false);
  assert.deepEqual(byId.deployment.evidence_refs, [
    "final-package.json#open_gates",
  ]);
});

test("Quality Runner's QR-6 review closes around raw-versus-actionable semantics", async () => {
  const review = await readFile(
    new URL("showcase-materials/quality-runner/claim-boundary-review.md", root),
    "utf8",
  );
  assert.match(review, /QR-6 closed/);
  assert.match(review, /537 raw/);
  assert.match(review, /0 open actionable/);
  assert.match(review, /deployment remains\s+`not_verified`/i);
  assert.match(review, /local_browser_rendered/);
});

test("Quality Runner's static publication package is locally verified while delivery and video remain separate", async () => {
  const [manifest, rehearsal, publicPage, publicFindingDrivers, ...captures] =
    await Promise.all([
      readFile(
        new URL("showcase-materials/quality-runner/final-package.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/quality-runner/rehearsal.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/quality-runner/public/index.html", root),
        "utf8",
      ),
      readFile(
        new URL(
          "showcase-materials/quality-runner/public/evidence/finding-drivers.json",
          root,
        ),
        "utf8",
      ),
      ...[
        "preview-16x9.png",
        "case-study-desktop.png",
        "case-study-mobile.png",
      ].map((name) =>
        readFile(new URL(`showcase-materials/quality-runner/${name}`, root)),
      ),
    ]);

  assert.equal(manifest.schema_version, "pronto-showcase-final-package/v1");
  assert.equal(manifest.status, "static_local_review_ready");
  assert.equal(manifest.case_id, "tenure-quality-values-burndown-2026-07");
  assert.equal(
    manifest.package_tracks.static_case_study.status,
    "local_browser_verified",
  );
  assert.equal(manifest.package_tracks.video.status, "deferred");
  assert.ok(
    manifest.open_gates.includes("host_page_without_authentication"),
    "no-auth hosting remains open",
  );
  assert.ok(
    manifest.open_gates.includes("obtain_owner_approval_for_public_copy"),
    "owner copy approval remains open",
  );
  assert.ok(
    manifest.package_tracks.video.open_gates.includes(
      "record_final_human_delivery",
    ),
    "video delivery remains deferred on its own track",
  );
  assert.ok(!manifest.superseded_materials.paths.includes("public/index.html"));
  assert.ok(!manifest.superseded_materials.paths.includes("preview-16x9.png"));
  assert.equal(rehearsal.status, "superseded");
  assert.equal(rehearsal.superseded_by_case_id, manifest.case_id);
  assert.match(
    publicPage,
    /4,022 findings, driven by what this codebase values/i,
  );
  assert.match(publicPage, /accepted[\s\S]*intentional tradeoff/i);
  assert.match(
    publicPage,
    /false positive[\s\S]*source-evidenced disposition/i,
  );
  assert.match(publicPage, /537[\s\S]*0[\s\S]*open actionable findings/i);
  assert.match(publicPage, /Deployment[\s\S]*Not verified/i);
  assert.doesNotMatch(publicFindingDrivers, /\/Users\/|file:\/\//);

  const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  for (const capture of captures) {
    assert.deepEqual(capture.subarray(0, 8), pngSignature);
  }
});

test("Pre-CR keeps its standalone IDE identity while Anti-Slop supports a separate stack story", async () => {
  const [
    contract,
    readiness,
    casePacket,
    preCrRoute,
    stackRoute,
    pcr3Receipt,
    pcr3BeforeCapture,
    pcr3AfterCapture,
    preCrPreview,
    preCrPreviewCapture,
    preCrClaims,
    preCrPackage,
    preCrSurfaceReview,
  ] = await Promise.all([
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/eslint-anti-slop/pronto-case.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/pre-cr-suite/route-plan.md", root),
      "utf8",
    ),
    readFile(
      new URL("showcase-materials/ai-code-quality-stack/route-plan.md", root),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/pre-cr-suite/evidence/pcr-3-readiness-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/pre-cr-suite/evidence/pcr-3-uncovered-line.png",
        root,
      ),
    ),
    readFile(
      new URL(
        "showcase-materials/pre-cr-suite/evidence/pcr-3-passing-check.png",
        root,
      ),
    ),
    readFile(
      new URL("showcase-materials/pre-cr-suite/preview.html", root),
      "utf8",
    ),
    readFile(new URL("showcase-materials/pre-cr-suite/preview-16x9.png", root)),
    readFile(
      new URL("showcase-materials/pre-cr-suite/claim-ledger.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/pre-cr-suite/final-package.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/pre-cr-suite/surface-review.md", root),
      "utf8",
    ),
  ]);

  const antiSlop = contract.projects.find(
    (project) => project.repository_name === "eslint-plugin-anti-slop",
  );
  const stack = contract.projects.find(
    (project) => project.repository_name === "pre-cr-suite-lsp",
  );
  const antiSlopReadiness = readiness.projects.find(
    (project) => project.repository_name === "eslint-plugin-anti-slop",
  );
  const preCrReadiness = readiness.projects.find(
    (project) => project.repository_name === "pre-cr-suite-lsp",
  );
  const stackStory = readiness.combined_stories.find(
    (story) => story.story_id === "ai-code-quality-stack",
  );

  assert.equal(antiSlop.public_eligibility, "not_applicable");
  assert.equal(antiSlop.product_readiness.score, null);
  assert.equal(antiSlopReadiness.supporting_role, "js_ts_ast_detector");
  assert.equal(antiSlopReadiness.rehearsal_status, "not_applicable");
  assert.equal(stack.display_name, "Pre-CR Suite");
  assert.equal(stack.work_disposition, "targeted_gap_closure");
  assert.equal(stack.next_step_category, "packaging");
  assert.match(stack.next_step, /PCR-4/);
  assert.deepEqual(stack.missing_materials, []);
  assert.equal(stack.demo_materials.score, 4.5);
  assert.equal(preCrReadiness.display_name, "Pre-CR Suite");
  assert.equal(preCrReadiness.current_stage, "reviewed");
  assert.equal(preCrReadiness.rehearsal_status, "optional");
  assert.equal(preCrReadiness.first_required_closure, "PCR-4");
  assert.equal(preCrReadiness.remaining_gap_count_before_rehearsal, 0);
  assert.match(
    preCrRoute,
    /PCR-0 \| Define the dependable IDE subset \| \*\*Closed 2026-08-12\.\*\*/,
  );
  assert.match(
    preCrRoute,
    /PCR-1 \| Prove context continuity\s+\| \*\*Closed 2026-08-12\.\*\*/,
  );
  assert.match(
    preCrRoute,
    /PCR-2 \| Prove the useful command flow\s+\| \*\*Closed 2026-08-12\.\*\*/,
  );
  assert.match(
    preCrRoute,
    /PCR-3 \| Close one native readiness gap\s+\| \*\*Closed 2026-08-12\.\*\*/,
  );
  assert.match(
    preCrRoute,
    /PCR-4 \| Package the standalone story\s+\| \*\*Closed 2026-08-12\.\*\*/,
  );
  assert.equal(pcr3Receipt.installed_surface.before.problems, 1);
  assert.equal(pcr3Receipt.installed_surface.before.changed_line_percent, 0);
  assert.equal(pcr3Receipt.installed_surface.after.problems, 0);
  assert.equal(pcr3Receipt.installed_surface.after.changed_line_percent, 100);
  assert.equal(pcr3Receipt.headless_receipt.before.ok, false);
  assert.equal(pcr3Receipt.headless_receipt.before.gate_decision, "warn");
  assert.equal(pcr3Receipt.headless_receipt.after.ok, true);
  assert.deepEqual(
    pcr3BeforeCapture.subarray(0, 8),
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  );
  assert.deepEqual(
    pcr3AfterCapture.subarray(0, 8),
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  );
  assert.deepEqual(
    preCrPreviewCapture.subarray(0, 8),
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  );
  assert.match(preCrPreview, /Return[\s\S]*Recall[\s\S]*Act[\s\S]*Prove/);
  assert.match(preCrPreview, /Where Was I\?/);
  assert.match(preCrPreview, /Quick Actions/);
  assert.match(preCrPreview, /Pre-CR Check/);
  assert.doesNotMatch(preCrPreview, /Anti-Slop|Quality Runner/);
  assert.equal(preCrClaims.levels.length, 5);
  assert.equal(
    preCrClaims.levels.find((level) => level.id === "cli_policy").status,
    "warning_only_in_fixture",
  );
  assert.equal(preCrPackage.status, "static_local_review_ready");
  assert.equal(
    preCrPackage.capture_receipts.preview_16x9.horizontal_overflow,
    false,
  );
  assert.equal(
    preCrPackage.capture_receipts.preview_16x9.console_error_count,
    0,
  );
  assert.match(preCrSurfaceReview, /800 × 450 downscale/);
  assert.equal(stackStory.first_required_closure, "QS-0");
  assert.equal(stackStory.does_not_replace_project_routes, true);

  assert.equal(casePacket.candidate_summary.reported, 3);
  assert.equal(casePacket.candidate_summary.accepted, 2);
  assert.equal(casePacket.candidate_summary.rejected_false_positive, 1);
  assert.ok(
    casePacket.boundaries.includes("lint_candidates_are_not_confirmed_defects"),
  );

  assert.match(preCrRoute, /standalone IDE product/);
  assert.match(preCrRoute, /must not require Anti-Slop or Quality Runner/);
  assert.match(stackRoute, /Anti-Slop names it at the line/);
  assert.match(stackRoute, /Pre-CR blocks the changed-file/);
  assert.match(stackRoute, /Quality Runner places the same evidence/);
  assert.match(stackRoute, /advisory fallback/);
  assert.match(stackRoute, /deduplication/);
});
