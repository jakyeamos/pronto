export type PromotionDecision =
  "defer" | "reject" | "public" | "private" | "both";

export interface PromotionCandidate {
  candidate_id: string;
  title: string;
  asset_kind: string;
  improvement_key?: string | null;
  source_refs: string[];
  evidence_refs: string[];
  quantification?: Record<string, unknown> | null;
  portability: string;
  status: string;
  review_status: string;
  maturity?: string | null;
  package_status: string;
  candidate_kind: "draft" | "complete" | string;
  candidate_source: string;
  candidate_artifact: string;
  candidate_provenance_hash: string;
  decision?: PromotionDecision | null;
  decision_at?: string | null;
  decision_reason?: string | null;
  decision_reviewer?: string | null;
  decision_artifact?: string | null;
  next_action: string;
  jas_projection_status?: "ready" | "missing" | string;
  jas_projection_visibility?: string | null;
  jas_admission?: PromotionJasAdmission | null;
}

interface PromotionJasAdmission {
  schema_version?: string;
  status: string;
  candidate_id?: string | null;
  decision?: PromotionDecision | string | null;
  mutated: boolean;
  target?: string | null;
  install_status?: string | null;
  message?: string | null;
  reason?: string | null;
  receipt_status?: string | null;
  receipt_message?: string | null;
}

interface PromotionCounts {
  total: number;
  pending: number;
  deferred: number;
  rejected: number;
  accepted: number;
  complete: number;
  drafts: number;
}

interface PromotionCoverageSource {
  source_id: string;
  category: string;
  path?: string | null;
  status:
    "assessed" | "partial" | "unassessed" | "excluded" | "blocked" | string;
  scan_mode: string;
  match_policy: string;
  files_seen: number;
  bytes_seen: number;
  file_kinds: Record<string, number>;
  repository_count: number;
  unknown_reason?: string | null;
  exclusion_reason?: string | null;
  notes?: string | null;
}

export interface PromotionCoverage {
  schema_version: string;
  visibility: string;
  generated_at: string;
  source_owner: string;
  status: "pass" | "blocked" | string;
  coverage_status: "assessed" | "partial" | "unassessed" | "blocked" | string;
  source_manifest: PromotionCoverageSource[];
  assessed_sources: number;
  partial_sources: number;
  unassessed_sources: number;
  excluded_sources: number;
  blocked_sources: number;
  unknown_sources: string[];
  excluded_source_labels: string[];
  files_seen: number;
  bytes_seen: number;
  errors: unknown[];
  raw_source_bytes_copied: boolean;
  jas_mutation: boolean;
}

export interface PromotionDiscoverySummary {
  schema_version: string;
  generated_at: string;
  status: string;
  observations_seen: number;
  observations_inserted: number;
  duplicates: number;
  asset_observation_documents: number;
  asset_observations_inserted: number;
  asset_observation_duplicates: number;
  candidate_drafts: number;
  asset_roots: string[];
  raw_source_bytes_copied: boolean;
  jas_mutation: boolean;
}

export interface PromotionFunnel {
  schema_version: string;
  status: string;
  evaluation_candidate_drafts: number;
  ready_behavior_identity_clusters: number;
  selected_forward_test_work_items: number;
  promotion_packets: number;
  forward_test_pass: number;
  forward_test_failed: number;
  forward_test_blocked: number;
  packets_blocked: number;
  packets_failed: number;
  quantification_pending: number;
  promotion_candidates: number;
  source_triage_artifact?: string | null;
  manual_review_required: boolean;
  jas_mutation: boolean;
}

export interface PromotionInbox {
  schema_version: string;
  visibility: string;
  generated_at: string;
  source_root: string;
  candidates: PromotionCandidate[];
  counts: PromotionCounts;
  coverage?: PromotionCoverage | null;
  discovery?: PromotionDiscoverySummary | null;
  funnel?: PromotionFunnel | null;
  errors: unknown[];
  manual_review_required: boolean;
  jas_mutation: boolean;
  status: "pass" | "blocked" | "unavailable" | string;
  provenance_hash: string;
  message?: string | null;
  jas_admission?: PromotionJasAdmission | null;
}
