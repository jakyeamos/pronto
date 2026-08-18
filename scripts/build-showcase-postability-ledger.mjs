import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const root = resolve(new URL("../", import.meta.url).pathname);
const readJson = async (relativePath) =>
  JSON.parse(await readFile(resolve(root, relativePath), "utf8"));
const exists = async (relativePath) => {
  try {
    await access(resolve(root, relativePath));
    return true;
  } catch {
    return false;
  }
};

const [goal, targets, readiness] = await Promise.all([
  readJson(".pronto/showcase-goal.json"),
  readJson("showcase-materials/public-release-targets.json"),
  readJson("showcase-materials/rehearsal-readiness.json"),
]);

const publicProjects = goal.projects.filter(
  (project) => project.public_eligibility === "public_showcase",
);
const targetByRepo = new Map(
  targets.project_targets.map((project) => [project.repository_name, project]),
);
const readinessByRepo = new Map(
  readiness.projects.map((project) => [project.repository_name, project]),
);

const packageOverrides = {
  "mac-control": "mac-control",
  "quality-runner": "quality-runner",
  "chirons-forge": "chirons-forge",
  "pre-cr-suite-lsp": "pre-cr-suite",
  Terrace: "terrace",
  "context-compiler-contract": "context-compiler-contract",
  "participant-dedup": "participant-deduplication",
  "jakyeamos-agent-skills": "portable-agentic-workbench",
  "browser-control": "codex-browser-control",
};

const statusOverrides = {
  "mac-control": {
    state: "local_package_incomplete",
    deferral: {
      type: "installed_live_evidence",
      reason:
        "The packet has a case page and short copy, but the real 16:9 capture and redacted authorization/outcome receipt require an installed external macOS session and the bounded MC-2 route.",
      reentry:
        "Return to MC-2 after the installed document-open/layout proof and structured receipt are available.",
    },
  },
  "chirons-forge": {
    state: "blocked_owner_provenance",
    deferral: {
      type: "owner_provenance",
      reason:
        "No owner-visible backing repository or exact deployed revision is available, so a real case, judge trace, output, or deletion proof would be fabricated.",
      reentry:
        "Resume at CF-0 when the owner supplies the backing repository and immutable deployed revision.",
    },
  },
  "participant-dedup": {
    state: "local_packet_ready_with_live_gate",
    deferral: {
      type: "owner_authenticated_uat",
      reason:
        "The local synthetic packet is complete, but live Google Sheets UAT requires an owner-authenticated path and must not touch participant data from this thread.",
      reentry:
        "Resume at PD-6 when an owner-authenticated, privacy-safe UAT path is explicitly available.",
    },
  },
};

const classifyGate = (project) => {
  const text = [project.next_step, ...project.missing_materials].join(" ");
  if (/rights|attribution|editorial|contribution approval/i.test(text)) {
    return "rights_or_editorial_authority";
  }
  if (
    /authenticated|Google Sheets|live-sheet|owner-authenticated/i.test(text)
  ) {
    return "owner_authenticated_provider";
  }
  if (
    /hosted|external destination|readback|Handshake upload|public access|deploy/i.test(
      text,
    )
  ) {
    return "hosting_or_external_readback";
  }
  if (
    /owner contract|owner|collaborator|approval|confidence|fallback|version|deployment/i.test(
      text,
    )
  ) {
    return "owner_or_provider_contract";
  }
  if (
    /runtime|installed|VS Code|native|producer|adapter|scenario|receipt|proof|matrix|capture/i.test(
      text,
    )
  ) {
    return "product_or_evidence_gate";
  }
  return "review_gate";
};

const artifactCandidates = (packagePath) => ({
  story_route: `${packagePath}/route-plan.md`,
  evidence: [
    `${packagePath}/claim-ledger.json`,
    `${packagePath}/evidence`,
    `${packagePath}/case-study.json`,
    `${packagePath}/case-study.md`,
  ],
  public_case: [
    `${packagePath}/public/index.html`,
    `${packagePath}/case-study.html`,
    `${packagePath}/comparison.html`,
    `${packagePath}/workflow-preview.html`,
    `${packagePath}/preview.html`,
  ],
  preview: [
    `${packagePath}/assets/preview-16x9.png`,
    `${packagePath}/assets/preview.svg`,
    `${packagePath}/assets/fleet-radar-preview-1600x900.png`,
    `${packagePath}/preview-16x9.png`,
    `${packagePath}/preview.svg`,
  ],
  short_copy: `${packagePath}/public-description.txt`,
  role_review: [
    `${packagePath}/claim-ledger.json`,
    `${packagePath}/evidence/claim-ledger.json`,
    `${packagePath}/route-plan.md`,
  ],
});

const localFixSlots = [
  "story_route",
  "evidence",
  "public_case",
  "preview",
  "short_copy",
  "role_review",
];

const firstExisting = async (candidates) => {
  for (const candidate of Array.isArray(candidates)
    ? candidates
    : [candidates]) {
    if (await exists(candidate)) return candidate;
  }
  return null;
};

