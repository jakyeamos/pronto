import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

test("Participant Deduplication PD-1 binds a synthetic workbook to native matcher evidence", async () => {
  const [fixture, receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/synthetic-fixture.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-1-fixture-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/route-plan.md",
        root,
      ),
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

  assert.equal(
    fixture.schema_version,
    "pronto-showcase-participant-dedup-synthetic-workbook/v1",
  );
  assert.equal(fixture.project, "participant-dedup");
  assert.equal(fixture.gap, "PD-1");
  assert.equal(fixture.source_kind, "synthetic_original");
  assert.equal(
    fixture.fixture_policy.visible_label,
    "Synthetic showcase fixture · no participant data",
  );
  assert.equal(fixture.fixture_policy.real_participant_data, false);
  assert.equal(fixture.fixture_policy.external_mutation, false);
  assert.equal(fixture.workbook.rows.length, 8);
  assert.equal(fixture.cases.length, 4);
  assert.deepEqual(
    fixture.cases.map((candidate) => candidate.case_type),
    ["exact", "fuzzy", "shared_contact_negative", "ambiguous_non_duplicate"],
  );
  assert.equal(
    new Set(fixture.workbook.rows.map((row) => row[0])).size,
    fixture.workbook.rows.length,
  );
  assert.ok(
    fixture.workbook.rows.every((row) => row[9].endsWith("@example.test")),
  );
  assert.deepEqual(
    fixture.cases.map((candidate) => candidate.reviewer_outcome.mode),
    ["SELECT_RECORDS", "SELECT_RECORDS", "KEEP_ALL", "UNRESOLVED"],
  );
  assert.equal(fixture.expected_review_summary.candidate_pair_count, 4);
  assert.equal(fixture.expected_review_summary.approved_pair_count, 2);
  assert.equal(fixture.expected_review_summary.delete_count_if_approved, 2);
  assert.equal(fixture.negative_cases.length, 3);
  assert.equal(fixture.privacy_review.status, "passed");
  assert.ok(
    fixture.claim_boundary.some((claim) => /does not prove/i.test(claim)),
  );

  assert.equal(
    receipt.schema_version,
    "pronto-showcase-participant-dedup-gap-receipt/v1",
  );
  assert.equal(receipt.gap, "PD-1");
  assert.equal(receipt.status, "passed_as_synthetic_fixture");
  assert.equal(receipt.source.checkout_status, "clean_at_inspection");
  assert.equal(
    receipt.source.commit,
    "81569f4bff60e4f2d52cb0a401289ca1c5d80ff8",
  );
  assert.equal(
    receipt.verification.repository_quality.typecheck.status,
    "passed",
  );
  assert.equal(receipt.verification.repository_quality.build.status, "passed");
  assert.equal(receipt.verification.repository_quality.tests.test_files, 48);
  assert.equal(receipt.verification.repository_quality.tests.tests_passed, 418);
  assert.equal(receipt.verification.repository_quality.tests.tests_skipped, 3);
  assert.equal(
    receipt.verification.native_matcher_probe.candidate_pair_count,
    4,
  );
  assert.equal(receipt.verification.native_matcher_probe.truncated, false);
  assert.deepEqual(
    receipt.verification.native_matcher_probe.pairs.map(
      (candidate) => candidate.confidence,
    ),
    ["HIGH", "HIGH", "EXCLUDED", "LOW"],
  );
  assert.equal(receipt.verification.installed_surface.status, "not_proven");
  assert.ok(
    Object.entries(receipt.verification.installed_surface)
      .filter(([key]) => key !== "status")
      .every(
        ([, value]) =>
          value === "not exercised" ||
          value === "not exercised live" ||
          value === "not captured" ||
          value === "not hosted",
      ),
  );
  assert.equal(receipt.next_gap, "PD-2");

  assert.match(route, /PD-1 closure/);
  assert.match(route, /PD-2/);
  assert.match(route, /SAME_HOUSEHOLD_ONLY/);
  assert.match(route, /no Google account/i);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(project);
  assert.equal(project.next_step_category, "demo_integration");
  assert.equal(project.demo_materials.score, 3.4);
  assert.match(project.next_step, /PD-6/);
  assert.ok(
    project.missing_materials.every(
      (material) =>
        !/synthetic workbook|atomic apply|stale-state|copy-only/i.test(
          material,
        ),
    ),
  );

  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(readinessProject);
  assert.equal(readinessProject.first_required_closure, "PD-6");
  assert.equal(readinessProject.remaining_gap_count_before_rehearsal, 2);
  assert.equal(
    readinessProject.rehearsal_disposition,
    "authenticated_live_sheet_and_public_case_required",
  );
});

