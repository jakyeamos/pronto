import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { pathToFileURL, URL } from "node:url";

const root = new URL("../", import.meta.url);
const allowedDispositions = new Set([
  "largely_product_ready",
  "targeted_gap_closure",
  "material_build_or_restoration",
  "conditional_gate",
]);
const allowedCategories = new Set([
  "product",
  "demo_integration",
  "evidence",
  "content",
  "packaging",
]);
const allowedDimensionStatuses = new Set([
  "assessed",
  "unknown",
  "blocked",
  "not_applicable",
]);

test("the Showcase goal and readiness ledger stay in lockstep", async () => {
  const [contract, readiness, workspaceReadme] = await Promise.all([
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(new URL("showcase-materials/README.md", root), "utf8"),
  ]);

  const publicProjects = contract.projects.filter(
    (project) => project.public_eligibility === "public_showcase",
  );
  const readinessProjects = readiness.projects.filter(
    (project) =>
      project.showcase_eligible !== false &&
      allowedDispositions.has(project.work_disposition),
  );

  assert.equal(
    contract.target_publishable_demo_count,
    publicProjects.length,
    "goal target must equal its public project count",
  );
  assert.equal(
    readiness.summary.project_count,
    readiness.projects.length,
    "readiness project count must equal its ledger length",
  );
  assert.equal(
    readiness.summary.showcase_eligible_count,
    readinessProjects.length,
    "readiness eligible count must equal its eligible ledger rows",
  );
  assert.deepEqual(
    publicProjects.map((project) => project.repository_name).sort(),
    readinessProjects.map((project) => project.repository_name).sort(),
    "goal and readiness must name the same public projects",
  );
  assert.match(
    workspaceReadme,
    new RegExp(
      `exact ${readiness.summary.project_count}-project\\s+video-enhancement ledger`,
    ),
  );

  for (const project of publicProjects) {
    const readinessProject = readinessProjects.find(
      (candidate) => candidate.repository_name === project.repository_name,
    );
    assert.ok(
      readinessProject,
      `${project.repository_name} is missing from readiness`,
    );
    assert.equal(
      readinessProject.work_disposition,
      project.work_disposition,
      `${project.repository_name} disposition differs between ledgers`,
    );
    const activeGap = project.next_step.match(/[A-Z]{2,3}-\d+/)?.[0];
    assert.equal(
      readinessProject.first_required_closure,
      activeGap,
      `${project.repository_name} active gap differs between ledgers`,
    );
  }
});

test("video is optional across the active Showcase publication goal", async () => {
  const [contract, contractDoc, targets, readiness] = await Promise.all([
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(new URL("docs/showcase-contract.md", root), "utf8"),
    readFile(new URL("showcase-materials/ideal-demo-targets.md", root), "utf8"),
    readFile(
      new URL("showcase-materials/rehearsal-readiness.json", root),
      "utf8",
    ).then(JSON.parse),
  ]);

  assert.match(
    contract.quality_bar_source,
    /video and human narration are optional/i,
  );
  assert.match(
    contractDoc,
    /human voice recording is not a fleet-wide requirement/i,
  );
  assert.match(targets, /video is an optional enhancement/i);
  assert.equal(readiness.publication_gate, false);
  assert.equal(readiness.optional_stage, "video_enhancement");

  const videoOnlyRequirement =
    /(?:45|60|90).{0,20}second|human recording|final recording|captioned (?:demo|walkthrough|recording)|rehearsal/i;
  for (const project of contract.projects.filter(
    (candidate) => candidate.public_eligibility === "public_showcase",
  )) {
    assert.doesNotMatch(
      project.missing_materials.join(" | "),
      videoOnlyRequirement,
      `${project.repository_name} still treats video as required`,
    );
  }
});

