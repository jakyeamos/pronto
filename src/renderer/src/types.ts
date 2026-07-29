export interface RootConfig {
  id: string;
  path: string;
  label: string;
  ignore_patterns: string[];
  refresh_policy: "Manual" | "On open" | "Periodic" | string;
  background_monitoring: boolean;
  registered_at: string;
}

export interface ProductConfig {
  id: string;
  name: string;
  repository_ids: string[];
  release_mode: string;
  created_at: string;
  updated_at: string;
}

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

export interface GroupConfig {
  id: string;
  name: string;
  repository_ids: string[];
  created_at: string;
  updated_at: string;
}

export interface ProviderIdentity {
  id: string;
  provider: string;
  login: string;
  display_name?: string;
  organizations: string[];
  credential_state: string;
  updated_at: string;
}

export interface RemoteRepositorySnapshot {
  id: string;
  provider: string;
  full_name: string;
  name: string;
  owner: string;
  html_url: string;
  default_branch?: string;
  archived: boolean;
  locality: string;
  identity_id: string;
  last_refreshed_at: string;
  pull_requests: PullRequestSnapshot[];
  releases: ReleaseSnapshot[];
  ci_checks: CheckSnapshot[];
  ci_branch?: string;
  ci_commit?: string;
}

interface CheckSnapshot {
  context: string;
  state: string;
  required: boolean;
  conclusion?: string;
  last_refreshed_at: string;
  html_url?: string;
  head_sha?: string;
}

interface PullRequestSnapshot {
  id: string;
  provider: string;
  repository_id: string;
  number: number;
  html_url: string;
  title: string;
  head_branch: string;
  base_branch: string;
  state: string;
  draft: boolean;
  checks_state: string;
  reviews_state: string;
  mergeability: string;
  checks: CheckSnapshot[];
  last_refreshed_at: string;
  head_commit?: string;
}

interface ReleaseSnapshot {
  id: string;
  provider: string;
  repository_id: string;
  tag: string;
  name: string;
  target_commit?: string;
  published_at?: string;
  draft: boolean;
  prerelease: boolean;
  last_refreshed_at: string;
}

export interface ProviderStatus {
  provider: string;
  state: string;
  message: string;
  last_refresh_at?: string;
  identity_count: number;
  repository_count: number;
}

interface EvidenceItem {
  label: string;
  value: string;
  source: string;
  observed_at: string;
}

export interface Condition {
  id: string;
  kind: string;
  title: string;
  summary: string;
  priority: number;
  status: "Active" | "Expected" | "Resolved" | "Superseded";
  fingerprint: string;
  rule: string;
  evidence: EvidenceItem[];
  missing: string[];
  confidence?: string;
  freshness?: string;
}

interface AgentManifest {
  task_id?: string;
  title?: string;
  target_branch?: string;
  agent_type?: string;
  start_time?: string;
  status?: string;
  source_session_id?: string;
}

interface ActivitySignal {
  source: string;
  summary: string;
  confidence: string;
  observed_at: string;
  process_name?: string;
  process_id?: number;
  started_at?: string;
  working_directory?: string;
}

interface WorkspaceActivity {
  state: string;
  confidence: string;
  signals: ActivitySignal[];
  manifest?: AgentManifest;
}

export type ExternalTool =
  "file_browser" | "terminal" | "editor" | "git_client";

export interface WorkspaceSummary {
  id: string;
  path: string;
  is_primary: boolean;
  branch: string;
  dirty: boolean;
  added: number;
  removed: number;
  line_totals_partial: boolean;
  sync_state: string;
  remote_freshness: string;
  ahead: number;
  behind: number;
  upstream?: string;
  operation?: string;
  last_commit?: string;
  last_commit_at?: string;
  last_activity_at?: string;
  integration_state: string;
  target_branch?: string;
  target_confidence: string;
  role: string;
  role_confidence: string;
  activity: WorkspaceActivity;
}

interface BranchSummary {
  name: string;
  role: string;
  role_confidence: string;
  target_branch?: string;
  target_confidence: string;
  ahead: number;
  behind: number;
  integration_state: string;
  workspace_id?: string;
  last_commit?: string;
  last_commit_at?: string;
}

