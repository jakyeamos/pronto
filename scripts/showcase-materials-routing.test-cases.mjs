import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

test("Agent Router AR-1 binds a labeled replay case to a native typed graph probe", async () => {
  const [fixture, receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL("showcase-materials/agent-router/replay-case.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/agent-router/evidence/ar-1-contract-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/agent-router/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(fixture.gap, "AR-1");
  assert.equal(fixture.source_kind, "synthetic_reproducibility_appendix");
  assert.equal(fixture.role, "appendix_only");
  assert.equal(fixture.expected_graph.subtasks.length, 3);
  assert.deepEqual(
    fixture.expected_graph.subtasks.map((subtask) => subtask.id),
    [
      "ar-1-launch-readiness-plan",
      "ar-1-launch-readiness-execute",
      "ar-1-launch-readiness-verify",
    ],
  );
  assert.deepEqual(fixture.expected_graph.edges, [
    {
      from: "ar-1-launch-readiness-plan",
      to: "ar-1-launch-readiness-execute",
    },
    {
      from: "ar-1-launch-readiness-execute",
      to: "ar-1-launch-readiness-verify",
    },
  ]);
  assert.equal(fixture.negative_cases.length, 3);
  assert.ok(
    fixture.negative_cases.every(
      (negativeCase) =>
        negativeCase.expected === "fail_closed_in_replay_harness",
    ),
  );

  assert.equal(receipt.status, "passed_as_replay_spec");
  assert.equal(receipt.source.package_manager, "pnpm@10.12.4");
  assert.equal(receipt.verification.runtime_probe.status, "passed");
  assert.equal(receipt.verification.runtime_probe.graph.subtask_count, 3);
  assert.equal(receipt.verification.runtime_probe.graph.edge_count, 2);
  assert.equal(
    receipt.verification.runtime_probe.graph.intent_constraints_preserved,
    true,
  );
  assert.ok(
    receipt.verification.repository_quality.every(
      (check) => check.status === "passed",
    ),
  );
  assert.equal(receipt.implementation_comparison.status, "partial_native");
  assert.equal(receipt.synthetic_appendix.negative_cases, 3);
  assert.match(route, /AR-1 closure/);
  assert.match(route, /AR-2\/AR-3 closure/);
  assert.match(route, /does not\s+claim provider execution/i);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  assert.equal(project?.next_step_category, "product");
  assert.match(project?.next_step ?? "", /AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 2);
});

test("Agent Router AR-2 and AR-3 preserve candidate alternatives and selection traceability", async () => {
  const [receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/agent-router/evidence/ar-2-3-routing-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/agent-router/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(receipt.status, "passed_as_native_replay_candidate_evidence");
  assert.deepEqual(receipt.gaps, ["AR-2", "AR-3"]);
  assert.equal(receipt.case.provider_invocations, 0);
  assert.equal(receipt.verification.native_probe.status, "passed");
  assert.equal(
    receipt.verification.native_probe.candidate_order_stable_across_subtasks,
    true,
  );
  assert.equal(receipt.candidate_packet.candidates.length, 7);
  assert.ok(
    receipt.candidate_packet.candidates.filter(
      (candidate) => candidate.eligible && candidate.observed_task_evidence,
    ).length >= 2,
  );
  const winner = receipt.candidate_packet.candidates.find(
    (candidate) => candidate.provider_id === "codex",
  );
  const cursor = receipt.candidate_packet.candidates.find(
    (candidate) => candidate.provider_id === "cursor",
  );
  const claude = receipt.candidate_packet.candidates.find(
    (candidate) => candidate.provider_id === "claude",
  );
  assert.equal(winner?.eligible, true);
  assert.equal(winner?.evidence_confidence, "high");
  assert.equal(cursor?.eligible, true);
  assert.equal(cursor?.evidence_confidence, "low");
  assert.deepEqual(claude?.disqualifiers, ["below_quality_floor"]);
  assert.equal(receipt.selection_trace.length, 7);
  assert.ok(
    receipt.selection_trace.every(
      (trace) => trace.evidence_refs.length > 0 && trace.reason.length > 0,
    ),
  );
  assert.match(route, /AR-2\/AR-3 closure/);
  assert.match(
    route,
    /does not turn the relative usage proxy into currency cost/,
  );
  assert.match(route, /AR-4 execution appendix/);
  assert.match(route, /queue moves on/);
  assert.match(route, /Park AR-5/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  assert.equal(project?.demo_materials?.score, 3.2);
  assert.match(project?.next_step ?? "", /AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 2);
});

test("Agent Router AR-4 closes only the explicitly labeled bounded execution replay", async () => {
  const [fixture, receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL("showcase-materials/agent-router/execution-replay.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/agent-router/evidence/ar-2-3-routing-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/agent-router/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(fixture.gap, "AR-4");
  assert.equal(fixture.execution_mode, "bounded_replay");
  assert.equal(fixture.worker_receipts.length, 3);
  assert.deepEqual(
    fixture.worker_receipts.map((worker) => worker.subtaskId),
    fixture.selected_route.subtask_ids,
  );
  assert.ok(
    fixture.worker_receipts.every(
      (worker) =>
        worker.status === "success" &&
        worker.providerId === "codex" &&
        worker.usage.source === "unknown" &&
        worker.usage.redactionApplied === true,
    ),
  );
  assert.equal(fixture.reconciliation.every_result_attributable, true);
  assert.equal(fixture.reconciliation.provider_execution_invocations, 0);
  assert.equal(fixture.reconciliation.worktree_mutations, 0);
  assert.equal(fixture.replay_matrix.status, "passed");
  assert.equal(fixture.replay_matrix.case_count, 5);
  assert.equal(receipt.status, "passed_as_native_replay_candidate_evidence");
  assert.match(route, /AR-4 bounded execution replay/);
  assert.match(route, /zero provider invocations/);
  assert.match(route, /queue moves on/);
  assert.match(route, /Park AR-5/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  assert.equal(project?.demo_materials?.score, 3.2);
  assert.match(project?.next_step ?? "", /AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 2);
});

test("Agent Router AR-5 records the native conflict boundary without overclaiming confidence or fallback", async () => {
  const [receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/agent-router/evidence/ar-5-blocker.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/agent-router/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(receipt.gap, "AR-5");
  assert.equal(receipt.status, "partial_native_conflict_boundary");
  assert.equal(
    receipt.disposition,
    "parked_pending_synthesis_confidence_or_fallback_contract",
  );
  assert.equal(
    receipt.native_probe.status,
    "passed_for_conflict_detection_and_mode_selection",
  );
  assert.equal(receipt.native_probe.invocations, 0);
  assert.equal(receipt.native_probe.mode, "model_synthesis");
  assert.equal(receipt.native_probe.conflicts.length, 5);
  assert.match(
    receipt.missing_contract.required.join(" "),
    /confidence field.*fallback disposition/i,
  );
  assert.match(receipt.blocked_action, /Do not claim AR-5 complete/);
  assert.match(route, /AR-5 partial conflict boundary/);
  assert.match(route, /confidence field/);
  assert.match(route, /queue moves on/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "agent-router",
  );
  assert.match(project?.blockers?.[0] ?? "", /AR-5 is blocked/);
  assert.match(project?.next_step ?? "", /Park AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "local_candidate_package_complete; blocked_native_synthesis_output_contract",
  );
});

test("Chiron's Forge CF-0 records the genuine provenance blocker before the queue moves on", async () => {
  const [receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/chirons-forge/evidence/cf-0-blocker.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/chirons-forge/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(receipt.gap, "CF-0");
  assert.equal(receipt.status, "blocked");
  assert.equal(
    receipt.disposition,
    "parked_pending_backing_repository_and_deployment_provenance",
  );
  assert.equal(receipt.public_surface.observed_http_status, 200);
  assert.deepEqual(receipt.public_surface.repository_links_found, []);
  assert.deepEqual(receipt.public_surface.revision_identifiers_found, []);
  assert.equal(receipt.public_surface.authenticated_build_receipt_found, false);
  assert.deepEqual(receipt.local_provenance_search.matching_directories, []);
  assert.match(receipt.blocked_action, /Do not strengthen/);
  assert.match(route, /CF-0 provenance disposition/);
  assert.match(route, /genuine provenance boundary/);
  assert.match(route, /queue should move to the next runnable project/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "chirons-forge",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "chirons-forge",
  );
  assert.match(project?.blockers?.[0] ?? "", /CF-0 is blocked/);
  assert.match(project?.next_step ?? "", /Park CF-0/);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "blocked_pending_repository_and_deployment_provenance",
  );
});

test("Codex Browser Control CB-1 binds a synthetic page to the source target contract", async () => {
  const [fixture, receipt, html, route, contract, readiness] =
    await Promise.all([
      readFile(
        new URL(
          "showcase-materials/codex-browser-control/synthetic-fixture.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/codex-browser-control/evidence/cb-1-fixture-receipt.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/codex-browser-control/demo-site.html",
          root,
        ),
        "utf8",
      ),
      readFile(
        new URL("showcase-materials/codex-browser-control/route-plan.md", root),
        "utf8",
      ),
      readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
        JSON.parse,
      ),
      readFile(
        new URL("showcase-materials/rehearsal-readiness.json", root),
        "utf8",
      ).then(JSON.parse),
    ]);

  assert.equal(fixture.project, "browser-control");
  assert.equal(fixture.gap, "CB-1");
  assert.equal(fixture.source_kind, "synthetic_original");
  assert.equal(fixture.fixture_policy.label_required, true);
  assert.equal(fixture.fixture_policy.network_calls, false);
  assert.deepEqual(fixture.semantic_target.protocol_identity, {
    tag: "select",
    role: "combobox",
    name: "Seat preference",
    disabled: false,
  });
  assert.equal(fixture.target_action.action.type, "page_select");
  assert.equal(fixture.target_action.action.value, "aisle");
  assert.equal(fixture.target_action.expected_plan_state, "approval_required");
  assert.equal(
    fixture.target_action.expected_next_tool,
    "browser.await_approval",
  );
  assert.equal(
    fixture.stale_mutation.expected_refusal.error_code,
    "target_changed",
  );
  assert.equal(
    fixture.stale_mutation.expected_refusal.mutation_attempted,
    false,
  );
  assert.equal(fixture.negative_cases.length, 3);

  assert.match(html, /Synthetic showcase fixture/);
  assert.match(html, /aria-label="Seat preference"/);
  assert.match(html, /id="change-itinerary"/);
  assert.match(html, /id="reset-fixture"/);
  assert.doesNotMatch(html, /fetch\s*\(|XMLHttpRequest|localStorage/);

  assert.equal(receipt.status, "passed_as_synthetic_fixture");
  assert.equal(receipt.source.checkout_status, "dirty_at_inspection");
  assert.equal(receipt.verification.repository_quality.status, "passed");
  assert.equal(receipt.verification.installed_surface.status, "not_proven");
  assert.ok(
    receipt.contract_checks.every((check) => check.status === "passed"),
  );
  assert.equal(receipt.next_gap, "CB-2");
  assert.match(route, /CB-1 closure/);
  assert.match(route, /CB-2[\s\S]*installed[\s\S]*observe-to-plan/);
  assert.match(route, /does not claim an installed extension/i);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "browser-control",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "browser-control",
  );
  assert.equal(project?.next_step_category, "demo_integration");
  assert.match(project?.next_step ?? "", /CB-2\/CB-3/);
  assert.equal(readinessProject?.first_required_closure, "CB-2");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
});

test("Codex Browser Control CB-2 records the installed version and protocol blocker", async () => {
  const [receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/codex-browser-control/evidence/cb-2-blocker.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/codex-browser-control/route-plan.md", root),
      "utf8",
    ),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(receipt.gap, "CB-2");
  assert.equal(receipt.status, "blocked_installed_round_trip");
  assert.equal(
    receipt.disposition,
    "parked_pending_reviewed_extension_reload_and_protocol_match",
  );
  assert.equal(receipt.source.source_quality.status, "passed");
  assert.equal(receipt.installed_probe.configured, true);
  assert.equal(receipt.installed_probe.registered, true);
  assert.equal(receipt.installed_probe.connected, true);
  assert.equal(receipt.installed_probe.round_trip_verified, false);
  assert.equal(receipt.installed_probe.connected_extension_version, "0.3.0");
  assert.match(receipt.installed_probe.round_trip_error, /schema validation/i);
  assert.equal(receipt.installed_probe.side_panel_plan_receipt, "not_captured");
  assert.match(
    receipt.blocked_action,
    /Do not claim installed observe-to-plan/,
  );
  assert.match(route, /CB-2 installed round-trip boundary/);
  assert.match(route, /reloaded together/);
  assert.match(route, /Park CB-2\/CB-3/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "browser-control",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "browser-control",
  );
  assert.match(project?.blockers?.[0] ?? "", /CB-2 is blocked/);
  assert.match(project?.next_step ?? "", /Park CB-2\/CB-3/);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "local_candidate_package_complete; blocked_installed_version_and_protocol_mismatch",
  );
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
});
