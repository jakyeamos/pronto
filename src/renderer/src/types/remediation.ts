interface RemediationEvidence {
  source: string;
  label: string;
  status: string;
  freshness: string;
  observed_at?: string | null;
  report_path?: string | null;
  detail: string;
}

export type RemediationActionStatus =
  "open" | "in_progress" | "blocked" | "deferred" | "verified";

export interface RemediationAction {
  id: string;
  stable_key: string;
  repository_id: string;
  domain: string;
  title: string;
  summary: string;
  severity: string;
  priority: string;
  weight: number;
  status: RemediationActionStatus;
  acceptance_criteria: string[];
  evidence: RemediationEvidence[];
  related_finding_ids: string[];
  source_run_id?: string | null;
  updated_at: string;
  completed_at?: string | null;
  notes?: string | null;
}

interface RemediationProgress {
  verified_weight: number;
  total_weight: number;
  deferred_weight: number;
  percentage: number;
}

interface RemediationTrack {
  domain: string;
  label: string;
  status: string;
  action_ids: string[];
  verified_weight: number;
  total_weight: number;
}

interface RemediationCoverage {
  surface: string;
  label: string;
  status: string;
  detail: string;
  action_ids: string[];
}

interface RemediationMaturityPolicy {
  minimum_closure_score: number;
  ideal_score: number;
  scoring_owner: string;
  improvement_rule: string;
  integrity_rule: string;
}

interface RemediationGoalProfile {
  schema_version: string;
  target_state: string;
  label: string;
  source: string;
  confidence: string;
  reason: string;
  contract_path: string;
  required_gate_ids: string[];
  optional_gate_ids: string[];
  evidence_max_age_days: number;
  closure_criteria: string[];
  maturity_policy?: RemediationMaturityPolicy | null;
  error?: string | null;
}

interface RemediationPlan {
  schema_version: string;
  id: string;
  repository_id: string;
  repository_name: string;
  repository_path: string;
  generated_at: string;
  source_refresh_id?: string | null;
  goal: RemediationGoalProfile;
  current_stage: string;
  status: string;
  progress: RemediationProgress;
  coverage: RemediationCoverage[];
  tracks: RemediationTrack[];
  actions: RemediationAction[];
}

interface RemediationExclusion {
  repository_id: string;
  repository_name: string;
  repository_path: string;
  reason: string;
}

interface RemediationClosure {
  id: string;
  repository_id: string;
  repository_name: string;
  repository_path: string;
  plan_id: string;
  target_state: string;
  goal_source: string;
  maturity_policy?: RemediationMaturityPolicy | null;
  closed_at: string;
  source_refresh_id?: string | null;
  disposition: string;
  summary: string;
  resolved_action_count: number;
  verified_action_count: number;
  deferred_action_count: number;
  last_evidence_at?: string | null;
}

interface RemediationRefreshStep {
  id: string;
  label: string;
  status: string;
  started_at?: string | null;
  completed_at?: string | null;
  detail: string;
  evidence_path?: string | null;
}

export interface RemediationRun {
  schema_version: string;
  id: string;
  generated_at: string;
  source_refresh_id?: string | null;
  status: string;
  message?: string | null;
  eligible_repository_ids: string[];
  eligible_repository_paths: string[];
  refresh_steps: RemediationRefreshStep[];
  excluded_repositories: RemediationExclusion[];
  closures: RemediationClosure[];
  plans: RemediationPlan[];
}

export interface RemediationExport {
  run_id: string;
  output_path: string;
  files: string[];
}