test("every public Showcase project preserves a granular gap disposition", async () => {
  const contract = JSON.parse(
    await readFile(new URL(".pronto/showcase-goal.json", root), "utf8"),
  );
  assert.equal(contract.schema_version, "pronto-showcase-goal/v2");

  const publicProjects = contract.projects.filter(
    (project) => project.public_eligibility === "public_showcase",
  );
  assert.ok(publicProjects.length > 0, "expected public Showcase projects");

  for (const project of publicProjects) {
    for (const dimension of [
      "product_readiness",
      "demo_materials",
      "career_signal",
    ]) {
      assert.ok(
        allowedDimensionStatuses.has(project[dimension]?.status),
        `${project.repository_name}.${dimension} has an invalid status`,
      );
    }
    assert.ok(
      allowedDispositions.has(project.work_disposition),
      `${project.repository_name} has an invalid work disposition`,
    );
    assert.ok(
      project.work_disposition_summary?.trim(),
      `${project.repository_name} is missing a disposition summary`,
    );
    assert.ok(
      allowedCategories.has(project.next_step_category),
      `${project.repository_name} has an invalid next-step category`,
    );

    const routeMatch = project.next_step.match(
      /showcase-materials\/([^ ]+)\/route-plan\.md/,
    );
    assert.ok(
      routeMatch,
      `${project.repository_name} next step has no route plan`,
    );
    const routePlan = await readFile(
      new URL(`showcase-materials/${routeMatch[1]}/route-plan.md`, root),
      "utf8",
    );
    assert.ok(
      routePlan.includes(
        "Project disposition: `" + project.work_disposition + "`",
      ),
      `${project.repository_name} route disposition differs from the contract`,
    );

    const ledgerIds = [
      ...routePlan.matchAll(/^\| ([A-Z]{2,3}-\d+)\s*\|/gm),
    ].map((match) => match[1]);
    assert.ok(
      ledgerIds.length > 0,
      `${project.repository_name} has no gap ledger`,
    );

    const classBlock = routePlan.match(/Gap classes:\s*([\s\S]*?)\n\n\| ID/);
    assert.ok(
      classBlock,
      `${project.repository_name} has no gap classifications`,
    );
    const classifiedIds = [...classBlock[1].matchAll(/[A-Z]{2,3}-\d+/g)].map(
      (match) => match[0],
    );
    assert.deepEqual(
      [...classifiedIds].sort(),
      [...ledgerIds].sort(),
      `${project.repository_name} must classify every gap exactly once`,
    );

    const categoryByGap = new Map();
    for (const match of classBlock[1].matchAll(/([a-z_]+)\s+—\s+([^;.]+)/g)) {
      for (const gapId of match[2].matchAll(/[A-Z]{2,3}-\d+/g)) {
        categoryByGap.set(gapId[0], match[1]);
      }
    }
    const activeGap = project.next_step.match(/[A-Z]{2,3}-\d+/)?.[0];
    assert.ok(
      activeGap,
      `${project.repository_name} next step has no active gap ID`,
    );
    assert.equal(
      project.next_step_category,
      categoryByGap.get(activeGap),
      `${project.repository_name} next-step category differs from ${activeGap}`,
    );
  }
});

