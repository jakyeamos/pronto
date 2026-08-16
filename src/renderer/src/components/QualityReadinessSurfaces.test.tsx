// @vitest-environment happy-dom
import { cleanup } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type {
  ReleaseRuleConfig,
  WebReadinessSnapshot,
} from "./QualityComponents.test-support";
import {
  makeGate,
  makeEvidence,
  makeQuality,
  makeRepository,
  makePortfolio,
  noop,
  noopRepository,
  noopReport,
  analyticsSnapshot,
  preparation,
  aiPreview,
  PreparationDrawer,
  qualityGateChoices,
  CommandCenterSurface,
  QualityAttentionList,
  QualityEvidenceList,
  qualityAttentionItems,
  QualityGatesSurface,
  navItems,
  WebReadinessSummary,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("quality readiness and release surfaces", () => {
  it("surfaces imported maturity and CI configuration against the ideal profile", () => {
    const repository = makeRepository();
    const markup = renderToStaticMarkup(
      <CommandCenterSurface
        activeConditionCount={0}
        dirtyCount={0}
        unsyncedCount={0}
        repositoryCount={1}
        rootCount={1}
        snapshotGeneratedAt="2026-08-11T17:00:00Z"
        isRefreshing={false}
        quality={
          makePortfolio([repository], {
            maturity_score_display: "1.933",
            ci_readiness_score: 2.67,
            ci_readiness_score_display: "2.67",
            ci_readiness_full_repository_count: 0,
            ci_readiness_repository_count: 1,
            ci_configuration_configured_gate_count: 1,
            ci_configuration_ideal_gate_count: 6,
            ci_configuration_full_repository_count: 0,
            ci_configuration_repository_count: 1,
            ci_evidence_fresh_passing_gate_count: 0,
            ci_evidence_ideal_gate_count: 6,
            ci_readiness_open_gate_counts: { tests: 1 },
            mac_control_ideal_state: {
              status: "Blocked",
              freshness: "Unknown",
              ideal_state: false,
              applicable_repository_count: 1,
              not_applicable_repository_count: 0,
              evaluated_repository_count: 1,
              implementation_status: "Blocked",
              implementation_score: 0,
              implementation_score_display: "0.0",
              implementation_criteria_passed_count: 0,
              implementation_criteria_total: 8,
              implementation_declaration_criteria_count: 8,
              live_status: "Blocked",
              live_score: 0,
              live_score_display: "0.0",
              live_task_count: 5,
              measured_task_count: 0,
              failure_reasons: [],
            },
            quality_outcome_counts: { verification_blocked: 1 },
            quality_outcome_taxonomy: {
              verification_blocked: {
                label: "Quality verification blocked",
                meaning:
                  "Setup, execution, timeout, or target provenance prevented a trustworthy verdict.",
              },
            },
          }).quality
        }
        analytics={analyticsSnapshot}
        repositories={[repository]}
        allRepositories={[repository]}
        events={[]}
        filter="all"
        onFilterChange={() => undefined}
        onClearFilters={() => undefined}
        onAddRoot={() => undefined}
        onOpenRepository={noopRepository}
        onCondition={() => undefined}
        onRefreshQuality={() => undefined}
      />,
    );

    expect(markup).toContain("CI configuration vs ideal");
    expect(markup).toContain("Consolidated fleet maturity");
    expect(markup).toContain("1.933");
    expect(markup).toContain("1/6");
    expect(markup).toContain("Fresh passing evidence");
    expect(markup).toContain("0/6");
    expect(markup).toContain("Tests (1)");
    expect(markup).toContain("Mac Control maturity");
    expect(markup).toContain("0.0");
    expect(markup).not.toContain("Mac Control semantic evidence");
    expect(markup).toContain(
      "Legacy declarations: 8 recorded · non-scoring until v4 source evidence is established",
    );
    expect(markup).toContain(
      "Fleet freshness incomplete—inspect repository blockers",
    );
    expect(markup).toContain("Live tasks: 0/5 measured · Blocked");
    expect(markup).toContain("Repository quality outcomes");
    expect(markup).toContain("Quality verification blocked");
    expect(markup).toContain("Quality audit");
    expect(markup).toContain("No audit imported");
    expect(markup).toContain("Pronto snapshot");
  });

  it("shows generic fleet and repository audit requirements when an evidence contract changes", () => {
    const repositoryContract = {
      contract_id: "example-task-manifest",
      label: "Example task evidence",
      target_schema: "example-task-manifest/v3",
      observed_schema: "example-task-manifest/v2",
      status: "audit_required",
      repository_id: "repo-1",
      repository_name: "pronto",
      message:
        "Example task evidence was assessed against v2; re-audit against v3.",
    };
    const repository = makeRepository({
      quality: makeQuality({ evidence_contracts: [repositoryContract] }),
    });
    const snapshot = makePortfolio([repository], {
      evidence_contracts: [
        {
          contract_id: "example-task-manifest",
          label: "Example task evidence",
          target_schema: "example-task-manifest/v3",
          status: "audit_required",
          repository_count: 4,
          current_repository_count: 1,
          legacy_repository_count: 2,
          missing_repository_count: 1,
          message:
            "Full fleet audit required: 1/4 repositories are assessed against v3.",
          next_safe_step:
            "Run the owning producer's fleet audit, then refresh Pronto.",
        },
      ],
    });

    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={snapshot}
        repositories={[repository]}
        onOpenRepository={noopRepository}
      />,
    );
    expect(markup).toContain("Full fleet audit required");
    expect(markup).toContain("Example task evidence");
    expect(markup).toContain("1/4");
    expect(markup).toContain("2 legacy");
    expect(qualityAttentionItems(repository)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "contract",
          label: "Example task evidence · re-audit required",
        }),
      ]),
    );
  });

  it("filters attention to failed, stale, high-severity, and required missing evidence", () => {
    const repository = makeRepository({
      release_rule: {
        name: "Quality rule",
        operator: "AND",
        required_commit_types: [],
        allow_first_release: false,
        required_quality_gates: [{ gate_id: "dead_code", source: "QR" }],
      },
      quality: makeQuality({
        gates: [
          makeGate("build", "Build", "Passed", "Stale", [
            makeEvidence({ id: "build", freshness: "Stale" }),
          ]),
          makeGate("runtime_smoke", "Smoke", "Not configured", "Unknown"),
          makeGate("lint", "Lint", "Failed", "Fresh", [
            makeEvidence({ id: "lint", status: "Failed" }),
          ]),
          makeGate("dead_code", "Dead-code"),
        ],
      }),
    });
    const items = qualityAttentionItems(repository);
    expect(items.map((item) => item.label)).toEqual(
      expect.arrayContaining(["Build", "Lint", "Dead-code · release required"]),
    );
    expect(items.map((item) => item.label)).not.toContain("Smoke");
    const markup = renderToStaticMarkup(
      <QualityAttentionList repository={repository} onOpenRepository={noop} />,
    );
    expect(markup).toContain("Stale");
    expect(markup).toContain("release required");
  });

  it("keeps canonical and discovered custom gate choices with explicit sources in the editor", () => {
    const customGate = makeGate("custom:security_scan", "Security Scan");
    const choices = qualityGateChoices([customGate]);
    expect(choices.map((gate) => gate.id)).toEqual(
      expect.arrayContaining(["build", "dead_code", "custom:security_scan"]),
    );

    const rule: ReleaseRuleConfig = {
      name: "Quality rule",
      operator: "AND",
      required_commit_types: [],
      allow_first_release: false,
      required_quality_gates: [{ gate_id: customGate.id, source: "QR" }],
    };
    const repository = makeRepository({
      release_rule: rule,
      quality: makeQuality({ gates: [...makeQuality().gates, customGate] }),
    });
    const markup = renderToStaticMarkup(
      <PreparationDrawer
        repository={repository}
        preparation={preparation()}
        onClose={() => undefined}
        onSaveReleaseRule={noop}
        onSaveReleaseRecipe={noop}
        onConfirmReleaseVersion={noop}
        onSaveAiPermission={noop}
        onPreviewAiSummary={async () => aiPreview()}
      />,
    );
    expect(markup).toContain("Required quality gates");
    expect(markup).toContain("Security Scan");
    expect(markup).toContain("CI checks");
    expect(markup).toContain("Local command");
    expect(markup).toContain("QR report");
    expect(markup).toContain("Deployment verified");
    expect(markup).toContain("Warn only");
  });

  it("shows the enforced public-release boundary in preparation preview", () => {
    const preview = preparation();
    preview.release.release_boundary_status = "Blocked · Stale";
    preview.release.reasons = [
      "Public-release boundary evidence is blocked and stale; regenerate the v2 receipt for this exact target",
    ];
    const markup = renderToStaticMarkup(
      <PreparationDrawer
        repository={makeRepository()}
        preparation={preview}
        onClose={() => undefined}
        onSaveReleaseRule={noop}
        onSaveReleaseRecipe={noop}
        onConfirmReleaseVersion={noop}
        onSaveAiPermission={noop}
        onPreviewAiSummary={async () => aiPreview()}
      />,
    );

    expect(markup).toContain("Public boundary");
    expect(markup).toContain("Blocked · Stale");
    expect(markup).toContain("regenerate the v2 receipt for this exact target");
  });

  it("renders categorical web readiness with target identity and route drilldown", () => {
    const webReadiness: WebReadinessSnapshot = {
      status: "Warnings",
      applicability: "public_web",
      applicability_reason: "Public product",
      freshness: "Fresh",
      observed_at: "2026-08-10T20:00:00Z",
      scanned_commit: "abcdef1234567890",
      scanned_branch: "main",
      report_path: "/tmp/report.json",
      target: {
        kind: "deployment",
        commit: "abcdef1234567890",
        url: "https://preview.example.test",
        provider: "fixture",
        deployment_id: "dep-123",
      },
      checks: [
        {
          id: "route_titles",
          label: "Route titles",
          category: "baseline",
          policy: "block",
          status: "passed",
          verification_level: "deployment_verified",
          detail: "Every route has a unique title.",
          routes: ["/", "/about"],
        },
      ],
      passed_count: 1,
      failed_count: 0,
      blocked_count: 0,
      unknown_count: 0,
      warning_count: 1,
    };

    const markup = renderToStaticMarkup(
      <WebReadinessSummary
        webReadiness={webReadiness}
        onOpenReport={() => undefined}
      />,
    );
    expect(markup).toContain("Warnings");
    expect(markup).toContain("public web");
    expect(markup).toContain("fixture · dep-123");
    expect(markup).toContain("https://preview.example.test");
    expect(markup).toContain("deployment verified");
    expect(markup).toContain("/about");
  });

  it("renders source expansion without exposing command output or file contents", () => {
    const evidence = makeEvidence({
      command: "pnpm lint",
      detail: "Passed",
      report_path:
        "/tmp/pronto/.quality-runner/runs/run-1/gate-verification.json",
    });
    const markup = renderToStaticMarkup(
      <QualityEvidenceList evidence={[evidence]} onOpenReport={noopReport} />,
    );
    expect(markup).toContain("pnpm lint");
    expect(markup).toContain("Detailed report");
    expect(markup).not.toContain("stdout");
    expect(markup).not.toContain("tracked file contents");
  });

  it("uses Portfolio as the merged destination and embeds the matrix without duplicate overview cards", () => {
    expect(navItems.map((item) => item.label)).toContain("Portfolio");
    expect(navItems.map((item) => item.label)).toContain("AI showcase");
    expect(navItems.map((item) => item.label)).not.toContain("Quality gates");
    expect(navItems.map((item) => item.label)).not.toContain("Products");

    const repository = makeRepository();
    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository])}
        repositories={[repository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );
    expect(markup).toContain("Quality gate matrix");
    expect(markup).not.toContain("Repositories matched");
    expect(markup).not.toContain("Consolidated fleet maturity");
  });

  it("separates Tier-0 release assurance from whole-inventory edge durability", () => {
    const repository = makeRepository({
      quality: makeQuality({
        behavior_assurance: {
          schema: "quality-runner-behavior-assurance/v2",
          applicability: "applicable",
          contract_status: "current",
          contract_schema: "pronto-behavior-assurance/v2",
          edge_profile_status: "partially_profiled",
          result_status: "passed",
          freshness: "current",
          release_ready: true,
          contract_path: ".pronto/behavior-assurance.json",
          receipt_directory: ".quality-runner/behavior-assurance/receipts",
          required_scenario_count: 1,
          passed_scenario_count: 1,
          accepted_defect_count: 0,
          receipt_count: 1,
          coverage: {
            total: 4,
            profiled: 3,
            verified: 1,
            stale: 1,
            failed: 1,
            blocked: 0,
            unknown: 1,
            profile_status: "partially_profiled",
            per_tier: {
              "0": {
                total: 1,
                profiled: 1,
                verified: 1,
                stale: 0,
                failed: 0,
                blocked: 0,
                unknown: 0,
              },
            },
            per_edge_category: {},
            category_gaps: [
              { category: "state_and_ordering", scenario_count: 2 },
            ],
            scenarios: [],
            truncated: false,
          },
          gaps: [],
          next_step: "Review edge coverage separately.",
        },
      }),
    });
    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository])}
        repositories={[repository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup).toContain("Whole-inventory assurance");
    expect(markup).toContain("Edge durability");
    expect(markup).toContain("Release 1/1");
    expect(markup).toContain("Edge 1/4");
    expect(markup).toContain("3/4 profiled");
    expect(markup).toContain("Legacy v1");
    expect(markup).toContain("Reproducible failures");
    expect(markup).toContain("state and ordering");
  });
});
