export interface ReleaseRuleConfig {
  name: string;
  operator: "AND" | "OR" | string;
  min_commits?: number;
  min_elapsed_days?: number;
  required_commit_types: string[];
  allow_first_release: boolean;
  required_quality_gates: QualityGateRequirement[];
}

export type QualityGateStatus =
  "Passed" | "Failed" | "Blocked" | "Not configured";
export type QualitySource = "CI" | "Local" | "QR";
export type QualityFreshness = "Fresh" | "Stale" | "Unknown" | "Conflicted";
export type QualityVerificationLevel =
  | "unknown"
  | "source_inferred"
  | "artifact_inspected"
  | "browser_rendered"
  | "deployment_verified";
export type QualityRequirementPolicy = "block" | "warn";

export type {
  QualityPortfolioSnapshot,
  QualitySnapshot,
} from "./qualityEvidence";

export interface QualityGateRequirement {
  gate_id: string;
  source: QualitySource;
  minimum_verification_level?: QualityVerificationLevel;
  policy?: QualityRequirementPolicy;
}

export interface QualityEvidence {
  id: string;
  source: QualitySource;
  status: QualityGateStatus;
  freshness: QualityFreshness;
  observed_at?: string;
  scanned_commit?: string;
  scanned_branch?: string;
  command?: string;
  source_label: string;
  report_path?: string;
  report_url?: string;
  report_kind?: string;
  detail: string;
  verification_level?: QualityVerificationLevel;
  target_kind?: string;
  target_url?: string;
  target_provider?: string;
  deployment_id?: string;
}

interface WebReadinessCheck {
  id: string;
  label: string;
  category: string;
  policy: QualityRequirementPolicy | string;
  status: string;
  verification_level: QualityVerificationLevel;
  detail: string;
  routes: string[];
}

interface WebReadinessTarget {
  kind: string;
  commit?: string;
  url?: string;
  provider?: string;
  deployment_id?: string;
  artifact_digest?: string;
}

export interface WebReadinessSnapshot {
  status:
    "Ready" | "Warnings" | "Blocked" | "Unknown" | "Not applicable" | string;
  applicability: string;
  applicability_reason?: string;
  freshness: QualityFreshness;
  observed_at?: string;
  scanned_commit?: string;
  scanned_branch?: string;
  report_path?: string;
  target: WebReadinessTarget;
  checks: WebReadinessCheck[];
  passed_count: number;
  failed_count: number;
  blocked_count: number;
  unknown_count: number;
  warning_count: number;
}

export interface QualityGate {
  id: string;
  label: string;
  status: QualityGateStatus;
  freshness: QualityFreshness;
  evidence: QualityEvidence[];
}

export interface QualityFindings {
  total: number;
  detector_findings_total?: number;
  category_counts?: Record<string, number>;
  actionable_category_counts?: Record<string, number>;
  actionable_total: number;
  detector_actionable_total?: number;
  reviewed_total: number;
  unreviewed_total: number;
  detector_unreviewed_total?: number;
  disposition_counts: Record<string, number>;
  stale_disposition_total: number;
  disposition_status: string;
  disposition_contract_path?: string;
  disposition_message?: string;
  severity_counts: Record<string, number>;
  high_severity_total: number;
  source?: QualitySource;
  observed_at?: string;
  scanned_commit?: string;
  scanned_branch?: string;
  freshness: QualityFreshness;
  report_path?: string;
  report_paths?: string[];
  enabled_detector_count?: number;
  enabled_rule_count?: number;
  producer_versions?: Record<string, string>;
  producer_source_shas?: Record<string, string>;
  ruleset_fingerprints?: Record<string, string>;
  configuration_fingerprints?: Record<string, string>;
  qr_version?: string;
  target_sha?: string;
  refresh_time?: string;
  delta_total?: number | null;
  refresh_required?: boolean;
  refresh_required_reason?: string;
  detector_status?: string;
}

export interface QualityMaturity {
  score?: number;
  score_display?: string;
  scored_dimension_count?: number;
  dimension_scores?: Record<string, number>;
  gaps?: Array<{
    dimension: string;
    status: string;
    score?: number;
    message: string;
  }>;
  quality_outcome?: QualityRepositoryOutcome;
  agent_usability?: AgentUsabilityMaturity;
  repository_maturity?: RepositoryMaturityModel;
  audit_id?: string;
  observed_at?: string;
  scanned_commit?: string;
  scanned_branch?: string;
  freshness: QualityFreshness;
  report_path?: string;
}

