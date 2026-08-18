import type {
  InstalledRuntimeSnapshot,
  QualityFindings,
  QualityFreshness,
  QualityGate,
  QualityMaturity,
  QualityReadiness,
  WebReadinessSnapshot,
} from "./quality";

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

export interface MaturityCheckpointSnapshot {
  status: string;
  publication_status: string;
  quality_status: string;
  freshness: QualityFreshness;
  checkpoint_id?: string;
  observed_at?: string;
  qr_audit_id?: string;
  mac_control_audit_id?: string;
  path?: string;
  reason?: string;
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
  installed_runtime?: InstalledRuntimeSnapshot;
  last_ingested_at?: string;
  ingestion_status: string;
  ingestion_message?: string;
}

export interface QualityMeasurementConfidence {
  level: "low" | "medium" | "high";
  basis: string[];
  limitations: string[];
  population_status: string;
  expected_repository_count: number;
  observed_repository_count: number;
  excluded_repository_count: number;
  unresolved_measurement_gap_count: number;
  deterministic_replay: boolean;
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
  measurement_confidence?: QualityMeasurementConfidence;
  source_maturity_score?: number;
  source_maturity_score_display?: string;
  source_scored_dimension_count?: number;
  maturity_pillars?: Array<{
    id: string;
    label: string;
    score?: number;
    assessed_repository_count: number;
  }>;
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
  ci_profile_repository_contract_count?: number;
  ci_profile_compatibility_count?: number;
  ci_profile_invalid_count?: number;
  ci_profile_unavailable_count?: number;
  feed_schema?: string;
  provenance_hash?: string;
  quality_outcome_counts?: Record<string, number>;
  quality_outcome_taxonomy?: Record<
    string,
    { label: string; meaning: string; next_step?: string }
  >;
  mac_control_ideal_state?: MacControlPortfolioSnapshot;
  maturity_checkpoint?: MaturityCheckpointSnapshot;
  behavior_assurance?: BehaviorAssurancePortfolioState;
  evidence_contracts?: EvidenceContractFleetCoverage[];
}
