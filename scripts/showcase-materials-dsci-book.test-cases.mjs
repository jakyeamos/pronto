import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

test("Dsci-proj DS-0 defines a reusable contract without claiming generalized implementation", async () => {
  const [contract, fixture, receipt, route, goal, readiness] =
    await Promise.all([
      readFile(
        new URL("showcase-materials/dsci-proj/decision-contract.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/dsci-proj/synthetic-fixture.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/dsci-proj/evidence/ds-0-contract-receipt.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/dsci-proj/route-plan.md", root),
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
    contract.schema_version,
    "pronto-showcase-dsci-proj-decision-contract/v1",
  );
  assert.equal(contract.status, "passed_as_product_contract");
  assert.equal(contract.implementation_status, "not_yet_proven");
  assert.deepEqual(contract.item_model.required, [
    "id",
    "title",
    "state",
    "source_type",
    "evidence",
  ]);
  assert.equal(contract.criteria.length, 5);
  assert.equal(contract.scoring.weights.must_sum_to, 1);
  assert.equal(contract.scoring.weights.user_editable, true);
  assert.equal(
    contract.constraints.missing_input_policy.startsWith("label_unknown"),
    true,
  );
  assert.equal(
    contract.explanation.per_item_required.includes("source_evidence_refs"),
    true,
  );
  assert.equal(
    contract.scenario_comparison.determinism.startsWith(
      "Same normalized inputs",
    ),
    true,
  );
  assert.ok(contract.current_native_mapping.currently_supported.length >= 4);
  assert.ok(
    contract.current_native_mapping.not_yet_supported.includes(
      "generic adapter contract for a second backlog schema",
    ),
  );

  assert.equal(fixture.source_kind, "synthetic_reproducibility_appendix");
  assert.equal(fixture.role, "appendix_only");
  assert.equal(fixture.backlogs.length, 2);
  assert.equal(fixture.backlogs[0].source_type, "research_issue_backlog");
  assert.equal(fixture.backlogs[1].source_type, "product_backlog");
  assert.equal(fixture.negative_cases.length, 3);
  assert.ok(
    fixture.negative_cases.every((item) => item.expected === "fail_closed"),
  );

  assert.equal(receipt.gap, "DS-0");
  assert.equal(receipt.status, "passed_as_product_contract");
  assert.ok(
    receipt.contract_checks.every((check) => check.status === "passed"),
  );
  assert.equal(receipt.implementation_comparison.status, "not_yet_proven");
  assert.equal(receipt.synthetic_appendix.role, "appendix_only");
  assert.equal(receipt.next_gap, "DS-1");

  const project = goal.projects.find(
    (candidate) => candidate.repository_name === "Dsci-proj",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "Dsci-proj",
  );
  assert.equal(project?.next_step_category, "product");
  assert.match(project?.next_step ?? "", /DS-1/);
  assert.equal(readinessProject?.first_required_closure, "DS-1");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "implementation_adapter_required",
  );
  assert.match(route, /DS-0 closure/);
  assert.match(route, /decision-contract\.json/);
  assert.match(route, /does not yet accept/i);
});

test("Dsci-proj exposes a labeled synthetic contract preview without overclaiming product behavior", async () => {
  const [preview, materialReceipt, route, goal, readiness] = await Promise.all([
    readFile(
      new URL("showcase-materials/dsci-proj/synthetic-preview.html", root),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/dsci-proj/evidence/ds-0-synthetic-material-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/dsci-proj/route-plan.md", root),
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

  assert.match(
    preview,
    /Synthetic showcase fixture · not current product output/,
  );
  assert.match(preview, /Two backlog shapes/);
  assert.match(preview, /One canonical item/);
  assert.match(preview, /What this preview proves/);
  assert.match(preview, /DS-1 adapter proof remains open/);
  assert.doesNotMatch(preview, /fonts\.googleapis|<script[^>]+src=/i);

  assert.equal(materialReceipt.status, "passed_as_synthetic_contract_preview");
  assert.equal(materialReceipt.artifact.label_required, true);
  assert.equal(materialReceipt.artifact.network, "none");
  assert.equal(
    materialReceipt.surface_probe.status,
    "passed_as_static_http_fetch",
  );
  assert.match(
    materialReceipt.claim_boundary.join(" | "),
    /does not prove DS-1 adapters/i,
  );
  assert.equal(materialReceipt.next_gap, "DS-1");

  const project = goal.projects.find(
    (candidate) => candidate.repository_name === "Dsci-proj",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "Dsci-proj",
  );
  assert.equal(project?.demo_materials?.score, 2.1);
  assert.match(project?.demo_materials?.evidence ?? "", /shareable/i);
  assert.match(
    project?.missing_materials?.join(" | ") ?? "",
    /interactive product walkthrough/i,
  );
  assert.equal(readinessProject?.first_required_closure, "DS-1");
  assert.match(route, /synthetic-preview\.html/);
  assert.match(route, /does not close DS-1/i);
});

test("Book uses owner-approved synthetic text and copyright-free music while preserving the real-asset boundary", async () => {
  const [
    ledger,
    fixture,
    blocker,
    approval,
    preview,
    materialReceipt,
    route,
    goal,
    readiness,
  ] = await Promise.all([
    readFile(
      new URL("showcase-materials/book/contribution-ledger.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/book/synthetic-fixture.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/book/evidence/bk-1-blocker.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/book/evidence/bk-1-synthetic-scope-approval.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/book/synthetic-preview.html", root),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/book/evidence/bk-1-synthetic-material-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/book/route-plan.md", root), "utf8"),
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.equal(
    ledger.schema_version,
    "pronto-showcase-book-contribution-ledger/v1",
  );
  assert.equal(ledger.status, "scoped_synthetic_case");
  assert.equal(ledger.publication_ready, false);
  assert.equal(ledger.showcase_scope.mode, "synthetic_only");
  assert.equal(ledger.showcase_scope.real_repository_chapter_used, false);
  assert.equal(ledger.showcase_scope.real_repository_audio_used, false);
  assert.equal(ledger.candidate.chapter_id, "chapter1");
  assert.equal(
    ledger.contributions.find((item) => item.id === "audio")?.rights_status,
    "unknown_pending_license_or_permission",
  );
  assert.equal(
    ledger.contributions.find((item) => item.id === "ai_assistance")
      ?.observed_status,
    "not_determined",
  );
  assert.match(
    ledger.claim_boundary.join(" | "),
    /No rights clearance for the real chapter/,
  );

  assert.equal(
    fixture.schema_version,
    "pronto-showcase-book-synthetic-fixture/v1",
  );
  assert.equal(fixture.source_kind, "synthetic_original");
  assert.equal(fixture.role, "owner_approved_synthetic_showcase_case");
  assert.equal(fixture.fixture_policy.owner_approved_synthetic_case, true);
  assert.equal(fixture.fixture_policy.third_party_audio, false);
  assert.equal(fixture.fixture_policy.copyright_free_music, true);
  assert.equal(fixture.media_plan.audio.kind, "original_synthetic_music_motif");
  assert.equal(fixture.chapter.beats.length, 4);
  assert.equal(fixture.media_plan.controls.includes("reduced_motion"), true);
  assert.match(fixture.display_rules.join(" | "), /synthetic-fixture label/i);

  assert.equal(blocker.gap, "BK-1");
  assert.equal(blocker.status, "resolved_scoped");
  assert.equal(blocker.disposition, "owner_approved_scoped_synthetic_case");
  assert.equal(blocker.resolution.selected_mode, "synthetic_only");
  assert.match(blocker.blocked_action, /Do not publish/i);
  assert.equal(blocker.synthetic_fallback.does_not_clear_real_asset_path, true);

  assert.equal(approval.status, "resolved_scoped");
  assert.equal(approval.disposition, "owner_approved_scoped_synthetic_case");
  assert.equal(approval.approval.approver, "Jakye Amos");
  assert.equal(approval.scope.real_book_excerpt_used, false);
  assert.equal(approval.scope.third_party_recording_used, false);
  assert.equal(
    approval.asset_decisions.music.classification,
    "copyright_free_original_synthetic_music",
  );

  assert.match(preview, /Synthetic showcase fixture · not product authorship/);
  assert.match(preview, /The Signal Room/);
  assert.match(preview, /Reader controls stay in the story/);
  assert.match(preview, /What this page proves/);
  assert.doesNotMatch(
    preview,
    /fonts\.googleapis|Eva_Angelina|Mojo_Pin|Rose_Parade/,
  );
  assert.equal(
    materialReceipt.status,
    "passed_as_owner_approved_synthetic_case",
  );
  assert.equal(materialReceipt.artifact.label_required, true);
  assert.equal(
    materialReceipt.surface_probe.status,
    "passed_as_static_http_fetch",
  );
  assert.match(
    materialReceipt.claim_boundary.join(" | "),
    /supports BK-1 only for the synthetic scope/,
  );

  const project = goal.projects.find(
    (candidate) => candidate.repository_name === "Book",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "Book",
  );
  assert.equal(project?.next_step_category, "content");
  assert.match(project?.next_step ?? "", /BK-2/);
  assert.equal(project?.blockers?.length, 0);
  assert.equal(readinessProject?.first_required_closure, "BK-2");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "local_material_package_complete_synthetic_scope_approved_direct_surface_and_hosting_required",
  );
  assert.match(route, /BK-1 synthetic scope/);
  assert.match(route, /synthetic-fixture\.json/);
  assert.match(route, /synthetic-preview\.html/);
  assert.match(route, /original synthetic music motif/);
});