test("Participant Deduplication PD-2 preserves field-level reasoning without exposing apply", async () => {
  const [reasoning, receipt, route, contract, readiness] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/candidate-reasoning.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-2-explanation-review.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/route-plan.md",
        root,
      ),
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

  assert.equal(
    reasoning.schema_version,
    "pronto-showcase-participant-dedup-candidate-reasoning/v1",
  );
  assert.equal(reasoning.gap, "PD-2");
  assert.equal(reasoning.presentation_policy.mutating_actions, false);
  assert.deepEqual(reasoning.queue.default_visible_case_ids, [
    "exact-duplicate",
    "fuzzy-name-typo",
  ]);
  assert.deepEqual(
    reasoning.queue.items.map((item) => item.native_score.confidence),
    ["HIGH", "HIGH", "LOW", "EXCLUDED"],
  );
  assert.ok(
    reasoning.queue.items.every(
      (item) =>
        item.field_contributions.length >= 5 &&
        item.presentation.confidence_limit &&
        item.presentation.why_review_is_required &&
        item.native_score.total_score >= 0,
    ),
  );
  assert.deepEqual(
    reasoning.queue.items.map((item) => item.presentation.reviewer_action),
    ["SELECT_RECORDS", "SELECT_RECORDS", "UNRESOLVED", "KEEP_ALL"],
  );
  assert.deepEqual(reasoning.queue.items[2].conflicts, [
    "CONFLICTING_VALID_DOB",
    "DIFFERENT_ADDRESS",
  ]);
  assert.deepEqual(reasoning.queue.items[3].conflicts, [
    "PLACEHOLDER_DOB",
    "SAME_HOUSEHOLD_ONLY",
  ]);
  assert.equal(reasoning.review_contract.approved_delete_count, 2);
  assert.equal(reasoning.review_contract.original_rows_changed, 0);
  assert.equal(reasoning.next_gap, "PD-3");

  assert.equal(
    receipt.schema_version,
    "pronto-showcase-participant-dedup-explanation-receipt/v1",
  );
  assert.equal(receipt.gap, "PD-2");
  assert.equal(receipt.status, "passed_as_source_aligned_queue_fixture");
  assert.equal(receipt.source.checkout_status, "clean_at_inspection");
  assert.equal(
    receipt.verification.native_matcher_probe.field_components_captured,
    true,
  );
  assert.ok(
    receipt.verification.explanation_checks.every(
      (check) => check.status === "passed",
    ),
  );
  assert.equal(receipt.verification.installed_surface.status, "not_proven");
  assert.equal(receipt.next_gap, "PD-3");

  assert.match(route, /PD-2 closure/);
  assert.match(route, /no authenticated Google Sheet/i);
  assert.match(route, /PD-3 reviewer-controlled atomic\s+apply/i);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(project);
  assert.equal(project.next_step_category, "demo_integration");
  assert.equal(project.demo_materials.score, 3.4);
  assert.match(project.next_step, /PD-6/);
  assert.ok(
    project.missing_materials.some((material) => /live-sheet/i.test(material)),
  );

  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(readinessProject);
  assert.equal(readinessProject.first_required_closure, "PD-6");
  assert.equal(readinessProject.remaining_gap_count_before_rehearsal, 2);
});

