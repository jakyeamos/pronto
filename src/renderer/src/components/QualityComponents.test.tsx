// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import type {
  AiPayloadPreview,
  AnalyticsSnapshot,
  Condition,
  ProductConfig,
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
  RemediationRun,
  ReleaseRuleConfig,
  RepositoryPreparation,
  RepositorySnapshot,
  WorkspaceSummary,
  WebReadinessSnapshot,
} from "../types";
import { PreparationDrawer } from "./PreparationDrawer";
import { qualityGateChoices } from "./ReleaseRuleEditor";
import { CommandCenterSurface } from "./CommandCenterSurface";
import { AppSidebar } from "./AppSidebar";
import { AttentionQueue, RepositoryRow } from "./PortfolioComponents";
import {
  QualityAttentionList,
  QualityEvidenceList,
  QualityOutcomeSummary,
  qualityAttentionItems,
} from "./QualityComponents";
import { QualityGatesSurface } from "./QualityGatesSurface";
import { QualityFindingsSummary } from "./QualityFindingsSummary";
import { RepositoryDetailSurface } from "./Drawers";
import { PortfolioCollectionsSurface } from "./PortfolioCollectionsSurface";
import { navItems } from "../navigation";
import { ProjectCompassDetail } from "./ProjectCompassDetail";
import { WebReadinessSummary } from "./WebReadinessSummary";

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
  last_commit: "abcdef1234567890",
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
    category_counts: {},
    actionable_category_counts: {},
    actionable_total: 0,
    reviewed_total: 0,
    unreviewed_total: 0,
    disposition_counts: {},
    stale_disposition_total: 0,
    disposition_status: "Missing",
    severity_counts: {},
    high_severity_total: 0,
    scanned_commit: "abcdef1234567890",
    scanned_branch: "main",
    freshness: "Unknown",
    ...overrides,
  };
}

function makeMaturity(
  overrides: Partial<QualityMaturity> = {},
): QualityMaturity {
  return {
    freshness: "Unknown",
    scanned_commit: "abcdef1234567890",
    scanned_branch: "main",
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
    configured_gate_ids: [],
    unconfigured_gate_ids: [],
    covered_gate_ids: [],
    fresh_passing_gate_ids: [],
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
    behavior_assurance: overrides.behavior_assurance ?? {
      schema: "quality-runner-behavior-assurance/v1",
      applicability: "unknown",
      contract_status: "missing",
      result_status: "unknown",
      freshness: "unknown",
      release_ready: false,
      contract_path: ".pronto/behavior-assurance.json",
      receipt_directory: ".quality-runner/behavior-assurance/receipts",
      required_scenario_count: 0,
      passed_scenario_count: 0,
      accepted_defect_count: 0,
      receipt_count: 0,
      gaps: [],
      next_step: "Run Quality Runner.",
    },
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
    target_branch: "main",
    target_branch_configured: false,
    workspace,
    workspaces: [workspace],
    branches: [],
    submodules: [],
    pull_requests: [],
    releases: [],
    quality: makeQuality(),
    project_compass: {
      status: "Missing",
      contract_path: ".project-compass/contract.json",
      revision: null,
      updated_at: null,
      project_name: null,
      identity: null,
      audience: null,
      mvp: {
        progress_percent: null,
        scored_outcome_count: 0,
        covered_pillar_count: 0,
        total_pillar_count: 0,
        confidence: "unknown",
        confidence_percent: 0,
      },
      complete_product: {
        progress_percent: null,
        scored_outcome_count: 0,
        covered_pillar_count: 0,
        total_pillar_count: 0,
        confidence: "unknown",
        confidence_percent: 0,
      },
      open_blockers: 0,
      open_drift: 0,
      open_blocker_items: [],
      open_drift_items: [],
      error: null,
    },
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
  const remediation: RemediationRun = {
    schema_version: "pronto-remediation/v3",
    id: "remediation-1",
    generated_at: "2026-07-26T11:00:00Z",
    source_refresh_id: null,
    status: "not_run",
    message: null,
    eligible_repository_ids: repositories.map((repository) => repository.id),
    eligible_repository_paths: repositories.map(
      (repository) => repository.path,
    ),
    refresh_steps: [],
    excluded_repositories: [],
    closures: [],
    plans: [],
  };
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
    remediation,
    showcase: {
      schema_version: "pronto-showcase/v2",
      status: "Missing",
      contract_path: ".pronto/showcase-goal.json",
      reviewed_at: null,
      quality_bar_source: null,
      goal: {
        target_publishable_demo_count: 0,
        publishable_demo_count: 0,
        remaining_demo_count: 0,
        status: "Not configured",
      },
      scoring: null,
      public_queue: [],
      private_client_count: 0,
      projects: [],
      error: null,
    },
    retention_days: 90,
    generated_at: "2026-07-26T11:00:00Z",
    storage_path: "/tmp/pronto/registry.db",
  };
}

