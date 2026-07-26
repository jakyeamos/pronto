import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  AiPayloadPreview,
  PortfolioSnapshot,
  QualityEvidence,
  QualityFindings,
  QualityFreshness,
  QualityGate,
  QualityMaturity,
  QualityPortfolioSnapshot,
  QualityReadiness,
  QualitySnapshot,
  QualityGateStatus,
  ReleaseRuleConfig,
  RepositoryPreparation,
  RepositorySnapshot,
  WorkspaceSummary,
} from "../types";
import { PreparationDrawer } from "./PreparationDrawer";
import { qualityGateChoices } from "./ReleaseRuleEditor";
import {
  QualityAttentionList,
  QualityEvidenceList,
  qualityAttentionItems,
} from "./QualityComponents";
import { QualityGatesSurface } from "./QualityGatesSurface";

const canonicalGateDefinitions = [
  ["build", "Build"],
  ["runtime_smoke", "Smoke"],
  ["tests", "Tests"],
  ["lint", "Lint"],
  ["formatter", "Formatter"],
  ["typecheck", "Typecheck"],
  ["dead_code", "Dead-code"],
  ["secrets_scan", "Secrets scan"],
] as const;

const workspace: WorkspaceSummary = {
  id: "workspace-1",
  path: "/tmp/pronto",
  is_primary: true,
  branch: "main",
  dirty: false,
  added: 0,
  removed: 0,
  line_totals_partial: false,
  sync_state: "Synced",
  remote_freshness: "Fresh",
  ahead: 0,
  behind: 0,
  integration_state: "Synced",
  target_branch: "main",
  target_confidence: "High",
  role: "Primary",
  role_confidence: "High",
  activity: { state: "Unknown", confidence: "Low", signals: [] },
};

function makeGate(
  id: string,
  label: string,
  status: QualityGateStatus = "Not configured",
  freshness: QualityFreshness = "Unknown",
  evidence: QualityEvidence[] = [],
): QualityGate {
  return { id, label, status, freshness, evidence };
}

function makeEvidence(
  overrides: Partial<QualityEvidence> = {},
): QualityEvidence {
  return {
    id: "build",
    source: "CI",
    status: "Passed",
    freshness: "Fresh",
    observed_at: "2026-07-26T11:00:00Z",
    scanned_commit: "abcdef1234567890",
    scanned_branch: "main",
    source_label: "GitHub check · build",
    detail: "success",
    ...overrides,
  };
}

function makeFindings(
  overrides: Partial<QualityFindings> = {},
): QualityFindings {
  return {
    total: 0,
    severity_counts: {},
    high_severity_total: 0,
    freshness: "Unknown",
    ...overrides,
  };
}

function makeMaturity(
  overrides: Partial<QualityMaturity> = {},
): QualityMaturity {
  return {
    freshness: "Unknown",
    ...overrides,
  };
}

function makeReadiness(
  overrides: Partial<QualityReadiness> = {},
): QualityReadiness {
  return {
    applicable_gate_ids: [
      "build",
      "tests",
      "lint",
      "formatter",
      "typecheck",
      "secrets_scan",
    ],
    missing_gate_ids: [],
    stale_gate_ids: [],
    failed_gate_ids: [],
    blocked_gate_ids: [],
    ...overrides,
  };
}

function makeQuality(
  overrides: Partial<QualitySnapshot> = {},
): QualitySnapshot {
  return {
    gates: canonicalGateDefinitions.map(([id, label]) => makeGate(id, label)),
    findings: makeFindings(),
    maturity: makeMaturity(),
    ci_readiness: makeReadiness(),
    ingestion_status: "No evidence",
    ingestion_message: "No imported evidence",
    ...overrides,
  };
}

function makeRepository(
  overrides: Partial<RepositorySnapshot> = {},
): RepositorySnapshot {
  return {
    id: "repo-1",
    name: "pronto",
    path: "/tmp/pronto",
    locality: "Local only",
    lifecycle: "Active",
    lifecycle_candidate: "Active",
    provider_state: "Local",
    branch: "main",
    default_branch: "main",
    workspace,
    workspaces: [workspace],
    branches: [],
    submodules: [],
    pull_requests: [],
    releases: [],
    quality: makeQuality(),
    ai_permission: "Disabled",
    conditions: [],
    last_scan_at: "2026-07-26T11:00:00Z",
    ...overrides,
  };
}

