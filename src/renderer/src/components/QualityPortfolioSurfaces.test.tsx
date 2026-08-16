// @vitest-environment happy-dom
import { cleanup } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import {
  canonicalGateDefinitions,
  workspace,
  makeGate,
  makeEvidence,
  makeFindings,
  makeMaturity,
  makeReadiness,
  makeQuality,
  makeRepository,
  makePortfolio,
  noopRepository,
  QualityGatesSurface,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("portfolio quality surfaces", () => {
  it("labels v2 fleet maturity and keeps product progress separate", () => {
    const repository = makeRepository();
    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository], {
          maturity_score_display: "3.250",
          scored_dimension_count: 6,
          source_maturity_score_display: "3.100",
          feed_schema: "quality-runner-maturity-feed/v2",
          maturity_evidence_coverage: 0.8,
          maturity_fresh_evidence_coverage: 0.7,
          maturity_provisional_repository_count: 1,
          maturity_capped_repository_count: 0,
          maturity_pillars: [
            {
              id: "security_privacy_supply_chain",
              label: "Security, privacy, and supply chain",
              score: 3,
              assessed_repository_count: 1,
            },
          ],
        })}
        repositories={[repository]}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup).toContain("6 pillar assessments");
    expect(markup).toContain("QR source holistic");
    expect(markup).toContain("80% evidence");
    expect(markup).toContain("70% fresh");
    expect(markup).toContain("1 provisional");
    expect(markup).toContain(
      "Product readiness and Project Compass progress are reported separately",
    );
  });

  it("keeps Tenure dev evidence visible while marking its older head stale", () => {
    const targetCommit = "a6d1dac112534b1d2722f92f3fb4e4e40170dd61";
    const scannedCommit = "0b7ffd91f5d8e35896e3d517967cef3ee30468fd";
    const staleEvidence = {
      scanned_branch: "dev",
      scanned_commit: scannedCommit,
    };
    const repository = makeRepository({
      name: "tenure",
      branch: "dev",
      target_branch: "dev",
      target_branch_configured: true,
      workspace: { ...workspace, branch: "dev", last_commit: targetCommit },
      workspaces: [{ ...workspace, branch: "dev", last_commit: targetCommit }],
      branches: [
        {
          name: "dev",
          role: "Development",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "Synced",
          last_commit: targetCommit,
        },
      ],
      quality: makeQuality({
        gates: canonicalGateDefinitions.map(([id, label]) =>
          makeGate(id, label, "Blocked", "Stale", [
            makeEvidence({
              ...staleEvidence,
              status: "Blocked",
              freshness: "Stale",
            }),
          ]),
        ),
        findings: makeFindings({
          total: 4022,
          scanned_branch: "dev",
          scanned_commit: scannedCommit,
          freshness: "Stale",
        }),
        maturity: makeMaturity({
          score: 1.909,
          score_display: "1.909",
          audit_id: "audit-tenure",
          freshness: "Fresh",
          scanned_branch: undefined,
          scanned_commit: undefined,
        }),
        ci_readiness: makeReadiness({
          configuration_score: 4,
          configuration_score_display: "4.0",
          configured_gate_ids: canonicalGateDefinitions.map(([id]) => id),
          covered_gate_ids: canonicalGateDefinitions.map(([id]) => id),
          fresh_passing_gate_ids: [],
          stale_gate_ids: canonicalGateDefinitions.map(([id]) => id),
          blocked_gate_ids: canonicalGateDefinitions.map(([id]) => id),
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

    expect(markup).toContain("Target dev @ a6d1dac1");
    expect(markup).toContain("1.909");
    expect(markup).toContain("Unscoped maturity evidence");
    expect(markup).toContain("4,022");
    expect(markup).toContain("Detector findings from stale branch evidence");
    expect(markup).toContain("Stale branch evidence");
    expect(markup).toContain("Blocked");
    expect(markup).not.toContain("Target maturity unavailable");
    expect(markup).not.toContain("Target readiness unavailable");
    expect(markup).not.toContain("Target detector findings unavailable");
  });

  it("projects gate, maturity, and readiness evidence from the selected target", () => {
    const targetCommit = "target-head-1234567890";
    const targetBranch = "main";
    const targetBranches = [
      {
        name: targetBranch,
        role: "Production",
        role_confidence: "High",
        target_confidence: "High",
        ahead: 0,
        behind: 0,
        integration_state: "Synced",
        last_commit: targetCommit,
      },
    ];
    const readiness = makeReadiness({
      applicable_gate_ids: ["build"],
      configured_gate_ids: ["build"],
      configuration_score: 1,
      configuration_score_display: "1",
      covered_gate_ids: ["build"],
      fresh_passing_gate_ids: ["build"],
    });
    const mismatchedEvidence = {
      scanned_branch: "dev",
      scanned_commit: "stale-head-1234567890",
    };
    const mismatchedRepository = makeRepository({
      name: "tenure",
      target_branch: targetBranch,
      target_branch_configured: true,
      branches: targetBranches,
      quality: makeQuality({
        gates: [
          makeGate("build", "Build", "Passed", "Fresh", [
            makeEvidence(mismatchedEvidence),
          ]),
        ],
        findings: makeFindings(mismatchedEvidence),
        maturity: makeMaturity({
          score: 3.4,
          score_display: "3.4",
          audit_id: "audit-stale",
          freshness: "Fresh",
          ...mismatchedEvidence,
        }),
        ci_readiness: readiness,
      }),
    });
    const mismatch = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([mismatchedRepository])}
        repositories={[mismatchedRepository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );

    expect(mismatch).toContain("Target maturity unavailable");
    expect(mismatch).toContain(
      "Target main is not verified; this scan is from dev.",
    );
    expect(mismatch).toContain("Target readiness unavailable");
    expect(mismatch).toContain("Target evidence unavailable");
    expect(mismatch).toContain("Raw maturity evidence");
    expect(mismatch).toContain("Raw readiness evidence");
    expect(mismatch).toContain("Raw scanned evidence");
    expect(mismatch).toContain("Target detector findings unavailable");

    const matchingEvidence = {
      scanned_branch: targetBranch,
      scanned_commit: targetCommit,
    };
    const matchingRepository = {
      ...mismatchedRepository,
      branch: targetBranch,
      quality: makeQuality({
        gates: [
          makeGate("build", "Build", "Passed", "Fresh", [
            makeEvidence(matchingEvidence),
          ]),
        ],
        findings: makeFindings(matchingEvidence),
        maturity: makeMaturity({
          score: 3.4,
          score_display: "3.4",
          audit_id: "audit-target",
          freshness: "Fresh",
          ...matchingEvidence,
        }),
        ci_readiness: readiness,
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

    expect(matching).toContain("Verified for target main @ target-h");
    expect(matching).toContain("3.4");
    expect(matching).toContain("Fresh passing evidence: 1/1");
    expect(matching).toContain("Passed");
    expect(matching).not.toContain("Target maturity unavailable");
    expect(matching).not.toContain("Target readiness unavailable");
    expect(matching).not.toContain("Target evidence unavailable");
    expect(matching).not.toContain("Target detector findings unavailable");

    const ambiguousRepository = {
      ...mismatchedRepository,
      quality: makeQuality({
        gates: [
          makeGate("build", "Build", "Passed", "Fresh", [
            makeEvidence({
              scanned_branch: undefined,
              scanned_commit: undefined,
            }),
          ]),
        ],
        findings: makeFindings({
          scanned_branch: undefined,
          scanned_commit: undefined,
        }),
        maturity: makeMaturity({
          score: 3.4,
          score_display: "3.4",
          audit_id: "audit-ambiguous",
          freshness: "Fresh",
          scanned_branch: undefined,
          scanned_commit: undefined,
        }),
        ci_readiness: readiness,
      }),
    };
    const ambiguous = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([ambiguousRepository])}
        repositories={[ambiguousRepository]}
        showOverview={false}
        onOpenRepository={noopRepository}
      />,
    );

    expect(ambiguous).toContain(
      "Target main is not verified; branch/commit provenance is incomplete.",
    );
    expect(ambiguous).toContain("Unscoped maturity evidence");
    expect(ambiguous).toContain("Unscoped evidence");
    expect(ambiguous).toContain("0");
    expect(ambiguous).not.toContain("Target maturity unavailable");
    expect(ambiguous).not.toContain("Target readiness unavailable");
    expect(ambiguous).not.toContain("Target detector findings unavailable");
  });

  it("renders empty and unconfigured repository states", () => {
    const empty = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([])}
        repositories={[]}
        onOpenRepository={noopRepository}
      />,
    );
    expect(empty).toContain("Register a repository root");

    const repository = makeRepository();
    const unconfigured = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository])}
        repositories={[repository]}
        onOpenRepository={noopRepository}
      />,
    );
    expect(unconfigured).toContain("Not configured");
    expect(unconfigured).toContain("No CI, local, or QR gate evidence");
    expect(unconfigured).toContain("No matched recommendation profile");
  });

  it("shows configured conditional gates before passing evidence exists", () => {
    const repository = makeRepository({
      quality: makeQuality({
        ci_readiness: makeReadiness({
          applicable_gate_ids: [
            "build",
            "runtime_smoke",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
            "secrets_scan",
            "dependency_audit",
          ],
          configured_gate_ids: [
            "build",
            "runtime_smoke",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
            "secrets_scan",
            "dependency_audit",
          ],
          missing_gate_ids: [
            "build",
            "runtime_smoke",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
            "secrets_scan",
            "dependency_audit",
          ],
        }),
      }),
    });

    const markup = renderToStaticMarkup(
      <QualityGatesSurface
        snapshot={makePortfolio([repository])}
        repositories={[repository]}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup).toContain("Dependency audit");
    expect(markup).toContain("Configured");
    expect(markup).not.toContain("No CI, local, or QR gate evidence");
    expect(markup).not.toContain("Not applicable for this repository");
    expect(
      markup.match(
        />Configured<\/span><span class="quality-gate-evidence-count">No evidence/g,
      ),
    ).toHaveLength(9);
  });
});
