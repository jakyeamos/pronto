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

interface ReleaseBoundaryArtifact {
  kind: string;
  path: string;
  sha256: string;
  size_bytes: number;
}

interface ReleaseBoundaryCheck {
  id: string;
  status: string;
  reason?: string;
}

interface ReleaseBoundarySnapshot {
  schema?: string;
  status: string;
  freshness: string;
  generated_at?: string;
  scanned_commit?: string;
  scanned_branch?: string;
  producer_version?: string;
  report_path?: string;
  matrix_path?: string;
  matrix_sha256?: string;
  artifacts: ReleaseBoundaryArtifact[];
  checks: ReleaseBoundaryCheck[];
  blocking_check_ids: string[];
  detail: string;
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
  category_counts?: Record<string, number>;
  actionable_category_counts?: Record<string, number>;
  actionable_total: number;
  reviewed_total: number;
  unreviewed_total: number;
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

interface PortfolioMaturityPillar {
  id: string;
  label: string;
  score?: number;
  assessed_repository_count: number;
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

interface EvidenceContractRepositoryStatus {
  contract_id: string;
  label: string;
  target_schema: string;
  observed_schema?: string;
  status: "current" | "audit_required" | "missing" | string;
  repository_id: string;
  repository_name: string;
  message: string;
}

interface EvidenceContractFleetCoverage {
  contract_id: string;
  label: string;
  target_schema: string;
  status: "current" | "audit_required" | string;
  repository_count: number;
  current_repository_count: number;
  legacy_repository_count: number;
  missing_repository_count: number;
  observed_schema_counts?: Record<string, number>;
  affected_repository_ids?: string[];
  message: string;
  next_safe_step: string;
}

interface MacControlRepositoryState {
  repository_id: string;
  repository_name: string;
  applicability: string;
  status: string;
  freshness: string;
  ideal_state: boolean;
  supported_task_count: number;
  measured_route_count: number;
  implementation_status?: string;
  implementation_criteria_passed_count?: number;
  implementation_criteria_total?: number;
  implementation_declaration_criteria_count?: number;
  implementation_evidence_level?: string;
  source_provenance_status?: string;
  source_provenance_dirty_paths?: string[];
  live_status?: string;
  live_task_count?: number;
  live_attempt_count?: number;
  live_success_count?: number;
  criteria?: Record<string, boolean>;
  failure_reasons?: string[];
  evidence_contract?: EvidenceContractRepositoryStatus;
  observed_at?: string;
  observed_commit?: string;
  report_path?: string;
}

interface MacControlPortfolioSnapshot {
  status: string;
  freshness: string;
  ideal_state: boolean;
  applicable_repository_count: number;
  not_applicable_repository_count: number;
  evaluated_repository_count: number;
  implementation_status?: string;
  implementation_score?: number;
  implementation_score_display?: string;
  implementation_criteria_passed_count?: number;
  implementation_criteria_total?: number;
  implementation_declaration_criteria_count?: number;
  live_status?: string;
  live_score?: number;
  live_score_display?: string;
  live_task_count?: number;
  measured_task_count?: number;
  live_attempt_count?: number;
  live_success_count?: number;
  repository_states?: MacControlRepositoryState[];
  failure_reasons?: string[];
  evidence_contract?: EvidenceContractFleetCoverage;
  observed_at?: string;
  report_path?: string;
  run_id?: string;
}

interface BehaviorAssuranceGap {
  kind: string;
  message: string;
  behavior_id?: string;
  scenario_id?: string;
}

interface BehaviorCoverageCounts {
  total: number;
  profiled: number;
  verified: number;
  stale: number;
  failed: number;
  blocked: number;
  unknown: number;
}

interface BehaviorScenarioCoverage {
  behavior_id: string;
  scenario_id: string;
  tier: number;
  profiled: boolean;
  categories: string[];
  risk?: string;
  side_effects?: string;
  status: "verified" | "stale" | "failed" | "blocked" | "unknown" | string;
  verification_level?: string;
  receipt_id?: string;
  freshness: string;
}

interface BehaviorCoverage extends BehaviorCoverageCounts {
  profile_status: string;
  per_tier: Record<string, BehaviorCoverageCounts>;
  per_edge_category: Record<string, BehaviorCoverageCounts>;
  category_gaps: Array<{ category: string; scenario_count: number }>;
  scenarios: BehaviorScenarioCoverage[];
  truncated: boolean;
}

interface BehaviorAssuranceRepositoryState {
  schema: string;
  applicability: string;
  state?: string;
  contract_status: string;
  contract_schema?: string;
  edge_profile_status?: string;
  result_status: string;
  freshness: string;
  release_ready: boolean;
  score?: number;
  contract_path: string;
  receipt_directory: string;
  contract_digest?: string;
  target_branch?: string;
  target_commit?: string;
  observed_at?: string;
  required_scenario_count: number;
  passed_scenario_count: number;
  accepted_defect_count: number;
  receipt_count: number;
  coverage?: BehaviorCoverage;
  gaps: BehaviorAssuranceGap[];
  detail?: string;
  next_step: string;
}

interface BehaviorAssurancePortfolioState {
  schema: string;
  status: string;
  repository_count: number;
  ready_repository_count: number;
  applicability_counts: Record<string, number>;
  result_status_counts: Record<string, number>;
  contract_schema_counts?: Record<string, number>;
  edge_profile_status_counts?: Record<string, number>;
  state_counts?: Record<string, number>;
  required_scenario_count: number;
  passed_scenario_count: number;
  gap_count: number;
  coverage?: BehaviorCoverageCounts;
}

export interface QualitySnapshot {
  gates: QualityGate[];
  findings: QualityFindings;
  maturity: QualityMaturity;
  target_fleet_audit_root?: string;
  ci_readiness: QualityReadiness;
  mac_control_ideal_state?: MacControlRepositoryState;
  behavior_assurance: BehaviorAssuranceRepositoryState;
  evidence_contracts?: EvidenceContractRepositoryStatus[];
  web_readiness?: WebReadinessSnapshot;
  release_boundary?: ReleaseBoundarySnapshot;
  last_ingested_at?: string;
  ingestion_status: string;
  ingestion_message?: string;
}

export interface QualityPortfolioSnapshot {
  audit_root?: string;
  latest_audit_id?: string;
  latest_audit_at?: string;
  latest_audit_path?: string;
  matched_repository_count: number;
  maturity_score?: number;
  maturity_score_display?: string;
  scored_dimension_count?: number;
  source_maturity_score?: number;
  source_maturity_score_display?: string;
  source_scored_dimension_count?: number;
  maturity_pillars?: PortfolioMaturityPillar[];
  maturity_evidence_coverage?: number;
  maturity_fresh_evidence_coverage?: number;
  maturity_provisional_repository_count?: number;
  maturity_capped_repository_count?: number;
  audit_status: string;
  ci_readiness_score?: number;
  ci_readiness_score_display?: string;
  ci_evidence_coverage_score?: number;
  ci_evidence_coverage_score_display?: string;
  ci_configuration_score?: number;
  ci_configuration_score_display?: string;
  ci_readiness_full_repository_count?: number;
  ci_readiness_repository_count?: number;
  ci_readiness_unscored_repository_count?: number;
  ci_readiness_open_gate_counts?: Record<string, number>;
  ci_evidence_fresh_passing_gate_count?: number;
  ci_evidence_ideal_gate_count?: number;
  ci_configuration_configured_gate_count?: number;
  ci_configuration_ideal_gate_count?: number;
  ci_configuration_full_repository_count?: number;
  ci_configuration_repository_count?: number;
  ci_configuration_unscored_repository_count?: number;
  feed_schema?: string;
  provenance_hash?: string;
  quality_outcome_counts?: Record<string, number>;
  quality_outcome_taxonomy?: Record<
    string,
    { label: string; meaning: string; next_step?: string }
  >;
  mac_control_ideal_state?: MacControlPortfolioSnapshot;
  behavior_assurance?: BehaviorAssurancePortfolioState;
  evidence_contracts?: EvidenceContractFleetCoverage[];
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
