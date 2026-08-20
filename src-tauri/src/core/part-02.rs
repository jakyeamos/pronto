#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProvenance {
    #[serde(default = "default_workspace_provenance_kind")]
    pub kind: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub lease: Option<String>,
    #[serde(default)]
    pub canonical_repository: String,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub preservation_evidence: Option<String>,
    #[serde(default = "default_workspace_cleanup_state")]
    pub cleanup_state: String,
}

impl Default for WorkspaceProvenance {
    fn default() -> Self {
        Self {
            kind: default_workspace_provenance_kind(),
            owner: None,
            lease: None,
            canonical_repository: String::new(),
            head: None,
            preservation_evidence: None,
            cleanup_state: default_workspace_cleanup_state(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub path: String,
    pub is_primary: bool,
    pub branch: String,
    #[serde(default = "default_status_available")]
    pub status_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_error: Option<String>,
    pub dirty: bool,
    pub added: u64,
    pub removed: u64,
    pub line_totals_partial: bool,
    pub sync_state: String,
    pub remote_freshness: String,
    pub ahead: u64,
    pub behind: u64,
    pub upstream: Option<String>,
    pub operation: Option<String>,
    pub last_commit: Option<String>,
    pub last_commit_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub integration_state: String,
    pub target_branch: Option<String>,
    pub target_confidence: String,
    pub role: String,
    pub role_confidence: String,
    #[serde(default)]
    pub activity: WorkspaceActivity,
    #[serde(default)]
    pub provenance: WorkspaceProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_detail: Option<WorkspaceSyncDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummary {
    pub name: String,
    pub role: String,
    pub role_confidence: String,
    pub target_branch: Option<String>,
    pub target_confidence: String,
    pub ahead: u64,
    pub behind: u64,
    pub integration_state: String,
    pub workspace_id: Option<String>,
    pub last_commit: Option<String>,
    pub last_commit_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleSummary {
    pub path: String,
    pub commit: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySnapshot {
    pub id: String,
    pub name: String,
    pub path: String,
    pub locality: String,
    pub lifecycle: String,
    pub lifecycle_candidate: String,
    pub remote_url: Option<String>,
    pub provider_state: String,
    pub branch: String,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub target_branch: Option<String>,
    #[serde(default)]
    pub target_branch_configured: bool,
    pub workspace: WorkspaceSummary,
    pub workspaces: Vec<WorkspaceSummary>,
    pub branches: Vec<BranchSummary>,
    #[serde(default)]
    pub submodules: Vec<SubmoduleSummary>,
    #[serde(default)]
    pub pull_requests: Vec<PullRequestSnapshot>,
    #[serde(default)]
    pub releases: Vec<ReleaseSnapshot>,
    #[serde(default)]
    pub quality: QualitySnapshot,
    #[serde(default)]
    pub project_compass: ProjectCompassSummary,
    #[serde(default)]
    pub custody: crate::custody::CustodySnapshot,
    #[serde(default)]
    pub release_rule: Option<ReleaseRuleConfig>,
    #[serde(default)]
    pub release_recipe: Option<ReleaseRecipeConfig>,
    #[serde(default)]
    pub confirmed_release_version: Option<String>,
    #[serde(default = "default_ai_permission")]
    pub ai_permission: String,
    pub conditions: Vec<Condition>,
    pub last_scan_at: String,
    pub last_fetch_at: Option<String>,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub repository_id: String,
    pub kind: String,
    pub summary: String,
    pub fingerprint: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAudit {
    pub id: String,
    pub action: String,
    pub target_ids: Vec<String>,
    pub risk: String,
    pub status: String,
    pub summary: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCodexHandoffReceipt {
    pub schema_version: String,
    pub status: String,
    pub repository: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub failure_signature: Option<String>,
    pub prompt_directory: String,
    pub started: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPreflight {
    pub audit: ActionAudit,
    pub allowed: bool,
    pub target_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestPreparation {
    pub repository_id: String,
    pub workspace_id: String,
    pub head_branch: String,
    pub base_branch: Option<String>,
    pub commit_count: u64,
    #[serde(default = "default_status_available")]
    pub status_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_error: Option<String>,
    pub dirty: bool,
    pub ahead: u64,
    pub behind: u64,
    pub upstream: Option<String>,
    pub provider_state: String,
    pub checks_state: String,
    pub reviews_state: String,
    pub mergeability: String,
    pub status: String,
    pub reasons: Vec<String>,
    pub evidence: Vec<EvidenceItem>,
    pub existing_pull_request: Option<PullRequestSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCommitSummary {
    pub sha: String,
    pub subject: String,
    pub category: String,
    pub bump: Option<String>,
    pub committed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNoteSection {
    pub category: String,
    pub commits: Vec<ReleaseCommitSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRuleTrace {
    pub label: String,
    pub status: String,
    pub value: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRecommendation {
    pub disposition: String,
    pub label: String,
    pub suggested_bump: Option<String>,
    pub suggested_version: Option<String>,
    pub basis: String,
    pub reasons: Vec<String>,
    pub advisory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePreparation {
    pub repository_id: String,
    pub target_branch: Option<String>,
    pub baseline_status: String,
    pub baseline: Option<ReleaseSnapshot>,
    pub commits_since_baseline: Vec<ReleaseCommitSummary>,
    pub rule_status: String,
    pub threshold_label: Option<String>,
    pub rule_trace: Vec<ReleaseRuleTrace>,
    pub candidate_bump: Option<String>,
    pub candidate_version: Option<String>,
    pub version_status: String,
    pub recommendation: ReleaseRecommendation,
    #[serde(default)]
    pub release_boundary_status: Option<String>,
    pub notes: Vec<ReleaseNoteSection>,
    pub status: String,
    pub reasons: Vec<String>,
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRecipeStep {
    pub order: u8,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRecipePreview {
    pub repository_id: String,
    pub recipe_name: String,
    pub candidate_version: Option<String>,
    pub version_status: String,
    pub status: String,
    pub reasons: Vec<String>,
    pub steps: Vec<ReleaseRecipeStep>,
    pub actions_performed: bool,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPayloadCategory {
    pub category: String,
    pub included: bool,
    pub item_count: usize,
    pub byte_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSourceReference {
    pub sha: String,
    pub subject: String,
    pub committed_at: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPayloadPreview {
    pub repository_id: String,
    pub workspace_id: String,
    pub permission: String,
    pub provider: String,
    pub model: Option<String>,
    pub status: String,
    pub reasons: Vec<String>,
    pub categories: Vec<AiPayloadCategory>,
    pub source_references: Vec<AiSourceReference>,
    pub payload_text: String,
    pub payload_bytes: usize,
    pub uncommitted_included: bool,
    pub request_performed: bool,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPreparation {
    pub repository_id: String,
    pub pull_request: PullRequestPreparation,
    pub release: ReleasePreparation,
    pub recipe: ReleaseRecipePreview,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationHandoffCheck {
    pub schema_version: String,
    pub generated_at: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub workspace_id: String,
    pub workspace_path: String,
    pub branch: String,
    pub head_commit: Option<String>,
    pub status: String,
    pub ready: bool,
    pub status_available: bool,
    pub ownership_status: String,
    pub ownership_coordination_required: bool,
    pub checkpoint_required: bool,
    pub workspace_dirty: bool,
    pub persisted_snapshot_dirty: bool,
    pub operation: Option<String>,
    pub reasons: Vec<String>,
    pub next_safe_step: String,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationExecutionBlocker {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub workspace_id: String,
    pub workspace_path: String,
    pub branch: String,
    pub blocked_operations: Vec<String>,
    pub source: String,
    pub evidence_state: String,
    pub observed_at: Option<String>,
    pub next_safe_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationClosureGate {
    pub status: String,
    pub ready: bool,
    pub plan_status: Option<String>,
    pub active_action_count: usize,
    pub blocked_action_count: usize,
    pub blocked_action_ids: Vec<String>,
    pub source_generated_at: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAuthorizationBoundary {
    pub status: String,
    pub evaluated: bool,
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationExecutionGate {
    pub schema_version: String,
    pub generated_at: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub scope: String,
    pub selected_workspace_id: Option<String>,
    pub status: String,
    pub ready: bool,
    pub workspace_checks: Vec<RemediationHandoffCheck>,
    pub blockers: Vec<RemediationExecutionBlocker>,
    pub blocked_operations: Vec<String>,
    pub closure_gate: RemediationClosureGate,
    pub authorization: RemediationAuthorizationBoundary,
    pub next_safe_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedCondition {
    pub repository_id: String,
    pub condition_id: String,
    pub fingerprint: String,
    pub marked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreState {
    pub version: u8,
    pub roots: Vec<RootConfig>,
    pub repositories: Vec<RepositorySnapshot>,
    #[serde(default)]
    pub products: Vec<ProductConfig>,
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
    pub expected_conditions: Vec<ExpectedCondition>,
    pub events: Vec<EventRecord>,
    #[serde(default)]
    pub action_audits: Vec<ActionAudit>,
    #[serde(default)]
    pub provider_identities: Vec<ProviderIdentity>,
    #[serde(default)]
    pub remote_repositories: Vec<RemoteRepositorySnapshot>,
    #[serde(default)]
    pub provider_status: ProviderStatus,
    #[serde(default)]
    pub quality: QualityPortfolioSnapshot,
    #[serde(default)]
    pub remediation: RemediationRun,
    pub retention_days: i64,
}

impl Default for StoreState {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            roots: Vec::new(),
            repositories: Vec::new(),
            products: Vec::new(),
            groups: Vec::new(),
            expected_conditions: Vec::new(),
            events: Vec::new(),
            action_audits: Vec::new(),
            provider_identities: Vec::new(),
            remote_repositories: Vec::new(),
            provider_status: ProviderStatus::default(),
            quality: QualityPortfolioSnapshot::default(),
            remediation: remediation::empty_run(),
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub roots: Vec<RootConfig>,
    pub repositories: Vec<RepositorySnapshot>,
    pub products: Vec<ProductConfig>,
    pub groups: Vec<GroupConfig>,
    pub events: Vec<EventRecord>,
    pub action_audits: Vec<ActionAudit>,
    pub provider_identities: Vec<ProviderIdentity>,
    pub remote_repositories: Vec<RemoteRepositorySnapshot>,
    pub provider_status: ProviderStatus,
    #[serde(default)]
    pub quality: QualityPortfolioSnapshot,
    #[serde(default)]
    pub remediation: RemediationRun,
    #[serde(default)]
    pub showcase: ShowcasePortfolioSnapshot,
    pub retention_days: i64,
    pub generated_at: String,
    pub storage_path: String,
}

const REFRESH_BATCH_SCHEMA: &str = "pronto-refresh-batch/v1";