const packageFor = (project) => {
  const readinessProject = readinessByRepo.get(project.repository_name);
  const routeDir = readinessProject?.route?.split("/")[0];
  return (
    packageOverrides[project.repository_name] ??
    routeDir ??
    project.repository_name
  );
};

const rows = [];
for (const project of publicProjects) {
  const target = targetByRepo.get(project.repository_name);
  const readinessProject = readinessByRepo.get(project.repository_name);
  const packagePath = `showcase-materials/${packageFor(project)}`;
  const candidates = artifactCandidates(packagePath);
  const artifacts = {
    story_route: await exists(candidates.story_route),
    evidence: Boolean(await firstExisting(candidates.evidence)),
    public_case: Boolean(await firstExisting(candidates.public_case)),
    preview: await firstExisting(candidates.preview),
    short_copy: Boolean(await exists(candidates.short_copy)),
    role_review: Boolean(await firstExisting(candidates.role_review)),
  };
  const localPackageComplete =
    artifacts.story_route &&
    artifacts.evidence &&
    artifacts.public_case &&
    artifacts.preview &&
    artifacts.short_copy &&
    artifacts.role_review;
  const localFix = {
    state: localPackageComplete ? "complete" : "partial",
    required: localFixSlots,
    applied: localFixSlots.filter((slot) => Boolean(artifacts[slot])),
    remaining: localFixSlots.filter((slot) => !artifacts[slot]),
    boundary:
      "This is a local material fix receipt. It does not close live product, installed-surface, authority, hosting, or destination-readback gates.",
  };
  const override = statusOverrides[project.repository_name];
  let state = override?.state;
  if (!state) {
    if (target.release_state === "deferred") state = "deferred_by_decision";
    else if (target.release_state === "blocked") state = "blocked_by_gate";
    else if (!localPackageComplete) state = "local_package_incomplete";
    else if (project.missing_materials.length > 0)
      state = "local_packet_ready_with_open_gate";
    else state = "local_packet_ready";
  }
  const deferral =
    override?.deferral ??
    (state === "deferred_by_decision"
      ? {
          type: "owner_or_provider_contract",
          reason: project.next_step,
          reentry: project.next_step,
        }
      : state === "blocked_by_gate"
        ? {
            type: classifyGate(project),
            reason: project.blockers.join(" ") || project.next_step,
            reentry: project.next_step,
          }
        : project.missing_materials.length > 0
          ? {
              type: classifyGate(project),
              reason: project.next_step,
              reentry: project.next_step,
            }
          : null);
  rows.push({
    repository_name: project.repository_name,
    display_name: project.display_name,
    package_path: packagePath,
    release_state: target.release_state,
    postability_state: state,
    local_package_complete: localPackageComplete,
    local_fix: localFix,
    artifacts,
    required_channels: targets.eligibility_policy.required_channels,
    open_materials: project.missing_materials,
    active_gate: target.active_gate,
    next_step: project.next_step,
    readiness: readinessProject
      ? {
          current_stage: readinessProject.current_stage,
          rehearsal_status: readinessProject.rehearsal_status,
          first_required_closure: readinessProject.first_required_closure,
        }
      : null,
    deferral,
    external_posting_proof: false,
  });
}

const summary = {
  public_project_count: rows.length,
  local_packet_ready_count: rows.filter((row) => row.local_package_complete)
    .length,
  local_package_incomplete_count: rows.filter(
    (row) => !row.local_package_complete,
  ).length,
  local_fix_complete_count: rows.filter(
    (row) => row.local_fix.state === "complete",
  ).length,
  local_fix_partial_count: rows.filter(
    (row) => row.local_fix.state === "partial",
  ).length,
  deferred_or_blocked_count: rows.filter((row) => row.deferral).length,
  externally_postable_count: rows.filter(
    (row) =>
      row.local_package_complete &&
      row.open_materials.length === 0 &&
      row.external_posting_proof,
  ).length,
  publication_receipts_recorded: rows.filter(
    (row) => row.external_posting_proof,
  ).length,
};

const ledger = {
  schema_version: "pronto-showcase-postability/v1",
  reviewed_at: new Date().toISOString(),
  purpose:
    "Track whether each public Showcase project has a locally postable packet without confusing local materials, live proof, authority, hosting, or external publication.",
  publication_policy:
    "This ledger never authorizes external posting. A project is not posted until a fresh destination receipt and readback are recorded.",
  state_definitions: {
    local_packet_ready:
      "Story, evidence, no-auth source, preview, short copy, and role/claim boundary are present locally; any optional or external follow-up stays explicit.",
    local_packet_ready_with_open_gate:
      "The local packet is present, but one or more required product, evidence, authority, hosting, or destination gates remain open.",
    local_package_incomplete:
      "A safe local packet artifact is still missing; do not substitute a generated or aspirational proof.",
    blocked_by_gate:
      "A named provenance, rights, owner, or product boundary prevents honest packet completion.",
    deferred_by_decision:
      "The owner or a required provider contract has intentionally deferred the next closure.",
  },
  summary,
  projects: rows,
};

const outputPath = resolve(root, "showcase-materials/postability-ledger.json");
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(ledger, null, 2)}\n`);
console.log(`wrote ${outputPath}`);