test("Participant Deduplication PD-3 through PD-7 close local safety and case material", async () => {
  const [
    atomic,
    stale,
    recovery,
    publicCase,
    caseHtml,
    route,
    contract,
    readiness,
  ] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-3-atomic-apply-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-4-stale-state-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-5-copy-recovery-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/evidence/pd-7-public-case-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/case-study.html",
        root,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/participant-deduplication/route-plan.md",
        root,
      ),
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

  for (const [receipt, gap] of [
    [atomic, "PD-3"],
    [stale, "PD-4"],
    [recovery, "PD-5"],
  ]) {
    assert.equal(receipt.project, "participant-dedup");
    assert.equal(receipt.gap, gap);
    assert.equal(receipt.status, "passed_as_local_behavior_fixture");
    assert.equal(receipt.source.checkout_status, "clean_at_inspection");
    assert.equal(receipt.verification.targeted_tests.tests_passed, 66);
    assert.equal(receipt.verification.installed_surface.status, "not_proven");
  }

  assert.equal(atomic.verification.apply_probe.apply_result.deleted_rows, 3);
  assert.equal(atomic.verification.apply_probe.apply_result.filled_fields, 1);
  assert.equal(atomic.verification.apply_probe.apply_result.audit_written, 7);
  assert.equal(stale.verification.stale_probe.observed_error, "STALE_ROW");
  assert.equal(stale.verification.stale_probe.rows_unchanged_by_guard, true);
  assert.equal(recovery.verification.copy_probe.original.unchanged, true);
  assert.equal(recovery.verification.copy_probe.copy.rows_after, 6);
  assert.equal(
    recovery.verification.copy_probe.recovery.snapshots_present,
    true,
  );

  assert.equal(
    publicCase.schema_version,
    "pronto-showcase-participant-dedup-public-case-receipt/v1",
  );
  assert.equal(publicCase.gap, "PD-7");
  assert.equal(publicCase.status, "passed_as_local_static_case");
  assert.equal(publicCase.material.no_auth_required, true);
  assert.equal(publicCase.material.mutating_controls, false);
  assert.equal(
    publicCase.verification.local_rendered_surface.status,
    "blocked",
  );
  assert.match(
    publicCase.verification.local_rendered_surface.observation,
    /status 134/,
  );
  assert.equal(
    publicCase.verification.static_marker_review.queue_case_count,
    4,
  );
  assert.equal(
    publicCase.verification.static_marker_review.proof_panel_count,
    4,
  );
  assert.equal(publicCase.next_gap, "PD-6");
  for (const marker of publicCase.verification.static_marker_review
    .required_markers) {
    assert.match(
      caseHtml,
      new RegExp(marker.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
    );
  }

  assert.match(route, /PD-3\/PD-4\/PD-5 behavior closure/);
  assert.match(route, /parked:\s+authenticated provider/);
  assert.match(route, /PD-7 local no-auth case closure/);
  assert.match(route, /closed locally after the park/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(project);
  assert.equal(project.next_step_category, "demo_integration");
  assert.equal(project.demo_materials.score, 3.4);
  assert.match(project.next_step, /PD-6/);
  assert.deepEqual(project.missing_materials, [
    "authenticated live-sheet UAT receipt",
    "hosted no-auth case URL and responsive readback",
  ]);
  assert.equal(project.blockers.length, 1);
  assert.match(project.blockers[0], /owner-authenticated Google Sheets/i);

  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "participant-dedup",
  );
  assert.ok(readinessProject);
  assert.equal(readinessProject.first_required_closure, "PD-6");
  assert.equal(readinessProject.remaining_gap_count_before_rehearsal, 2);
  assert.equal(
    readinessProject.rehearsal_disposition,
    "authenticated_live_sheet_and_public_case_required",
  );
});