const noop = async (): Promise<void> => undefined;
const noopRepository = (_repository: RepositorySnapshot): void => undefined;
const noopReport = (_reportPath: string): void => undefined;
const noopCondition = (
  _repository: RepositorySnapshot,
  _condition: Condition,
): void => undefined;

const analyticsSnapshot: AnalyticsSnapshot = {
  schema_version: "pronto-analytics/v1",
  generated_at: "2026-07-26T11:00:00Z",
  source: "Local refresh snapshots",
  freshness: "Unavailable until the first local refresh",
  range_days: 30,
  retention_days: 90,
  portfolio_samples: [],
  repositories: [],
};

afterEach(cleanup);

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

  it("nests release products under the Groups destination", () => {
    const repository = makeRepository();
    const product: ProductConfig = {
      id: "product-1",
      name: "Pronto",
      repository_ids: [repository.id],
      release_mode: "Independent",
      created_at: "2026-07-26T11:00:00Z",
      updated_at: "2026-07-26T11:00:00Z",
    };
    const markup = renderToStaticMarkup(
      <PortfolioCollectionsSurface
        groups={[]}
        products={[product]}
        repositories={[repository]}
        onSaveGroup={noop}
        onDeleteGroup={noop}
        onSaveProduct={noop}
        onDeleteProduct={noop}
      />,
    );
    expect(markup).toContain("Groups");
    expect(markup).toContain("Release products");
    expect(markup).toContain('class="surface-panel collection-subsection"');
  });

  it("starts the Attention queue and repository groups collapsed", () => {
    const repository = makeRepository({
      quality: makeQuality({
        gates: [makeGate("build", "Build", "Failed", "Fresh")],
      }),
    });
    const markup = renderToStaticMarkup(
      <AttentionQueue
        repositories={[repository]}
        onCondition={noopCondition}
        onOpenRepository={noopRepository}
      />,
    );
    expect(markup).toContain("Attention queue");
    expect(markup).toContain('class="rail-section attention-queue"');
    expect(markup).not.toContain('class="rail-section attention-queue" open');
    expect(markup).not.toContain(
      'class="attention-group quality-attention-group" open',
    );
  });

  it("keeps the sidebar repository index searchable and status-only", () => {
    const repository = makeRepository({ name: "Local project" });
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={[repository]}
        remediation={makePortfolio([repository]).remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );
    expect(markup).toContain("Repositories");
    expect(markup).toContain("Find a repository");
    expect(markup).toContain("Local project");
    expect(markup).toContain(
      'aria-label="Open repository Local project, item 1"',
    );
    expect(markup).not.toContain("/tmp/pronto");
    expect(markup).not.toContain("main");
    expect(markup).not.toContain("Quality gates");
    expect(markup).not.toContain('class="brand"');
    expect(markup).not.toContain("Portfolio command center");
    expect(markup).not.toContain("sidebar-rule");
    expect(markup).not.toContain("Local evidence only");
    expect(markup).not.toContain("Private by default");
  });

  it("keeps remediation-excluded repositories out of the sidebar", () => {
    const eligible = makeRepository({
      id: "eligible-repository",
      name: "Eligible project",
      path: "/tmp/eligible-project",
    });
    const excluded = makeRepository({
      id: "excluded-repository",
      name: "Excluded project",
      path: "/tmp/excluded-project",
    });
    const remediation = makePortfolio([eligible, excluded]).remediation;
    remediation.excluded_repositories = [
      {
        repository_id: excluded.id,
        repository_name: excluded.name,
        repository_path: excluded.path,
        reason: "Currently in progress; excluded from this refresh.",
      },
    ];

    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={[eligible, excluded]}
        remediation={remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup).toContain("1 eligible local");
    expect(markup).toContain("Eligible project");
    expect(markup).not.toContain("Excluded project");
  });

  it("shows stale-only repositories in blue and preserves red precedence", () => {
    const staleQuality = makeQuality({
      gates: [makeGate("build", "Build", "Passed", "Stale")],
    });
    const staleCondition: Condition = {
      id: "condition-stale",
      kind: "remote-stale",
      title: "Remote state stale",
      summary: "Pronto has not recorded a successful fetch.",
      priority: 8,
      status: "Active",
      fingerprint: "remote-stale",
      rule: "Remote comparisons require a recorded fetch.",
      evidence: [],
      missing: [],
    };
    const repositories = [
      makeRepository({
        id: "quality-stale",
        name: "Quality stale only",
        quality: staleQuality,
      }),
      makeRepository({
        id: "remote-stale",
        name: "Remote stale only",
        conditions: [staleCondition],
      }),
      makeRepository({
        id: "stale-and-failed",
        name: "Stale and failed",
        quality: makeQuality({
          gates: [makeGate("build", "Build", "Failed", "Stale")],
        }),
      }),
      makeRepository({
        id: "stale-and-dirty",
        name: "Stale and dirty",
        quality: staleQuality,
        workspace: { ...workspace, dirty: true },
      }),
    ];
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={repositories}
        remediation={makePortfolio(repositories).remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup.match(/sidebar-repository-status-stale/g)).toHaveLength(2);
    expect(markup.match(/title="Stale evidence only"/g)).toHaveLength(2);
    expect(markup.match(/sidebar-repository-status-attention/g)).toHaveLength(
      2,
    );
    expect(markup.match(/title="Needs attention"/g)).toHaveLength(2);
  });

  it("shows integration eligibility in violet only when it is the sole signal", () => {
    const integrationCondition: Condition = {
      id: "condition-integration",
      kind: "integration-eligible",
      title: "Branch is ready to integrate",
      summary: "The branch is ahead of its target and the workspace is clean.",
      priority: 5,
      status: "Active",
      fingerprint: "integration-eligible",
      rule: "Clean branches ahead of their target are integration eligible.",
      evidence: [],
      missing: [],
    };
    const staleCondition: Condition = {
      id: "condition-stale",
      kind: "remote-stale",
      title: "Remote state stale",
      summary: "Pronto has not recorded a successful fetch.",
      priority: 8,
      status: "Active",
      fingerprint: "remote-stale",
      rule: "Remote comparisons require a recorded fetch.",
      evidence: [],
      missing: [],
    };
    const repositories = [
      makeRepository({
        id: "integration-only",
        name: "Integration only",
        conditions: [integrationCondition],
      }),
      makeRepository({
        id: "integration-and-stale",
        name: "Integration and stale",
        conditions: [integrationCondition, staleCondition],
      }),
      makeRepository({
        id: "integration-and-dirty",
        name: "Integration and dirty",
        conditions: [integrationCondition],
        workspace: { ...workspace, dirty: true },
      }),
      makeRepository({
        id: "integration-and-blocked",
        name: "Integration condition with broader blockers",
        conditions: [integrationCondition],
      }),
    ];
    const remediation = makePortfolio(repositories).remediation;
    remediation.plans = [
      {
        repository_id: "integration-only",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-stale",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-dirty",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-blocked",
        status: "blocked",
        integration_only_remaining: false,
      } as RemediationRun["plans"][number],
    ];
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={repositories}
        remediation={remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup.match(/sidebar-repository-status-opportunity/g)).toHaveLength(
      1,
    );
    expect(
      markup.match(/title="Integration is the only remaining remediation"/g),
    ).toHaveLength(1);
    expect(markup.match(/sidebar-repository-status-attention/g)).toHaveLength(
      3,
    );
    expect(markup.match(/title="Needs attention"/g)).toHaveLength(3);
  });

  it("renders repository detail as a full page with quality, maturity, QR, and release context", () => {
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
        gates: [makeGate("build", "Build")],
        ci_readiness: makeReadiness({
          applicable_gate_ids: ["build", "dependency_audit"],
          configured_gate_ids: ["build", "dependency_audit"],
        }),
      }),
      release_rule: {
        name: "Ready to release",
        operator: "AND",
        required_commit_types: [],
        allow_first_release: false,
        required_quality_gates: [{ gate_id: "build", source: "CI" }],
      },
      target_branch: "develop",
      target_branch_configured: true,
      branches: [
        {
          name: "main",
          role: "Production",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
        },
        {
          name: "develop",
          role: "Integration",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
        },
      ],
    });
    const markup = renderToStaticMarkup(
      <RepositoryDetailSurface
        repository={repository}
        analytics={analyticsSnapshot}
        isRefreshing={false}
        onBack={() => undefined}
        onOpenWorkspace={async () => undefined}
        onPrepareRepository={async () => undefined}
        onTargetBranchChange={async () => undefined}
        onLifecycleChange={async () => undefined}
        onCondition={() => undefined}
      />,
    );
    expect(markup).toContain("Back to Portfolio");
    expect(markup).toContain("Target branch");
    expect(markup).toContain("Pronto override · Git default: main");
    expect(markup).toContain(
      "Selecting a branch or refreshing evidence checks existing target evidence first, then runs QR quality and fleet audits in a clean disposable worktree when the target head changed or matching evidence is unavailable; your active workspace is not switched.",
    );
    expect(markup).toContain('aria-label="Target branch for pronto"');
    expect(markup).toContain('aria-label="Refresh target evidence for pronto"');
    expect(markup).toContain(
      '<option value="develop" selected="">develop</option>',
    );
    expect(markup).toContain("/tmp/pronto");
    expect(markup).toContain("Quality gates");
    expect(markup).toContain("Project Compass");
    expect(markup).toContain("Contract valid");
    expect(markup).toContain("Provider-native release proof is missing.");
    expect(markup).toContain(
      "Release evidence trails the intended product finish line.",
    );
    expect(markup).toContain("Verification Gap");
    expect(markup).toContain("MVP");
    expect(markup).toContain("75%");
    expect(markup).toContain("Complete product");
    expect(markup).toContain("Release rule trace");
    expect(markup).toContain("Workspaces");
    expect(markup).toContain("Conditions");
    expect(markup).toContain("Branches");
    expect(markup).toContain("Not scored");
    expect(markup).toContain("Configured");
    expect(markup).toContain("Dependency audit");
    expect(markup).toContain("Awaiting evidence");
    expect(markup).not.toContain("drawer-layer");
    expect(markup).not.toContain("drawer-scrim");
  });

  it("shows a loading state and withholds old quality projections during a branch refresh", () => {
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
          integration_state: "No unique commits",
          last_commit: "main-commit",
        },
        {
          name: "dev",
          role: "Integration",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
          last_commit: "dev-commit",
        },
      ],
      quality: makeQuality({
        ingestion_status: "Available",
        ingestion_message: "Existing main evidence",
        gates: [makeGate("build", "Build", "Failed", "Fresh")],
      }),
    });

    function Harness() {
      const [isRefreshing, setIsRefreshing] = useState(false);
      return (
        <RepositoryDetailSurface
          repository={repository}
          analytics={analyticsSnapshot}
          isRefreshing={isRefreshing}
          onBack={() => undefined}
          onOpenWorkspace={async () => undefined}
          onPrepareRepository={async () => undefined}
          onTargetBranchChange={async () => setIsRefreshing(true)}
          onLifecycleChange={async () => undefined}
          onCondition={() => undefined}
          onOpenReport={noopReport}
        />
      );
    }

    render(<Harness />);
    fireEvent.change(
      screen.getByRole("combobox", { name: "Target branch for pronto" }),
      { target: { value: "dev" } },
    );

    expect(screen.getByText("Refreshing evidence for dev…")).toBeTruthy();
    expect(
      screen.getByText(
        "Resolving the target head and checking existing evidence. A QR audit runs only when the target head changed or matching evidence is unavailable. Existing evidence is held until the check completes.",
      ),
    ).toBeTruthy();
    expect(screen.getByText("Ingesting dev evidence…")).toBeTruthy();
    expect(
      screen.getByText("Refreshing release remediation evidence…"),
    ).toBeTruthy();
    expect(screen.queryByText("Existing main evidence")).toBeNull();
    expect(
      screen.getByRole("combobox", { name: "Target branch for pronto" }),
    ).toHaveProperty("disabled", true);
  });

  it("refreshes target evidence without requiring a branch value change", () => {
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
          integration_state: "No unique commits",
          last_commit: "main-commit",
        },
      ],
    });
    const requestedBranches: string[] = [];

    render(
      <RepositoryDetailSurface
        repository={repository}
        analytics={analyticsSnapshot}
        isRefreshing={false}
        onBack={() => undefined}
        onOpenWorkspace={async () => undefined}
        onPrepareRepository={async () => undefined}
        onTargetBranchChange={async (branch) => {
          requestedBranches.push(branch);
        }}
        onLifecycleChange={async () => undefined}
        onCondition={() => undefined}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: "Refresh target evidence for pronto",
      }),
    );

    expect(requestedBranches).toEqual(["main"]);
  });

  it("makes missing Compass item details explicit for legacy snapshots", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 6,
        updated_at: "2026-07-28T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 79,
          scored_outcome_count: 4,
          covered_pillar_count: 3,
          total_pillar_count: 4,
          confidence: "high",
          confidence_percent: 100,
        },
        complete_product: {
          progress_percent: 75,
          scored_outcome_count: 5,
          covered_pillar_count: 4,
          total_pillar_count: 4,
          confidence: "high",
          confidence_percent: 100,
        },
        open_blockers: 2,
        open_drift: 1,
        open_blocker_items: [],
        open_drift_items: [],
        error: null,
      },
    });

    const markup = renderToStaticMarkup(
      <ProjectCompassDetail repository={repository} />,
    );

    expect(markup).toContain("Contract valid");
    expect(markup).not.toContain(">Ready<");
    expect(markup).toContain(
      "2 blocker descriptions are unavailable in this snapshot",
    );
    expect(markup).toContain(
      "1 drift description is unavailable in this snapshot",
    );
    expect(markup).toContain("Refresh the repository to load the details");
    expect(markup).toContain("Refresh the repository to load the detail");
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
