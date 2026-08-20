#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshBatchRepositoryResult {
    pub repository_id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub scan_order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshBatchReport {
    pub schema_version: String,
    pub generated_at: String,
    pub status: String,
    pub scope: String,
    pub parallelism: usize,
    pub repository_count: usize,
    pub conflict_retries: usize,
    pub scan_phase: String,
    pub merge_phase: String,
    pub repositories: Vec<RefreshBatchRepositoryResult>,
    pub snapshot: PortfolioSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsMetricSample {
    pub observed_at: String,
    pub repository_count: u64,
    pub workspace_count: u64,
    pub branch_count: u64,
    pub active_condition_count: u64,
    pub dirty_workspace_count: u64,
    pub unsynced_workspace_count: u64,
    pub active_workspace_count: u64,
    pub interrupted_workspace_count: u64,
    pub idle_workspace_count: u64,
    pub unknown_workspace_count: u64,
    pub ahead_commit_count: u64,
    pub behind_commit_count: u64,
    pub commits_last_30_days: Option<u64>,
    pub ci_readiness_score: Option<f64>,
    pub maturity_score: Option<f64>,
    pub findings_total: Option<u64>,
    pub high_severity_findings: Option<u64>,
    #[serde(default)]
    pub detector_findings_total: Option<u64>,
    #[serde(default)]
    pub detector_actionable_findings: Option<u64>,
    #[serde(default)]
    pub detector_unreviewed_findings: Option<u64>,
    #[serde(default)]
    pub maturity_gap_total: Option<u64>,
    #[serde(default)]
    pub detector_refresh_required: Option<bool>,
    #[serde(default)]
    pub quality_evidence_fingerprint: Option<String>,
    pub ci_readiness_scored_repository_count: u64,
    pub maturity_scored_repository_count: u64,
    pub findings_repository_count: u64,
    pub release_rule_repository_count: u64,
    pub release_ready_repository_count: u64,
    #[serde(default)]
    pub remediation_open_action_count: Option<u64>,
    #[serde(default)]
    pub remediation_in_progress_action_count: Option<u64>,
    #[serde(default)]
    pub remediation_blocked_action_count: Option<u64>,
    #[serde(default)]
    pub remediation_deferred_action_count: Option<u64>,
    #[serde(default)]
    pub remediation_verified_action_count: Option<u64>,
    #[serde(default)]
    pub remediation_progress_percent: Option<f64>,
    pub quality_freshness: Option<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricDefinition {
    pub id: String,
    pub label: String,
    pub description: String,
    pub unit: String,
    pub denominator: String,
    pub scope: String,
    pub time_semantics: String,
    pub window_days: Option<i64>,
    pub aggregation: String,
    pub polarity: String,
    pub source: String,
    pub freshness: String,
    pub allowed_visualizations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsFinding {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub metric_ids: Vec<String>,
    pub repository_id: Option<String>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsViewFilters {
    pub range_days: i64,
    pub repository_ids: Vec<String>,
    pub group_ids: Vec<String>,
    pub product_ids: Vec<String>,
    pub freshness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsWidgetConfig {
    pub id: String,
    pub title: String,
    pub metric_ids: Vec<String>,
    pub chart_type: String,
    pub grouping: String,
    pub width: u8,
    pub height: u8,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsView {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub is_default: bool,
    pub filters: AnalyticsViewFilters,
    pub widgets: Vec<AnalyticsWidgetConfig>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRepositorySeries {
    pub repository_id: String,
    pub name: String,
    pub samples: Vec<AnalyticsMetricSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub source: String,
    pub freshness: String,
    pub range_days: i64,
    pub retention_days: i64,
    pub history_available_from: Option<String>,
    pub portfolio_samples: Vec<AnalyticsMetricSample>,
    pub repositories: Vec<AnalyticsRepositorySeries>,
    pub metric_catalog: Vec<MetricDefinition>,
    pub findings: Vec<AnalyticsFinding>,
    pub views: Vec<AnalyticsView>,
    pub default_view_id: String,
}

const AGENT_SUMMARY_SCHEMA: &str = "pronto-agent-summary/v1";

const AGENT_REPOSITORY_SCHEMA: &str = "pronto-agent-repository/v1";

const AGENT_QUALITY_SCHEMA: &str = "pronto-agent-quality/v1";

const AGENT_ATTENTION_SCHEMA: &str = "pronto-agent-attention/v1";

const AGENT_ACTIVITY_SCHEMA: &str = "pronto-agent-activity/v1";

const AGENT_PREPARATION_SCHEMA: &str = "pronto-agent-preparation/v1";

const AGENT_RELEASE_SCHEMA: &str = "pronto-agent-release/v2";

const AGENT_NEXT_SCHEMA: &str = "pronto-agent-next/v1";

const DEFAULT_AGENT_NEXT_LIMIT: usize = 12;

const MAX_AGENT_NEXT_LIMIT: usize = 50;

const AGENT_FOLD_PREVIEW_SCHEMA: &str = "pronto-agent-fold-preview/v2";

const DEFAULT_AGENT_FOLD_PREVIEW_LIMIT: usize = 24;

const MAX_AGENT_FOLD_PREVIEW_LIMIT: usize = 100;

const AGENT_FOLD_CURSOR_VERSION: &str = "v1";

const AGENT_DOCTOR_SCHEMA: &str = "pronto-agent-doctor/v1";

const DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES: i64 = 2_880;

const WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES: i64 = DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES;

const MAX_AGENT_DOCTOR_MAX_AGE_MINUTES: i64 = 10_080;

const AGENT_ROUTE_SCHEMA: &str = "pronto-agent-route/v1";

const REMEDIATION_HANDOFF_SCHEMA: &str = "pronto-remediation-handoff/v1";

const REMEDIATION_EXECUTION_GATE_SCHEMA: &str = "pronto-remediation-execution-gate/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConditionSummary {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub priority: u8,
    pub status: String,
    pub missing: Vec<String>,
    pub confidence: Option<String>,
    pub freshness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkspaceSummary {
    pub id: String,
    pub path: String,
    pub is_primary: bool,
    pub branch: String,
    #[serde(default = "default_status_available")]
    pub status_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_error: Option<String>,
    pub dirty: bool,
    pub sync_state: String,
    pub ahead: u64,
    pub behind: u64,
    pub upstream: Option<String>,
    pub operation: Option<String>,
    pub integration_state: String,
    pub target_branch: Option<String>,
    pub target_confidence: String,
    pub activity_state: String,
    pub activity_confidence: String,
    pub last_commit: Option<String>,
    pub last_commit_at: Option<String>,
    pub last_activity_at: Option<String>,
    #[serde(default)]
    pub provenance: WorkspaceProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_detail: Option<WorkspaceSyncDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRepositorySummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub locality: String,
    pub lifecycle: String,
    pub branch: String,
    pub default_branch: Option<String>,
    pub target_branch: Option<String>,
    pub target_branch_configured: bool,
    pub workspaces: Vec<AgentWorkspaceSummary>,
    pub active_conditions: Vec<AgentConditionSummary>,
    pub quality_status: String,
    pub installed_runtime_status: String,
    pub installed_runtime_summary: String,
    pub maturity_score: Option<f64>,
    pub maturity_score_display: Option<String>,
    pub maturity_freshness: String,
    pub ci_readiness_score: Option<f64>,
    pub ci_readiness_score_display: Option<String>,
    pub ci_readiness_fresh_passing_gate_count: usize,
    pub ci_readiness_ideal_gate_count: usize,
    pub ci_configuration_configured_gate_count: usize,
    pub ci_configuration_ideal_gate_count: usize,
    pub findings_total: u64,
    pub high_severity_findings: u64,
    pub project_compass: ProjectCompassSummary,
    pub last_scan_at: String,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummary {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub repository_count: usize,
    pub active_condition_count: usize,
    pub dirty_workspace_count: usize,
    pub unsynced_workspace_count: usize,
    pub attention_count: usize,
    pub provider_status: ProviderStatus,
    pub quality: QualityPortfolioSnapshot,
    pub showcase: ShowcasePortfolioSnapshot,
    pub repositories: Vec<AgentRepositorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRepositoryDetail {
    pub schema_version: String,
    pub generated_at: String,
    pub repository: RepositorySnapshot,
    pub products: Vec<ProductConfig>,
    pub groups: Vec<GroupConfig>,
    pub task_lanes: TaskLaneReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRepositoryQuality {
    pub id: String,
    pub name: String,
    pub path: String,
    pub branch: String,
    pub quality: QualitySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQualityReport {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub portfolio: QualityPortfolioSnapshot,
    pub repositories: Vec<AgentRepositoryQuality>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvidenceReference {
    pub source: String,
    pub label: String,
    pub status: Option<String>,
    pub freshness: Option<String>,
    pub observed_at: Option<String>,
    pub value: Option<String>,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAttentionItem {
    pub id: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub workspace_id: Option<String>,
    pub workspace_path: Option<String>,
    pub category: String,
    pub severity: String,
    pub status: String,
    pub freshness: Option<String>,
    pub summary: String,
    pub evidence: Vec<AgentEvidenceReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAttentionReport {
    pub schema_version: String,
    pub generated_at: String,
    pub items: Vec<AgentAttentionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivityReport {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub events: Vec<EventRecord>,
    pub action_audits: Vec<ActionAudit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreparationReport {
    pub schema_version: String,
    pub generated_at: String,
    pub preparation: RepositoryPreparation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReleaseReport {
    pub schema_version: String,
    pub generated_at: String,
    pub repository_id: String,
    pub release: ReleasePreparation,
    pub recipe: ReleaseRecipePreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNextAction {
    pub attention_id: String,
    pub repository_id: String,
    pub repository_name: String,
    pub workspace_id: Option<String>,
    pub category: String,
    pub severity: String,
    pub status: String,
    pub summary: String,
    pub recommended_projection: String,
    pub next_safe_step: String,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNextReport {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub summary: AgentSummary,
    pub current_repository: Option<AgentRepositorySummary>,
    pub attention_total: usize,
    pub attention: Vec<AgentAttentionItem>,
    pub actions: Vec<AgentNextAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFoldCandidate {
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub source_branch: String,
    pub target_branch: Option<String>,
    pub target_source: String,
    pub target_confidence: String,
    pub workspace_id: Option<String>,
    pub workspace_path: Option<String>,
    pub role: String,
    pub role_confidence: String,
    pub integration_state: String,
    pub dirty: Option<bool>,
    pub sync_state: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub upstream: Option<String>,
    pub operation: Option<String>,
    pub activity_state: Option<String>,
    pub activity_confidence: Option<String>,
    pub merge_preview: Option<AgentFoldMergePreview>,
    pub decision: String,
    pub reason: String,
    pub next_safe_step: String,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFoldMergePreview {
    pub merge_strategy: String,
    pub fast_forwardable: bool,
    pub target_is_ancestor: bool,
    pub source_is_ancestor: bool,
    pub merge_base: String,
    pub target_only_commits: u64,
    pub source_only_commits: u64,
    pub conflict_count: u64,
    pub conflict_breakdown: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFoldPreview {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub repository_count: usize,
    pub branch_total: usize,
    pub candidate_total: usize,
    pub returned_count: usize,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub candidates: Vec<AgentFoldCandidate>,
    pub live_verification_required: bool,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDoctorCheck {
    pub id: String,
    pub status: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub next_safe_step: String,
}