test("public release targets cover exactly the eligible Showcase projects", async () => {
  const [contract, targets, targetsDoc] = await Promise.all([
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/public-release-targets.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/public-release-targets.md", root),
      "utf8",
    ),
  ]);

  assert.equal(targets.schema_version, "pronto-showcase-release-targets/v1");
  assert.equal(contract.schema_version, "pronto-showcase-goal/v2");
  assert.equal(
    contract.public_release_target_policy.matrix_path,
    "showcase-materials/public-release-targets.json",
  );
  assert.equal(
    contract.public_release_target_policy.required_before_public_showcase,
    true,
  );
  assert.deepEqual(contract.public_release_target_policy.required_channels, [
    "github",
    "portfolio",
    "handshake",
  ]);
  assert.match(
    contract.public_release_target_policy.rule,
    /Every project labeled public_showcase/i,
  );
  assert.equal(
    targets.eligibility_policy.source_contract,
    ".pronto/showcase-goal.json",
  );
  assert.equal(
    targets.eligibility_policy.public_eligibility_value,
    "public_showcase",
  );
  assert.deepEqual(targets.eligibility_policy.required_channels, [
    "github",
    "portfolio",
    "handshake",
  ]);
  assert.match(
    targets.eligibility_policy.rule,
    /may not be labeled public_showcase/i,
  );
  assert.match(
    targetsDoc,
    /does not authorize posting to an external service/i,
  );

  const publicNames = contract.projects
    .filter((project) => project.public_eligibility === "public_showcase")
    .map((project) => project.repository_name)
    .sort();
  const targetNames = targets.project_targets
    .map((project) => project.repository_name)
    .sort();
  assert.deepEqual(
    targetNames,
    publicNames,
    "release target matrix must cover exactly the public Showcase projects",
  );

  const channelIds = new Set(
    targets.channel_catalog.map((channel) => channel.id),
  );
  const allowedRequirements = new Set([
    "required",
    "recommended",
    "conditional",
    "optional",
  ]);
  const allowedStatuses = new Set([
    "planned",
    "in_progress",
    "gated",
    "deferred",
    "blocked",
  ]);

  for (const project of targets.project_targets) {
    assert.ok(
      project.active_gate?.trim(),
      `${project.repository_name} needs an active gate`,
    );
    assert.ok(
      project.primary_sequence.length >= 3,
      `${project.repository_name} needs a canonical-to-distribution sequence`,
    );
    const projectChannels = new Set();
    for (const target of project.targets) {
      assert.ok(
        channelIds.has(target.channel),
        `${project.repository_name} uses unknown channel ${target.channel}`,
      );
      assert.ok(
        !projectChannels.has(target.channel),
        `${project.repository_name} repeats ${target.channel}`,
      );
      projectChannels.add(target.channel);
      assert.ok(
        allowedRequirements.has(target.requirement),
        `${project.repository_name}.${target.channel} has invalid requirement`,
      );
      assert.ok(
        allowedStatuses.has(target.status),
        `${project.repository_name}.${target.channel} has invalid status`,
      );
      assert.ok(
        target.artifact?.trim(),
        `${project.repository_name}.${target.channel} needs an artifact`,
      );
    }
    for (const requiredChannel of targets.eligibility_policy
      .required_channels) {
      const requiredTarget = project.targets.find(
        (target) => target.channel === requiredChannel,
      );
      assert.ok(
        requiredTarget,
        `${project.repository_name} is missing required ${requiredChannel} target`,
      );
      assert.equal(
        requiredTarget.requirement,
        "required",
        `${project.repository_name}.${requiredChannel} must be a required destination`,
      );
    }
  }
});

test("release material inventory joins every public project and destination row", async () => {
  const [contract, targets, inventory] = await Promise.all([
    readFile(new URL(".pronto/showcase-goal.json", root), "utf8").then(
      JSON.parse,
    ),
    readFile(
      new URL("showcase-materials/public-release-targets.json", root),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/release-material-inventory.md", root),
      "utf8",
    ),
  ]);

  const publicProjects = contract.projects.filter(
    (project) => project.public_eligibility === "public_showcase",
  );
  const missingMaterialCount = publicProjects.reduce(
    (total, project) => total + project.missing_materials.length,
    0,
  );
  const targetRowCount = targets.project_targets.reduce(
    (total, project) => total + project.targets.length,
    0,
  );

  assert.equal(publicProjects.length, targets.project_targets.length);
  assert.match(
    inventory,
    new RegExp(`\\*\\*${publicProjects.length}\\*\\* active`),
  );
  assert.match(
    inventory,
    new RegExp(`\\*\\*${missingMaterialCount}\\*\\* current material/gap`),
  );
  assert.match(
    inventory,
    new RegExp(`\\*\\*${targetRowCount}\\*\\* destination rows`),
  );
  for (const project of publicProjects) {
    assert.ok(
      inventory
        .split("\n")
        .some(
          (line) =>
            line.startsWith("|") &&
            line.split("|")[1]?.trim() === project.display_name,
        ),
      `${project.display_name} is missing from the joined material inventory`,
    );
  }
});

test("Quality Runner's compact case packet preserves evidence boundaries", async () => {
  const packet = JSON.parse(
    await readFile(
      new URL("showcase-materials/quality-runner/case-study.json", root),
      "utf8",
    ),
  );
  assert.equal(packet.schema_version, "pronto-showcase-case/v1");
  assert.equal(packet.project, "quality-runner");
  assert.equal(packet.headline_case.repository, "tenure");
  assert.ok(packet.stages.length >= 6, "expected a complete decision trail");
  assert.equal(packet.burndown.baseline_raw, 4022);
  assert.equal(packet.burndown.terminal_raw, 537);
  assert.equal(packet.burndown.terminal_open_actionable, 0);
  assert.equal(packet.finding_drivers.tenure_attribution, true);
  assert.ok(
    packet.boundaries.includes("raw_findings_are_not_confirmed_defects"),
  );
  assert.ok(packet.boundaries.includes("zero_actionable_is_not_zero_raw"));
  assert.equal(packet.reproducibility_appendix.role, "appendix_only");
});

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

