import type {
  AiPayloadPreview,
  AnalyticsSnapshot,
  Condition,
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
  RepositoryPreparation,
  RepositorySnapshot,
  WorkspaceSummary,
} from "../types";
export type {
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

export { PreparationDrawer } from "./PreparationDrawer";
export { qualityGateChoices } from "./ReleaseRuleEditor";
export { CommandCenterSurface } from "./CommandCenterSurface";
export { AppSidebar } from "./AppSidebar";
export { AttentionQueue, RepositoryRow } from "./PortfolioComponents";
export {
  QualityAttentionList,
  QualityEvidenceList,
  QualityFindingsSummary,
  QualityOutcomeSummary,
  qualityAttentionItems,
} from "./QualityComponents";
export { QualityGatesSurface } from "./QualityGatesSurface";
export { RepositoryDetailSurface } from "./Drawers";
export { PortfolioCollectionsSurface } from "./PortfolioCollectionsSurface";
export { navItems } from "../navigation";
export { ProjectCompassDetail } from "./ProjectCompassDetail";
export { WebReadinessSummary } from "./WebReadinessSummary";

export const canonicalGateDefinitions = [
  ["build", "Build"],
  ["runtime_smoke", "Smoke"],
  ["tests", "Tests"],
  ["lint", "Lint"],
  ["formatter", "Formatter"],
  ["typecheck", "Typecheck"],
  ["dead_code", "Dead-code"],
  ["secrets_scan", "Secrets scan"],
] as const;

export const workspace: WorkspaceSummary = {
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

export function makeGate(
  id: string,
  label: string,
  status: QualityGateStatus = "Not configured",
  freshness: QualityFreshness = "Unknown",
  evidence: QualityEvidence[] = [],
): QualityGate {
  return { id, label, status, freshness, evidence };
}

export function makeEvidence(
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

export function makeFindings(
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

export function makeMaturity(
  overrides: Partial<QualityMaturity> = {},
): QualityMaturity {
  return {
    freshness: "Unknown",
    scanned_commit: "abcdef1234567890",
    scanned_branch: "main",
    ...overrides,
  };
}

export function makeReadiness(
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

export function makeQuality(
  overrides: Partial<QualitySnapshot> = {},
): QualitySnapshot {
  return {
    gates: canonicalGateDefinitions.map(([id, label]) => makeGate(id, label)),
    findings: makeFindings(),
    maturity: makeMaturity(),
    foundation_readiness: {
      schema: "pronto-foundation-readiness/v1",
      label: "Modernization readiness",
      disposition: "unknown",
      confidence: "low",
      freshness: "Unknown",
      advisory_only: true,
      execution_authority: false,
      blocks_urgent_fixes: false,
      summary: "Repository modernization readiness is unknown.",
      reasons: [],
      unknowns: ["repository_maturity_evidence"],
      next_step: "Refresh repository maturity evidence.",
    },
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

export function makeRepository(
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

export function makePortfolio(
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

export const noop = async (): Promise<void> => undefined;
export const noopRepository = (_repository: RepositorySnapshot): void =>
  undefined;
export const noopReport = (_reportPath: string): void => undefined;
export const noopCondition = (
  _repository: RepositorySnapshot,
  _condition: Condition,
): void => undefined;

export const analyticsSnapshot: AnalyticsSnapshot = {
  schema_version: "pronto-analytics/v1",
  generated_at: "2026-07-26T11:00:00Z",
  source: "Local refresh snapshots",
  freshness: "Unavailable until the first local refresh",
  range_days: 30,
  retention_days: 90,
  portfolio_samples: [],
  repositories: [],
};

export function preparation(): RepositoryPreparation {
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
      recommendation: {
        disposition: "review_required",
        label: "Review v1.0.1 (patch)",
        suggested_bump: "patch",
        suggested_version: "v1.0.1",
        basis: "1 commit since last published tag v1.0.0",
        reasons: ["The configured release threshold has not passed."],
        advisory: true,
      },
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

export function aiPreview(): AiPayloadPreview {
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
