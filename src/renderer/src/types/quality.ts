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

export interface QualityGateRequirement {
  gate_id: string;
  source: QualitySource;
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
  audit_id?: string;
  observed_at?: string;
  freshness: QualityFreshness;
  report_path?: string;
}

export interface QualityReadiness {
  score?: number;
  score_display?: string;
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

export interface QualitySnapshot {
  gates: QualityGate[];
  findings: QualityFindings;
  maturity: QualityMaturity;
  ci_readiness: QualityReadiness;
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
  audit_status: string;
  ci_readiness_score?: number;
  ci_readiness_score_display?: string;
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