test("AI Workflow Leverage AL-1 binds a real Tenure task to one shared oracle", async () => {
  const [fixture, synthetic, receipt, blocker, protocol, route] =
    await Promise.all([
      readFile(
        new URL(
          "showcase-materials/ai-workflow-leverage/case-fixture.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/ai-workflow-leverage/synthetic-fixture.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/ai-workflow-leverage/evidence/protocol-receipt.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/ai-workflow-leverage/evidence/al-2-blocker.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/ai-workflow-leverage/protocol.md", root),
        "utf8",
      ),
      readFile(
        new URL("showcase-materials/ai-workflow-leverage/route-plan.md", root),
        "utf8",
      ),
    ]);

  assert.equal(
    fixture.schema_version,
    "pronto-showcase-ai-workflow-leverage-fixture/v1",
  );
  assert.equal(fixture.source_kind, "real_attributable_maintenance_task");
  assert.equal(
    fixture.protected_baseline.revision,
    "a304ce866e1bced294a12f9915cae55ac2b65b13",
  );
  assert.equal(
    fixture.reference_implementation.revision,
    "5dd52328a1a847466db8a7f12c7e6f71b468182e",
  );
  assert.equal(fixture.reference_implementation.not_measurement_evidence, true);
  assert.equal(fixture.paired_protocol.identical_inputs, true);
  assert.equal(fixture.paired_protocol.identical_completion_criteria, true);
  assert.equal(fixture.paired_protocol.identical_oracle, true);
  assert.deepEqual(
    fixture.paired_protocol.lanes.map((lane) => lane.id),
    ["manual", "assisted"],
  );
  assert.equal(fixture.quality_oracle.behavioral_contract.length, 4);
  assert.equal(synthetic.mutations.length, 2);
  assert.ok(
    synthetic.mutations.every(
      (mutation) => mutation.expected === "fail_closed",
    ),
  );
  assert.equal(receipt.gap, "AL-1");
  assert.equal(receipt.status, "passed");
  assert.ok(
    receipt.protocol_checks.every((check) => check.status === "passed"),
  );
  assert.equal(receipt.paired_runs.manual, "not_run");
  assert.equal(receipt.paired_runs.assisted, "not_run");
  assert.equal(receipt.paired_runs.result_claim, "not_claimed");
  assert.equal(blocker.gap, "AL-2");
  assert.equal(blocker.status, "blocked");
  assert.equal(blocker.disposition, "parked_pending_owner_contract");
  assert.equal(
    blocker.owner_boundary.paired_measurement_owner,
    "agent-eval-runtime",
  );
  assert.deepEqual(blocker.missing_contract.required_fields, [
    "active_work_seconds",
    "wait_seconds",
    "human_touches",
    "retries",
    "failure_events",
    "outcome_evidence",
  ]);
  assert.match(
    blocker.blocked_action,
    /Do not add a second paired-measurement engine/,
  );
  assert.match(protocol, /reference contract only/i);
  assert.match(protocol, /same fixed\s+inputs/i);
  assert.match(protocol, /does not prove that AI is faster/i);
  assert.match(route, /AL-1 closure/);
  assert.match(route, /AL-2 must instrument/);
  assert.match(route, /AL-2 blocker/);
  assert.match(route, /agent-eval-runtime/);
});

