import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { URL } from "node:url";

const root = new URL("../", import.meta.url);

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