export interface RepositorySnapshot {
  id: string;
  name: string;
  path: string;
  locality: string;
  lifecycle: string;
  lifecycle_candidate: string;
  remote_url?: string;
  provider_state: string;
  branch: string;
  default_branch?: string;
  workspace: WorkspaceSummary;
  workspaces: WorkspaceSummary[];
  branches: BranchSummary[];
  submodules: SubmoduleSummary[];
  pull_requests: PullRequestSnapshot[];
  releases: ReleaseSnapshot[];
  quality: QualitySnapshot;
  project_compass: ProjectCompassSummary;
  release_rule?: ReleaseRuleConfig;
  release_recipe?: ReleaseRecipeConfig;
  confirmed_release_version?: string;
  ai_permission: string;
  conditions: Condition[];
  last_scan_at: string;
  last_fetch_at?: string;
  last_activity_at?: string;
}

interface ProjectCompassTargetSummary {
  progress_percent: number | null;
  confidence: string;
  confidence_percent: number;
}

interface ProjectCompassSummary {
  status: "Ready" | "Missing" | "Invalid";
  contract_path: string;
  revision: number | null;
  updated_at: string | null;
  project_name: string | null;
  identity: string | null;
  audience: string | null;
  mvp: ProjectCompassTargetSummary;
  complete_product: ProjectCompassTargetSummary;
  open_blockers: number;
  open_drift: number;
  error: string | null;
}

interface PullRequestPreparation {
  repository_id: string;
  workspace_id: string;
  head_branch: string;
  base_branch?: string;
  commit_count: number;
  dirty: boolean;
  ahead: number;
  behind: number;
  upstream?: string;
  provider_state: string;
  checks_state: string;
  reviews_state: string;
  mergeability: string;
  status: string;
  reasons: string[];
  evidence: EvidenceItem[];
  existing_pull_request?: PullRequestSnapshot;
}

interface ReleaseCommitSummary {
  sha: string;
  subject: string;
  category: string;
  bump?: string;
  committed_at: string;
}

interface ReleaseNoteSection {
  category: string;
  commits: ReleaseCommitSummary[];
}

interface ReleaseRuleTrace {
  label: string;
  status: string;
  value: string;
  source: string;
}

interface ReleasePreparation {
  repository_id: string;
  target_branch?: string;
  baseline_status: string;
  baseline?: ReleaseSnapshot;
  commits_since_baseline: ReleaseCommitSummary[];
  rule_status: string;
  threshold_label?: string;
  rule_trace: ReleaseRuleTrace[];
  candidate_bump?: string;
  candidate_version?: string;
  version_status: string;
  notes: ReleaseNoteSection[];
  status: string;
  reasons: string[];
  evidence: EvidenceItem[];
}

interface ReleaseRecipeStep {
  order: number;
  label: string;
  status: string;
  detail: string;
}

export interface ReleaseRecipePreview {
  repository_id: string;
  recipe_name: string;
  candidate_version?: string;
  version_status: string;
  status: string;
  reasons: string[];
  steps: ReleaseRecipeStep[];
  actions_performed: boolean;
  generated_at: string;
}

export interface RepositoryPreparation {
  repository_id: string;
  pull_request: PullRequestPreparation;
  release: ReleasePreparation;
  recipe: ReleaseRecipePreview;
  generated_at: string;
}

interface SubmoduleSummary {
  path: string;
  commit?: string;
  status: string;
}

export interface EventRecord {
  id: string;
  repository_id: string;
  kind: string;
  summary: string;
  fingerprint: string;
  created_at: string;
}

export interface ActionAudit {
  id: string;
  action: string;
  target_ids: string[];
  risk: string;
  status: string;
  summary: string;
  created_at: string;
  completed_at?: string | null;
}

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

export interface PortfolioSnapshot {
  roots: RootConfig[];
  repositories: RepositorySnapshot[];
  products: ProductConfig[];
  groups: GroupConfig[];
  events: EventRecord[];
  action_audits: ActionAudit[];
  provider_identities: ProviderIdentity[];
  remote_repositories: RemoteRepositorySnapshot[];
  provider_status: ProviderStatus;
  quality: QualityPortfolioSnapshot;
  remediation: RemediationRun;
  retention_days: number;
  generated_at: string;
  storage_path: string;
}

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