test("Marketing Autoresearch MA-1 binds a public decision brief to privacy and refusal boundaries", async () => {
  const [brief, privacy, synthetic, blocker, route, contract, readiness] =
    await Promise.all([
      readFile(
        new URL("showcase-materials/marketing-autoresearch/brief.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/marketing-autoresearch/privacy-review.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/marketing-autoresearch/synthetic-fixture.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/marketing-autoresearch/evidence/ma-2-blocker.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/marketing-autoresearch/route-plan.md",
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
    brief.schema_version,
    "pronto-showcase-marketing-autoresearch-brief/v1",
  );
  assert.equal(brief.source_kind, "real_public_portfolio_maintenance_question");
  assert.equal(brief.status, "ready_for_shadow_run");
  assert.equal(brief.primary_profile, "portfolio");
  assert.equal(brief.decision.candidate_lanes.length, 3);
  assert.ok(brief.research_scope.allowed_source_registry.length >= 3);
  assert.ok(
    brief.research_scope.exclusions.some((item) =>
      /BidCamp|client/i.test(item),
    ),
  );
  assert.equal(
    brief.report_contract.publication_state,
    "blocked_pending_human_review",
  );
  assert.equal(brief.claim_boundary.length, 3);

  assert.equal(privacy.status, "passed_with_conditions");
  assert.ok(
    privacy.checks
      .filter((check) => check.id !== "future-run-redaction")
      .every((check) => check.status === "passed"),
  );
  assert.equal(privacy.publication_decision, "not_a_publication_candidate");
  assert.equal(synthetic.source_kind, "synthetic_reproducibility_appendix");
  assert.equal(synthetic.expected.external_mutations, 0);
  assert.ok(
    synthetic.negative_cases.every(
      (caseFile) => caseFile.expected === "fail_closed",
    ),
  );
  assert.equal(blocker.gap, "MA-2");
  assert.equal(blocker.status, "blocked");
  assert.equal(
    blocker.disposition,
    "parked_pending_brief_source_claim_contract",
  );
  assert.equal(blocker.observed_surface.probe_external_mutations, 0);
  assert.ok(
    blocker.owner_boundary.missing_contract.includes(
      "brief input bound to a run",
    ),
  );
  assert.match(blocker.blocked_action, /Do not claim MA-2 complete/);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "marketing-autoresearch",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "marketing-autoresearch",
  );
  assert.equal(project?.next_step_category, "demo_integration");
  assert.match(project?.next_step ?? "", /MA-2/);
  assert.equal(readinessProject?.first_required_closure, "MA-2");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
  assert.match(route, /MA-1 closure/);
  assert.match(route, /MA-2 blocker/);
  assert.match(route, /source-to-claim receipt/i);
  assert.match(route, /privacy review/i);
});

test("RemodelVision parks attribution safely while closing the synthetic fixture gap", async () => {
  const [ledger, fixture, blocker, runtimeBlocker, route, contract, readiness] =
    await Promise.all([
      readFile(
        new URL(
          "showcase-materials/remodelvision/contribution-ledger.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/remodelvision/asset-manifest.json", root),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/remodelvision/evidence/rv-1-blocker.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL(
          "showcase-materials/remodelvision/evidence/rv-3-blocker.json",
          root,
        ),
        "utf8",
      ).then(JSON.parse),
      readFile(
        new URL("showcase-materials/remodelvision/route-plan.md", root),
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
    ledger.schema_version,
    "pronto-showcase-remodelvision-contribution-ledger/v1",
  );
  assert.equal(ledger.status, "review_required");
  assert.equal(
    ledger.observed_attribution.project_statement.status,
    "observed",
  );
  assert.equal(
    ledger.boundary_status.personal_contribution_role,
    "unknown_pending_owner_review",
  );
  assert.equal(
    ledger.boundary_status.collaborator_approval_for_public_showcase,
    "unknown_pending_collaborator_review",
  );
  assert.match(ledger.claim_boundary.join(" | "), /not proof/i);

  assert.equal(
    fixture.schema_version,
    "pronto-showcase-remodelvision-asset-manifest/v1",
  );
  assert.equal(fixture.status, "passed");
  assert.equal(fixture.fixture_policy.source_kind, "synthetic_original");
  assert.equal(fixture.fixture_policy.private_location_data, false);
  assert.equal(fixture.fixture_policy.third_party_assets, false);
  assert.equal(fixture.assets[0].metadata_review.review_status, "passed");
  assert.match(fixture.display_rules.join(" | "), /synthetic label/i);

  assert.equal(blocker.gap, "RV-1");
  assert.equal(blocker.status, "blocked");
  assert.equal(blocker.disposition, "parked_pending_collaborator_approval");
  assert.ok(
    blocker.missing_contract.required.some((item) =>
      /collaborator approval/i.test(item),
    ),
  );
  assert.match(blocker.blocked_action, /Do not publish/i);

  assert.equal(runtimeBlocker.gap, "RV-3");
  assert.equal(runtimeBlocker.status, "blocked");
  assert.equal(
    runtimeBlocker.disposition,
    "parked_pending_runtime_prerequisites_and_direct_surface",
  );
  assert.equal(runtimeBlocker.observed_surface.checks[0].status, "passed");
  assert.equal(runtimeBlocker.observed_surface.checks[3].status, "failed");
  assert.equal(runtimeBlocker.observed_surface.checks[4].status, "blocked");
  assert.match(runtimeBlocker.blocked_action, /Do not claim/i);

  const project = contract.projects.find(
    (candidate) => candidate.repository_name === "remodelvision",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "remodelvision",
  );
  assert.equal(project?.next_step_category, "evidence");
  assert.match(project?.next_step ?? "", /RV-1/);
  assert.match(project?.blockers?.join(" | ") ?? "", /collaborator-approval/i);
  assert.equal(readinessProject?.first_required_closure, "RV-1");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 5);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "blocked_attribution_and_runtime_owner_boundary",
  );
  assert.match(route, /RV-1 blocker/);
  assert.match(route, /RV-2 closure/);
  assert.match(route, /RV-3 blocker/);
  assert.match(route, /rights-safe-fixture\.svg/);
});

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

test("Book parks real chapter rights while preserving a labeled synthetic appendix", async () => {
  const [
    ledger,
    fixture,
    blocker,
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
  assert.equal(ledger.status, "review_required");
  assert.equal(ledger.publication_ready, false);
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
  assert.match(ledger.claim_boundary.join(" | "), /No public chapter/);

  assert.equal(
    fixture.schema_version,
    "pronto-showcase-book-synthetic-fixture/v1",
  );
  assert.equal(fixture.source_kind, "synthetic_original");
  assert.equal(fixture.role, "rights_safe_reproducibility_appendix_only");
  assert.equal(fixture.fixture_policy.third_party_audio, false);
  assert.equal(fixture.chapter.beats.length, 4);
  assert.equal(fixture.media_plan.controls.includes("reduced_motion"), true);
  assert.match(fixture.display_rules.join(" | "), /synthetic-fixture label/i);

  assert.equal(blocker.gap, "BK-1");
  assert.equal(blocker.status, "blocked");
  assert.equal(
    blocker.disposition,
    "parked_pending_chapter_and_asset_rights_record",
  );
  assert.ok(
    blocker.missing_contract.required.some((item) =>
      /music permission/i.test(item),
    ),
  );
  assert.match(blocker.blocked_action, /Do not publish/i);
  assert.equal(blocker.synthetic_fallback.does_not_clear_bk_1, true);

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
    "passed_as_rights_safe_synthetic_appendix",
  );
  assert.equal(materialReceipt.artifact.label_required, true);
  assert.equal(
    materialReceipt.surface_probe.status,
    "passed_as_static_http_fetch",
  );
  assert.match(
    materialReceipt.claim_boundary.join(" | "),
    /does not clear BK-1/,
  );

  const project = goal.projects.find(
    (candidate) => candidate.repository_name === "Book",
  );
  const readinessProject = readiness.projects.find(
    (candidate) => candidate.repository_name === "Book",
  );
  assert.equal(project?.next_step_category, "evidence");
  assert.match(project?.next_step ?? "", /BK-1/);
  assert.match(project?.blockers?.join(" | ") ?? "", /chapter authorship/i);
  assert.equal(readinessProject?.first_required_closure, "BK-1");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 6);
  assert.equal(
    readinessProject?.rehearsal_disposition,
    "rights_evidence_required",
  );
  assert.match(route, /BK-1 evidence boundary/);
  assert.match(route, /synthetic-fixture\.json/);
  assert.match(route, /synthetic-preview\.html/);
});

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

test("the durable route uses gap closure instead of generic product build-out", async () => {
  const files = [
    "docs/showcase-contract.md",
    "showcase-materials/README.md",
    "showcase-materials/mac-control/README.md",
  ];
  const contents = await Promise.all(
    files.map((path) => readFile(new URL(path, root), "utf8")),
  );
  assert.ok(contents.every((content) => !content.includes("product_build")));
  assert.ok(contents.every((content) => content.includes("gap_closure")));
});

test("RDW's real case preserves the semantic and deterministic gate boundary", async () => {
  const [casePacket, ledger, validation] = await Promise.all(
    [
      "showcase-materials/research-domain-writing/case-study.json",
      "showcase-materials/research-domain-writing/claim-ledger.json",
      "showcase-materials/research-domain-writing/rdw-run/validation-record.json",
    ].map(async (path) =>
      JSON.parse(await readFile(new URL(path, root), "utf8")),
    ),
  );

  assert.equal(casePacket.case_id, "tatum-2023-24-claim-boundary");
  assert.equal(casePacket.headline_case.source_count, 2);
  assert.deepEqual(casePacket.unsupported_terms, [
    "prove",
    "elite",
    "two-way engine",
  ]);
  assert.match(
    casePacket.deterministic_gate_boundary,
    /agent performs semantic/i,
  );
  assert.match(
    casePacket.deterministic_gate_boundary,
    /RDW validates exact draft binding/i,
  );
  assert.equal(
    validation.checks.basketball_pack_maturity,
    "pass_specialized_production",
  );

  const unsafeStop = ledger.claims.find((claim) => claim.id === "unsafe_stop");
  assert.match(unsafeStop.forbidden_inference, /independently inferred/i);
});

test("RDW's publication candidate keeps release and deployment claims gated", async () => {
  const [manifest, provenance, page, description] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/final-package.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/rdw-0-provenance.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/public/index.html",
        root,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/public-description.txt",
        root,
      ),
      "utf8",
    ),
  ]);

  assert.equal(manifest.status, "publication_candidate_provenance_gated");
  assert.equal(manifest.checks.release_provenance_claim_allowed, false);
  assert.equal(manifest.checks.deployment_claim_allowed, false);
  assert.equal(description.trim().length <= 500, true);
  assert.equal(
    provenance.installed_basketball_pack.validation_result,
    "OK basketball: specialized/production",
  );
  assert.equal(
    provenance.matching_repository_content.domain_config_sha1,
    provenance.installed_basketball_pack.domain_config_sha1,
  );
  assert.equal(
    provenance.disposition.release_artifact_to_source_revision,
    "not_verified",
  );
  assert.match(page, /Plausible is not proven/);
  assert.match(page, /Human decision required/);
  assert.match(page, /release provenance and deployment not\s+yet verified/i);
});

