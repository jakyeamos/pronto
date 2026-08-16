// @vitest-environment happy-dom
import { cleanup } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import {
  makeFindings,
  makeMaturity,
  makeQuality,
  makeRepository,
  makePortfolio,
  noopRepository,
  QualityGatesSurface,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("quality maturity surfaces", () => {
  it("shows exact target evidence provenance instead of implying branch-specific stats", () => {
    const repository = makeRepository({
      target_branch: "main",
      target_branch_configured: true,
      branches: [
        {
          name: "main",
          role: "Production",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "Synced",
          last_commit: "target-commit-1234567890",
        },
      ],
      quality: makeQuality({
        findings: makeFindings({
          total: 4022,
          source: "QR",
          scanned_branch: "dev",
          scanned_commit: "scanned-commit-1234567890",
          freshness: "Stale",
        }),
      }),
    });
    const mismatch = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository])}
        repositories={[repository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );
    expect(mismatch).toContain("main @ target-c");
    expect(mismatch).toContain("branch dev · commit scanned");
    expect(mismatch).toContain(
      "Target main is not verified; this scan is from dev.",
    );
    expect(mismatch).toContain("Target detector findings unavailable");
    expect(mismatch).toContain("Raw scanned evidence");
    expect(mismatch).toContain(
      '<details class="quality-findings-raw-details">',
    );
    expect(mismatch).toContain("4,022");
    expect(mismatch).toContain(
      "Breakdown below is from the scanned evidence and is not a target result.",
    );

    const matchingRepository = {
      ...repository,
      branch: "main",
      quality: makeQuality({
        findings: makeFindings({
          total: 2,
          source: "QR",
          scanned_branch: "main",
          scanned_commit: "target-commit-1234567890",
          freshness: "Fresh",
        }),
      }),
    };
    const matching = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([matchingRepository])}
        repositories={[matchingRepository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );
    expect(matching).toContain("Verified for target main @ target-c");
    expect(matching).toContain("Detector findings verified for target");
    expect(matching).not.toContain("Target detector findings unavailable");
    expect(matching).not.toContain("quality-findings-raw-details");

    const staleCommitRepository = {
      ...repository,
      target_branch: "dev",
      branches: [
        {
          name: "dev",
          role: "Development",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "Synced",
          last_commit: "a6d1dac112534b1d2722f92f3fb4e4e40170dd61",
        },
      ],
      quality: makeQuality({
        findings: makeFindings({
          total: 4022,
          source: "QR",
          scanned_branch: "dev",
          scanned_commit: "0b7ffd91f5d8e35896e3d517967cef3ee30468fd",
          freshness: "Stale",
        }),
      }),
    };
    const staleCommit = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([staleCommitRepository])}
        repositories={[staleCommitRepository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );
    expect(staleCommit).toContain(
      "Target dev is not verified; this scan is 0b7ffd91, target is a6d1dac1.",
    );
    expect(staleCommit).toContain("4,022");
    expect(staleCommit).toContain(
      "Detector findings from stale branch evidence",
    );
    expect(staleCommit).toContain(
      "Breakdown is from the selected branch at an older head",
    );
    expect(staleCommit).not.toContain("Target detector findings unavailable");
  });

  it("exposes native, skill, and future detector finding categories without a consumer whitelist", () => {
    const repository = makeRepository({
      quality: makeQuality({
        findings: makeFindings({
          total: 7,
          actionable_total: 5,
          unreviewed_total: 7,
          freshness: "Fresh",
          category_counts: {
            "maintenance-surface": 3,
            speed: 2,
            "skill:performance-readiness": 1,
            "future-category": 1,
          },
          actionable_category_counts: {
            "maintenance-surface": 1,
            speed: 2,
            "skill:performance-readiness": 1,
            "future-category": 1,
          },
        }),
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

    expect(markup).toContain("4 finding categories");
    expect(markup).toContain('aria-label="Detector finding categories"');
    expect(markup).toContain("maintenance surface");
    expect(markup).toContain("skill performance readiness");
    expect(markup).toContain("future category");
    expect(markup).toContain("1</b> actionable · 3 detected");
  });

  it("exposes every imported fleet finding dimension without a consumer whitelist", () => {
    const repository = makeRepository({
      quality: makeQuality({
        maturity: makeMaturity({
          score: 2,
          score_display: "2.000",
          scored_dimension_count: 4,
          freshness: "Fresh",
          dimension_scores: {
            "agent_usability.behavior_evidence": 1,
            change_surface_coverage: 2,
            "diagnosability.stable_error_codes": 2,
            dynamic_verification: 1,
            future_dimension: 4,
          },
        }),
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

    expect(markup).toContain("5 raw diagnostic dimensions");
    expect(markup).toContain('aria-label="Maturity dimension scores"');
    expect(markup).toContain("agent usability behavior evidence");
    expect(markup).toContain("dynamic verification");
    expect(markup).toContain("Stable error codes");
    expect(markup).toContain("future dimension");
  });

  it("renders holistic maturity pillars, coverage, unknowns, and critical caps", () => {
    const repository = makeRepository({
      quality: makeQuality({
        maturity: makeMaturity({
          score: 2,
          score_display: "2.000",
          scored_dimension_count: 3,
          freshness: "Fresh",
          repository_maturity: {
            schema: "quality-runner-repository-maturity/v2",
            score: 2,
            uncapped_score: 3.5,
            status: "blocked",
            pillars: [
              {
                id: "correctness_reliability",
                label: "Correctness and reliability",
                weight: 0.22,
                applicability: "applicable",
                status: "attention",
                score: 3.5,
                dimension_scores: { quality_commands: 3.5 },
                missing_capabilities: ["behavior_outcomes"],
                critical_dimensions: [],
              },
              {
                id: "security_privacy_supply_chain",
                label: "Security, privacy, and supply chain",
                weight: 0.22,
                applicability: "applicable",
                status: "blocked",
                score: 4,
                dimension_scores: { security_constraints: 4 },
                missing_capabilities: ["artifact_provenance"],
                critical_dimensions: ["security_constraints"],
              },
              {
                id: "maintainability_evolvability",
                label: "Maintainability and evolvability",
                weight: 0.16,
                applicability: "applicable",
                status: "unknown",
                dimension_scores: {},
                missing_capabilities: ["architecture_boundaries"],
                critical_dimensions: [],
              },
              {
                id: "operability_release_safety",
                label: "Operability and release safety",
                weight: 0.14,
                applicability: "applicable",
                status: "unknown",
                dimension_scores: {},
                missing_capabilities: ["diagnosability"],
                critical_dimensions: [],
              },
              {
                id: "user_facing_quality",
                label: "User-facing quality",
                weight: 0.1,
                applicability: "unknown",
                status: "unknown",
                dimension_scores: {},
                missing_capabilities: ["user_journey_evidence"],
                critical_dimensions: [],
              },
              {
                id: "human_agent_usability",
                label: "Human and agent usability",
                weight: 0.1,
                applicability: "not_applicable",
                status: "not_applicable",
                dimension_scores: {},
                missing_capabilities: [],
                critical_dimensions: [],
              },
              {
                id: "governance_sustainability",
                label: "Governance and sustainability",
                weight: 0.06,
                applicability: "unknown",
                status: "unknown",
                dimension_scores: {},
                missing_capabilities: ["ownership_and_governance"],
                critical_dimensions: [],
              },
            ],
            evidence: {
              applicable_pillar_count: 4,
              assessed_pillar_count: 2,
              applicable_weight: 0.74,
              assessed_weight: 0.44,
              evidence_coverage: 0.595,
              fresh_evidence_coverage: 0.297,
              unknown_applicability: [
                "user_facing_quality",
                "governance_sustainability",
              ],
              unmapped_dimensions: [],
            },
            critical_cap: {
              applied: true,
              maximum_score: 2,
              reasons: ["security_privacy_supply_chain:security_constraints"],
            },
          },
        }),
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

    expect(markup).toContain('aria-label="Repository maturity pillars"');
    expect(markup).toContain("Security, privacy, and supply chain");
    expect(markup).toContain("60% evidence");
    expect(markup).toContain("30% fresh");
    expect(markup).toContain("Score capped at 2/4");
    expect(markup).toContain("N/A");
    expect(markup).toContain("Unknown");
  });
});
