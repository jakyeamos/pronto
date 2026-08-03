export interface AnalyticsMetricSample {
  observed_at: string;
  repository_count: number;
  workspace_count: number;
  branch_count: number;
  active_condition_count: number;
  dirty_workspace_count: number;
  unsynced_workspace_count: number;
  active_workspace_count: number;
  interrupted_workspace_count: number;
  idle_workspace_count: number;
  unknown_workspace_count: number;
  ahead_commit_count: number;
  behind_commit_count: number;
  commits_last_30_days?: number | null;
  ci_readiness_score?: number | null;
  maturity_score?: number | null;
  findings_total?: number | null;
  high_severity_findings?: number | null;
  ci_readiness_scored_repository_count: number;
  maturity_scored_repository_count: number;
  findings_repository_count: number;
  release_rule_repository_count: number;
  release_ready_repository_count: number;
  quality_freshness?: string;
}

export interface AnalyticsRepositorySeries {
  repository_id: string;
  name: string;
  samples: AnalyticsMetricSample[];
}

export interface AnalyticsSnapshot {
  schema_version: string;
  generated_at: string;
  source: string;
  freshness: string;
  range_days: number;
  retention_days: number;
  history_available_from?: string;
  portfolio_samples: AnalyticsMetricSample[];
  repositories: AnalyticsRepositorySeries[];
}

export interface SkillUsage {
  recent_count: number;
  all_time_count: number;
  by_provider: Record<string, number>;
  last_seen_at?: string;
  telemetry_source: string;
}

export interface SkillProviderState {
  state: string;
  reason: string;
  source_path?: string;
}

export interface SkillSource {
  path: string;
  root: string;
  provenance: string;
  sha256: string;
  hosted_in_jakye_agent_setup: boolean;
}

export interface SkillRecord {
  id: string;
  name: string;
  description: string;
  category: string;
  family: string;
  lifecycle: string;
  hosted_in_jakye_agent_setup: boolean;
  sources: SkillSource[];
  providers: Record<string, SkillProviderState>;
  parity_score?: number | null;
  parity_evidence: string[];
  usage: SkillUsage;
}

export interface SkillsSnapshot {
  schema_version: string;
  generated_at: string;
  refreshed_at?: string;
  freshness: string;
  source: string;
  recent_days: number;
  roots: string[];
  skills: SkillRecord[];
  telemetry_gap: string;
}