function makePortfolio(
  repositories: RepositorySnapshot[],
  qualityOverrides: Partial<QualityPortfolioSnapshot> = {},
): PortfolioSnapshot {
  return {
    roots: [],
    repositories,
    products: [],
    groups: [],
    events: [],
    action_audits: [],
    provider_identities: [],
    remote_repositories: [],
    provider_status: {
      provider: "GitHub",
      state: "Not connected",
      message: "Not connected",
      identity_count: 0,
      repository_count: 0,
    },
    quality: {
      matched_repository_count: 0,
      audit_status: "Not configured",
      ...qualityOverrides,
    },
    retention_days: 90,
    generated_at: "2026-07-26T11:00:00Z",
    storage_path: "/tmp/pronto/registry.db",
  };
}

const noop = async (): Promise<void> => undefined;
const noopRepository = (_repository: RepositorySnapshot): void => undefined;
const noopReport = (_reportPath: string): void => undefined;

function preparation(): RepositoryPreparation {
  return {
    repository_id: "repo-1",
    pull_request: {
      repository_id: "repo-1",
      workspace_id: "workspace-1",
      head_branch: "main",
      base_branch: "main",
      commit_count: 0,
      dirty: false,
      ahead: 0,
      behind: 0,
      provider_state: "Local",
      checks_state: "Unknown",
      reviews_state: "Unknown",
      mergeability: "Unknown",
      status: "Evidence ready",
      reasons: [],
      evidence: [],
    },
    release: {
      repository_id: "repo-1",
      target_branch: "main",
      baseline_status: "No baseline",
      commits_since_baseline: [],
      rule_status: "Configured release threshold met",
      rule_trace: [],
      candidate_bump: "patch",
      candidate_version: "v1.0.1",
      version_status: "Candidate version not confirmed",
      notes: [],
      status: "Ready",
      reasons: [],
      evidence: [],
    },
    recipe: {
      repository_id: "repo-1",
      recipe_name: "No recipe",
      candidate_version: "v1.0.1",
      version_status: "Candidate version not confirmed",
      status: "Blocked",
      reasons: [],
      steps: [],
      actions_performed: false,
      generated_at: "2026-07-26T11:00:00Z",
    },
    generated_at: "2026-07-26T11:00:00Z",
  };
}

function aiPreview(): AiPayloadPreview {
  return {
    repository_id: "repo-1",
    workspace_id: "workspace-1",
    permission: "Disabled",
    provider: "None",
    status: "Blocked",
    reasons: [],
    categories: [],
    source_references: [],
    payload_text: "",
    payload_bytes: 0,
    uncommitted_included: false,
    request_performed: false,
    generated_at: "2026-07-26T11:00:00Z",
  };
}

// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("quality evidence surfaces", () => {
  it("renders the canonical matrix, maturity, QR severity, and source evidence", () => {
    const repository = makeRepository({
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
        }),
        ci_readiness: makeReadiness({
          score: 2.67,
          score_display: "2.67",
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
          ci_readiness_score: 2.67,
          ci_readiness_score_display: "2.67",
          ci_readiness_full_repository_count: 0,
          ci_readiness_repository_count: 1,
        })}
        repositories={[repository]}
        onOpenRepository={noopRepository}
        onOpenReport={noopReport}
      />,
    );

    expect(markup).toContain("Quality gate matrix");
    expect(markup).toContain("1.933");
    expect(markup).toContain("CI readiness");
    expect(markup).toContain("2.67");
    expect(markup).toContain("Tests");
    expect(markup).toContain("8 canonical");
    expect(markup).toContain("Show 1 custom gate");
    expect(markup).not.toContain("Security Scan");
    expect(markup).toContain("4");
    expect(markup).toContain("critical");
    expect(markup).toContain("CI · GitHub check · build");
    expect(markup).toContain("Expand evidence");
    expect(markup).toContain("Detailed report");
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
    expect(unconfigured).toContain(
      "Not assessed; refresh to evaluate gate updates",
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
});
