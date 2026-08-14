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
  remediation_open_action_count?: number;
  remediation_in_progress_action_count?: number;
  remediation_blocked_action_count?: number;
  remediation_deferred_action_count?: number;
  remediation_verified_action_count?: number;
  remediation_progress_percent?: number | null;
  quality_freshness?: string;
  /** Governed v2 observations. Legacy fixed fields remain for v1 history adaptation. */
  metrics?: Record<string, number | null>;
}

export type AnalyticsChartType =
  | "line"
  | "bar"
  | "diverging-bar"
  | "scatter"
  | "stacked-bar"
  | "heatmap"
  | "table";

export interface MetricDefinition {
  id: string;
  label: string;
  description: string;
  unit: string;
  denominator: string;
  scope: "portfolio" | "repository";
  time_semantics: "point-in-time" | "trailing-window";
  window_days?: number | null;
  aggregation: "sum" | "average" | "count" | "latest";
  polarity: "higher-is-better" | "lower-is-better" | "neutral";
  source: string;
  freshness: string;
  allowed_visualizations: AnalyticsChartType[];
}

export interface AnalyticsFinding {
  id: string;
  kind: "change" | "outlier" | "stale" | "conflict" | "coverage-gap";
  severity: "info" | "attention" | "high";
  title: string;
  detail: string;
  metric_ids: string[];
  repository_id?: string | null;
  observed_at?: string | null;
}

export interface AnalyticsWidgetConfig {
  id: string;
  title: string;
  metric_ids: string[];
  chart_type: AnalyticsChartType;
  grouping: "portfolio" | "repository";
  width: 1 | 2;
  height: 1 | 2;
  order: number;
}

export interface AnalyticsViewFilters {
  range_days: number;
  repository_ids: string[];
  group_ids: string[];
  product_ids: string[];
  freshness: "all" | "fresh" | "stale" | "conflicted" | "unavailable";
}

export interface AnalyticsView {
  schema_version: "pronto-analytics-view/v1";
  id: string;
  name: string;
  builtin: boolean;
  is_default: boolean;
  filters: AnalyticsViewFilters;
  widgets: AnalyticsWidgetConfig[];
  created_at: string;
  updated_at: string;
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
  metric_catalog?: MetricDefinition[];
  findings?: AnalyticsFinding[];
  views?: AnalyticsView[];
  default_view_id?: string;
}

export interface SkillUsage {
  state: "observed" | "unavailable";
  recent_count: number;
  all_time_count: number;
  by_provider: Record<string, number>;
  last_seen_at?: string;
  telemetry_source: string;
  reason: string;
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

export interface SkillFindingClass {
  id: string;
  label: string;
  state: string;
  evidence: string;
}

export interface SkillBackfillPhase {
  id: string;
  state: string;
  evidence: string;
}

export interface SkillBackfillCapability {
  mode: string;
  phases: SkillBackfillPhase[];
  safety: string;
}

export interface SkillQualityRunnerCoverage {
  rule_count: number;
  finding_count: number;
  statuses: string[];
}

export interface SkillQualityRunnerRepresentation {
  status: string;
  adapter: string;
  finding_categories: string[];
  coverage: SkillQualityRunnerCoverage;
  evidence: string[];
  gaps: string[];
}

export interface SkillFindingCapability {
  finding_expectation: string;
  finding_expectation_reason: string;
  finding_classes: SkillFindingClass[];
  backfill: SkillBackfillCapability;
  quality_runner: SkillQualityRunnerRepresentation;
  gaps: string[];
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
  finding_capability?: SkillFindingCapability | null;
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
