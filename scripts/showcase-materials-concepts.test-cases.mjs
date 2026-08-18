import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

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

test("RDW's publication candidate keeps hosting and deployment claims gated", async () => {
  const [manifest, approval, provenance, page, description, checkpoint, preview] = await Promise.all([
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/final-package.json",
        root,
      ),
      "utf8",
      ).then(JSON.parse),
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/evidence/editorial-approval.json",
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
    readFile(
      new URL(
        "showcase-materials/research-domain-writing/evidence/rdw-6-material-checkpoint.json",
        root,
      ),
      "utf8",
    ).then(JSON.parse),
    readFile(
      new URL("showcase-materials/research-domain-writing/preview.html", root),
      "utf8",
    ),
  ]);

  assert.equal(manifest.status, "publication_candidate_hosting_gated");
  assert.equal(manifest.checks.release_provenance_claim_allowed, true);
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
    "verified_attested",
  );
  assert.equal(provenance.disposition.rdw_0_status, "closed_by_owner_attestation");
  assert.equal(provenance.asset_manifest.all_installed_assets_match, true);
  assert.equal(approval.approver, "Jakye Amos");
  assert.equal(approval.review.human_publication_boundary_preserved, true);
  assert.match(page, /Plausible is not proven/);
  assert.match(page, /Human decision required/);
  assert.match(page, /hosting and deployment not\s+yet\s+verified/i);
  assert.equal(checkpoint.status, "candidate_local");
  assert.equal(checkpoint.checks.preview_visual_review, "pass");
  assert.equal(
    checkpoint.artifacts.find((artifact) => artifact.path.endsWith("preview-16x9.png"))?.dimensions,
    "1600x900",
  );
  assert.match(preview, /data-material-status="candidate-local"/);
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
