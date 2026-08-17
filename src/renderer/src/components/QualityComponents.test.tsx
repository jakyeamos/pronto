// @vitest-environment happy-dom
import { cleanup } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import {
  makeGate,
  makeEvidence,
  makeFindings,
  makeMaturity,
  makeReadiness,
  makeQuality,
  makeRepository,
  makePortfolio,
  noopRepository,
  noopReport,
  ProjectCompassDetail,
  QualityFindingsSummary,
  QualityOutcomeSummary,
  QualityGatesSurface,
  RepositoryRow,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("quality evidence surfaces", () => {
  it("renders the canonical matrix, maturity, QR severity, and source evidence", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 3,
        updated_at: "2026-07-28T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 75,
          scored_outcome_count: 3,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "high",
          confidence_percent: 100,
        },
        complete_product: {
          progress_percent: 50,
          scored_outcome_count: 4,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "medium",
          confidence_percent: 60,
        },
        open_blockers: 2,
        open_drift: 1,
        open_blocker_items: [
          {
            outcome_id: "release-preparation",
            outcome_name: "A release candidate can be inspected",
            kind: "verification",
            summary: "Provider-native release proof is missing.",
          },
          {
            outcome_id: "release-safety",
            outcome_name: "Release mutation remains governed",
            kind: "evidence",
            summary: "The release boundary still needs fresh evidence.",
          },
        ],
        open_drift_items: [
          {
            kind: "verification-gap",
            summary:
              "Release evidence trails the intended product finish line.",
            observed_at: "2026-07-28T00:00:00Z",
          },
        ],
        error: null,
      },
      quality: makeQuality({
        gates: [
          makeGate("build", "Build", "Passed", "Fresh", [
            makeEvidence({
              id: "build",
              source_label: "GitHub check · build",
              report_url: "https://github.com/example/pronto/checks",
            }),
          ]),
          makeGate("custom:security_scan", "Security Scan", "Failed", "Fresh", [
            makeEvidence({
              id: "custom:security_scan",
              source: "QR",
              source_label: "Security Scan · quality-audit",
              status: "Failed",
              report_path:
                "/tmp/pronto/.quality-runner/runs/run-1/quality-audit.json",
              detail: "finding threshold exceeded",
            }),
          ]),
        ],
        findings: makeFindings({
          total: 4,
          actionable_total: 3,
          reviewed_total: 2,
          unreviewed_total: 2,
          disposition_counts: { confirmed: 1, false_positive: 1 },
          disposition_status: "Ready",
          severity_counts: { critical: 1, high: 1, medium: 1, low: 1 },
          high_severity_total: 2,
          source: "QR",
          observed_at: "2026-07-26T11:00:00Z",
          freshness: "Fresh",
          report_path:
            "/tmp/pronto/.quality-runner/runs/run-1/quality-audit.json",
        }),
        maturity: makeMaturity({
          score: 2.7,
          score_display: "2.7",
          scored_dimension_count: 10,
          audit_id: "audit-1",
          observed_at: "2026-07-26T11:00:00Z",
          freshness: "Fresh",
          gaps: [
            {
              dimension: "change_surface_coverage",
              status: "missing",
              score: 0,
              message: "No repository-owned change-surface matrix was found.",
            },
          ],
          agent_usability: {
            schema: "quality-runner-agent-usability/v1",
            status: "attention",
            manifest_status: "present",
            manifest_path: ".agents/agent-usability.json",
            applicable_lane_count: 4,
            covered_lane_count: 3,
            lanes: [
              {
                id: "documentation_contract",
                label: "Documentation contract",
                applicable: true,
                score: 4,
                status: "maintained",
                message: "Every declared tool has fresh, routed documentation.",
              },
              {
                id: "tool_skill_coverage",
                label: "Tool-to-skill coverage",
                applicable: true,
                score: 3,
                status: "static_validated",
                message: "Every declared tool maps to a known skill.",
              },
              {
                id: "behavior_evidence",
                label: "Behavior evidence",
                applicable: true,
                score: 2,
                status: "partial",
                message: "Fresh passing receipts are incomplete.",
              },
              {
                id: "freshness_portability",
                label: "Freshness and portability",
                applicable: true,
                score: 3,
                status: "static_validated",
                message: "Repository-relative references passed validation.",
              },
            ],
            growth_health: {
              status: "healthy",
              score: 4,
              message:
                "Documentation and skill structure remains proportionate and routed.",
              document_count: 12,
              agent_document_count: 3,
              routed_agent_document_count: 3,
              unrouted_agent_document_count: 0,
              oversized_document_count: 0,
              skill_count: 4,
              family_count: 2,
              largest_family_size: 2,
              unclassified_skill_count: 0,
              oversized_skill_count: 0,
              tool_count: 2,
              documented_tool_count: 2,
              skill_covered_tool_count: 2,
              behavior_declared_tool_count: 1,
              behavior_verified_tool_count: 0,
              inventory_truncated: false,
            },
          },
        }),
        ci_readiness: makeReadiness({
          score: 2.67,
          score_display: "2.67",
          configuration_score: 0.67,
          configuration_score_display: "0.67",
          configured_gate_ids: ["build"],
          unconfigured_gate_ids: [
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "secrets_scan",
          ],
          covered_gate_ids: ["build"],
          fresh_passing_gate_ids: ["build"],
          missing_gate_ids: ["tests"],
        }),
        ingestion_status: "Available",
        last_ingested_at: "2026-07-26T11:00:00Z",
      }),
    });
    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository], {
          audit_status: "Ready",
          maturity_score: 1.933,
          maturity_score_display: "1.933",
          scored_dimension_count: 10,
          matched_repository_count: 1,
          latest_audit_at: "2026-07-26T11:00:00Z",
          quality_outcome_counts: {
            checks_failing: 2,
            verification_blocked: 1,
            review_needed: 3,
            evidence_unknown: 1,
          },
          quality_outcome_taxonomy: {
            checks_failing: {
              label: "Quality checks failing",
              meaning: "Observed quality checks or blocker findings failed.",
            },
            verification_blocked: {
              label: "Quality verification blocked",
              meaning:
                "Setup, execution, timeout, or target provenance prevented a trustworthy verdict.",
            },
            review_needed: {
              label: "Quality review needed",
              meaning:
                "No blocker is known, but applicable evidence remains below ideal or unverified.",
            },
            evidence_unknown: {
              label: "Quality evidence unknown",
              meaning:
                "Required evidence is unavailable, stale, or explicitly unknown.",
            },
          },
          ci_readiness_score: 2.67,
          ci_readiness_score_display: "2.67",
          ci_readiness_full_repository_count: 0,
          ci_readiness_repository_count: 1,
          ci_configuration_configured_gate_count: 1,
          ci_configuration_ideal_gate_count: 6,
          ci_configuration_full_repository_count: 0,
          ci_configuration_repository_count: 1,
          ci_evidence_fresh_passing_gate_count: 1,
          ci_evidence_ideal_gate_count: 6,
          mac_control_ideal_state: {
            status: "Failed",
            freshness: "Unknown",
            ideal_state: false,
            applicable_repository_count: 1,
            not_applicable_repository_count: 0,
            evaluated_repository_count: 1,
            implementation_status: "Failed",
            implementation_score: 3.5,
            implementation_score_display: "3.5",
            implementation_criteria_passed_count: 7,
            implementation_criteria_total: 8,
            implementation_declaration_criteria_count: 0,
            live_status: "Review required",
            live_score: 0,
            live_score_display: "0.0",
            live_task_count: 3,
            measured_task_count: 0,
            failure_reasons: ["The report has an unresolved task contract."],
          },
        })}
        repositories={[repository]}
        onOpenRepository={noopRepository}
        onOpenReport={noopReport}
      />,
    );
    expect(markup).toContain("Quality checks failing");
    expect(markup).toContain("Quality verification blocked");
    expect(markup).toContain("Quality review needed");
    expect(markup).toContain("Evidence review required");
    expect(markup).not.toContain("Quality evidence unknown");

    expect(markup).toContain("Quality gate matrix");
    expect(markup).toContain("1.933");
    expect(markup).toContain("CI configuration");
    expect(markup).toContain("1/6");
    expect(markup).toContain("Fresh passing evidence: 1/6");
    expect(markup).toContain("Mac Control ideal state");
    expect(markup).toContain(
      "1 applicable repositories · Fleet freshness incomplete—inspect repository blockers",
    );
    expect(markup).toContain(
      "Semantic source evidence: 7/8 dimensions · 3.5/4 · Failed",
    );
    expect(markup).toContain(
      "Live tasks: 0/3 measured · 0.0/4 · Review required",
    );
    expect(markup).toContain(
      "Source-grounded semantics and live task evidence are both required for the 4.0/4.0 maturity ideal",
    );
    expect(markup).toContain("Tests");
    expect(markup).toContain("8 canonical");
    expect(markup).toContain("Show 1 custom gate");
    expect(markup).not.toContain("Security Scan");
    expect(markup).toContain("4");
    expect(markup).toContain("Detector findings verified for target");
    expect(markup).not.toContain(
      "Detector findings detected in scanned evidence",
    );
    expect(markup).toContain("3</b> actionable");
    expect(markup).toContain("2</b> unreviewed detector findings");
    expect(markup).toContain("1</b> false positive");
    expect(markup).toContain("Review ledger: Ready");
    expect(markup).toContain("critical");
    expect(markup).toContain("CI · GitHub check · build");
    expect(markup).toContain("Expand evidence");
    expect(markup).toContain("Detailed report");
    expect(markup).toContain(
      "No repository-owned change-surface matrix was found.",
    );
    expect(markup).toContain("Agent usability");
    expect(markup).toContain("3/4 lanes");
    expect(markup).toContain("Documentation contract");
    expect(markup).toContain("3/3 agent docs routed");
    expect(markup).toContain("4 skills in 2 families");
    expect(markup).toContain("4/4");
  });

  it("uses the evidence-review disposition when an older feed omits taxonomy", () => {
    const markup = renderToStaticMarkup(
      <QualityOutcomeSummary
        quality={{
          matched_repository_count: 1,
          audit_status: "Ready",
          quality_outcome_counts: { evidence_unknown: 1 },
        }}
      />,
    );

    expect(markup).toContain("Evidence review required");
    expect(markup).toContain("Required evidence is not current or confirmed");
    expect(markup).not.toContain("evidence_unknown");
    expect(markup).not.toContain("unknown");
  });

  it("shows refresh-required detector evidence without presenting the retained count as current", () => {
    const markup = renderToStaticMarkup(
      <QualityFindingsSummary
        findings={makeFindings({
          total: 3,
          detector_findings_total: 3,
          actionable_total: 2,
          detector_actionable_total: 2,
          unreviewed_total: 1,
          detector_unreviewed_total: 1,
          source: "QR",
          scanned_branch: "main",
          scanned_commit: "target-commit-1234567890",
          freshness: "Fresh",
          enabled_detector_count: 1,
          enabled_rule_count: 3,
          producer_versions: { "anti-slop": "0.8.0" },
          producer_source_shas: { "anti-slop": "producer-sha" },
          ruleset_fingerprints: { "anti-slop": "ruleset-sha" },
          configuration_fingerprints: { "anti-slop": "config-sha" },
          qr_version: "0.7.0",
          target_sha: "target-commit-1234567890",
          refresh_time: "2026-08-16T03:00:00Z",
          refresh_required: true,
          refresh_required_reason:
            "The anti-slop ruleset changed; refresh required.",
          detector_status: "blocked",
        })}
        targetBranch="main"
        targetCommit="target-commit-1234567890"
      />,
    );

    expect(markup).toContain("Detector findings refresh required");
    expect(markup).toContain("Refresh-required detector evidence");
    expect(markup).toContain(
      "Detector findings retained from scanned evidence",
    );
    expect(markup).toContain(">3</strong>");
    expect(markup).toContain("1 detector");
    expect(markup).toContain("3 rules");
    expect(markup).toContain("anti-slop 0.8.0");
    expect(markup).toContain("Detector fingerprints");
    expect(markup).toContain("refresh required");
    expect(markup).toContain("quality-findings-total-unverified");
  });

  it("shows Compass coverage beside progress instead of implying complete evidence", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 7,
        updated_at: "2026-08-08T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 50,
          scored_outcome_count: 1,
          covered_pillar_count: 1,
          total_pillar_count: 2,
          confidence: "medium",
          confidence_percent: 60,
        },
        complete_product: {
          progress_percent: 75,
          scored_outcome_count: 3,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "high",
          confidence_percent: 100,
        },
        open_blockers: 0,
        open_drift: 0,
        open_blocker_items: [],
        open_drift_items: [],
        error: null,
      },
    });

    const row = renderToStaticMarkup(
      <RepositoryRow
        repository={repository}
        onOpen={() => undefined}
        onCondition={() => undefined}
      />,
    );
    expect(row).toContain("MVP 50% · 1/2 pillars");
    expect(row).toContain(
      'title="Coverage incomplete · 1 scoped outcome · 1/2 pillars covered"',
    );

    const detail = renderToStaticMarkup(
      <ProjectCompassDetail repository={repository} />,
    );
    expect(detail).toContain(
      "Coverage incomplete · 1 scoped outcome · 1/2 pillars covered",
    );
    expect(detail).toContain("3 scoped outcomes · 2/2 pillars covered");
  });
});