interface RepositoryMaturityPillar {
  id: string;
  label: string;
  weight: number;
  applicability: "applicable" | "unknown" | "not_applicable" | string;
  status: string;
  score?: number;
  dimension_scores: Record<string, number>;
  missing_capabilities: string[];
  not_applicable_capabilities?: string[];
  critical_dimensions: string[];
}

interface RepositoryMaturityModel {
  schema: string;
  score?: number;
  uncapped_score?: number;
  status: string;
  pillars: RepositoryMaturityPillar[];
  evidence: {
    applicable_pillar_count: number;
    assessed_pillar_count: number;
    applicable_dimension_count?: number;
    assessed_dimension_count?: number;
    applicable_weight: number;
    assessed_weight: number;
    evidence_coverage: number;
    fresh_evidence_coverage: number;
    unknown_applicability: string[];
    unmapped_dimensions: string[];
  };
  critical_cap: {
    applied: boolean;
    maximum_score?: number;
    reasons: string[];
  };
}

export interface QualityRepositoryOutcome {
  state: string;
  label: string;
  disposition?: string;
  next_step?: string;
}

interface AgentUsabilityLane {
  id: string;
  label: string;
  applicable: boolean;
  score?: number;
  status: string;
  message: string;
}

interface AgentUsabilityGrowthHealth {
  status: string;
  score?: number;
  message: string;
  document_count: number;
  agent_document_count: number;
  routed_agent_document_count: number;
  unrouted_agent_document_count: number;
  oversized_document_count: number;
  skill_count: number;
  family_count: number;
  largest_family_size: number;
  unclassified_skill_count: number;
  oversized_skill_count: number;
  tool_count: number;
  documented_tool_count: number;
  skill_covered_tool_count: number;
  behavior_declared_tool_count: number;
  behavior_verified_tool_count: number;
  inventory_truncated: boolean;
}

interface AgentUsabilityMaturity {
  schema: string;
  status: string;
  applicability?: "applicable" | "not_applicable";
  manifest_status: string;
  manifest_path: string;
  applicable_lane_count: number;
  covered_lane_count: number;
  lanes: AgentUsabilityLane[];
  growth_health: AgentUsabilityGrowthHealth;
}

export interface QualityReadiness {
  score?: number;
  score_display?: string;
  evidence_coverage_score?: number;
  evidence_coverage_score_display?: string;
  configuration_score?: number;
  configuration_score_display?: string;
  applicable_gate_ids: string[];
  configured_gate_ids: string[];
  unconfigured_gate_ids: string[];
  covered_gate_ids: string[];
  fresh_passing_gate_ids: string[];
  missing_gate_ids: string[];
  stale_gate_ids: string[];
  failed_gate_ids: string[];
  blocked_gate_ids: string[];
}

export interface InstalledRuntimeIssue {
  stage: string;
  status: string;
  message: string;
}

export interface InstalledRuntimeTargetSnapshot {
  id: string;
  label: string;
  status: string;
  source_revision?: string;
  build_revision?: string;
  process_id?: number;
  observed_at?: string;
  issues: InstalledRuntimeIssue[];
}

export interface InstalledRuntimeSnapshot {
  schema_version: string;
  applicability: string;
  status: string;
  summary: string;
  config_path?: string;
  targets: InstalledRuntimeTargetSnapshot[];
}
export interface ReleaseRecipeConfig {
  name: string;
  validation_commands: string[];
  release_commands: string[];
  generated_paths: string[];
  commit_message: string;
}

interface AiPayloadCategory {
  category: string;
  included: boolean;
  item_count: number;
  byte_count: number;
}

interface AiSourceReference {
  sha: string;
  subject: string;
  committed_at: string;
  category: string;
}

export interface AiPayloadPreview {
  repository_id: string;
  workspace_id: string;
  permission: string;
  provider: string;
  model?: string;
  status: string;
  reasons: string[];
  categories: AiPayloadCategory[];
  source_references: AiSourceReference[];
  payload_text: string;
  payload_bytes: number;
  uncommitted_included: boolean;
  request_performed: boolean;
  generated_at: string;
}