test("the AI Code Quality Stack concept keeps layer ownership and proof state explicit", async () => {
  const [contract, integratedCase, page] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/ai-code-quality-stack/stack-contract.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/ai-code-quality-stack/integrated-case.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/ai-code-quality-stack/concept/index.html",
        root,
      ),
      "utf8",
    ),
  ]);

  assert.deepEqual(
    contract.layers.map((layer) => layer.id),
    ["detect", "enforce", "contextualize"],
  );
  assert.equal(
    contract.fallback_contract.proof_state,
    "not_yet_proven_cross_repo",
  );
  assert.equal(integratedCase.reviewed_problem.review_disposition, "accepted");
  assert.equal(
    integratedCase.target_trace[0].state,
    "source_evidence_available",
  );
  assert.ok(
    integratedCase.target_trace
      .slice(1)
      .every((stage) => stage.state === "integration_not_yet_executed"),
  );
  assert.match(page, /One finding\. Three responsibilities\./);
  assert.match(page, /Concept surface/);
  assert.match(page, /integrated execution\s+not\s+yet proven/i);
});

test("the Pre-CR concept stands on its IDE workflow without stack dependencies", async () => {
  const page = await readFile(
    new URL("showcase-materials/pre-cr-suite/concept/index.html", root),
    "utf8",
  );

  assert.match(page, /Where Was I\?/);
  assert.match(page, /Continuity/);
  assert.match(page, /Visibility/);
  assert.match(page, /Readiness/);
  assert.doesNotMatch(page, /Anti-Slop/);
  assert.doesNotMatch(page, /Quality Runner/);
});

