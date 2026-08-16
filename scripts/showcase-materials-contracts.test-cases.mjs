import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

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
