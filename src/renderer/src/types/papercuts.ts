export type PapercutSource = "manual" | "design-friction" | string;
export type PapercutPriority = "P0" | "P1" | "P2" | "P3";
export type PapercutStatus = "open" | "in_progress" | "deferred" | "resolved";
export type PapercutSignalKind =
  | "dissatisfaction"
  | "correction"
  | "failure_report"
  | "failed_verification"
  | "repeated_failure"
  | "agent_suggestion"
  | "capability_gap"
  | "manual_handoff"
  | "legacy_manual";
export type PapercutTargetKind =
  | "agent_answer"
  | "workflow"
  | "tool"
  | "repository"
  | "artifact"
  | "user_preference_model"
  | "other";
export type MultiplierProposalStatus =
  "draft" | "accepted" | "deferred" | "rejected";

export interface PapercutObservationInput {
  event_key: string;
  scope_id: string;
  scope_kind: "repository" | "project" | "global";
  domain: string;
  signal_kind: PapercutSignalKind;
  target_kind: PapercutTargetKind;
  summary: string;
  excerpt?: string | null;
  source: string;
  evidence_refs: string[];
  phenomenon_key: string;
  failure_mode: string;
  priority: PapercutPriority;
  urgent: boolean;
  verified: boolean;
  observed_at?: string | null;
}

export interface PapercutObservation extends Omit<
  PapercutObservationInput,
  "observed_at"
> {
  id: string;
  excerpt_hash: string;
  excerpt_expires_at?: string | null;
  observed_at: string;
}

export interface PapercutPattern {
  id: string;
  fingerprint: string;
  fingerprint_version: string;
  scope_kind: "local" | "cross_scope" | string;
  scope_id?: string | null;
  title: string;
  detail: string;
  domain: string;
  target_kind: PapercutTargetKind | string;
  phenomenon_key: string;
  failure_mode: string;
  surface: string;
  source: string;
  evidence_refs: string[];
  impact: string;
  priority: PapercutPriority | string;
  status: PapercutStatus | string;
  next_action: string;
  evidence_tier: string;
  occurrence_count: number;
  scope_count: number;
  first_observed_at: string;
  last_observed_at: string;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
}

export interface MultiplierProposalInput {
  pattern_ids: string[];
  title: string;
  hypothesis: string;
  root_cause: string;
  multiplier: string;
  evidence_tier: "single" | "local_recurring" | "cross_scope" | "urgent";
}

export interface MultiplierProposal extends MultiplierProposalInput {
  id: string;
  status: MultiplierProposalStatus;
  created_at: string;
  updated_at: string;
  reviewed_at?: string | null;
}

export interface PapercutDigest {
  id: string;
  week_start: string;
  week_end: string;
  generated_at: string;
  observation_count: number;
  local_pattern_count: number;
  cross_scope_pattern_count: number;
  draft_proposal_count: number;
  top_patterns: PapercutPattern[];
}

export interface PapercutCaptureHealth {
  status: string;
  database_writable: boolean;
  consecutive_failures: number;
  spooled_events: number;
  oldest_spool_at?: string | null;
  last_success_at?: string | null;
  warning?: string | null;
  last_error?: PapercutCaptureDiagnostic | null;
  excerpt_retention_days: number;
}

export interface PapercutCaptureDiagnostic {
  error_code: string;
  failure_kind: string;
  stage: string;
  message: string;
  operation: string;
  observed_at: string;
  retryable: boolean;
  recovery_command: string;
  attempt: number;
  timeout_seconds?: number | null;
  exit_code?: number | null;
}

/** Compatibility projection retained for the original manual backlog UI. */
export interface Papercut {
  id: string;
  title: string;
  detail: string;
  family: "design-audit" | string;
  surface: string;
  source: PapercutSource;
  evidence_refs: string[];
  impact: string;
  priority: PapercutPriority | string;
  status: PapercutStatus | string;
  next_action: string;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
}

export interface PapercutCounts {
  total: number;
  open: number;
  in_progress: number;
  deferred: number;
  resolved: number;
  observations: number;
  local_patterns: number;
  cross_scope_patterns: number;
  draft_proposals: number;
}

export interface PapercutBacklog {
  schema_version: "pronto-papercuts/v2" | string;
  family: "design-audit" | string;
  generated_at: string;
  papercuts: Papercut[];
  counts: PapercutCounts;
  observations: PapercutObservation[];
  patterns: PapercutPattern[];
  proposals: MultiplierProposal[];
  digests: PapercutDigest[];
  health: PapercutCaptureHealth;
}

export interface CreatePapercutInput {
  title: string;
  detail: string;
  surface: string;
  source: PapercutSource;
  priority: PapercutPriority;
  evidenceRefs: string[];
  impact: string;
  nextAction: string;
}