test("Portable Agentic Workbench PW-1 proves a real safe-tool-guards install contract", async () => {
  const [
    fixture,
    expected,
    receipt,
    claims,
    runtimeComparison,
    comparison,
    preview,
    route,
  ] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/case-fixture.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/expected-results.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/evidence/validation-receipt.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/evidence/claim-ledger.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/evidence/runtime-comparison.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/comparison.html",
        root,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/preview.svg",
        root,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "showcase-materials/portable-agentic-workbench/route-plan.md",
        root,
      ),
      "utf8",
    ),
  ]);

  assert.equal(fixture.case_id, "PW-1-safe-tool-guards");
  assert.equal(
    fixture.source_commit,
    "4d813b4235859588aa1ff3549f15cf53768258af",
  );
  assert.equal(fixture.asset.id, "safe-tool-guards");
  assert.deepEqual(
    fixture.environments.map((environment) => environment.target),
    ["generic", "codex"],
  );
  assert.deepEqual(
    fixture.contract.required_behaviors.map((behavior) => behavior.id),
    [
      "dry-run-no-mutation",
      "scope-rejection",
      "no-overwrite",
      "manual-host-registration",
      "result-taxonomy",
      "receipt-scoped-recovery",
    ],
  );
  assert.equal(fixture.observed.dry_run.generic.target_file_count_after, 0);
  assert.equal(fixture.observed.dry_run.codex.target_file_count_after, 0);
  assert.equal(
    fixture.observed.overwrite_guard.generic.status,
    "blocked-existing-files",
  );
  assert.equal(
    fixture.observed.overwrite_guard.codex.status,
    "blocked-existing-files",
  );
  assert.equal(fixture.observed.recovery.generic.apply.status, "uninstalled");
  assert.equal(
    fixture.observed.recovery.generic.modified_guard.status,
    "blocked-uninstall-safety",
  );
  assert.equal(fixture.observed.recovery.codex.apply.status, "uninstalled");

  assert.equal(expected.cases.length, 13);
  assert.equal(receipt.source.selection_validation_result.ok, true);
  assert.equal(receipt.parity.equivalent_install_contract, true);
  assert.equal(receipt.parity.same_file_hashes, true);
  assert.equal(receipt.recovery.status, "supported");
  assert.equal(receipt.runtime_comparison.status, "blocked");
  assert.equal(runtimeComparison.overall.status, "blocked");
  assert.equal(runtimeComparison.runtime_probes[0].availability, "unavailable");
  assert.equal(runtimeComparison.runtime_probes[1].execution.status, "blocked");
  assert.equal(runtimeComparison.scenarios.length, 3);
  assert.ok(
    runtimeComparison.scenarios.every(
      (scenario) => scenario.comparison_status === "blocked",
    ),
  );
  assert.equal(receipt.runs.generic.recovery.apply.status, "uninstalled");
  assert.equal(
    receipt.runs.generic.recovery.modified_guard.status,
    "blocked-uninstall-safety",
  );
  assert.equal(receipt.runs.codex.recovery.apply.status, "uninstalled");
  assert.equal(receipt.equivalent_runs, true);
  assert.equal(
    claims.claims.find((claim) => claim.id === "runtime-equivalence").status,
    "blocked",
  );
  assert.equal(
    claims.claims.find((claim) => claim.id === "recovery").status,
    "supported",
  );
  assert.match(comparison, /Carry the safety contract/);
  assert.match(comparison, /manual review/);
  assert.match(comparison, /Recover what you own/);
  assert.match(preview, /safe-tool-guards/);
  assert.match(preview, /registration: manual/);
  assert.match(preview, /PW-4 blocked by host-runtime boundary/);
  assert.match(route, /PW-1 through PW-3, PW-5, and PW-6 are closed/);
  assert.match(route, /Receipt-scoped dry-run\/apply uninstall/);
  assert.match(route, /PW-4 was attempted and is explicitly blocked/);
});

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
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 3);
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
  assert.equal(project?.demo_materials?.score, 2.8);
  assert.match(project?.next_step ?? "", /AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 3);
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
  assert.equal(project?.demo_materials?.score, 2.8);
  assert.match(project?.next_step ?? "", /AR-5/);
  assert.equal(readinessProject?.first_required_closure, "AR-5");
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 3);
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
    "blocked_native_synthesis_output_contract",
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
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 6);
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
    "blocked_installed_version_and_protocol_mismatch",
  );
  assert.equal(readinessProject?.remaining_gap_count_before_rehearsal, 6);
});

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
