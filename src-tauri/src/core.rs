use crate::change_matrix;
use crate::mac_control_maturity;
use crate::papercuts;
use crate::project_compass::{self, ProjectCompassSummary};
use crate::quality::{
    self, QualityFreshness, QualityGateRequirement, QualityGateStatus, QualityPortfolioSnapshot,
    QualitySnapshot,
};
use crate::remediation::{self, RemediationRun};
use crate::skills::{self, SkillsSnapshot};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection as SqliteConnection, OpenFlags, OptionalExtension, Row};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};

const STORE_VERSION: u8 = 5;
const SQLITE_SCHEMA_VERSION: i64 = 10;
const DEFAULT_RETENTION_DAYS: i64 = 90;
const DEFAULT_MAX_UNTRACKED_BYTES: u64 = 2_000_000;
const DEFAULT_MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_AI_DIFF_BYTES: usize = 2_000_000;
const ANALYTICS_SCHEMA: &str = "pronto-analytics/v1";
const ANALYTICS_RANGE_DAYS: i64 = 30;
const ANALYTICS_DEDUP_MINUTES: i64 = 15;
const DEFAULT_QR_AUDIT_TIMEOUT_SECONDS: u64 = 120;
const STORE_WRITE_LOCK_WAIT_SECONDS: u64 = 5;
const STORE_WRITE_LOCK_STALE_SECONDS: u64 = 1_800;
const QUALITY_READ_TIMEOUT_SECONDS: u64 = 10;
const TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS: u64 = 120;
const TARGET_EVIDENCE_TOTAL_TIMEOUT_SECONDS: u64 = 600;

static NEXT_ACTION_AUDIT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_CONFIG_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_ANALYTICS_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_TARGET_EVIDENCE_ID: AtomicU64 = AtomicU64::new(0);

struct StoreWriteLock {
    path: PathBuf,
}

impl Drop for StoreWriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn store_write_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("registry.db");
    path.with_file_name(format!("{file_name}.write.lock"))
}

fn store_write_lock_is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= StdDuration::from_secs(STORE_WRITE_LOCK_STALE_SECONDS))
}

fn acquire_store_write_lock(path: &Path) -> Result<StoreWriteLock, String> {
    acquire_store_write_lock_with_timeout(
        path,
        StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS),
    )
}

fn acquire_store_write_lock_with_timeout(
    path: &Path,
    timeout: StdDuration,
) -> Result<StoreWriteLock, String> {
    ensure_store_parent(path)?;
    let lock_path = store_write_lock_path(path);
    let deadline = Instant::now() + timeout;
    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut lock) => {
                let _ = writeln!(lock, "pid={} started_at={}", std::process::id(), iso_now());
                return Ok(StoreWriteLock { path: lock_path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if store_write_lock_is_stale(&lock_path) {
                    let _ = fs::remove_file(&lock_path);
                    continue;
                }
                if Instant::now() >= deadline {
                    return Err(format!(
                        "Another Pronto write is already in progress; retry after it completes (lock: {}).",
                        lock_path.display()
                    ));
                }
                thread::sleep(StdDuration::from_millis(50));
            }
            Err(error) => {
                return Err(format!(
                    "Could not create the Pronto write lock {}: {error}",
                    lock_path.display()
                ));
            }
        }
    }
}

fn default_refresh_policy() -> String {
    "On open".to_string()
}

fn default_ai_permission() -> String {
    "Disabled".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    pub id: String,
    pub path: String,
    pub label: String,
    pub ignore_patterns: Vec<String>,
    #[serde(default = "default_refresh_policy")]
    pub refresh_policy: String,
    #[serde(default)]
    pub background_monitoring: bool,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductConfig {
    pub id: String,
    pub name: String,
    pub repository_ids: Vec<String>,
    pub release_mode: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRuleConfig {
    pub name: String,
    pub operator: String,
    pub min_commits: Option<u64>,
    pub min_elapsed_days: Option<u64>,
    pub required_commit_types: Vec<String>,
    pub allow_first_release: bool,
    #[serde(default)]
    pub required_quality_gates: Vec<QualityGateRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRecipeConfig {
    pub name: String,
    pub validation_commands: Vec<String>,
    pub release_commands: Vec<String>,
    pub generated_paths: Vec<String>,
    pub commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub id: String,
    pub name: String,
    pub repository_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderIdentity {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub display_name: Option<String>,
    pub organizations: Vec<String>,
    pub credential_state: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRepositorySnapshot {
    pub id: String,
    pub provider: String,
    pub full_name: String,
    pub name: String,
    pub owner: String,
    pub html_url: String,
    pub default_branch: Option<String>,
    pub archived: bool,
    pub locality: String,
    pub identity_id: String,
    pub last_refreshed_at: String,
    #[serde(default)]
    pub pull_requests: Vec<PullRequestSnapshot>,
    #[serde(default)]
    pub releases: Vec<ReleaseSnapshot>,
    #[serde(default)]
    pub ci_checks: Vec<CheckSnapshot>,
    #[serde(default)]
    pub ci_branch: Option<String>,
    #[serde(default)]
    pub ci_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckSnapshot {
    pub context: String,
    pub state: String,
    pub required: bool,
    pub conclusion: Option<String>,
    pub last_refreshed_at: String,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PullRequestSnapshot {
    pub id: String,
    pub provider: String,
    pub repository_id: String,
    pub number: u64,
    pub html_url: String,
    pub title: String,
    pub head_branch: String,
    pub base_branch: String,
    pub state: String,
    pub draft: bool,
    pub checks_state: String,
    pub reviews_state: String,
    pub mergeability: String,
    pub checks: Vec<CheckSnapshot>,
    pub last_refreshed_at: String,
    #[serde(default)]
    pub head_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseSnapshot {
    pub id: String,
    pub provider: String,
    pub repository_id: String,
    pub tag: String,
    pub name: String,
    pub target_commit: Option<String>,
    pub published_at: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub last_refreshed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub state: String,
    pub message: String,
    pub last_refresh_at: Option<String>,
    pub identity_count: usize,
    pub repository_count: usize,
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            provider: "GitHub".to_string(),
            state: "Not connected".to_string(),
            message:
                "Connect GitHub through the existing credential manager to load remote context."
                    .to_string(),
            last_refresh_at: None,
            identity_count: 0,
            repository_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRefresh {
    pub identities: Vec<ProviderIdentity>,
    pub repositories: Vec<RemoteRepositorySnapshot>,
    #[serde(default)]
    pub pull_requests: Vec<PullRequestSnapshot>,
    #[serde(default)]
    pub releases: Vec<ReleaseSnapshot>,
    pub refreshed_at: String,
}

pub trait ProviderAdapter {
    fn provider_id(&self) -> &str;
    fn refresh(&self) -> Result<ProviderRefresh, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub label: String,
    pub value: String,
    pub source: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub priority: u8,
    pub status: String,
    pub fingerprint: String,
    pub rule: String,
    pub evidence: Vec<EvidenceItem>,
    pub missing: Vec<String>,
    pub confidence: Option<String>,
    pub freshness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentManifest {
    #[serde(default, alias = "taskId")]
    pub task_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "targetBranch")]
    pub target_branch: Option<String>,
    #[serde(default, alias = "agentType")]
    pub agent_type: Option<String>,
    #[serde(default, alias = "startTime")]
    pub start_time: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, alias = "sourceSessionId")]
    pub source_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySignal {
    pub source: String,
    pub summary: String,
    pub confidence: String,
    pub observed_at: String,
    pub process_name: Option<String>,
    pub process_id: Option<u32>,
    pub started_at: Option<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceActivity {
    pub state: String,
    pub confidence: String,
    pub signals: Vec<ActivitySignal>,
    #[serde(default)]
    pub manifest: Option<AgentManifest>,
}

impl Default for WorkspaceActivity {
    fn default() -> Self {
        Self {
            state: "Unknown".to_string(),
            confidence: "Low".to_string(),
            signals: Vec::new(),
            manifest: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSyncDetail {
    pub reason: String,
    pub evidence_observed_at: Option<String>,
    pub evidence_expires_at: Option<String>,
    pub evidence_window_minutes: i64,
    pub next_safe_action: String,
    pub scoped_refresh_command: String,
    pub authorization: String,
}

fn default_status_available() -> bool {
    true
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
    pub checkpoint_required: bool,
    pub workspace_dirty: bool,
    pub persisted_snapshot_dirty: bool,
    pub operation: Option<String>,
    pub reasons: Vec<String>,
    pub next_safe_step: String,
    pub authorization: String,
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
    pub retention_days: i64,
    pub generated_at: String,
    pub storage_path: String,
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
    pub ci_readiness_scored_repository_count: u64,
    pub maturity_scored_repository_count: u64,
    pub findings_repository_count: u64,
    pub release_rule_repository_count: u64,
    pub release_ready_repository_count: u64,
    pub quality_freshness: Option<String>,
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
}

const AGENT_SUMMARY_SCHEMA: &str = "pronto-agent-summary/v1";
const AGENT_REPOSITORY_SCHEMA: &str = "pronto-agent-repository/v1";
const AGENT_QUALITY_SCHEMA: &str = "pronto-agent-quality/v1";
const AGENT_ATTENTION_SCHEMA: &str = "pronto-agent-attention/v1";
const AGENT_ACTIVITY_SCHEMA: &str = "pronto-agent-activity/v1";
const AGENT_PREPARATION_SCHEMA: &str = "pronto-agent-preparation/v1";
const AGENT_RELEASE_SCHEMA: &str = "pronto-agent-release/v1";
const AGENT_NEXT_SCHEMA: &str = "pronto-agent-next/v1";
const DEFAULT_AGENT_NEXT_LIMIT: usize = 12;
const MAX_AGENT_NEXT_LIMIT: usize = 50;
const AGENT_FOLD_PREVIEW_SCHEMA: &str = "pronto-agent-fold-preview/v1";
const DEFAULT_AGENT_FOLD_PREVIEW_LIMIT: usize = 24;
const MAX_AGENT_FOLD_PREVIEW_LIMIT: usize = 100;
const AGENT_DOCTOR_SCHEMA: &str = "pronto-agent-doctor/v1";
const DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES: i64 = 2_880;
const WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES: i64 = DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES;
const MAX_AGENT_DOCTOR_MAX_AGE_MINUTES: i64 = 10_080;
const AGENT_ROUTE_SCHEMA: &str = "pronto-agent-route/v1";
const REMEDIATION_HANDOFF_SCHEMA: &str = "pronto-remediation-handoff/v1";

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
    pub repositories: Vec<AgentRepositorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRepositoryDetail {
    pub schema_version: String,
    pub generated_at: String,
    pub repository: RepositorySnapshot,
    pub products: Vec<ProductConfig>,
    pub groups: Vec<GroupConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDoctorReport {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub status: String,
    pub ready: bool,
    pub storage_path: String,
    pub max_age_minutes: i64,
    pub root_count: usize,
    pub repository_count: usize,
    pub workspace_count: usize,
    pub oldest_scan_at: Option<String>,
    pub oldest_scan_age_minutes: Option<i64>,
    pub stale_repository_ids: Vec<String>,
    pub invalid_scan_repository_ids: Vec<String>,
    pub unavailable_paths: Vec<String>,
    pub checks: Vec<AgentDoctorCheck>,
    pub next_safe_step: String,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRouteReport {
    pub schema_version: String,
    pub generated_at: String,
    pub scope: String,
    pub status: String,
    pub ready: bool,
    pub doctor: AgentDoctorReport,
    pub next: Option<AgentNextReport>,
    pub repository: Option<AgentRepositoryDetail>,
    pub quality: Option<AgentQualityReport>,
    pub fold_preview: Option<AgentFoldPreview>,
    pub change_maturity: Option<AgentChangeMaturitySummary>,
    pub next_safe_step: String,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChangeMaturitySummary {
    pub score: Option<f64>,
    pub status: String,
    pub gaps: Vec<String>,
    pub recommended_inspection: String,
}

#[derive(Debug)]
struct GitOutput {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

#[derive(Debug, Default)]
struct ParsedStatus {
    branch: String,
    upstream: Option<String>,
    ahead: u64,
    behind: u64,
    dirty: bool,
}

#[derive(Debug, Default)]
struct DiffTotals {
    added: u64,
    removed: u64,
    partial: bool,
}

#[derive(Debug, Clone)]
struct WorktreeRecord {
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct BranchRecord {
    name: String,
    last_commit: Option<String>,
    last_commit_at: Option<String>,
}

fn iso_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn data_directory() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pronto")
    } else if cfg!(target_os = "windows") {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pronto")
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pronto")
    }
}

fn store_path() -> PathBuf {
    data_directory().join("registry.db")
}

fn legacy_store_path(path: &Path) -> PathBuf {
    path.with_file_name("registry.json")
}

fn ensure_store_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "Pronto storage path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create Pronto storage directory: {error}"))
}

fn metadata_value(connection: &SqliteConnection, key: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("Could not read Pronto database metadata: {error}"))
}

fn table_has_column(
    connection: &SqliteConnection,
    table: &str,
    column: &str,
) -> Result<bool, String> {
    let statement = format!("PRAGMA table_info({table})");
    let mut query = connection
        .prepare(&statement)
        .map_err(|error| format!("Could not inspect Pronto database table {table}: {error}"))?;
    let columns = query
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Could not read Pronto database table {table}: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto database table {table}: {error}"))?;
    Ok(columns.iter().any(|value| value == column))
}

fn initialize_store(connection: &SqliteConnection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS roots (
                 id TEXT PRIMARY KEY,
                 path TEXT NOT NULL,
                 label TEXT NOT NULL,
                 ignore_patterns_json TEXT NOT NULL,
                 refresh_policy TEXT NOT NULL DEFAULT 'On open',
                 background_monitoring INTEGER NOT NULL DEFAULT 0,
                 registered_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS repositories (
                 id TEXT PRIMARY KEY,
                 payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS products (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 repository_ids_json TEXT NOT NULL,
                 release_mode TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS groups_config (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 repository_ids_json TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS expected_conditions (
                 repository_id TEXT NOT NULL,
                 condition_id TEXT NOT NULL,
                 fingerprint TEXT NOT NULL,
                 marked_at TEXT NOT NULL,
                 PRIMARY KEY (repository_id, condition_id)
             );
             CREATE TABLE IF NOT EXISTS events (
                 id TEXT PRIMARY KEY,
                 repository_id TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 summary TEXT NOT NULL,
                 fingerprint TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS action_audits (
                 id TEXT PRIMARY KEY,
                 action TEXT NOT NULL,
                 target_ids_json TEXT NOT NULL,
                 risk TEXT NOT NULL,
                 status TEXT NOT NULL,
                 summary TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 completed_at TEXT
             );
             CREATE TABLE IF NOT EXISTS provider_identities (
                 id TEXT PRIMARY KEY,
                 payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS remote_repositories (
                 id TEXT PRIMARY KEY,
                 payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS analytics_samples (
                 id TEXT PRIMARY KEY,
                 repository_id TEXT,
                 observed_at TEXT NOT NULL,
                 payload_json TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_analytics_samples_scope_time
                 ON analytics_samples (repository_id, observed_at);
             CREATE TABLE IF NOT EXISTS analytics_views (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 is_default INTEGER NOT NULL DEFAULT 0,
                 payload_json TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS skills_snapshots (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS remediation_runs (
                 id TEXT PRIMARY KEY,
                 generated_at TEXT NOT NULL,
                 payload_json TEXT NOT NULL
             );",
        )
        .map_err(|error| format!("Could not initialize Pronto database: {error}"))?;

    if !table_has_column(connection, "roots", "refresh_policy")? {
        connection
            .execute(
                "ALTER TABLE roots ADD COLUMN refresh_policy TEXT NOT NULL DEFAULT 'On open'",
                [],
            )
            .map_err(|error| format!("Could not migrate root refresh policy: {error}"))?;
    }
    if !table_has_column(connection, "roots", "background_monitoring")? {
        connection
            .execute(
                "ALTER TABLE roots ADD COLUMN background_monitoring INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|error| format!("Could not migrate root monitoring setting: {error}"))?;
    }

    let schema_version = metadata_value(connection, "schema_version")?;
    match schema_version {
        Some(value) => {
            let version = value.parse::<i64>().map_err(|error| {
                format!("Could not parse Pronto database schema version: {error}")
            })?;
            if (1..SQLITE_SCHEMA_VERSION).contains(&version) {
                connection
                    .execute(
                        "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
                        params![SQLITE_SCHEMA_VERSION.to_string()],
                    )
                    .map_err(|error| {
                        format!("Could not migrate Pronto database schema version: {error}")
                    })?;
            } else if version != SQLITE_SCHEMA_VERSION {
                return Err(format!(
                    "Unsupported Pronto database schema version {version}; expected {SQLITE_SCHEMA_VERSION}"
                ));
            }
        }
        None => {
            connection
                .execute(
                    "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
                    params!["schema_version", SQLITE_SCHEMA_VERSION.to_string()],
                )
                .map_err(|error| {
                    format!("Could not record Pronto database schema version: {error}")
                })?;
        }
    }

    match metadata_value(connection, "store_version")? {
        Some(value) => {
            let version = value
                .parse::<u8>()
                .map_err(|error| format!("Could not parse Pronto store version: {error}"))?;
            if version < STORE_VERSION {
                connection
                    .execute(
                        "UPDATE metadata SET value = ?1 WHERE key = 'store_version'",
                        params![STORE_VERSION.to_string()],
                    )
                    .map_err(|error| format!("Could not migrate Pronto store version: {error}"))?;
            } else if version > STORE_VERSION {
                return Err(format!(
                    "Unsupported Pronto store version {version}; expected {STORE_VERSION}"
                ));
            }
        }
        None => {
            connection
                .execute(
                    "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
                    params!["store_version", STORE_VERSION.to_string()],
                )
                .map_err(|error| format!("Could not record Pronto store version: {error}"))?;
        }
    }

    Ok(())
}

fn open_store(path: &Path) -> Result<SqliteConnection, String> {
    ensure_store_parent(path)?;
    let connection = SqliteConnection::open(path)
        .map_err(|error| format!("Could not open local Pronto database: {error}"))?;
    connection
        .busy_timeout(StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS))
        .map_err(|error| format!("Could not configure Pronto database busy timeout: {error}"))?;
    initialize_store(&connection)?;
    Ok(connection)
}

fn load_legacy_store(path: &Path) -> Result<Option<StoreState>, String> {
    let legacy_path = legacy_store_path(path);
    if legacy_path == path || !legacy_path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&legacy_path)
        .map_err(|error| format!("Could not read legacy Pronto state: {error}"))?;
    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|error| format!("Could not decode legacy Pronto state: {error}"))
}

fn load_store(path: &Path) -> Result<StoreState, String> {
    if !path.exists() {
        if let Some(mut legacy_state) = load_legacy_store(path)? {
            legacy_state.version = legacy_state.version.max(STORE_VERSION);
            sort_repositories_by_name(&mut legacy_state.repositories);
            legacy_state.remote_repositories = classify_remote_repositories(
                &legacy_state.repositories,
                legacy_state.remote_repositories,
            );
            legacy_state.provider_status.repository_count = legacy_state.remote_repositories.len();
            save_store(path, &legacy_state)?;
            return Ok(legacy_state);
        }
    }

    let connection = open_store(path)?;
    load_store_from_connection(&connection)
}

fn open_store_read_only(path: &Path) -> Result<SqliteConnection, String> {
    if !path.is_file() {
        return Err(format!(
            "Pronto database does not exist at {}",
            path.display()
        ));
    }
    let connection = SqliteConnection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Could not open local Pronto database read-only: {error}"))?;
    connection
        .busy_timeout(StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS))
        .map_err(|error| format!("Could not configure Pronto read-only busy timeout: {error}"))?;
    Ok(connection)
}

pub(crate) fn local_store_path() -> PathBuf {
    store_path()
}

pub(crate) fn with_store_write_for_extension<T, F>(path: &Path, operation: F) -> Result<T, String>
where
    F: FnOnce(&mut SqliteConnection) -> Result<T, String>,
{
    let _lock = acquire_store_write_lock(path)?;
    let mut connection = open_store(path)?;
    operation(&mut connection)
}

fn load_store_read_only(path: &Path) -> Result<StoreState, String> {
    let connection = open_store_read_only(path)?;
    load_store_from_connection(&connection)
}

fn load_store_from_connection(connection: &SqliteConnection) -> Result<StoreState, String> {
    let version = metadata_value(connection, "store_version")?
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(STORE_VERSION)
        .max(STORE_VERSION);
    let retention_days = metadata_value(connection, "retention_days")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS);
    let mut provider_status = match metadata_value(connection, "provider_status_json")? {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode provider status: {error}"))?,
        None => ProviderStatus::default(),
    };
    let quality = match metadata_value(connection, "quality_summary_json")? {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode quality summary: {error}"))?,
        None => QualityPortfolioSnapshot::default(),
    };

    let root_rows = connection
        .prepare(
            "SELECT id, path, label, ignore_patterns_json, refresh_policy,
                    background_monitoring, registered_at
             FROM roots ORDER BY id",
        )
        .map_err(|error| format!("Could not prepare Pronto roots query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)? != 0,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto roots: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto roots: {error}"))?;
    let roots = root_rows
        .into_iter()
        .map(
            |(
                id,
                path,
                label,
                ignore_patterns_json,
                refresh_policy,
                background_monitoring,
                registered_at,
            )| {
                let ignore_patterns = serde_json::from_str(&ignore_patterns_json)
                    .map_err(|error| format!("Could not decode root ignore patterns: {error}"))?;
                Ok(RootConfig {
                    id,
                    path,
                    label,
                    ignore_patterns,
                    refresh_policy,
                    background_monitoring,
                    registered_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let product_rows = connection
        .prepare(
            "SELECT id, name, repository_ids_json, release_mode, created_at, updated_at
             FROM products ORDER BY name, id",
        )
        .map_err(|error| format!("Could not prepare Pronto products query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto products: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto products: {error}"))?;
    let products = product_rows
        .into_iter()
        .map(
            |(id, name, repository_ids_json, release_mode, created_at, updated_at)| {
                let repository_ids = serde_json::from_str(&repository_ids_json)
                    .map_err(|error| format!("Could not decode product repositories: {error}"))?;
                Ok(ProductConfig {
                    id,
                    name,
                    repository_ids,
                    release_mode,
                    created_at,
                    updated_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let group_rows = connection
        .prepare(
            "SELECT id, name, repository_ids_json, created_at, updated_at
             FROM groups_config ORDER BY name, id",
        )
        .map_err(|error| format!("Could not prepare Pronto groups query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto groups: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto groups: {error}"))?;
    let groups = group_rows
        .into_iter()
        .map(|(id, name, repository_ids_json, created_at, updated_at)| {
            let repository_ids = serde_json::from_str(&repository_ids_json)
                .map_err(|error| format!("Could not decode group repositories: {error}"))?;
            Ok(GroupConfig {
                id,
                name,
                repository_ids,
                created_at,
                updated_at,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let provider_identity_payloads = connection
        .prepare("SELECT payload_json FROM provider_identities ORDER BY id")
        .map_err(|error| format!("Could not prepare provider identities query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read provider identities: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode provider identities: {error}"))?;
    let provider_identities = provider_identity_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode provider identity: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let remote_repository_payloads = connection
        .prepare("SELECT payload_json FROM remote_repositories ORDER BY id")
        .map_err(|error| format!("Could not prepare remote repositories query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read remote repositories: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode remote repositories: {error}"))?;
    let remote_repositories = remote_repository_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode remote repository: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let repository_payloads = connection
        .prepare("SELECT payload_json FROM repositories ORDER BY id")
        .map_err(|error| format!("Could not prepare Pronto repositories query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read Pronto repositories: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto repositories: {error}"))?;
    let mut repositories = repository_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode repository snapshot: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    sort_repositories_by_name(&mut repositories);
    let remote_repositories = classify_remote_repositories(&repositories, remote_repositories);
    provider_status.repository_count = remote_repositories.len();

    let expected_conditions = connection
        .prepare(
            "SELECT repository_id, condition_id, fingerprint, marked_at
             FROM expected_conditions ORDER BY repository_id, condition_id",
        )
        .map_err(|error| format!("Could not prepare Pronto expected conditions query: {error}"))?
        .query_map([], |row| {
            Ok(ExpectedCondition {
                repository_id: row.get(0)?,
                condition_id: row.get(1)?,
                fingerprint: row.get(2)?,
                marked_at: row.get(3)?,
            })
        })
        .map_err(|error| format!("Could not read Pronto expected conditions: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto expected conditions: {error}"))?;

    let events = connection
        .prepare(
            "SELECT id, repository_id, kind, summary, fingerprint, created_at
             FROM events ORDER BY created_at, id",
        )
        .map_err(|error| format!("Could not prepare Pronto events query: {error}"))?
        .query_map([], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                kind: row.get(2)?,
                summary: row.get(3)?,
                fingerprint: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("Could not read Pronto events: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto events: {error}"))?;

    let action_audit_rows = connection
        .prepare(
            "SELECT id, action, target_ids_json, risk, status, summary, created_at, completed_at
             FROM action_audits ORDER BY created_at, rowid",
        )
        .map_err(|error| format!("Could not prepare Pronto action audits query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto action audits: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto action audits: {error}"))?;
    let action_audits = action_audit_rows
        .into_iter()
        .map(
            |(id, action, target_ids_json, risk, status, summary, created_at, completed_at)| {
                let target_ids = serde_json::from_str(&target_ids_json)
                    .map_err(|error| format!("Could not decode action audit targets: {error}"))?;
                Ok(ActionAudit {
                    id,
                    action,
                    target_ids,
                    risk,
                    status,
                    summary,
                    created_at,
                    completed_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let remediation = connection
        .query_row(
            "SELECT payload_json FROM remediation_runs ORDER BY generated_at DESC, id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Could not read Pronto remediation run: {error}"))?
        .map(|payload| {
            serde_json::from_str::<RemediationRun>(&payload)
                .map_err(|error| format!("Could not decode Pronto remediation run: {error}"))
        })
        .transpose()?
        .unwrap_or_else(remediation::empty_run);

    Ok(StoreState {
        version,
        roots,
        repositories,
        products,
        groups,
        expected_conditions,
        events,
        action_audits,
        provider_identities,
        remote_repositories,
        provider_status,
        quality,
        remediation,
        retention_days,
    })
}

fn load_store_with_quality(path: &Path) -> Result<StoreState, String> {
    let mut state = load_store(path)?;
    apply_quality_evidence_scoped(&mut state, None, None);
    Ok(state)
}

fn load_store_read_only_with_quality(path: &Path) -> Result<StoreState, String> {
    let mut state = load_store_read_only(path)?;
    apply_quality_evidence_scoped(&mut state, None, None);
    Ok(state)
}

fn load_store_read_only_with_quality_bounded(path: &Path) -> Result<StoreState, String> {
    let path = path.to_path_buf();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(load_store_read_only_with_quality(&path));
    });
    match receiver.recv_timeout(StdDuration::from_secs(QUALITY_READ_TIMEOUT_SECONDS)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "Fresh quality projection exceeded the {} second deadline; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.",
            QUALITY_READ_TIMEOUT_SECONDS
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(
            "Fresh quality projection stopped before it returned a result; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.".to_string(),
        ),
    }
}

fn save_store(path: &Path, state: &StoreState) -> Result<(), String> {
    let mut connection = open_store(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not begin Pronto database transaction: {error}"))?;

    for table in [
        "roots",
        "repositories",
        "products",
        "groups_config",
        "expected_conditions",
        "events",
        "action_audits",
        "provider_identities",
        "remote_repositories",
        "remediation_runs",
    ] {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| format!("Could not clear Pronto {table} table: {error}"))?;
    }

    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params![
                "store_version",
                state.version.max(STORE_VERSION).to_string()
            ],
        )
        .map_err(|error| format!("Could not save Pronto store version: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["retention_days", state.retention_days.to_string()],
        )
        .map_err(|error| format!("Could not save Pronto retention setting: {error}"))?;
    let provider_status_json = serde_json::to_string(&state.provider_status)
        .map_err(|error| format!("Could not encode provider status: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["provider_status_json", provider_status_json],
        )
        .map_err(|error| format!("Could not save provider status: {error}"))?;
    let quality_summary_json = serde_json::to_string(&state.quality)
        .map_err(|error| format!("Could not encode quality summary: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["quality_summary_json", quality_summary_json],
        )
        .map_err(|error| format!("Could not save quality summary: {error}"))?;
    for root in &state.roots {
        let ignore_patterns_json = serde_json::to_string(&root.ignore_patterns)
            .map_err(|error| format!("Could not encode root ignore patterns: {error}"))?;
        transaction
            .execute(
                "INSERT INTO roots
                 (id, path, label, ignore_patterns_json, refresh_policy,
                  background_monitoring, registered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    root.id,
                    root.path,
                    root.label,
                    ignore_patterns_json,
                    root.refresh_policy,
                    i64::from(root.background_monitoring),
                    root.registered_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto root: {error}"))?;
    }

    for repository in &state.repositories {
        let payload = serde_json::to_string(repository)
            .map_err(|error| format!("Could not encode repository snapshot: {error}"))?;
        transaction
            .execute(
                "INSERT INTO repositories (id, payload_json) VALUES (?1, ?2)",
                params![repository.id, payload],
            )
            .map_err(|error| format!("Could not save Pronto repository snapshot: {error}"))?;
    }

    for product in &state.products {
        let repository_ids_json = serde_json::to_string(&product.repository_ids)
            .map_err(|error| format!("Could not encode product repositories: {error}"))?;
        transaction
            .execute(
                "INSERT INTO products
                 (id, name, repository_ids_json, release_mode, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    product.id,
                    product.name,
                    repository_ids_json,
                    product.release_mode,
                    product.created_at,
                    product.updated_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto product: {error}"))?;
    }

    for group in &state.groups {
        let repository_ids_json = serde_json::to_string(&group.repository_ids)
            .map_err(|error| format!("Could not encode group repositories: {error}"))?;
        transaction
            .execute(
                "INSERT INTO groups_config
                 (id, name, repository_ids_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    group.id,
                    group.name,
                    repository_ids_json,
                    group.created_at,
                    group.updated_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto group: {error}"))?;
    }

    for expected in &state.expected_conditions {
        transaction
            .execute(
                "INSERT INTO expected_conditions
                 (repository_id, condition_id, fingerprint, marked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    expected.repository_id,
                    expected.condition_id,
                    expected.fingerprint,
                    expected.marked_at
                ],
            )
            .map_err(|error| format!("Could not save expected condition: {error}"))?;
    }

    for event in &state.events {
        transaction
            .execute(
                "INSERT INTO events
                 (id, repository_id, kind, summary, fingerprint, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.id,
                    event.repository_id,
                    event.kind,
                    event.summary,
                    event.fingerprint,
                    event.created_at
                ],
            )
            .map_err(|error| format!("Could not save Pronto event: {error}"))?;
    }

    for audit in &state.action_audits {
        let target_ids_json = serde_json::to_string(&audit.target_ids)
            .map_err(|error| format!("Could not encode action audit targets: {error}"))?;
        transaction
            .execute(
                "INSERT INTO action_audits
                 (id, action, target_ids_json, risk, status, summary, created_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    audit.id,
                    audit.action,
                    target_ids_json,
                    audit.risk,
                    audit.status,
                    audit.summary,
                    audit.created_at,
                    audit.completed_at
                ],
            )
            .map_err(|error| format!("Could not save Pronto action audit: {error}"))?;
    }

    for identity in &state.provider_identities {
        let payload = serde_json::to_string(identity)
            .map_err(|error| format!("Could not encode provider identity: {error}"))?;
        transaction
            .execute(
                "INSERT INTO provider_identities (id, payload_json) VALUES (?1, ?2)",
                params![identity.id, payload],
            )
            .map_err(|error| format!("Could not save provider identity: {error}"))?;
    }

    for repository in &state.remote_repositories {
        let payload = serde_json::to_string(repository)
            .map_err(|error| format!("Could not encode remote repository: {error}"))?;
        transaction
            .execute(
                "INSERT INTO remote_repositories (id, payload_json) VALUES (?1, ?2)",
                params![repository.id, payload],
            )
            .map_err(|error| format!("Could not save remote repository: {error}"))?;
    }

    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    let remediation_payload = serde_json::to_string(&remediation_run)
        .map_err(|error| format!("Could not encode remediation run: {error}"))?;
    transaction
        .execute(
            "INSERT INTO remediation_runs (id, generated_at, payload_json)
             VALUES (?1, ?2, ?3)",
            params![
                remediation_run.id,
                remediation_run.generated_at,
                remediation_payload
            ],
        )
        .map_err(|error| format!("Could not save remediation run: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("Could not commit Pronto database transaction: {error}"))
}

fn quality_metric_freshness(repository: &RepositorySnapshot) -> Option<String> {
    let has_evidence = repository.quality.ci_readiness.score.is_some()
        || repository.quality.maturity.score.is_some()
        || repository.quality.findings.source.is_some()
        || repository.quality.findings.observed_at.is_some();
    if !has_evidence {
        return None;
    }
    let mut values = Vec::new();
    if repository.quality.ci_readiness.score.is_some() {
        values.push(QualityFreshness::Fresh);
    }
    if repository.quality.maturity.score.is_some() {
        values.push(repository.quality.maturity.freshness.clone());
    }
    if quality_metric_is_available(repository) {
        values.push(repository.quality.findings.freshness.clone());
    }
    if values
        .iter()
        .any(|value| *value == QualityFreshness::Conflicted)
    {
        return Some(QualityFreshness::Conflicted.as_str().to_string());
    }
    if values.iter().any(|value| *value == QualityFreshness::Stale) {
        return Some(QualityFreshness::Stale.as_str().to_string());
    }
    if values.iter().any(|value| *value == QualityFreshness::Fresh) {
        return Some(QualityFreshness::Fresh.as_str().to_string());
    }
    None
}

fn quality_metric_is_available(repository: &RepositorySnapshot) -> bool {
    repository.quality.findings.source.is_some()
        || repository.quality.findings.observed_at.is_some()
        || repository.quality.findings.freshness != QualityFreshness::Unknown
}

fn local_commit_count_since(path: &Path, observed_at: &str) -> Option<u64> {
    let observed = DateTime::parse_from_rfc3339(observed_at)
        .ok()?
        .with_timezone(&Utc);
    let cutoff = observed - chrono::Duration::days(ANALYTICS_RANGE_DAYS);
    let cutoff = cutoff.to_rfc3339_opts(SecondsFormat::Secs, true);
    git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--all".to_string(),
            format!("--since={cutoff}"),
            "--count".to_string(),
        ],
    )
    .and_then(|value| value.parse::<u64>().ok())
}

fn analytics_workspace_activity_counts(repository: &RepositorySnapshot) -> (u64, u64, u64, u64) {
    let mut active = 0;
    let mut interrupted = 0;
    let mut idle = 0;
    let mut unknown = 0;
    for workspace in &repository.workspaces {
        if workspace.activity.state == "Active" {
            active += 1;
        } else if workspace.activity.state.starts_with("Interrupted") {
            interrupted += 1;
        } else if workspace.activity.state.starts_with("Unknown") {
            unknown += 1;
        } else {
            idle += 1;
        }
    }
    (active, interrupted, idle, unknown)
}

fn analytics_repository_sample(
    repository: &RepositorySnapshot,
    observed_at: &str,
) -> AnalyticsMetricSample {
    let (
        active_workspace_count,
        interrupted_workspace_count,
        idle_workspace_count,
        unknown_workspace_count,
    ) = analytics_workspace_activity_counts(repository);
    let active_condition_count = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .count() as u64;
    let dirty_workspace_count = repository
        .workspaces
        .iter()
        .filter(|workspace| workspace.dirty)
        .count() as u64;
    let unsynced_workspace_count = repository
        .workspaces
        .iter()
        .filter(|workspace| workspace_is_unsynced(workspace))
        .count() as u64;
    let release_ready = repository.conditions.iter().any(|condition| {
        condition.kind == "release-threshold"
            && matches!(condition.status.as_str(), "Active" | "Expected")
    });
    let ci_readiness_score = repository.quality.ci_readiness.score;
    let maturity_score = repository.quality.maturity.score;
    let findings_available = quality_metric_is_available(repository);
    AnalyticsMetricSample {
        observed_at: observed_at.to_string(),
        repository_count: 1,
        workspace_count: repository.workspaces.len() as u64,
        branch_count: repository.branches.len() as u64,
        active_condition_count,
        dirty_workspace_count,
        unsynced_workspace_count,
        active_workspace_count,
        interrupted_workspace_count,
        idle_workspace_count,
        unknown_workspace_count,
        ahead_commit_count: repository
            .workspaces
            .iter()
            .map(|workspace| workspace.ahead)
            .sum(),
        behind_commit_count: repository
            .workspaces
            .iter()
            .map(|workspace| workspace.behind)
            .sum(),
        commits_last_30_days: local_commit_count_since(Path::new(&repository.path), observed_at),
        ci_readiness_score,
        maturity_score,
        findings_total: findings_available.then_some(repository.quality.findings.total),
        high_severity_findings: findings_available
            .then_some(repository.quality.findings.high_severity_total),
        ci_readiness_scored_repository_count: u64::from(ci_readiness_score.is_some()),
        maturity_scored_repository_count: u64::from(maturity_score.is_some()),
        findings_repository_count: u64::from(findings_available),
        release_rule_repository_count: u64::from(repository.release_rule.is_some()),
        release_ready_repository_count: u64::from(release_ready),
        quality_freshness: quality_metric_freshness(repository),
    }
}

fn average_score(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<f64>,
) -> Option<f64> {
    let values = samples.iter().filter_map(selector).collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn sum_optional_metric(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<u64>,
) -> Option<u64> {
    let values = samples.iter().filter_map(selector).collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.into_iter().sum())
    }
}

fn sum_complete_optional_metric(
    samples: &[AnalyticsMetricSample],
    selector: fn(&AnalyticsMetricSample) -> Option<u64>,
) -> Option<u64> {
    if samples.is_empty() || samples.iter().any(|sample| selector(sample).is_none()) {
        None
    } else {
        Some(samples.iter().filter_map(selector).sum())
    }
}

fn aggregate_quality_freshness(samples: &[AnalyticsMetricSample]) -> Option<String> {
    let values = samples
        .iter()
        .filter_map(|sample| sample.quality_freshness.as_deref())
        .collect::<Vec<_>>();
    if values.iter().any(|value| *value == "Conflicted") {
        Some("Conflicted".to_string())
    } else if values.iter().any(|value| *value == "Stale") {
        Some("Stale".to_string())
    } else if values.iter().any(|value| *value == "Fresh") {
        Some("Fresh".to_string())
    } else {
        None
    }
}

fn analytics_portfolio_sample(
    repositories: &[RepositorySnapshot],
    observed_at: &str,
) -> AnalyticsMetricSample {
    let samples = repositories
        .iter()
        .map(|repository| analytics_repository_sample(repository, observed_at))
        .collect::<Vec<_>>();
    AnalyticsMetricSample {
        observed_at: observed_at.to_string(),
        repository_count: samples.iter().map(|sample| sample.repository_count).sum(),
        workspace_count: samples.iter().map(|sample| sample.workspace_count).sum(),
        branch_count: samples.iter().map(|sample| sample.branch_count).sum(),
        active_condition_count: samples
            .iter()
            .map(|sample| sample.active_condition_count)
            .sum(),
        dirty_workspace_count: samples
            .iter()
            .map(|sample| sample.dirty_workspace_count)
            .sum(),
        unsynced_workspace_count: samples
            .iter()
            .map(|sample| sample.unsynced_workspace_count)
            .sum(),
        active_workspace_count: samples
            .iter()
            .map(|sample| sample.active_workspace_count)
            .sum(),
        interrupted_workspace_count: samples
            .iter()
            .map(|sample| sample.interrupted_workspace_count)
            .sum(),
        idle_workspace_count: samples
            .iter()
            .map(|sample| sample.idle_workspace_count)
            .sum(),
        unknown_workspace_count: samples
            .iter()
            .map(|sample| sample.unknown_workspace_count)
            .sum(),
        ahead_commit_count: samples.iter().map(|sample| sample.ahead_commit_count).sum(),
        behind_commit_count: samples
            .iter()
            .map(|sample| sample.behind_commit_count)
            .sum(),
        commits_last_30_days: sum_complete_optional_metric(&samples, |sample| {
            sample.commits_last_30_days
        }),
        ci_readiness_score: average_score(&samples, |sample| sample.ci_readiness_score),
        maturity_score: average_score(&samples, |sample| sample.maturity_score),
        findings_total: sum_optional_metric(&samples, |sample| sample.findings_total),
        high_severity_findings: sum_optional_metric(&samples, |sample| {
            sample.high_severity_findings
        }),
        ci_readiness_scored_repository_count: samples
            .iter()
            .map(|sample| sample.ci_readiness_scored_repository_count)
            .sum(),
        maturity_scored_repository_count: samples
            .iter()
            .map(|sample| sample.maturity_scored_repository_count)
            .sum(),
        findings_repository_count: samples
            .iter()
            .map(|sample| sample.findings_repository_count)
            .sum(),
        release_rule_repository_count: samples
            .iter()
            .map(|sample| sample.release_rule_repository_count)
            .sum(),
        release_ready_repository_count: samples
            .iter()
            .map(|sample| sample.release_ready_repository_count)
            .sum(),
        quality_freshness: aggregate_quality_freshness(&samples),
    }
}

fn analytics_sample_fingerprint(sample: &AnalyticsMetricSample) -> Result<String, String> {
    let mut comparable = sample.clone();
    comparable.observed_at.clear();
    serde_json::to_string(&comparable)
        .map_err(|error| format!("Could not fingerprint analytics sample: {error}"))
}

fn analytics_scope_id(repository_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(repository_id.as_bytes());
    format!("repository:{:x}", digest.finalize())
}

fn latest_analytics_sample(
    connection: &SqliteConnection,
    repository_id: Option<&str>,
) -> Result<Option<AnalyticsMetricSample>, String> {
    let payload = match repository_id {
        Some(repository_id) => connection
            .query_row(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id = ?1 ORDER BY observed_at DESC, id DESC LIMIT 1",
                params![repository_id],
                |row| row.get::<_, String>(0),
            )
            .optional(),
        None => connection
            .query_row(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id IS NULL ORDER BY observed_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional(),
    }
    .map_err(|error| format!("Could not read latest analytics sample: {error}"))?;
    payload
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode analytics sample: {error}"))
        })
        .transpose()
}

fn should_deduplicate_analytics_sample(
    latest: Option<&AnalyticsMetricSample>,
    sample: &AnalyticsMetricSample,
) -> Result<bool, String> {
    let Some(latest) = latest else {
        return Ok(false);
    };
    if analytics_sample_fingerprint(latest)? != analytics_sample_fingerprint(sample)? {
        return Ok(false);
    }
    let observed_at = DateTime::parse_from_rfc3339(&sample.observed_at)
        .map_err(|error| format!("Could not parse analytics observation time: {error}"))?
        .with_timezone(&Utc);
    let previous = DateTime::parse_from_rfc3339(&latest.observed_at)
        .map_err(|error| format!("Could not parse latest analytics time: {error}"))?
        .with_timezone(&Utc);
    let elapsed = observed_at - previous;
    Ok(elapsed >= chrono::Duration::zero()
        && elapsed <= chrono::Duration::minutes(ANALYTICS_DEDUP_MINUTES))
}

fn record_analytics_samples_at(
    path: &Path,
    state: &StoreState,
    observed_at: &str,
) -> Result<(), String> {
    let portfolio = analytics_portfolio_sample(&state.repositories, observed_at);
    let mut samples = vec![(None, portfolio)];
    samples.extend(state.repositories.iter().map(|repository| {
        (
            Some(repository.id.clone()),
            analytics_repository_sample(repository, observed_at),
        )
    }));

    let mut connection = open_store(path)?;
    let latest_samples = samples
        .iter()
        .map(|(repository_id, _)| {
            let scope_id = repository_id.as_deref().map(analytics_scope_id);
            latest_analytics_sample(&connection, scope_id.as_deref())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not begin analytics transaction: {error}"))?;
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    transaction
        .execute(
            "DELETE FROM analytics_samples WHERE observed_at < ?1",
            params![cutoff.to_rfc3339_opts(SecondsFormat::Secs, true)],
        )
        .map_err(|error| format!("Could not prune analytics samples: {error}"))?;
    for ((repository_id, sample), latest) in samples.into_iter().zip(latest_samples) {
        if should_deduplicate_analytics_sample(latest.as_ref(), &sample)? {
            continue;
        }
        let payload = serde_json::to_string(&sample)
            .map_err(|error| format!("Could not encode analytics sample: {error}"))?;
        let sequence = NEXT_ANALYTICS_ID.fetch_add(1, Ordering::Relaxed);
        let scope_id = repository_id.as_deref().map(analytics_scope_id);
        let scope = scope_id.as_deref().unwrap_or("fleet");
        let id = format!("analytics:{scope}:{observed_at}:{sequence}");
        transaction
            .execute(
                "INSERT INTO analytics_samples
                 (id, repository_id, observed_at, payload_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, scope_id, observed_at, payload],
            )
            .map_err(|error| format!("Could not save analytics sample: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("Could not commit analytics samples: {error}"))
}

fn record_analytics_samples(path: &Path, state: &StoreState) -> Result<(), String> {
    record_analytics_samples_at(path, state, &iso_now())
}

fn prune_analytics_samples(path: &Path, retention_days: i64) -> Result<(), String> {
    let connection = open_store(path)?;
    let cutoff = Utc::now() - chrono::Duration::days(retention_days.max(1));
    connection
        .execute(
            "DELETE FROM analytics_samples WHERE observed_at < ?1",
            params![cutoff.to_rfc3339_opts(SecondsFormat::Secs, true)],
        )
        .map_err(|error| format!("Could not prune analytics samples: {error}"))?;
    Ok(())
}

fn analytics_payload(row: &Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}

fn load_analytics_samples(
    connection: &SqliteConnection,
    repository_id: Option<&str>,
    cutoff: &str,
) -> Result<Vec<AnalyticsMetricSample>, String> {
    let mut samples = Vec::new();
    let mut statement = match repository_id {
        Some(_) => connection
            .prepare(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id = ?1 AND observed_at >= ?2
                 ORDER BY observed_at, id",
            )
            .map_err(|error| format!("Could not prepare analytics query: {error}"))?,
        None => connection
            .prepare(
                "SELECT payload_json FROM analytics_samples
                 WHERE repository_id IS NULL AND observed_at >= ?1
                 ORDER BY observed_at, id",
            )
            .map_err(|error| format!("Could not prepare analytics query: {error}"))?,
    };
    match repository_id {
        Some(repository_id) => {
            let rows = statement
                .query_map(params![repository_id, cutoff], analytics_payload)
                .map_err(|error| format!("Could not read analytics samples: {error}"))?;
            for row in rows {
                let payload =
                    row.map_err(|error| format!("Could not decode analytics row: {error}"))?;
                samples.push(
                    serde_json::from_str(&payload)
                        .map_err(|error| format!("Could not decode analytics sample: {error}"))?,
                );
            }
        }
        None => {
            let rows = statement
                .query_map(params![cutoff], analytics_payload)
                .map_err(|error| format!("Could not read analytics samples: {error}"))?;
            for row in rows {
                let payload =
                    row.map_err(|error| format!("Could not decode analytics row: {error}"))?;
                samples.push(
                    serde_json::from_str(&payload)
                        .map_err(|error| format!("Could not decode analytics sample: {error}"))?,
                );
            }
        }
    }
    Ok(samples)
}

fn load_analytics_at(path: &Path) -> Result<AnalyticsSnapshot, String> {
    let state = load_store_read_only(path)?;
    let connection = open_store_read_only(path)?;
    let range_cutoff = Utc::now() - chrono::Duration::days(ANALYTICS_RANGE_DAYS);
    let retention_cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    let cutoff = range_cutoff.max(retention_cutoff);
    let cutoff = cutoff.to_rfc3339_opts(SecondsFormat::Secs, true);
    let portfolio_samples = load_analytics_samples(&connection, None, &cutoff)?;
    let repositories = state
        .repositories
        .iter()
        .map(|repository| {
            let scope_id = analytics_scope_id(repository.id.as_str());
            Ok(AnalyticsRepositorySeries {
                repository_id: repository.id.clone(),
                name: repository.name.clone(),
                samples: load_analytics_samples(&connection, Some(scope_id.as_str()), &cutoff)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let latest_observed_at = portfolio_samples
        .last()
        .map(|sample| sample.observed_at.clone())
        .or_else(|| {
            repositories
                .iter()
                .flat_map(|repository| repository.samples.last())
                .map(|sample| sample.observed_at.clone())
                .max()
        });
    let history_available_from = portfolio_samples
        .first()
        .map(|sample| sample.observed_at.clone())
        .or_else(|| {
            repositories
                .iter()
                .flat_map(|repository| repository.samples.first())
                .map(|sample| sample.observed_at.clone())
                .min()
        });
    Ok(AnalyticsSnapshot {
        schema_version: ANALYTICS_SCHEMA.to_string(),
        generated_at: iso_now(),
        source: "Local refresh snapshots".to_string(),
        freshness: latest_observed_at
            .map(|observed_at| format!("Observed through {observed_at}"))
            .unwrap_or_else(|| "Unavailable until the first local refresh".to_string()),
        range_days: ANALYTICS_RANGE_DAYS,
        retention_days: state.retention_days,
        history_available_from,
        portfolio_samples,
        repositories,
    })
}

fn snapshot_from_store(path: &Path, state: &StoreState) -> PortfolioSnapshot {
    let mut repositories = state.repositories.clone();
    sort_repositories_by_name(&mut repositories);
    for repository in &mut repositories {
        hydrate_workspace_sync_details(repository);
    }
    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    PortfolioSnapshot {
        roots: state.roots.clone(),
        repositories,
        products: state.products.clone(),
        groups: state.groups.clone(),
        events: state.events.iter().rev().take(24).cloned().collect(),
        action_audits: state.action_audits.iter().rev().take(24).cloned().collect(),
        provider_identities: state.provider_identities.clone(),
        remote_repositories: state.remote_repositories.clone(),
        provider_status: state.provider_status.clone(),
        quality: state.quality.clone(),
        remediation: remediation_run,
        retention_days: state.retention_days,
        generated_at: iso_now(),
        storage_path: path.to_string_lossy().to_string(),
    }
}

fn shell_quote_for_display(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn workspace_sync_reason(workspace: &WorkspaceSummary) -> String {
    if !workspace.status_available {
        return workspace_status_unavailable_reason(workspace);
    }
    match (
        workspace.upstream.as_deref(),
        workspace.ahead,
        workspace.behind,
    ) {
        (None, _, _) => format!(
            "Workspace branch '{}' has no tracked upstream, so Pronto cannot compare it to a remote branch.",
            workspace.branch
        ),
        (Some(upstream), ahead, behind) if ahead > 0 && behind > 0 => format!(
            "Workspace branch '{}' is ahead by {} commit{} and behind by {} commit{} relative to '{}'.",
            workspace.branch,
            ahead,
            if ahead == 1 { "" } else { "s" },
            behind,
            if behind == 1 { "" } else { "s" },
            upstream
        ),
        (Some(upstream), ahead, _) if ahead > 0 => format!(
            "Workspace branch '{}' is ahead by {} commit{} relative to '{}'.",
            workspace.branch,
            ahead,
            if ahead == 1 { "" } else { "s" },
            upstream
        ),
        (Some(upstream), _, behind) if behind > 0 => format!(
            "Workspace branch '{}' is behind by {} commit{} relative to '{}'.",
            workspace.branch,
            behind,
            if behind == 1 { "" } else { "s" },
            upstream
        ),
        _ => format!(
            "Pronto recorded sync state '{}' for workspace branch '{}', but the comparison counts are unavailable.",
            workspace.sync_state, workspace.branch
        ),
    }
}

fn workspace_status_unavailable_reason(workspace: &WorkspaceSummary) -> String {
    workspace
        .status_error
        .clone()
        .unwrap_or_else(|| "Git status could not be established for this workspace.".to_string())
}

fn workspace_sync_detail(
    workspace: &WorkspaceSummary,
    repository_path: &str,
    observed_at: &str,
) -> Option<WorkspaceSyncDetail> {
    if !workspace_requires_sync_attention(workspace) {
        return None;
    }

    let observed = DateTime::parse_from_rfc3339(observed_at)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc));
    let evidence_observed_at =
        observed.map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    let evidence_expires_at = observed.map(|timestamp| {
        (timestamp + Duration::minutes(WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES))
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });

    Some(WorkspaceSyncDetail {
        reason: workspace_sync_reason(workspace),
        evidence_observed_at,
        evidence_expires_at,
        evidence_window_minutes: WORKSPACE_SYNC_EVIDENCE_MAX_AGE_MINUTES,
        next_safe_action: "Run the repository-scoped local refresh command below, then reopen this detail to compare the newly observed evidence. Do not choose a merge, rebase, pull, or push from this view.".to_string(),
        scoped_refresh_command: format!(
            "pronto refresh {} --json",
            shell_quote_for_display(repository_path)
        ),
        authorization: "Read-only local Git scan; it persists Pronto evidence only and does not pull, push, merge, rebase, or edit repository files.".to_string(),
    })
}

fn hydrate_workspace_sync_details(repository: &mut RepositorySnapshot) {
    let repository_path = repository.path.clone();
    let observed_at = repository.last_scan_at.clone();

    if repository.workspaces.is_empty() {
        repository.workspace.sync_detail =
            workspace_sync_detail(&repository.workspace, &repository_path, &observed_at);
        return;
    }

    for workspace in &mut repository.workspaces {
        workspace.sync_detail = workspace_sync_detail(workspace, &repository_path, &observed_at);
    }
    repository.workspace.sync_detail = repository
        .workspaces
        .iter()
        .find(|workspace| workspace.is_primary)
        .or_else(|| {
            repository
                .workspaces
                .iter()
                .find(|workspace| workspace.id == repository.workspace.id)
        })
        .and_then(|workspace| workspace.sync_detail.clone())
        .or_else(|| workspace_sync_detail(&repository.workspace, &repository_path, &observed_at));
}

fn git_process(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE");
    command
}

fn run_git<I, S>(path: &Path, arguments: I) -> Result<GitOutput, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_process(path)
        .args(arguments)
        .output()
        .map_err(|error| format!("Could not run Git in {}: {error}", path.display()))?;
    Ok(GitOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code(),
    })
}

fn git_static(path: &Path, arguments: &[&str]) -> Option<String> {
    let result = run_git(path, arguments.iter()).ok()?;
    if result.success {
        Some(result.stdout.trim().to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct GitHubCliAdapter {
    executable: String,
    target_repository_names: Option<HashSet<String>>,
}

impl Default for GitHubCliAdapter {
    fn default() -> Self {
        Self {
            executable: "gh".to_string(),
            target_repository_names: None,
        }
    }
}

impl GitHubCliAdapter {
    fn for_repository_names(repository_names: HashSet<String>) -> Self {
        Self {
            executable: "gh".to_string(),
            target_repository_names: Some(repository_names),
        }
    }

    fn json(&self, arguments: &[&str]) -> Result<serde_json::Value, String> {
        let output = Command::new(&self.executable)
            .args(arguments)
            .output()
            .map_err(|_| {
                "GitHub provider unavailable: install GitHub CLI and authenticate it before refreshing."
                    .to_string()
            })?;
        if !output.status.success() {
            return Err(
                "GitHub provider unavailable: GitHub CLI authentication is missing or expired."
                    .to_string(),
            );
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_| "GitHub provider returned invalid JSON.".to_string())
    }
}

impl ProviderAdapter for GitHubCliAdapter {
    fn provider_id(&self) -> &str {
        "github"
    }

    fn refresh(&self) -> Result<ProviderRefresh, String> {
        let refreshed_at = iso_now();
        let identity_payload = self.json(&["api", "user"])?;
        let login = identity_payload
            .get("login")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "GitHub provider did not return an authenticated login.".to_string())?;
        let identity_id = format!("github:{login}");
        let identity = ProviderIdentity {
            id: identity_id.clone(),
            provider: self.provider_id().to_string(),
            login: login.to_string(),
            display_name: identity_payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            organizations: Vec::new(),
            credential_state: "Authenticated".to_string(),
            updated_at: refreshed_at.clone(),
        };
        let repositories_payload = self.json(&["api", "user/repos", "--paginate", "--slurp"])?;
        let mut repositories =
            parse_github_repositories(&repositories_payload, &identity_id, &refreshed_at)?;
        let mut pull_requests = Vec::new();
        let mut releases = Vec::new();
        let mut ci_updates = HashMap::<String, (Vec<CheckSnapshot>, String, Option<String>)>::new();
        let repositories_to_refresh = repositories
            .iter()
            .filter(|repository| {
                self.target_repository_names
                    .as_ref()
                    .map(|names| {
                        normalize_remote_name(&repository.full_name)
                            .is_some_and(|name| names.contains(&name))
                    })
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        for repository in repositories_to_refresh {
            let pull_request_endpoint = format!(
                "repos/{}/pulls?state=all&per_page=100",
                repository.full_name
            );
            if let Ok(payload) = self.json(&[
                "api",
                pull_request_endpoint.as_str(),
                "--paginate",
                "--slurp",
            ]) {
                if let Ok(parsed) =
                    parse_github_pull_requests(&payload, &repository.id, &refreshed_at)
                {
                    let mut parsed = parsed;
                    for pull_request in &mut parsed {
                        let Some(head_commit) = pull_request.head_commit.as_deref() else {
                            continue;
                        };
                        let check_endpoint = format!(
                            "repos/{}/commits/{head_commit}/check-runs?per_page=100",
                            repository.full_name
                        );
                        if let Ok(check_payload) = self.json(&["api", check_endpoint.as_str()]) {
                            if let Ok(checks) =
                                parse_github_check_runs(&check_payload, &refreshed_at)
                            {
                                pull_request.checks_state = summarize_check_state(&checks);
                                pull_request.checks = checks;
                            }
                        }
                    }
                    pull_requests.extend(parsed);
                }
            }
            if let Some(default_branch) = repository.default_branch.as_deref() {
                let check_endpoint = format!(
                    "repos/{}/commits/{default_branch}/check-runs?per_page=100",
                    repository.full_name
                );
                if let Ok(payload) = self.json(&["api", check_endpoint.as_str()]) {
                    if let Ok(checks) = parse_github_check_runs(&payload, &refreshed_at) {
                        let ci_commit = checks.iter().find_map(|check| check.head_sha.clone());
                        ci_updates.insert(
                            repository.id.clone(),
                            (checks, default_branch.to_string(), ci_commit),
                        );
                    }
                }
            }
            let release_endpoint = format!("repos/{}/releases?per_page=100", repository.full_name);
            if let Ok(payload) =
                self.json(&["api", release_endpoint.as_str(), "--paginate", "--slurp"])
            {
                if let Ok(parsed) = parse_github_releases(&payload, &repository.id, &refreshed_at) {
                    releases.extend(parsed);
                }
            }
        }
        for repository in &mut repositories {
            if let Some((checks, branch, commit)) = ci_updates.remove(&repository.id) {
                repository.ci_checks = checks;
                repository.ci_branch = Some(branch);
                repository.ci_commit = commit;
            }
            repository.pull_requests = pull_requests
                .iter()
                .filter(|pull_request| pull_request.repository_id == repository.id)
                .cloned()
                .collect();
            repository.releases = releases
                .iter()
                .filter(|release| release.repository_id == repository.id)
                .cloned()
                .collect();
        }
        Ok(ProviderRefresh {
            identities: vec![identity],
            repositories,
            pull_requests,
            releases,
            refreshed_at,
        })
    }
}

fn parse_github_repositories(
    payload: &serde_json::Value,
    identity_id: &str,
    refreshed_at: &str,
) -> Result<Vec<RemoteRepositorySnapshot>, String> {
    let pages = github_array_items(payload, "repository")?;
    Ok(pages
        .into_iter()
        .filter_map(|repository| {
            let full_name = repository.get("full_name")?.as_str()?.to_string();
            let owner = repository
                .get("owner")
                .and_then(|value| value.get("login"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let name = repository
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| full_name.rsplit('/').next().unwrap_or_default())
                .to_string();
            Some(RemoteRepositorySnapshot {
                id: format!(
                    "github:{}",
                    repository
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| full_name.clone())
                ),
                provider: "github".to_string(),
                full_name,
                name,
                owner,
                html_url: repository
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                default_branch: repository
                    .get("default_branch")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                archived: repository
                    .get("archived")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                locality: "Remote only".to_string(),
                identity_id: identity_id.to_string(),
                last_refreshed_at: refreshed_at.to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
            })
        })
        .collect())
}

fn github_array_items<'a>(
    payload: &'a serde_json::Value,
    resource: &str,
) -> Result<Vec<&'a serde_json::Value>, String> {
    match payload {
        serde_json::Value::Array(values) if values.iter().all(serde_json::Value::is_array) => {
            Ok(values
                .iter()
                .flat_map(|page| page.as_array().into_iter().flatten())
                .collect::<Vec<_>>())
        }
        serde_json::Value::Array(values) => Ok(values.iter().collect::<Vec<_>>()),
        _ => Err(format!("GitHub {resource} response was not an array.")),
    }
}

fn parse_github_pull_requests(
    payload: &serde_json::Value,
    repository_id: &str,
    refreshed_at: &str,
) -> Result<Vec<PullRequestSnapshot>, String> {
    Ok(github_array_items(payload, "pull-request")?
        .into_iter()
        .filter_map(|pull_request| {
            let number = pull_request
                .get("number")
                .and_then(serde_json::Value::as_u64)?;
            let head_branch = pull_request
                .get("head")
                .and_then(|value| value.get("ref"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let base_branch = pull_request
                .get("base")
                .and_then(|value| value.get("ref"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let head_commit = pull_request
                .get("head")
                .and_then(|value| value.get("sha"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            Some(PullRequestSnapshot {
                id: format!("github:pr:{repository_id}:{number}"),
                provider: "github".to_string(),
                repository_id: repository_id.to_string(),
                number,
                html_url: pull_request
                    .get("html_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                title: pull_request
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                head_branch,
                base_branch,
                state: pull_request
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                draft: pull_request
                    .get("draft")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                checks_state: "Unknown — provider snapshot unavailable".to_string(),
                reviews_state: "Unknown — provider snapshot unavailable".to_string(),
                mergeability: pull_request
                    .get("mergeable_state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unknown — provider snapshot unavailable")
                    .to_string(),
                checks: Vec::new(),
                last_refreshed_at: refreshed_at.to_string(),
                head_commit,
            })
        })
        .collect())
}

fn parse_github_check_runs(
    payload: &serde_json::Value,
    refreshed_at: &str,
) -> Result<Vec<CheckSnapshot>, String> {
    let values = match payload {
        serde_json::Value::Array(values) if values.iter().all(serde_json::Value::is_array) => {
            values
                .iter()
                .flat_map(|page| page.as_array().into_iter().flatten())
                .collect::<Vec<_>>()
        }
        serde_json::Value::Array(values) => values.iter().collect::<Vec<_>>(),
        serde_json::Value::Object(_) => vec![payload],
        _ => return Err("GitHub check-run response was not an object or array.".to_string()),
    };
    Ok(values
        .into_iter()
        .flat_map(|value| {
            value
                .get("check_runs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
        .filter_map(|check| {
            let context = check
                .get("name")
                .or_else(|| check.get("context"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            Some(CheckSnapshot {
                context,
                state: check
                    .get("status")
                    .or_else(|| check.get("state"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                required: false,
                conclusion: check
                    .get("conclusion")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                last_refreshed_at: refreshed_at.to_string(),
                html_url: check
                    .get("html_url")
                    .or_else(|| check.get("details_url"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                head_sha: check
                    .get("head_sha")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect())
}

fn summarize_check_state(checks: &[CheckSnapshot]) -> String {
    if checks.is_empty() {
        return "Not configured".to_string();
    }
    if checks.iter().any(|check| {
        matches!(
            check.conclusion.as_deref(),
            Some("failure" | "timed_out" | "cancelled" | "action_required")
        )
    }) {
        "Failed".to_string()
    } else if checks
        .iter()
        .all(|check| matches!(check.conclusion.as_deref(), Some("success" | "neutral")))
    {
        "Passed".to_string()
    } else {
        "Blocked".to_string()
    }
}

fn parse_github_releases(
    payload: &serde_json::Value,
    repository_id: &str,
    refreshed_at: &str,
) -> Result<Vec<ReleaseSnapshot>, String> {
    Ok(github_array_items(payload, "release")?
        .into_iter()
        .filter_map(|release| {
            let id = release
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .map(|value| value.to_string())
                .or_else(|| {
                    release
                        .get("tag_name")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })?;
            Some(ReleaseSnapshot {
                id: format!("github:release:{repository_id}:{id}"),
                provider: "github".to_string(),
                repository_id: repository_id.to_string(),
                tag: release
                    .get("tag_name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: release
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                target_commit: release
                    .get("target_commitish")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                published_at: release
                    .get("published_at")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                draft: release
                    .get("draft")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                prerelease: release
                    .get("prerelease")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                last_refreshed_at: refreshed_at.to_string(),
            })
        })
        .collect())
}

fn normalize_remote_name(value: &str) -> Option<String> {
    let mut normalized = value.trim().trim_end_matches('/').to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some(value) = normalized.strip_prefix("git@github.com:") {
        normalized = value.to_string();
    } else {
        for prefix in ["https://github.com/", "http://github.com/", "github.com/"] {
            if let Some(value) = normalized.strip_prefix(prefix) {
                normalized = value.to_string();
                break;
            }
        }
    }
    Some(normalized.trim_end_matches(".git").to_string())
}

fn quality_runner_identity_key(repository: &RepositorySnapshot) -> String {
    if let Some(remote) = repository.remote_url.as_deref() {
        let mut normalized = remote.trim().trim_end_matches('/').to_ascii_lowercase();
        if let Some(value) = normalized.strip_prefix("git@") {
            if let Some((host, path)) = value.split_once(':') {
                normalized = format!("{host}/{path}");
            }
        } else {
            for prefix in ["https://", "http://", "ssh://"] {
                if let Some(value) = normalized.strip_prefix(prefix) {
                    normalized = value.trim_start_matches('/').to_string();
                    if let Some(value) = normalized.strip_prefix("git@") {
                        normalized = value.to_string();
                    }
                    break;
                }
            }
            if let Some((host, path)) = normalized.split_once(':') {
                if !path.contains('/') || host.contains('.') {
                    normalized = format!("{host}/{path}");
                }
            }
        }
        return format!(
            "origin:{}",
            normalized.trim_start_matches('/').trim_end_matches(".git")
        );
    }
    let common = git_static(
        Path::new(&repository.path),
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .and_then(|path| canonical_path(Path::new(&path)).or_else(|| Some(PathBuf::from(path))));
    common
        .map(|path| format!("common:{}", path.display()))
        .unwrap_or_else(|| format!("path:{}", repository.path))
}

fn repository_feed_id(repository: &RepositorySnapshot) -> String {
    let identity = quality_runner_identity_key(repository);
    let payload = serde_json::to_string(&[identity]).unwrap_or_else(|_| "[]".to_string());
    let digest = Sha256::digest(payload.as_bytes());
    let hex = format!("{digest:x}");
    format!("repo-{}", &hex[..16])
}

fn classify_remote_repositories(
    repositories: &[RepositorySnapshot],
    remote_repositories: Vec<RemoteRepositorySnapshot>,
) -> Vec<RemoteRepositorySnapshot> {
    let local_names = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .remote_url
                .as_deref()
                .and_then(normalize_remote_name)
        })
        .collect::<HashSet<_>>();

    remote_repositories
        .into_iter()
        .filter_map(|mut remote| {
            let normalized_name = normalize_remote_name(&remote.full_name)?;
            remote.locality = if local_names.contains(&normalized_name) {
                "Local and remote".to_string()
            } else if remote.provider.eq_ignore_ascii_case("github") {
                remediation::GITHUB_ONLY_LOCALITY.to_string()
            } else {
                "Remote only".to_string()
            };
            Some(remote)
        })
        .collect()
}

fn apply_provider_refresh_at(
    path: &Path,
    refresh: ProviderRefresh,
    target_repository_ids: Option<&HashSet<String>>,
) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let remote_by_name = refresh
        .repositories
        .iter()
        .filter_map(|repository| {
            normalize_remote_name(&repository.full_name).map(|name| (name, repository))
        })
        .collect::<HashMap<_, _>>();
    for local in &mut state.repositories {
        if target_repository_ids.is_some_and(|targets| !targets.contains(&local.id)) {
            continue;
        }
        let Some(remote_name) = local.remote_url.as_deref().and_then(normalize_remote_name) else {
            continue;
        };
        if let Some(remote) = remote_by_name.get(&remote_name) {
            local.provider_state = format!("GitHub connected as {}", remote.identity_id);
            local.locality = "Local and remote".to_string();
            local.pull_requests = refresh
                .pull_requests
                .iter()
                .filter(|pull_request| pull_request.repository_id == remote.id)
                .cloned()
                .collect();
            local.releases = refresh
                .releases
                .iter()
                .filter(|release| release.repository_id == remote.id)
                .cloned()
                .collect();
        } else if remote_name.starts_with("github.com/") || local.provider_state.contains("GitHub")
        {
            local.provider_state =
                "GitHub repository unavailable to the connected identity".to_string();
        }
    }
    let refreshed_remote_repositories =
        classify_remote_repositories(&state.repositories, refresh.repositories);
    let remote_repositories = if let Some(targets) = target_repository_ids {
        let target_names = state
            .repositories
            .iter()
            .filter(|repository| targets.contains(&repository.id))
            .filter_map(|repository| {
                repository
                    .remote_url
                    .as_deref()
                    .and_then(normalize_remote_name)
            })
            .collect::<HashSet<_>>();
        let mut merged = state
            .remote_repositories
            .into_iter()
            .filter(|remote| {
                normalize_remote_name(&remote.full_name)
                    .map_or(true, |name| !target_names.contains(&name))
            })
            .collect::<Vec<_>>();
        merged.extend(refreshed_remote_repositories.into_iter().filter(|remote| {
            normalize_remote_name(&remote.full_name)
                .is_some_and(|name| target_names.contains(&name))
        }));
        merged
    } else {
        refreshed_remote_repositories
    };
    state.provider_identities = refresh.identities;
    state.remote_repositories = remote_repositories;
    state.provider_status = ProviderStatus {
        provider: "GitHub".to_string(),
        state: "Ready".to_string(),
        message: "Read-only GitHub context refreshed for connected local repositories and GitHub-only candidates.".to_string(),
        last_refresh_at: Some(refresh.refreshed_at),
        identity_count: state.provider_identities.len(),
        repository_count: state.remote_repositories.len(),
    };
    apply_quality_evidence_scoped(&mut state, target_repository_ids, None);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn apply_quality_evidence(state: &mut StoreState) {
    apply_quality_evidence_scoped(state, None, None);
}

fn persisted_fleet_audit_root(state: &StoreState) -> Option<PathBuf> {
    state
        .remediation
        .refresh_steps
        .iter()
        .find(|step| step.id == "qr_fleet_run")
        .and_then(|step| step.evidence_path.as_deref())
        .map(PathBuf::from)
        .filter(|root| root.is_dir())
}

fn apply_quality_evidence_scoped(
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    fleet_audit_root: Option<&Path>,
) {
    let persisted_fleet_root = persisted_fleet_audit_root(state);
    let fleet_audit_root = fleet_audit_root.or(persisted_fleet_root.as_deref());
    let feed_path = quality::canonical_maturity_feed_path();
    let audit = quality::maturity_feed_import(feed_path.as_deref(), &state.repositories);
    let fleet = quality::fleet_audit_import(fleet_audit_root, &state.repositories);
    let mac_control_scope = state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .filter(|repository| remediation::repository_requires_maturity(repository))
        .cloned()
        .collect::<Vec<_>>();
    let mac_control = mac_control_maturity::evaluate_canonical(&mac_control_scope);
    state.quality = audit.portfolio;
    state.quality.mac_control_ideal_state = mac_control.portfolio.clone();
    let remote_by_name = state
        .remote_repositories
        .iter()
        .filter_map(|remote| normalize_remote_name(&remote.full_name).map(|name| (name, remote)))
        .collect::<HashMap<_, _>>();
    for repository in &mut state.repositories {
        if target_repository_ids.is_some_and(|targets| !targets.contains(&repository.id)) {
            continue;
        }
        let remote = repository
            .remote_url
            .as_deref()
            .and_then(normalize_remote_name)
            .and_then(|name| remote_by_name.get(&name).copied());
        let target_fleet_import =
            repository
                .quality
                .target_fleet_audit_root
                .as_deref()
                .map(|root| {
                    quality::fleet_audit_import(
                        Some(Path::new(root)),
                        std::slice::from_ref(repository),
                    )
                });
        let target_provenance = repository.target_branch.as_deref().and_then(|branch| {
            repository
                .branches
                .iter()
                .find(|candidate| candidate.name == branch)
                .and_then(|candidate| candidate.last_commit.as_deref())
                .map(|commit| (branch.to_string(), commit.to_string()))
        });
        let target_fleet_evidence = target_fleet_import
            .as_ref()
            .and_then(|import| import.evidence.get(&repository.id))
            .map(|evidence| {
                let mut scoped = evidence.clone();
                if let Some((branch, commit)) = target_provenance.as_ref() {
                    quality::scope_fleet_audit_evidence_to_target(&mut scoped, branch, commit);
                }
                scoped
            });
        let fleet_evidence = target_fleet_evidence
            .as_ref()
            .or_else(|| fleet.evidence.get(&repository.id));
        let maturity = target_fleet_evidence
            .as_ref()
            .map(|evidence| evidence.maturity.clone())
            .or_else(|| audit.maturities.get(&repository.id).cloned())
            .or_else(|| fleet_evidence.map(|evidence| evidence.maturity.clone()));
        let ideal_gate_ids = quality::ideal_gate_ids_for_repository(repository);
        let mut imported = quality::ingest_repository_quality(
            repository,
            remote,
            maturity,
            ideal_gate_ids.as_deref(),
        );
        imported.mac_control_ideal_state = mac_control
            .by_repository
            .get(&repository.id)
            .cloned()
            .unwrap_or_default();
        imported.target_fleet_audit_root = repository.quality.target_fleet_audit_root.clone();
        if let Some(fleet_evidence) = fleet_evidence {
            if target_fleet_evidence.is_some()
                || imported.maturity.score.is_none()
                || (imported.maturity.freshness != quality::QualityFreshness::Fresh
                    && fleet_evidence.maturity.freshness == quality::QualityFreshness::Fresh)
            {
                imported.maturity = fleet_evidence.maturity.clone();
            }
            let stable_detector_report =
                quality::is_stable_detector_report(imported.findings.report_path.as_deref());
            if target_fleet_evidence.is_some()
                || (!stable_detector_report
                    && (imported.findings.source.is_none()
                        || (imported.findings.freshness != quality::QualityFreshness::Fresh
                            && fleet_evidence.findings.freshness
                                == quality::QualityFreshness::Fresh)))
            {
                imported.findings = fleet_evidence.findings.clone();
            }
            if imported.last_ingested_at.is_none() {
                imported.last_ingested_at = fleet_evidence.findings.observed_at.clone();
            }
            imported.ingestion_status = "Available".to_string();
            imported.ingestion_message = None;
        }
        if repository.target_branch_configured {
            if let Some((target_branch, target_commit)) = target_provenance.as_ref() {
                quality::project_quality_snapshot_for_target(
                    &mut imported,
                    target_branch,
                    target_commit,
                );
            }
        }
        quality::reconcile_finding_dispositions(
            Path::new(&repository.path),
            &mut imported.findings,
        );
        repository.quality = imported;
    }
    for repository in &mut state.repositories {
        if let Some(mac_control_state) = mac_control.by_repository.get(&repository.id) {
            repository.quality.mac_control_ideal_state = mac_control_state.clone();
        }
    }
    quality::update_ci_readiness_summary(&mut state.quality, &state.repositories);
    state.remediation = remediation::rebuild_run_with_fleet_root(
        &state.repositories,
        &state.remediation,
        state.quality.latest_audit_id.as_deref(),
        fleet_audit_root,
    );
}

fn refresh_quality_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    let mut state = load_store(path)?;
    apply_quality_evidence(&mut state);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn maturity_coverage_gaps(repositories: &[RepositorySnapshot]) -> Vec<String> {
    repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .filter(|repository| remediation::repository_requires_maturity(repository))
        .filter_map(|repository| {
            let maturity = &repository.quality.maturity;
            if maturity.score.is_none() {
                Some(format!("{} (missing)", repository.name))
            } else if maturity.freshness != QualityFreshness::Fresh {
                Some(format!(
                    "{} ({})",
                    repository.name,
                    maturity.freshness.as_str().to_ascii_lowercase()
                ))
            } else if remediation::repository_requires_maturity_gate(
                repository,
                mac_control_maturity::MAC_CONTROL_GATE_ID,
            ) && !(repository.quality.mac_control_ideal_state.ideal_state
                || (repository.quality.mac_control_ideal_state.status == "Not applicable"
                    && repository.quality.mac_control_ideal_state.freshness == "Fresh"))
            {
                Some(format!(
                    "{} (Mac Control ideal state: {} / {})",
                    repository.name,
                    repository.quality.mac_control_ideal_state.status,
                    repository.quality.mac_control_ideal_state.freshness,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn refresh_github_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    let adapter = GitHubCliAdapter::default();
    match adapter.refresh() {
        Ok(refresh) => apply_provider_refresh_at(path, refresh, None),
        Err(error) => {
            let mut state = load_store(path)?;
            state.provider_status = ProviderStatus {
                provider: "GitHub".to_string(),
                state: "Unavailable".to_string(),
                message: error.clone(),
                last_refresh_at: state.provider_status.last_refresh_at.clone(),
                identity_count: state.provider_identities.len(),
                repository_count: state.remote_repositories.len(),
            };
            apply_release_threshold_conditions(&mut state);
            save_store(path, &state)?;
            Ok(snapshot_from_store(path, &state))
        }
    }
}

fn refresh_github_scoped_at(
    path: &Path,
    target_repository_ids: &HashSet<String>,
) -> Result<PortfolioSnapshot, String> {
    let target_repository_names = load_store(path)?
        .repositories
        .iter()
        .filter(|repository| target_repository_ids.contains(&repository.id))
        .filter_map(|repository| {
            repository
                .remote_url
                .as_deref()
                .and_then(normalize_remote_name)
        })
        .collect::<HashSet<_>>();
    let adapter = GitHubCliAdapter::for_repository_names(target_repository_names);
    match adapter.refresh() {
        Ok(refresh) => apply_provider_refresh_at(path, refresh, Some(target_repository_ids)),
        Err(error) => {
            let mut state = load_store(path)?;
            state.provider_status = ProviderStatus {
                provider: "GitHub".to_string(),
                state: "Unavailable".to_string(),
                message: error.clone(),
                last_refresh_at: state.provider_status.last_refresh_at.clone(),
                identity_count: state.provider_identities.len(),
                repository_count: state.remote_repositories.len(),
            };
            apply_release_threshold_conditions(&mut state);
            save_store(path, &state)?;
            Err(error)
        }
    }
}

fn remediation_refresh_steps() -> Vec<remediation::RemediationRefreshStep> {
    [
        ("qr_doctor", "Quality Runner doctor"),
        ("local_scan", "Scoped local repository scan"),
        ("qr_fleet_run", "Fresh Quality Runner fleet run"),
        ("qr_replay", "Quality Runner replay verification"),
        ("qr_report", "Quality Runner aggregate report"),
        ("qr_feed", "Quality Runner maturity feed"),
        ("provider", "GitHub provider refresh"),
        ("quality_import", "Pronto quality and maturity import"),
        ("remediation_plan", "Ranked active remediation queue"),
    ]
    .into_iter()
    .map(|(id, label)| remediation::RemediationRefreshStep {
        id: id.to_string(),
        label: label.to_string(),
        status: "pending".to_string(),
        ..remediation::RemediationRefreshStep::default()
    })
    .collect()
}

fn set_remediation_refresh_step(
    steps: &mut [remediation::RemediationRefreshStep],
    step_id: &str,
    status: &str,
    detail: impl Into<String>,
    evidence_path: Option<String>,
) {
    let Some(step) = steps.iter_mut().find(|step| step.id == step_id) else {
        return;
    };
    let now = iso_now();
    if step.started_at.is_none() {
        step.started_at = Some(now.clone());
    }
    step.status = status.to_string();
    step.detail = detail.into();
    step.evidence_path = evidence_path;
    if matches!(status, "completed" | "blocked" | "skipped") {
        step.completed_at = Some(now);
    }
}

fn persist_remediation_refresh(
    path: &Path,
    refresh_id: &str,
    status: &str,
    message: Option<String>,
    steps: &[remediation::RemediationRefreshStep],
) -> Result<(), String> {
    let mut state = load_store(path)?;
    remediation::sync_scope_metadata(&mut state.remediation, &state.repositories);
    if state.remediation.id.is_empty() {
        state.remediation = remediation::rebuild_run(
            &state.repositories,
            &state.remediation,
            state.quality.latest_audit_id.as_deref(),
        );
    }
    let eligible = state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .collect::<Vec<_>>();
    remediation::set_refresh_metadata(
        &mut state.remediation,
        refresh_id,
        status,
        message,
        eligible
            .iter()
            .map(|repository| repository.id.clone())
            .collect(),
        eligible
            .iter()
            .map(|repository| repository.path.clone())
            .collect(),
        steps.to_vec(),
    );
    save_store(path, &state)
}

fn fail_remediation_refresh(
    path: &Path,
    refresh_id: &str,
    steps: &mut [remediation::RemediationRefreshStep],
    step_id: &str,
    error: String,
) -> Result<PortfolioSnapshot, String> {
    set_remediation_refresh_step(steps, step_id, "blocked", error.clone(), None);
    let _ = persist_remediation_refresh(path, refresh_id, "blocked", Some(error.clone()), steps);
    Err(error)
}

fn json_from_process_output(output: &std::process::Output) -> Result<serde_json::Value, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&stdout) {
        return Ok(payload);
    }
    let payload = stdout
        .find('{')
        .and_then(|start| stdout.rfind('}').map(|end| (start, end)))
        .and_then(|(start, end)| serde_json::from_str(&stdout[start..=end]).ok());
    payload.ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            "Quality Runner returned invalid or missing JSON on stdout.".to_string()
        } else {
            format!("Quality Runner returned invalid or missing JSON on stdout ({stderr})")
        }
    })
}

fn run_json_command(executable: &str, arguments: &[String]) -> Result<serde_json::Value, String> {
    run_json_command_in(executable, arguments, None)
}

fn run_json_command_in_with_status(
    executable: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> Result<(serde_json::Value, bool, Option<String>), String> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    let output = command.output().map_err(|error| {
        current_dir.map_or_else(
            || format!("Could not run {executable}: {error}"),
            |path| format!("Could not run {executable} in {}: {error}", path.display()),
        )
    })?;
    let success = output.status.success();
    let detail = {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!stderr.is_empty())
            .then_some(stderr)
            .or_else(|| (!stdout.is_empty()).then_some(stdout))
    };
    let payload = json_from_process_output(&output).map_err(|error| {
        if success {
            error
        } else {
            format!(
                "{executable} {} failed with status {}{}",
                arguments.join(" "),
                output.status,
                detail
                    .as_deref()
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default()
            )
        }
    })?;
    Ok((payload, success, detail))
}

fn run_json_command_in(
    executable: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let (payload, success, detail) =
        run_json_command_in_with_status(executable, arguments, current_dir)?;
    if !success {
        return Err(format!(
            "{executable} {} failed with status {}{}",
            arguments.join(" "),
            "non-zero",
            if let Some(detail) = detail {
                format!(": {detail}")
            } else {
                String::new()
            }
        ));
    }
    Ok(payload)
}

fn target_evidence_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "target".to_string()
    } else {
        slug.chars().take(80).collect()
    }
}

fn target_evidence_run_prefix(repository: &RepositorySnapshot, target_branch: &str) -> String {
    let sequence = NEXT_TARGET_EVIDENCE_ID.fetch_add(1, Ordering::Relaxed);
    format!(
        "pronto-target-{}-{}-{}-{}",
        target_evidence_slug(&repository.id),
        target_evidence_slug(target_branch),
        target_evidence_slug(&iso_now()),
        sequence,
    )
}

fn target_evidence_artifact_parent() -> PathBuf {
    std::env::temp_dir().join("pronto-target-evidence")
}

fn copy_target_evidence_tree_inner(
    source: &Path,
    destination: &Path,
    old_root: &str,
    new_root: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(source)
        .map_err(|error| {
            format!(
                "Could not inspect target evidence {}: {error}",
                source.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        fs::create_dir_all(destination).map_err(|error| {
            format!(
                "Could not create copied target evidence directory {}: {error}",
                destination.display()
            )
        })?;
        let mut copied = 0;
        let entries = fs::read_dir(source).map_err(|error| {
            format!(
                "Could not read target evidence directory {}: {error}",
                source.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Could not enumerate target evidence directory {}: {error}",
                    source.display()
                )
            })?;
            copied += copy_target_evidence_tree_inner(
                &entry.path(),
                &destination.join(entry.file_name()),
                old_root,
                new_root,
            )?;
        }
        return Ok(copied);
    }
    if !file_type.is_file() {
        return Ok(0);
    }
    if destination.exists() {
        return Err(format!(
            "Target evidence destination already exists: {}",
            destination.display()
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Could not create copied target evidence parent {}: {error}",
                parent.display()
            )
        })?;
    }
    let bytes = fs::read(source).map_err(|error| {
        format!(
            "Could not read target evidence file {}: {error}",
            source.display()
        )
    })?;
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        fs::write(destination, text.replace(old_root, new_root)).map_err(|error| {
            format!(
                "Could not write copied target evidence file {}: {error}",
                destination.display()
            )
        })?;
    } else {
        fs::write(destination, bytes).map_err(|error| {
            format!(
                "Could not write copied target evidence file {}: {error}",
                destination.display()
            )
        })?;
    }
    Ok(1)
}

fn copy_target_evidence_tree(
    source: &Path,
    destination: &Path,
    old_root: &Path,
    new_root: &Path,
) -> Result<usize, String> {
    copy_target_evidence_tree_inner(
        source,
        destination,
        &old_root.to_string_lossy(),
        &new_root.to_string_lossy(),
    )
}

fn rewrite_target_qr_branch_provenance_value(
    value: &mut serde_json::Value,
    target_branch: &str,
) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            let mut rewritten = false;
            for (key, child) in object.iter_mut() {
                if key == "branch" && child.as_str() == Some("HEAD") {
                    *child = serde_json::Value::String(target_branch.to_string());
                    rewritten = true;
                } else if key == "ref" && child.as_str() == Some("refs/heads/HEAD") {
                    *child = serde_json::Value::String(format!("refs/heads/{target_branch}"));
                    rewritten = true;
                }
                rewritten |= rewrite_target_qr_branch_provenance_value(child, target_branch);
            }
            rewritten
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .map(|item| rewrite_target_qr_branch_provenance_value(item, target_branch))
            .any(|rewritten| rewritten),
        _ => false,
    }
}

fn rewrite_target_qr_branch_provenance_inner(
    root: &Path,
    target_branch: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(root)
        .map_err(|error| {
            format!(
                "Could not inspect target QR artifact {}: {error}",
                root.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        let mut rewritten = 0;
        for entry in fs::read_dir(root).map_err(|error| {
            format!(
                "Could not read target QR artifact directory {}: {error}",
                root.display()
            )
        })? {
            rewritten += rewrite_target_qr_branch_provenance_inner(
                &entry
                    .map_err(|error| format!("Could not enumerate target QR artifacts: {error}"))?
                    .path(),
                target_branch,
            )?;
        }
        return Ok(rewritten);
    }
    if !file_type.is_file() || root.extension().and_then(|value| value.to_str()) != Some("json") {
        return Ok(0);
    }
    let bytes = fs::read(root).map_err(|error| {
        format!(
            "Could not read target QR artifact {}: {error}",
            root.display()
        )
    })?;
    let Ok(mut payload) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(0);
    };
    if !rewrite_target_qr_branch_provenance_value(&mut payload, target_branch) {
        return Ok(0);
    }
    let mut encoded = serde_json::to_vec_pretty(&payload).map_err(|error| {
        format!(
            "Could not encode target QR artifact {}: {error}",
            root.display()
        )
    })?;
    encoded.push(b'\n');
    fs::write(root, encoded).map_err(|error| {
        format!(
            "Could not rewrite target QR artifact {}: {error}",
            root.display()
        )
    })?;
    Ok(1)
}

fn rewrite_target_qr_branch_provenance(root: &Path, target_branch: &str) -> Result<usize, String> {
    rewrite_target_qr_branch_provenance_inner(root, target_branch)
}

fn copy_target_qr_runs(
    target_worktree: &Path,
    repository_path: &Path,
    run_id_prefix: &str,
    target_branch: &str,
) -> Result<usize, String> {
    let source_root = target_worktree.join(".quality-runner").join("runs");
    if !source_root.is_dir() {
        return Ok(0);
    }
    let destination_root = repository_path.join(".quality-runner").join("runs");
    fs::create_dir_all(&destination_root).map_err(|error| {
        format!(
            "Could not create Pronto target QR artifact directory {}: {error}",
            destination_root.display()
        )
    })?;
    let mut copied = 0;
    for entry in fs::read_dir(&source_root).map_err(|error| {
        format!(
            "Could not read target QR artifact directory {}: {error}",
            source_root.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "Could not enumerate target QR artifact directory {}: {error}",
                source_root.display()
            )
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(run_id_prefix)
            || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
        {
            continue;
        }
        let destination = destination_root.join(name.as_ref());
        copied += copy_target_evidence_tree(
            &entry.path(),
            &destination,
            target_worktree,
            repository_path,
        )?;
        rewrite_target_qr_branch_provenance(&destination, target_branch)?;
    }
    Ok(copied)
}

fn rewrite_target_evidence_paths_inner(
    root: &Path,
    old_root: &str,
    new_root: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(root)
        .map_err(|error| {
            format!(
                "Could not inspect target audit artifact {}: {error}",
                root.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        let mut rewritten = 0;
        for entry in fs::read_dir(root).map_err(|error| {
            format!(
                "Could not read target audit artifact directory {}: {error}",
                root.display()
            )
        })? {
            rewritten += rewrite_target_evidence_paths_inner(
                &entry
                    .map_err(|error| {
                        format!("Could not enumerate target audit artifacts: {error}")
                    })?
                    .path(),
                old_root,
                new_root,
            )?;
        }
        return Ok(rewritten);
    }
    if !file_type.is_file() {
        return Ok(0);
    }
    let bytes = fs::read(root).map_err(|error| {
        format!(
            "Could not read target audit artifact {}: {error}",
            root.display()
        )
    })?;
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(0);
    };
    if !text.contains(old_root) {
        return Ok(0);
    }
    fs::write(root, text.replace(old_root, new_root)).map_err(|error| {
        format!(
            "Could not rewrite target audit artifact {}: {error}",
            root.display()
        )
    })?;
    Ok(1)
}

fn rewrite_target_evidence_paths(
    root: &Path,
    old_root: &Path,
    new_root: &Path,
) -> Result<usize, String> {
    rewrite_target_evidence_paths_inner(
        root,
        &old_root.to_string_lossy(),
        &new_root.to_string_lossy(),
    )
}

fn bounded_target_artifact_root(base: &Path, payload: &serde_json::Value) -> Option<PathBuf> {
    let candidate = json_string(payload, &["artifact_root"])?;
    let candidate = canonical_path(Path::new(&candidate))?;
    let base = canonical_path(base)?;
    candidate
        .starts_with(&base)
        .then_some(candidate)
        .filter(|path| path.is_dir())
}

fn concise_target_command_error(error: &str) -> String {
    let detail = error
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(error)
        .trim();
    let mut concise = detail.chars().take(300).collect::<String>();
    if detail.chars().count() > 300 {
        concise.push_str("…");
    }
    concise
}

fn run_target_qr_refresh(
    qr_executable: &str,
    target_worktree: &Path,
    repository_path: &Path,
    run_id_prefix: &str,
    target_branch: &str,
) -> Result<String, String> {
    let arguments = vec![
        "refresh".to_string(),
        target_worktree.to_string_lossy().to_string(),
        "--run-id-prefix".to_string(),
        run_id_prefix.to_string(),
        "--execute-gates".to_string(),
        "--worktree-mode".to_string(),
        "disposable".to_string(),
        "--no-progress".to_string(),
        "--json".to_string(),
        "--timeout-seconds".to_string(),
        TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS.to_string(),
        "--total-timeout-seconds".to_string(),
        TARGET_EVIDENCE_TOTAL_TIMEOUT_SECONDS.to_string(),
    ];
    let result = run_json_command_in_with_status(qr_executable, &arguments, Some(target_worktree));
    let copied = copy_target_qr_runs(
        target_worktree,
        repository_path,
        run_id_prefix,
        target_branch,
    )?;
    match result {
        Ok((payload, success, detail)) => {
            let status =
                json_string(&payload, &["status"]).unwrap_or_else(|| "completed".to_string());
            if success {
                Ok(format!(
                    "QR refresh {status}; copied {copied} artifact files"
                ))
            } else {
                Ok(format!(
                    "QR refresh {status}; retained failed gate evidence and copied {copied} artifact files{}",
                    detail
                        .as_deref()
                        .map(|value| format!(": {}", concise_target_command_error(value)))
                        .unwrap_or_default()
                ))
            }
        }
        Err(error) => Err(format!(
            "QR refresh failed after copying {copied} artifact files: {}",
            concise_target_command_error(&error)
        )),
    }
}

fn run_target_fleet_audit(
    qr_executable: &str,
    target_worktree: &Path,
    repository_path: &Path,
    projects_root: &Path,
    output_base: &Path,
    target_branch: &str,
    repository_id: &str,
) -> Result<(String, PathBuf), String> {
    fs::create_dir_all(output_base).map_err(|error| {
        format!(
            "Could not create target fleet audit output directory {}: {error}",
            output_base.display()
        )
    })?;
    let arguments = vec![
        "fleet".to_string(),
        "audit".to_string(),
        "run".to_string(),
        "--repo-path".to_string(),
        target_worktree.to_string_lossy().to_string(),
        "--projects-root".to_string(),
        projects_root.to_string_lossy().to_string(),
        "--output-dir".to_string(),
        output_base.to_string_lossy().to_string(),
        "--dynamic".to_string(),
        "--no-changed-only".to_string(),
        "--timeout-seconds".to_string(),
        TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS.to_string(),
        "--target-override".to_string(),
        format!("{repository_id}={target_branch}"),
        "--json".to_string(),
    ];
    let payload = run_json_command_in(qr_executable, &arguments, Some(target_worktree))?;
    let artifact_root = bounded_target_artifact_root(output_base, &payload).ok_or_else(|| {
        format!(
            "Quality Runner fleet audit did not return a bounded artifact root under {}",
            output_base.display()
        )
    })?;
    let rewritten =
        rewrite_target_evidence_paths(&artifact_root, target_worktree, repository_path)?;
    let status = json_string(&payload, &["status"]).unwrap_or_else(|| "completed".to_string());
    Ok((
        format!(
            "fleet audit {status}; ingested {} rewritten artifact files",
            rewritten
        ),
        artifact_root,
    ))
}

fn json_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    if let Some(object) = value.as_object() {
        for key in keys {
            if let Some(item) = object.get(*key).and_then(serde_json::Value::as_str) {
                return Some(item.to_string());
            }
        }
        for child in object.values() {
            if let Some(found) = json_string(child, keys) {
                return Some(found);
            }
        }
    } else if let Some(items) = value.as_array() {
        for child in items {
            if let Some(found) = json_string(child, keys) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_qr_executable(requested: Option<&str>) -> String {
    if let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        return requested.to_string();
    }
    if let Some(from_environment) = std::env::var_os("PRONTO_QR_BIN") {
        if !from_environment.is_empty() {
            return from_environment.to_string_lossy().to_string();
        }
    }
    [
        "/Users/jakyeamos/projects/quality-runner/.venv/bin/qr",
        "qr",
        "quality-runner",
    ]
    .iter()
    .find(|candidate| !candidate.contains('/') || Path::new(candidate).is_file())
    .unwrap_or(&"qr")
    .to_string()
}

fn qr_projects_root(repository_paths: &[String]) -> Result<PathBuf, String> {
    let home_projects = dirs::home_dir().map(|home| home.join("projects"));
    if let Some(root) = home_projects.filter(|root| {
        root.is_dir()
            && repository_paths
                .iter()
                .all(|path| Path::new(path).starts_with(root))
    }) {
        return Ok(root);
    }
    let mut common = PathBuf::from(
        repository_paths
            .first()
            .ok_or_else(|| "No eligible repositories are registered for QR.".to_string())?,
    );
    for path in repository_paths.iter().skip(1) {
        let candidate = Path::new(path);
        while !candidate.starts_with(&common) {
            if !common.pop() {
                return Err("Could not derive a bounded Quality Runner projects root.".to_string());
            }
        }
    }
    if common == Path::new("/") || !common.is_dir() {
        return Err(format!(
            "Quality Runner scope root is not bounded to a usable directory: {}",
            common.display()
        ));
    }
    Ok(common)
}

fn process_qr_audit_lifecycle(
    path: &Path,
    refresh_id: &str,
    steps: &mut [remediation::RemediationRefreshStep],
    qr: &str,
    audit_id: &str,
    artifact_root: Option<String>,
) -> Result<bool, (String, String)> {
    let qr_audit_commands = [
        (
            "qr_replay",
            "replay",
            "Replay passed and the fleet artifacts are deterministic.",
        ),
        (
            "qr_report",
            "report",
            "Aggregate QR report written for review.",
        ),
        (
            "qr_feed",
            "feed",
            "Canonical maturity feed published from the replay-validated audit.",
        ),
    ];
    for (step_id, action, success_detail) in qr_audit_commands {
        set_remediation_refresh_step(
            steps,
            step_id,
            "in_progress",
            format!("Running qr fleet audit {action} for {audit_id}."),
            None,
        );
        if let Err(error) =
            persist_remediation_refresh(path, refresh_id, "in_progress", None, steps)
        {
            return Err((step_id.to_string(), error));
        }
        let arguments = vec![
            "fleet".to_string(),
            "audit".to_string(),
            action.to_string(),
            "--audit-id".to_string(),
            audit_id.to_string(),
            "--json".to_string(),
        ];
        match run_json_command(qr, &arguments) {
            Ok(payload) => {
                let status = json_string(&payload, &["status"]).unwrap_or_default();
                let valid = match step_id {
                    "qr_replay" => {
                        status == "passed"
                            && payload
                                .get("deterministic")
                                .and_then(serde_json::Value::as_bool)
                                == Some(true)
                    }
                    "qr_feed" => status == "published",
                    _ => status == "review_required",
                };
                if !valid {
                    let detail = format!("Quality Runner {action} returned status {status}.");
                    if step_id == "qr_feed" {
                        set_remediation_refresh_step(steps, step_id, "blocked", detail, None);
                        if let Err(error) = persist_remediation_refresh(
                            path,
                            refresh_id,
                            "partial",
                            Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                            steps,
                        ) {
                            return Err((step_id.to_string(), error));
                        }
                        return Ok(false);
                    }
                    return Err((step_id.to_string(), detail));
                }
                let evidence_path = json_string(&payload, &["feed_path", "artifact_root"])
                    .or_else(|| artifact_root.clone());
                set_remediation_refresh_step(
                    steps,
                    step_id,
                    "completed",
                    success_detail,
                    evidence_path,
                );
            }
            Err(error) if step_id == "qr_feed" => {
                set_remediation_refresh_step(steps, step_id, "blocked", error, None);
                if let Err(error) = persist_remediation_refresh(
                    path,
                    refresh_id,
                    "partial",
                    Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                    steps,
                ) {
                    return Err((step_id.to_string(), error));
                }
                return Ok(false);
            }
            Err(error) => return Err((step_id.to_string(), error)),
        }
        if let Err(error) =
            persist_remediation_refresh(path, refresh_id, "in_progress", None, steps)
        {
            return Err((step_id.to_string(), error));
        }
    }
    Ok(true)
}

fn refresh_remediation_at(
    path: &Path,
    qr_executable: Option<&str>,
    dynamic: bool,
    changed_only: bool,
    skip_provider: bool,
    timeout_seconds: u64,
) -> Result<PortfolioSnapshot, String> {
    let initial_state = load_store(path)?;
    let eligible_paths = initial_state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .map(|repository| repository.path.clone())
        .collect::<Vec<_>>();
    if eligible_paths.is_empty() {
        return Err("No eligible repositories remain for remediation refresh.".to_string());
    }
    let refresh_id = format!("remediation-refresh-{}", iso_now().replace([':', '-'], ""));
    let mut steps = remediation_refresh_steps();
    let _ = persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps);

    let qr = resolve_qr_executable(qr_executable);
    set_remediation_refresh_step(
        &mut steps,
        "qr_doctor",
        "in_progress",
        format!("Running {qr} doctor before any QR audit."),
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let doctor = match run_json_command(&qr, &["doctor".to_string(), "--json".to_string()]) {
        Ok(payload) => payload,
        Err(error) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, "qr_doctor", error)
        }
    };
    let doctor_status = json_string(&doctor, &["status"]).unwrap_or_else(|| "unknown".to_string());
    if doctor_status != "ready" {
        return fail_remediation_refresh(
            path,
            &refresh_id,
            &mut steps,
            "qr_doctor",
            format!("Quality Runner doctor did not report ready (status: {doctor_status})."),
        );
    }
    set_remediation_refresh_step(
        &mut steps,
        "qr_doctor",
        "completed",
        "Quality Runner doctor reported ready.",
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    set_remediation_refresh_step(
        &mut steps,
        "local_scan",
        "in_progress",
        "Refreshing local Git/workspace evidence for eligible repositories only.",
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let mut state = load_store(path)?;
    let eligible_ids = state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .map(|repository| repository.id.clone())
        .collect::<HashSet<_>>();
    if let Err(error) = audited_scan_and_persist_scoped(
        path,
        &mut state,
        Some(&eligible_ids),
        Some("eligible repositories"),
    ) {
        return fail_remediation_refresh(path, &refresh_id, &mut steps, "local_scan", error);
    }
    set_remediation_refresh_step(
        &mut steps,
        "local_scan",
        "completed",
        "Local evidence refreshed for eligible repositories.",
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let projects_root = qr_projects_root(&eligible_paths)?;
    let all_projects_scope = dirs::home_dir()
        .map(|home| projects_root == home.join("projects"))
        .unwrap_or(false);
    let mut fleet_arguments = vec!["fleet".to_string(), "audit".to_string(), "run".to_string()];
    if all_projects_scope {
        fleet_arguments.extend(["--all".to_string(), "--projects-root".to_string()]);
        fleet_arguments.push(projects_root.to_string_lossy().to_string());
    } else {
        fleet_arguments.extend([
            "--projects-root".to_string(),
            projects_root.to_string_lossy().to_string(),
        ]);
        for repository_path in &eligible_paths {
            fleet_arguments.push("--repo-path".to_string());
            fleet_arguments.push(repository_path.clone());
        }
    }
    append_qr_audit_runtime_arguments(&mut fleet_arguments, dynamic, changed_only, timeout_seconds);
    set_remediation_refresh_step(
        &mut steps,
        "qr_fleet_run",
        "in_progress",
        format!(
            "Running a fresh QR fleet audit from {}.",
            projects_root.display()
        ),
        None,
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
    let fleet = match run_json_command(&qr, &fleet_arguments) {
        Ok(payload) => payload,
        Err(error) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, "qr_fleet_run", error)
        }
    };
    let audit_id = match json_string(&fleet, &["audit_id"]) {
        Some(audit_id) => audit_id,
        None => {
            return fail_remediation_refresh(
                path,
                &refresh_id,
                &mut steps,
                "qr_fleet_run",
                "Quality Runner fleet run completed without an audit_id.".to_string(),
            )
        }
    };
    let artifact_root = json_string(&fleet, &["artifact_root"]);
    let scoped_artifact_root = artifact_root.clone();
    set_remediation_refresh_step(
        &mut steps,
        "qr_fleet_run",
        "completed",
        format!("Fresh QR audit {audit_id} completed."),
        artifact_root.clone(),
    );
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let mut feed_published = match process_qr_audit_lifecycle(
        path,
        &refresh_id,
        &mut steps,
        &qr,
        &audit_id,
        artifact_root.clone(),
    ) {
        Ok(published) => published,
        Err((step_id, error)) => {
            return fail_remediation_refresh(path, &refresh_id, &mut steps, &step_id, error)
        }
    };

    if !feed_published && !all_projects_scope {
        if let Some(canonical_root) = dirs::home_dir()
            .map(|home| home.join("projects"))
            .filter(|root| root.is_dir())
        {
            let canonical_arguments = {
                let mut arguments = vec![
                    "fleet".to_string(),
                    "audit".to_string(),
                    "run".to_string(),
                    "--all".to_string(),
                    "--projects-root".to_string(),
                    canonical_root.to_string_lossy().to_string(),
                ];
                append_qr_audit_runtime_arguments(
                    &mut arguments,
                    dynamic,
                    changed_only,
                    timeout_seconds,
                );
                arguments
            };
            set_remediation_refresh_step(
                &mut steps,
                "qr_fleet_run",
                "in_progress",
                format!(
                    "Scoped QR feed was not publishable; running the canonical all-projects QR audit from {}.",
                    canonical_root.display()
                ),
                None,
            );
            persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
            match run_json_command(&qr, &canonical_arguments) {
                Ok(canonical_fleet) => {
                    if let Some(canonical_audit_id) = json_string(&canonical_fleet, &["audit_id"]) {
                        let canonical_artifact_root =
                            json_string(&canonical_fleet, &["artifact_root"]);
                        set_remediation_refresh_step(
                            &mut steps,
                            "qr_fleet_run",
                            "completed",
                            format!(
                                "Scoped QR audit {audit_id} completed; canonical all-projects audit {canonical_audit_id} completed for maturity publication."
                            ),
                            scoped_artifact_root.clone(),
                        );
                        persist_remediation_refresh(
                            path,
                            &refresh_id,
                            "in_progress",
                            None,
                            &steps,
                        )?;
                        feed_published = match process_qr_audit_lifecycle(
                            path,
                            &refresh_id,
                            &mut steps,
                            &qr,
                            &canonical_audit_id,
                            canonical_artifact_root,
                        ) {
                            Ok(published) => published,
                            Err((step_id, error)) => {
                                return fail_remediation_refresh(
                                    path,
                                    &refresh_id,
                                    &mut steps,
                                    &step_id,
                                    error,
                                )
                            }
                        };
                        if feed_published {
                            if let Some(step) = steps.iter_mut().find(|step| step.id == "qr_feed") {
                                step.detail = format!(
                                    "Canonical maturity feed published from all-projects audit {canonical_audit_id}; scoped audit {audit_id} remains the per-repository evidence run."
                                );
                            }
                        }
                    } else {
                        let error =
                            "Canonical Quality Runner fleet run completed without an audit_id."
                                .to_string();
                        set_remediation_refresh_step(
                            &mut steps,
                            "qr_fleet_run",
                            "completed",
                            format!(
                                "Scoped QR audit {audit_id} completed, but the canonical all-projects maturity audit was incomplete: {error}"
                            ),
                            artifact_root.clone(),
                        );
                        set_remediation_refresh_step(&mut steps, "qr_feed", "blocked", error, None);
                        persist_remediation_refresh(
                            path,
                            &refresh_id,
                            "partial",
                            Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                            &steps,
                        )?;
                    }
                }
                Err(error) => {
                    set_remediation_refresh_step(
                        &mut steps,
                        "qr_fleet_run",
                        "completed",
                        format!(
                            "Scoped QR audit {audit_id} completed, but the canonical all-projects maturity audit failed: {error}"
                        ),
                        artifact_root.clone(),
                    );
                    set_remediation_refresh_step(
                        &mut steps,
                        "qr_feed",
                        "blocked",
                        format!(
                            "Canonical all-projects maturity publication failed after scoped audit {audit_id}: {error}"
                        ),
                        None,
                    );
                    persist_remediation_refresh(
                        path,
                        &refresh_id,
                        "partial",
                        Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                        &steps,
                    )?;
                }
            }
        }
    }

    if skip_provider {
        set_remediation_refresh_step(
            &mut steps,
            "provider",
            "skipped",
            "Provider refresh was explicitly skipped; existing provider evidence was retained.",
            None,
        );
    } else {
        set_remediation_refresh_step(
            &mut steps,
            "provider",
            "in_progress",
            "Refreshing GitHub context for eligible repositories.",
            None,
        );
        persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;
        match refresh_github_scoped_at(path, &eligible_ids) {
            Ok(_) => set_remediation_refresh_step(
                &mut steps,
                "provider",
                "completed",
                "GitHub provider context refreshed for eligible repositories.",
                None,
            ),
            Err(error) => {
                set_remediation_refresh_step(&mut steps, "provider", "blocked", error, None)
            }
        }
    }
    persist_remediation_refresh(path, &refresh_id, "in_progress", None, &steps)?;

    let mut final_state = load_store(path)?;
    let scoped_fleet_root = scoped_artifact_root.as_deref().map(Path::new);
    apply_quality_evidence_scoped(&mut final_state, Some(&eligible_ids), scoped_fleet_root);
    let maturity_repository_count = final_state
        .repositories
        .iter()
        .filter(|repository| !remediation::is_excluded_repository(repository))
        .filter(|repository| remediation::repository_requires_maturity(repository))
        .count();
    let maturity_gaps = maturity_coverage_gaps(&final_state.repositories);
    let quality_import_completed = feed_published && maturity_gaps.is_empty();
    let quality_import_detail = if !feed_published {
        "The canonical QR maturity feed was not published; prior maturity evidence was retained."
            .to_string()
    } else if maturity_gaps.is_empty() {
        format!(
            "Pronto imported the canonical feed plus replay-validated scoped audit evidence; all {maturity_repository_count} maturity-applicable repositories have fresh scores, and CI ideal-state projections were refreshed."
        )
    } else {
        format!(
            "The canonical QR maturity feed was published, but the checkpoint is incomplete because {} maturity-applicable repositories lack fresh scores: {}.",
            maturity_gaps.len(),
            maturity_gaps.join(", ")
        )
    };
    set_remediation_refresh_step(
        &mut steps,
        "quality_import",
        if quality_import_completed {
            "completed"
        } else {
            "blocked"
        },
        quality_import_detail,
        scoped_artifact_root
            .clone()
            .or_else(|| final_state.quality.latest_audit_path.clone()),
    );
    set_remediation_refresh_step(
        &mut steps,
        "remediation_plan",
        "completed",
        "Ranked active repository plans and retained terminal closure records.",
        None,
    );
    let has_blockers = steps.iter().any(|step| step.status == "blocked");
    final_state.remediation = remediation::rebuild_run_with_fleet_root(
        &final_state.repositories,
        &final_state.remediation,
        final_state.quality.latest_audit_id.as_deref(),
        scoped_fleet_root,
    );
    remediation::set_refresh_metadata(
        &mut final_state.remediation,
        &refresh_id,
        if has_blockers { "partial" } else { "completed" },
        has_blockers.then(|| {
            steps
                .iter()
                .filter(|step| step.status == "blocked")
                .map(|step| format!("{}: {}", step.label, step.detail))
                .collect::<Vec<_>>()
                .join(" ")
        }),
        final_state
            .repositories
            .iter()
            .filter(|repository| !remediation::is_excluded_repository(repository))
            .map(|repository| repository.id.clone())
            .collect(),
        final_state
            .repositories
            .iter()
            .filter(|repository| !remediation::is_excluded_repository(repository))
            .map(|repository| repository.path.clone())
            .collect(),
        steps,
    );
    apply_release_threshold_conditions(&mut final_state);
    save_store(path, &final_state)?;
    Ok(snapshot_from_store(path, &final_state))
}

fn git_owned(path: &Path, arguments: Vec<String>) -> Option<String> {
    let result = run_git(path, arguments).ok()?;
    if result.success {
        Some(result.stdout.trim().to_string())
    } else {
        None
    }
}

fn path_id(prefix: &str, path: &Path) -> String {
    format!("{prefix}:{}", path.to_string_lossy())
}

fn sort_repositories_by_name(repositories: &mut [RepositorySnapshot]) {
    repositories.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn generated_config_id(kind: &str, name: &str) -> String {
    let slug = name
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character == '-' || character == '_' {
                Some(character)
            } else {
                None
            }
        })
        .take(48)
        .collect::<String>();
    let slug = if slug.is_empty() { "item" } else { &slug };
    let sequence = NEXT_CONFIG_ID.fetch_add(1, Ordering::Relaxed);
    format!("{kind}:{slug}:{sequence}")
}

fn normalize_name(value: &str, kind: &str) -> Result<String, String> {
    let name = value.trim();
    if name.is_empty() {
        return Err(format!("{kind} name cannot be empty"));
    }
    if name.chars().count() > 80 {
        return Err(format!("{kind} name must be 80 characters or fewer"));
    }
    Ok(name.to_string())
}

fn normalize_repository_ids(
    state: &StoreState,
    repository_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let known_ids = state
        .repositories
        .iter()
        .map(|repository| repository.id.as_str())
        .collect::<HashSet<_>>();
    let mut normalized = repository_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if let Some(unknown) = normalized
        .iter()
        .find(|repository_id| !known_ids.contains(repository_id.as_str()))
    {
        return Err(format!("Repository {unknown} is not registered"));
    }
    Ok(normalized)
}

fn normalize_ignore_patterns(patterns: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for pattern in patterns {
        let value = pattern.trim().trim_matches('/').to_string();
        if value.is_empty() {
            continue;
        }
        if value == "."
            || value == ".."
            || value.contains('/')
            || value.contains('\\')
            || value.chars().count() > 120
        {
            return Err(format!(
                "Ignore pattern '{value}' must be a repository-relative name or suffix pattern"
            ));
        }
        normalized.push(value);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_refresh_policy(value: &str) -> Result<String, String> {
    match value.trim() {
        "Manual" => Ok("Manual".to_string()),
        "On open" => Ok("On open".to_string()),
        "Periodic" => Ok("Periodic".to_string()),
        _ => Err("Refresh policy must be Manual, On open, or Periodic".to_string()),
    }
}

fn normalize_lifecycle(value: &str) -> Result<String, String> {
    match value.trim() {
        "Unconfirmed" | "Active" | "Maintenance" | "Paused" | "Archived" => {
            Ok(value.trim().to_string())
        }
        _ => Err(
            "Lifecycle must be Unconfirmed, Active, Maintenance, Paused, or Archived".to_string(),
        ),
    }
}

fn normalize_release_mode(value: &str) -> Result<String, String> {
    match value.trim() {
        "Independent" => Ok("Independent".to_string()),
        "Coordinated independent versions" => Ok("Coordinated independent versions".to_string()),
        "Unified product version" => Ok("Unified product version".to_string()),
        _ => Err("Release mode is not supported".to_string()),
    }
}

fn normalize_ai_permission(value: &str) -> Result<String, String> {
    match value.trim() {
        "Disabled" => Ok("Disabled".to_string()),
        "Commit metadata only" => Ok("Commit metadata only".to_string()),
        "Committed diff allowed" => Ok("Committed diff allowed".to_string()),
        _ => Err(
            "AI permission must be Disabled, Commit metadata only, or Committed diff allowed"
                .to_string(),
        ),
    }
}

fn normalize_release_rule(rule: ReleaseRuleConfig) -> Result<ReleaseRuleConfig, String> {
    let name = normalize_name(&rule.name, "Release rule")?;
    let operator = match rule.operator.trim().to_ascii_uppercase().as_str() {
        "AND" => "AND".to_string(),
        "OR" => "OR".to_string(),
        _ => return Err("Release rule operator must be AND or OR".to_string()),
    };
    if rule.min_commits == Some(0) || rule.min_commits.is_some_and(|value| value > 100_000) {
        return Err("Minimum commits must be between 1 and 100000".to_string());
    }
    if rule.min_elapsed_days == Some(0) || rule.min_elapsed_days.is_some_and(|value| value > 36_500)
    {
        return Err("Minimum elapsed days must be between 1 and 36500".to_string());
    }
    let allowed_types = [
        "breaking", "feat", "fix", "perf", "docs", "refactor", "test", "chore",
    ];
    let mut required_commit_types = rule
        .required_commit_types
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    required_commit_types.sort();
    required_commit_types.dedup();
    if let Some(unknown) = required_commit_types
        .iter()
        .find(|value| !allowed_types.contains(&value.as_str()))
    {
        return Err(format!("Unsupported conventional commit type '{unknown}'"));
    }
    let mut required_quality_gates = rule
        .required_quality_gates
        .into_iter()
        .map(|requirement| QualityGateRequirement {
            gate_id: crate::quality::normalize_gate_id(&requirement.gate_id),
            source: requirement.source,
        })
        .collect::<Vec<_>>();
    required_quality_gates.sort_by(|left, right| {
        left.gate_id
            .cmp(&right.gate_id)
            .then_with(|| left.source.as_str().cmp(right.source.as_str()))
    });
    if required_quality_gates
        .windows(2)
        .any(|requirements| requirements[0].gate_id == requirements[1].gate_id)
    {
        return Err("Each release rule gate may specify only one evidence source".to_string());
    }
    if rule.min_commits.is_none()
        && rule.min_elapsed_days.is_none()
        && required_commit_types.is_empty()
        && required_quality_gates.is_empty()
    {
        return Err(
            "Release rule needs a commit count, elapsed time, commit type, or quality gate clause"
                .to_string(),
        );
    }
    Ok(ReleaseRuleConfig {
        name,
        operator,
        min_commits: rule.min_commits,
        min_elapsed_days: rule.min_elapsed_days,
        required_commit_types,
        allow_first_release: rule.allow_first_release,
        required_quality_gates,
    })
}

fn default_release_recipe() -> ReleaseRecipeConfig {
    ReleaseRecipeConfig {
        name: "Single repository release".to_string(),
        validation_commands: Vec::new(),
        release_commands: Vec::new(),
        generated_paths: Vec::new(),
        commit_message: "chore(release): prepare {version}".to_string(),
    }
}

fn normalize_release_commands(commands: Vec<String>, label: &str) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for command in commands {
        let value = command.trim();
        if value.is_empty() {
            continue;
        }
        if value.contains('\0') || value.contains('\n') || value.contains('\r') {
            return Err(format!("{label} cannot contain line breaks or null bytes"));
        }
        if value.chars().count() > 500 {
            return Err(format!(
                "{label} must be 500 characters or fewer per command"
            ));
        }
        let value = value.to_string();
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn normalize_generated_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for path in paths {
        let value = path.trim();
        if value.is_empty() {
            continue;
        }
        if value.starts_with('/')
            || value.contains('\\')
            || value.contains(':')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(format!(
                "Generated path '{value}' must be a repository-relative file path"
            ));
        }
        if value.chars().count() > 240 {
            return Err("Generated paths must be 240 characters or fewer".to_string());
        }
        let value = value.to_string();
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn normalize_release_recipe(recipe: ReleaseRecipeConfig) -> Result<ReleaseRecipeConfig, String> {
    let name = normalize_name(&recipe.name, "Release recipe")?;
    let commit_message = recipe.commit_message.trim();
    if commit_message.is_empty() {
        return Err("Release recipe commit message cannot be empty".to_string());
    }
    if commit_message.contains('\0')
        || commit_message.contains('\n')
        || commit_message.contains('\r')
    {
        return Err(
            "Release recipe commit message cannot contain line breaks or null bytes".to_string(),
        );
    }
    if commit_message.chars().count() > 160 {
        return Err("Release recipe commit message must be 160 characters or fewer".to_string());
    }
    Ok(ReleaseRecipeConfig {
        name,
        validation_commands: normalize_release_commands(
            recipe.validation_commands,
            "Validation commands",
        )?,
        release_commands: normalize_release_commands(recipe.release_commands, "Release commands")?,
        generated_paths: normalize_generated_paths(recipe.generated_paths)?,
        commit_message: commit_message.to_string(),
    })
}

fn canonical_path(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn canonical_repository_path(path: &Path) -> Option<PathBuf> {
    let top_level = git_static(path, &["rev-parse", "--show-toplevel"])?;
    let top = canonical_path(Path::new(&top_level)).unwrap_or_else(|| PathBuf::from(top_level));
    let common_raw = git_static(
        &top,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .or_else(|| git_static(&top, &["rev-parse", "--git-common-dir"]))?;
    let common = {
        let candidate = PathBuf::from(&common_raw);
        if candidate.is_absolute() {
            candidate
        } else {
            top.join(candidate)
        }
    };
    let common = canonical_path(&common).unwrap_or(common);
    if common.file_name().and_then(|name| name.to_str()) == Some(".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        Some(top)
    }
}

fn default_ignore(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "node_modules"
            | ".pnpm"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".cache"
            | "coverage"
            | "vendor"
            | ".idea"
            | ".vscode"
    )
}

fn matches_ignore(name: &str, patterns: &[String]) -> bool {
    let normalized_name = name.to_ascii_lowercase();
    default_ignore(&normalized_name)
        || patterns.iter().any(|pattern| {
            let trimmed = pattern.trim_matches('/').to_ascii_lowercase();
            trimmed == normalized_name
                || (trimmed.starts_with('*')
                    && normalized_name.ends_with(trimmed.trim_start_matches('*')))
        })
}

fn path_is_within(root: &Path, candidate: &Path) -> bool {
    let canonical_root = canonical_path(root).unwrap_or_else(|| root.to_path_buf());
    let canonical_candidate = canonical_path(candidate).unwrap_or_else(|| candidate.to_path_buf());
    canonical_candidate == canonical_root || canonical_candidate.starts_with(&canonical_root)
}

fn path_is_ignored_by_root(root: &RootConfig, candidate: &Path) -> bool {
    let root_path = Path::new(&root.path);
    if !path_is_within(root_path, candidate) {
        return false;
    }
    let candidate_path = canonical_path(candidate).unwrap_or_else(|| candidate.to_path_buf());
    let root_path = canonical_path(root_path).unwrap_or_else(|| root_path.to_path_buf());
    candidate_path
        .strip_prefix(root_path)
        .map(|relative| {
            relative.components().any(|component| {
                let name = component.as_os_str().to_string_lossy();
                matches_ignore(&name, &root.ignore_patterns)
            })
        })
        .unwrap_or(false)
}

fn repository_is_ignored_by_existing_root(
    state: &StoreState,
    repository: &RepositorySnapshot,
) -> bool {
    state
        .roots
        .iter()
        .any(|root| path_is_ignored_by_root(root, Path::new(&repository.path)))
}

fn discover_in_directory(
    directory: &Path,
    patterns: &[String],
    visited: &mut HashSet<PathBuf>,
    repositories: &mut HashSet<PathBuf>,
) {
    let canonical = canonical_path(directory).unwrap_or_else(|| directory.to_path_buf());
    if !visited.insert(canonical.clone()) {
        return;
    }
    if canonical.join(".git").exists() {
        if let Some(repository) = canonical_repository_path(&canonical) {
            repositories.insert(repository);
        }
        return;
    }
    let entries = match fs::read_dir(&canonical) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if matches_ignore(&name, patterns) {
            continue;
        }
        let metadata = match fs::metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            discover_in_directory(&entry.path(), patterns, visited, repositories);
        }
    }
}

fn discover_repositories(root: &RootConfig) -> Vec<PathBuf> {
    let mut visited = HashSet::new();
    let mut repositories = HashSet::new();
    discover_in_directory(
        Path::new(&root.path),
        &root.ignore_patterns,
        &mut visited,
        &mut repositories,
    );
    let mut sorted = repositories.into_iter().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

fn parse_status(output: &str) -> ParsedStatus {
    let mut status = ParsedStatus::default();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            status.branch = if value == "(detached)" {
                "Detached HEAD".to_string()
            } else {
                value.to_string()
            };
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            let mut values = value.split_whitespace();
            status.ahead = values
                .next()
                .unwrap_or("+0")
                .trim_start_matches('+')
                .parse()
                .unwrap_or(0);
            status.behind = values
                .next()
                .unwrap_or("-0")
                .trim_start_matches('-')
                .parse()
                .unwrap_or(0);
        } else if !line.is_empty() && !line.starts_with('#') {
            status.dirty = true;
        }
    }
    if status.branch.is_empty() {
        status.branch = "Detached HEAD".to_string();
    }
    status
}

fn parse_git_status(result: GitOutput) -> Result<ParsedStatus, String> {
    if !result.success {
        let detail = result.stderr.trim();
        let detail = if detail.is_empty() {
            result
                .exit_code
                .map(|code| format!("Git status exited with code {code}."))
                .unwrap_or_else(|| "Git status exited unsuccessfully.".to_string())
        } else {
            detail.to_string()
        };
        return Err(format!("Git status failed: {detail}"));
    }
    Ok(parse_status(&result.stdout))
}

fn parse_numstat(output: &str) -> DiffTotals {
    let mut totals = DiffTotals::default();
    for line in output.lines() {
        let mut fields = line.split('\t');
        let added = fields.next().unwrap_or_default();
        let removed = fields.next().unwrap_or_default();
        if added == "-" || removed == "-" {
            totals.partial = true;
            continue;
        }
        totals.added += added.parse::<u64>().unwrap_or(0);
        totals.removed += removed.parse::<u64>().unwrap_or(0);
    }
    totals
}

fn count_untracked_lines(path: &Path) -> DiffTotals {
    let mut totals = DiffTotals::default();
    let result = match run_git(
        path,
        ["ls-files", "--others", "--exclude-standard", "-z"].iter(),
    ) {
        Ok(result) if result.success => result,
        _ => return totals,
    };
    let workspace = canonical_path(path).unwrap_or_else(|| path.to_path_buf());
    for relative in result.stdout.split('\0').filter(|value| !value.is_empty()) {
        let candidate = path.join(relative);
        let canonical = match canonical_path(&candidate) {
            Some(value) if value.starts_with(&workspace) => value,
            _ => {
                totals.partial = true;
                continue;
            }
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(metadata) => metadata,
            Err(_) => {
                totals.partial = true;
                continue;
            }
        };
        if metadata.len() > DEFAULT_MAX_UNTRACKED_BYTES {
            totals.partial = true;
            continue;
        }
        let bytes = match fs::read(&canonical) {
            Ok(bytes) => bytes,
            Err(_) => {
                totals.partial = true;
                continue;
            }
        };
        if bytes.contains(&0) {
            totals.partial = true;
            continue;
        }
        totals.added += bytes.iter().filter(|byte| **byte == b'\n').count() as u64
            + u64::from(!bytes.is_empty() && !bytes.ends_with(b"\n"));
    }
    totals
}

fn diff_totals(path: &Path) -> DiffTotals {
    let tracked_output = git_static(path, &["diff", "--numstat", "HEAD", "--"])
        .or_else(|| git_static(path, &["diff", "--numstat", "--"]))
        .unwrap_or_default();
    let mut totals = parse_numstat(&tracked_output);
    let untracked = count_untracked_lines(path);
    totals.added += untracked.added;
    totals.removed += untracked.removed;
    totals.partial |= untracked.partial;
    totals
}

fn interrupted_operation(path: &Path) -> Option<String> {
    let markers = [
        ("Merge in progress", "MERGE_HEAD"),
        ("Cherry-pick in progress", "CHERRY_PICK_HEAD"),
        ("Revert in progress", "REVERT_HEAD"),
        ("Rebase in progress", "rebase-merge"),
        ("Rebase in progress", "rebase-apply"),
        ("Bisect in progress", "BISECT_LOG"),
    ];
    for (label, marker) in markers {
        let marker_path = git_static(path, &["rev-parse", "--git-path", marker]).map(|value| {
            let candidate = PathBuf::from(value);
            if candidate.is_absolute() {
                candidate
            } else {
                path.join(candidate)
            }
        });
        if marker_path
            .as_ref()
            .is_some_and(|candidate| candidate.exists())
        {
            return Some(label.to_string());
        }
    }
    None
}

fn parse_log(raw: Option<String>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw else {
        return (None, None);
    };
    let mut fields = raw.split('\t');
    let commit = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let committed_at = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    (commit, committed_at)
}

fn workspace_status(
    path: &Path,
) -> (
    Result<ParsedStatus, String>,
    DiffTotals,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let status = run_git(
        path,
        [
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ]
        .iter(),
    )
    .and_then(parse_git_status);
    let totals = diff_totals(path);
    let operation = interrupted_operation(path);
    let (last_commit, last_commit_at) =
        parse_log(git_static(path, &["log", "-1", "--format=%H\t%cI"]));
    let last_activity = last_commit_at.clone();
    (
        status,
        totals,
        operation,
        last_commit,
        last_commit_at,
        last_activity,
    )
}

fn parse_worktrees(path: &Path) -> Vec<WorktreeRecord> {
    let output = git_static(path, &["worktree", "list", "--porcelain"]).unwrap_or_default();
    let mut records = Vec::new();
    for block in output.split("\n\n") {
        let mut worktree_path = None;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                worktree_path = Some(PathBuf::from(value));
            }
        }
        if let Some(path) = worktree_path {
            records.push(WorktreeRecord { path });
        }
    }
    if records.is_empty() {
        records.push(WorktreeRecord {
            path: path.to_path_buf(),
        });
    }
    records
}

fn parse_branches(path: &Path) -> Vec<BranchRecord> {
    let output = git_static(
        path,
        &[
            "for-each-ref",
            "--format=%(refname:short)%09%(objectname)%09%(authordate:iso-strict)",
            "refs/heads",
        ],
    )
    .unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let last_commit = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let last_commit_at = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            Some(BranchRecord {
                name,
                last_commit,
                last_commit_at,
            })
        })
        .collect()
}

fn parse_submodules(path: &Path) -> Vec<SubmoduleSummary> {
    let output = git_static(path, &["submodule", "status", "--recursive"]).unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let marker = line.chars().next()?;
            let mut fields = line.get(marker.len_utf8()..)?.split_whitespace();
            let commit = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let submodule_path = fields.next()?.to_string();
            let status = match line.chars().next() {
                Some('-') => "Uninitialized",
                Some('+') => "Modified commit",
                Some('U') => "Merge conflict",
                _ => "Checked out",
            };
            Some(SubmoduleSummary {
                path: submodule_path,
                commit,
                status: status.to_string(),
            })
        })
        .collect()
}

fn detect_default_branch(path: &Path, current: &str) -> Option<String> {
    git_static(
        path,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|value| value.strip_prefix("origin/").map(str::to_string))
    .or_else(|| {
        ["main", "master", "dev", "develop"]
            .iter()
            .find(|candidate| {
                git_static(
                    path,
                    &["show-ref", "--verify", &format!("refs/heads/{candidate}")],
                )
                .is_some()
            })
            .map(|candidate| (*candidate).to_string())
    })
    .or_else(|| (!current.is_empty() && current != "Detached HEAD").then(|| current.to_string()))
}

fn branch_role(branch: &str, default_branch: Option<&str>) -> (String, String) {
    if default_branch.is_some_and(|default| default == branch) {
        return ("Production".to_string(), "High".to_string());
    }
    let lower = branch.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "dev" | "develop" | "development" | "staging"
    ) {
        return ("Integration".to_string(), "Medium".to_string());
    }
    if lower.starts_with("agent/") || lower.starts_with("task/") || lower.starts_with("codex/") {
        return ("Agent task".to_string(), "Medium".to_string());
    }
    if lower.starts_with("release/") {
        return ("Release".to_string(), "Medium".to_string());
    }
    if lower.starts_with("hotfix/") {
        return ("Hotfix".to_string(), "Low".to_string());
    }
    ("Feature".to_string(), "Low".to_string())
}

fn target_for_branch(branch: &str, default_branch: Option<&str>) -> (Option<String>, String) {
    if default_branch.is_some_and(|default| default == branch) {
        (None, "High".to_string())
    } else {
        (default_branch.map(str::to_string), "Medium".to_string())
    }
}

fn activity_signal(
    source: &str,
    summary: &str,
    confidence: &str,
    process_name: Option<&str>,
    process_id: Option<u32>,
    started_at: Option<&str>,
    working_directory: Option<&Path>,
) -> ActivitySignal {
    ActivitySignal {
        source: source.to_string(),
        summary: summary.to_string(),
        confidence: confidence.to_string(),
        observed_at: iso_now(),
        process_name: process_name.map(str::to_string),
        process_id,
        started_at: started_at.map(str::to_string),
        working_directory: working_directory.map(|path| path.to_string_lossy().to_string()),
    }
}

fn manifest_value_is_safe(value: &Option<String>) -> bool {
    value
        .as_ref()
        .map(|value| value.chars().count() <= 512 && !value.contains('\0'))
        .unwrap_or(true)
}

fn manifest_is_safe(manifest: &AgentManifest) -> bool {
    [
        &manifest.task_id,
        &manifest.title,
        &manifest.target_branch,
        &manifest.agent_type,
        &manifest.start_time,
        &manifest.status,
        &manifest.source_session_id,
    ]
    .into_iter()
    .all(manifest_value_is_safe)
}

fn read_agent_manifest(path: &Path) -> (Option<AgentManifest>, Option<ActivitySignal>) {
    let candidates = [
        path.join(".pronto").join("agent.json"),
        path.join(".pronto").join("agent-manifest.json"),
    ];
    for manifest_path in candidates {
        let Ok(metadata) = fs::metadata(&manifest_path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > DEFAULT_MAX_MANIFEST_BYTES {
            return (
                None,
                Some(activity_signal(
                    "Manifest",
                    "Activity state uncertain",
                    "Low",
                    None,
                    None,
                    None,
                    None,
                )),
            );
        }
        let payload = match fs::read_to_string(&manifest_path) {
            Ok(payload) => payload,
            Err(_) => {
                return (
                    None,
                    Some(activity_signal(
                        "Manifest",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )),
                );
            }
        };
        let manifest = match serde_json::from_str::<AgentManifest>(&payload) {
            Ok(manifest) if manifest_is_safe(&manifest) => manifest,
            _ => {
                return (
                    None,
                    Some(activity_signal(
                        "Manifest",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )),
                );
            }
        };
        let summary = if manifest.status.as_deref().is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "active" | "running" | "started"
            )
        }) {
            "Agent manifest reports active task"
        } else {
            "Agent manifest found"
        };
        return (
            Some(manifest),
            Some(activity_signal(
                "Manifest", summary, "High", None, None, None, None,
            )),
        );
    }
    (None, None)
}

fn process_name_is_activity_candidate(name: &str) -> bool {
    let normalized = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(name)
        .to_ascii_lowercase();
    [
        "codex", "claude", "aider", "cursor", "continue", "opencode", "copilot",
    ]
    .iter()
    .any(|candidate| normalized == *candidate || normalized.contains(candidate))
}

#[cfg(not(target_os = "windows"))]
fn process_working_directory_from_lsof(process_id: u32) -> Option<PathBuf> {
    let output = Command::new("lsof")
        .args(["-a", "-p", &process_id.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(PathBuf::from))
}

#[cfg(target_os = "linux")]
fn process_working_directory(process_id: u32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{process_id}/cwd"))
        .ok()
        .or_else(|| process_working_directory_from_lsof(process_id))
}

#[cfg(target_os = "macos")]
fn process_working_directory(process_id: u32) -> Option<PathBuf> {
    process_working_directory_from_lsof(process_id)
}

#[cfg(target_os = "windows")]
fn process_working_directory(_process_id: u32) -> Option<PathBuf> {
    None
}

fn workspace_contains(parent: &Path, candidate: &Path) -> bool {
    let Some(parent) = canonical_path(parent) else {
        return false;
    };
    let Some(candidate) = canonical_path(candidate) else {
        return false;
    };
    candidate == parent || candidate.starts_with(parent)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessActivityRow {
    process_id: u32,
    parent_process_id: u32,
    process_name: String,
    started_at: Option<String>,
}

fn parse_process_activity_rows(output: &str) -> Vec<ProcessActivityRow> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process_id = fields.next()?.parse::<u32>().ok()?;
            let parent_process_id = fields.next()?.parse::<u32>().ok()?;
            let process_name = fields.next()?.to_string();
            let started_at = fields.collect::<Vec<_>>().join(" ");
            Some(ProcessActivityRow {
                process_id,
                parent_process_id,
                process_name,
                started_at: (!started_at.is_empty()).then_some(started_at),
            })
        })
        .collect()
}

fn process_ancestor_ids(rows: &[ProcessActivityRow], process_id: u32) -> HashSet<u32> {
    let parents = rows
        .iter()
        .map(|row| (row.process_id, row.parent_process_id))
        .collect::<HashMap<_, _>>();
    let mut excluded = HashSet::new();
    let mut candidate = Some(process_id);
    while let Some(current) = candidate {
        if current == 0 || !excluded.insert(current) {
            break;
        }
        candidate = parents.get(&current).copied();
    }
    excluded
}

fn process_activity_signals(path: &Path) -> (Vec<ActivitySignal>, bool) {
    #[cfg(target_os = "windows")]
    {
        let _ = path;
        return (
            vec![activity_signal(
                "Process",
                "Activity state uncertain",
                "Low",
                None,
                None,
                None,
                None,
            )],
            false,
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = match Command::new("ps")
            .args(["-axo", "pid=,ppid=,comm=,lstart="])
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => {
                return (
                    vec![activity_signal(
                        "Process",
                        "Activity state uncertain",
                        "Low",
                        None,
                        None,
                        None,
                        None,
                    )],
                    false,
                );
            }
        };
        let rows = parse_process_activity_rows(&String::from_utf8_lossy(&output.stdout));
        let invoking_process_ids = process_ancestor_ids(&rows, std::process::id());
        let mut signals = Vec::new();
        let mut unresolved_candidate = false;
        for row in rows {
            if invoking_process_ids.contains(&row.process_id) {
                continue;
            }
            if !process_name_is_activity_candidate(&row.process_name) {
                continue;
            }
            let Some(working_directory) = process_working_directory(row.process_id) else {
                unresolved_candidate = true;
                continue;
            };
            if workspace_contains(path, &working_directory) {
                signals.push(activity_signal(
                    "Process",
                    "Process evidence found",
                    "Medium",
                    Some(&row.process_name),
                    Some(row.process_id),
                    row.started_at.as_deref(),
                    Some(&working_directory),
                ));
            }
        }
        if signals.is_empty() && unresolved_candidate {
            signals.push(activity_signal(
                "Process",
                "Activity state uncertain",
                "Low",
                None,
                None,
                None,
                None,
            ));
            return (signals, false);
        }
        (signals, true)
    }
}

fn collect_workspace_activity(path: &Path, dirty: bool, ahead: u64) -> WorkspaceActivity {
    let (manifest, manifest_signal) = read_agent_manifest(path);
    let (mut signals, process_inspection_complete) = process_activity_signals(path);
    if let Some(signal) = manifest_signal {
        signals.push(signal);
    }
    let manifest_active = manifest
        .as_ref()
        .and_then(|manifest| manifest.status.as_deref())
        .is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "active" | "running" | "started"
            )
        });
    let process_active = signals
        .iter()
        .any(|signal| signal.summary == "Process evidence found");
    let uncertain = signals
        .iter()
        .any(|signal| signal.summary == "Activity state uncertain");
    if !process_active && process_inspection_complete && !uncertain {
        signals.push(activity_signal(
            "Process",
            "No associated process detected",
            "Medium",
            None,
            None,
            None,
            None,
        ));
    }
    let state = if manifest_active || process_active {
        "Active"
    } else if dirty {
        "Interrupted with dirty work"
    } else if ahead > 0 {
        "Interrupted with unpushed commits"
    } else if manifest.is_some() {
        "Recently active"
    } else {
        "Unknown"
    };
    let confidence = if manifest_active {
        "High"
    } else if process_active {
        "Medium"
    } else if uncertain {
        "Low"
    } else {
        "Medium"
    };
    WorkspaceActivity {
        state: state.to_string(),
        confidence: confidence.to_string(),
        signals,
        manifest,
    }
}

fn unique_commits(path: &Path, branch: &str, target: Option<&str>) -> u64 {
    let Some(target) = target else {
        return 0;
    };
    git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{target}..{branch}"),
        ],
    )
    .and_then(|value| value.parse::<u64>().ok())
    .unwrap_or(0)
}

fn release_commit_details(subject: &str) -> (String, Option<String>) {
    let header = subject.split(':').next().unwrap_or(subject).trim();
    let breaking = header.contains('!') || subject.contains("BREAKING CHANGE");
    let commit_type = header
        .trim_end_matches('!')
        .split('(')
        .next()
        .unwrap_or(header)
        .to_ascii_lowercase();
    let (category, bump) = if breaking {
        ("Breaking", Some("major"))
    } else {
        match commit_type.as_str() {
            "feat" => ("Features", Some("minor")),
            "fix" => ("Fixes", Some("patch")),
            "perf" => ("Performance", Some("patch")),
            _ => ("Other", None),
        }
    };
    (category.to_string(), bump.map(str::to_string))
}

fn release_commits(path: &Path, base: &str, head: &str) -> Vec<ReleaseCommitSummary> {
    let range = format!("{base}..{head}");
    let raw = git_owned(
        path,
        vec![
            "log".to_string(),
            range,
            "--format=%H%x09%s%x09%cI".to_string(),
        ],
    )
    .unwrap_or_default();
    raw.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let sha = fields.next()?.trim();
            let subject = fields.next()?.trim();
            let committed_at = fields.next()?.trim();
            if sha.is_empty() || subject.is_empty() || committed_at.is_empty() {
                return None;
            }
            let (category, bump) = release_commit_details(subject);
            Some(ReleaseCommitSummary {
                sha: sha.to_string(),
                subject: subject.to_string(),
                category,
                bump,
                committed_at: committed_at.to_string(),
            })
        })
        .collect()
}

fn git_ref_exists(path: &Path, reference: &str) -> bool {
    git_owned(
        path,
        vec![
            "rev-parse".to_string(),
            "--verify".to_string(),
            reference.to_string(),
        ],
    )
    .is_some()
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn committed_diff(path: &Path, base: &str, head: &str) -> Option<(String, bool)> {
    let range = format!("{base}..{head}");
    let result = run_git(
        path,
        vec![
            "diff".to_string(),
            "--no-ext-diff".to_string(),
            range,
            "--".to_string(),
        ],
    )
    .ok()?;
    if !result.success {
        return None;
    }
    let truncated = result.stdout.len() > MAX_AI_DIFF_BYTES;
    Some((bounded_text(&result.stdout, MAX_AI_DIFF_BYTES), truncated))
}

fn empty_ai_preview(repository_id: &str, workspace_id: &str, permission: &str) -> AiPayloadPreview {
    AiPayloadPreview {
        repository_id: repository_id.to_string(),
        workspace_id: workspace_id.to_string(),
        permission: permission.to_string(),
        provider: "None — local preview only".to_string(),
        model: None,
        status: "Preview unavailable".to_string(),
        reasons: Vec::new(),
        categories: Vec::new(),
        source_references: Vec::new(),
        payload_text: String::new(),
        payload_bytes: 0,
        uncommitted_included: false,
        request_performed: false,
        generated_at: iso_now(),
    }
}

fn preview_ai_summary_at(
    path: &Path,
    repository_id: &str,
    workspace_id: Option<&str>,
) -> Result<AiPayloadPreview, String> {
    let state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    let permission = normalize_ai_permission(&repository.ai_permission)
        .unwrap_or_else(|_| default_ai_permission());
    let mut preview = empty_ai_preview(&repository.id, &workspace.id, &permission);
    if permission == "Disabled" {
        preview.status = "AI disabled by repository policy".to_string();
        preview
            .reasons
            .push("No external request was made.".to_string());
        return Ok(preview);
    }
    if !Path::new(&workspace.path).is_dir() {
        preview.status = "Workspace path unavailable".to_string();
        preview
            .reasons
            .push("The registered workspace path is not accessible.".to_string());
        return Ok(preview);
    }
    if !workspace.status_available {
        preview.status = "Git status unavailable".to_string();
        preview
            .reasons
            .push(workspace_status_unavailable_reason(workspace));
        return Ok(preview);
    }
    let base = latest_published_release(repository)
        .and_then(|release| release.target_commit)
        .or_else(|| workspace.target_branch.clone());
    let Some(base) = base.filter(|value| !value.trim().is_empty()) else {
        preview.status = "Committed evidence range unavailable".to_string();
        preview
            .reasons
            .push("A published baseline or verified target branch is required.".to_string());
        return Ok(preview);
    };
    let head = workspace.branch.trim();
    if head.is_empty()
        || !git_ref_exists(Path::new(&workspace.path), &base)
        || !git_ref_exists(Path::new(&workspace.path), head)
    {
        preview.status = "Committed evidence range unavailable".to_string();
        preview
            .reasons
            .push("The selected committed range could not be verified locally.".to_string());
        return Ok(preview);
    }

    let commits = release_commits(Path::new(&workspace.path), &base, head);
    let metadata_payload = serde_json::json!({
        "repository_id": repository.id.clone(),
        "workspace_id": workspace.id.clone(),
        "commits": commits,
    });
    let metadata_text = serde_json::to_string_pretty(&metadata_payload)
        .map_err(|error| format!("Could not encode AI metadata preview: {error}"))?;
    preview.source_references = commits
        .iter()
        .map(|commit| AiSourceReference {
            sha: commit.sha.clone(),
            subject: commit.subject.clone(),
            committed_at: commit.committed_at.clone(),
            category: commit.category.clone(),
        })
        .collect();
    preview.categories.push(AiPayloadCategory {
        category: "Committed metadata".to_string(),
        included: true,
        item_count: commits.len(),
        byte_count: metadata_text.len(),
    });

    let mut payload = metadata_payload;
    if permission == "Committed diff allowed" {
        let Some((diff_text, truncated)) = committed_diff(Path::new(&workspace.path), &base, head)
        else {
            preview.status = "Committed diff preview unavailable".to_string();
            preview
                .reasons
                .push("Git could not produce the committed-only diff.".to_string());
            return Ok(preview);
        };
        if truncated {
            preview.reasons.push(format!(
                "Committed diff preview is capped at {} bytes.",
                MAX_AI_DIFF_BYTES
            ));
        }
        payload["committed_diff"] = serde_json::Value::String(diff_text.clone());
        preview.categories.push(AiPayloadCategory {
            category: "Committed diff".to_string(),
            included: true,
            item_count: usize::from(!diff_text.is_empty()),
            byte_count: diff_text.len(),
        });
    }
    preview.payload_text = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Could not encode AI payload preview: {error}"))?;
    preview.payload_bytes = preview.payload_text.len();
    preview.status = if commits.is_empty() {
        "No committed changes in selected range".to_string()
    } else {
        "Payload ready for user inspection".to_string()
    };
    preview
        .reasons
        .push("Preview only; no external AI request was made.".to_string());
    if workspace.dirty {
        preview
            .reasons
            .push("Uncommitted changes are excluded from this payload.".to_string());
    }
    Ok(preview)
}

fn latest_published_release(repository: &RepositorySnapshot) -> Option<ReleaseSnapshot> {
    repository
        .releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease && release.published_at.is_some())
        .max_by(|left, right| left.published_at.cmp(&right.published_at))
        .cloned()
}

fn parse_release_version(tag: &str) -> Option<(u64, u64, u64)> {
    let trimmed = tag.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn normalize_release_version(value: &str) -> Result<String, String> {
    let (major, minor, patch) = parse_release_version(value)
        .ok_or_else(|| "Release version must use the form vMAJOR.MINOR.PATCH".to_string())?;
    Ok(format!("v{major}.{minor}.{patch}"))
}

fn highest_release_bump(commits: &[ReleaseCommitSummary]) -> Option<String> {
    commits
        .iter()
        .filter_map(|commit| commit.bump.as_deref())
        .max_by_key(|bump| match *bump {
            "major" => 3,
            "minor" => 2,
            "patch" => 1,
            _ => 0,
        })
        .map(str::to_string)
}

fn candidate_version(release: &ReleaseSnapshot, bump: Option<&str>) -> Option<String> {
    let (mut major, mut minor, mut patch) = parse_release_version(&release.tag)?;
    match bump {
        Some("major") => {
            major += 1;
            minor = 0;
            patch = 0;
        }
        Some("minor") => {
            minor += 1;
            patch = 0;
        }
        Some("patch") => patch += 1,
        _ => return None,
    }
    Some(format!("v{major}.{minor}.{patch}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseRuleResult {
    Passed,
    Failed,
    Blocked,
    Unknown,
}

fn combine_release_rule_results(
    operator: &str,
    results: &[ReleaseRuleResult],
) -> ReleaseRuleResult {
    if results.contains(&ReleaseRuleResult::Blocked) {
        return ReleaseRuleResult::Blocked;
    }
    if operator == "OR" {
        if results.contains(&ReleaseRuleResult::Passed) {
            ReleaseRuleResult::Passed
        } else if results
            .iter()
            .all(|result| *result == ReleaseRuleResult::Failed)
        {
            ReleaseRuleResult::Failed
        } else {
            ReleaseRuleResult::Unknown
        }
    } else if results.contains(&ReleaseRuleResult::Failed) {
        ReleaseRuleResult::Failed
    } else if results.contains(&ReleaseRuleResult::Unknown) {
        ReleaseRuleResult::Unknown
    } else {
        ReleaseRuleResult::Passed
    }
}

fn release_rule_status(result: ReleaseRuleResult) -> &'static str {
    match result {
        ReleaseRuleResult::Passed => "Passed",
        ReleaseRuleResult::Failed => "Failed",
        ReleaseRuleResult::Blocked => "Blocked",
        ReleaseRuleResult::Unknown => "Unknown",
    }
}

fn release_rule_needs_baseline(rule: &ReleaseRuleConfig) -> bool {
    rule.min_commits.is_some()
        || rule.min_elapsed_days.is_some()
        || !rule.required_commit_types.is_empty()
}

fn release_rule_commit_type_present(
    commits: &[ReleaseCommitSummary],
    requested_types: &[String],
) -> bool {
    requested_types.iter().any(|requested| {
        commits.iter().any(|commit| match requested.as_str() {
            "breaking" => commit.category == "Breaking",
            "feat" => commit.category == "Features",
            "fix" => commit.category == "Fixes",
            "perf" => commit.category == "Performance",
            requested => commit
                .subject
                .split_once(':')
                .map(|(kind, _)| kind.trim().trim_end_matches('!') == requested)
                .unwrap_or(false),
        })
    })
}

fn evaluate_release_rule(
    rule: &ReleaseRuleConfig,
    baseline: Option<&ReleaseSnapshot>,
    commits: &[ReleaseCommitSummary],
) -> (ReleaseRuleResult, Vec<ReleaseRuleTrace>) {
    let mut results = Vec::new();
    let mut trace = Vec::new();
    let (baseline_result, baseline_value) = if !release_rule_needs_baseline(rule) {
        (
            ReleaseRuleResult::Passed,
            "No commit threshold clauses · quality evidence drives this rule".to_string(),
        )
    } else {
        match baseline {
            Some(release) => (
                ReleaseRuleResult::Passed,
                format!("Published baseline {}", release.tag),
            ),
            None if rule.allow_first_release => (
                ReleaseRuleResult::Passed,
                "No published baseline · first-release path enabled".to_string(),
            ),
            None => (
                ReleaseRuleResult::Failed,
                "No published baseline · first-release path not enabled".to_string(),
            ),
        }
    };
    results.push(baseline_result);
    trace.push(ReleaseRuleTrace {
        label: "Published baseline".to_string(),
        status: release_rule_status(baseline_result).to_string(),
        value: baseline_value,
        source: "Published GitHub Release snapshot and local rule configuration".to_string(),
    });

    if let Some(min_commits) = rule.min_commits {
        let result = if commits.len() as u64 >= min_commits {
            ReleaseRuleResult::Passed
        } else {
            ReleaseRuleResult::Failed
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: format!("At least {min_commits} commits"),
            status: release_rule_status(result).to_string(),
            value: format!("{} commits since baseline", commits.len()),
            source: "git log".to_string(),
        });
    }

    if let Some(min_elapsed_days) = rule.min_elapsed_days {
        let (result, value) = match baseline.and_then(|release| release.published_at.as_deref()) {
            Some(published_at) => match DateTime::parse_from_rfc3339(published_at) {
                Ok(date) => {
                    let elapsed_days = Utc::now()
                        .signed_duration_since(date.with_timezone(&Utc))
                        .num_days()
                        .max(0);
                    (
                        if elapsed_days >= min_elapsed_days as i64 {
                            ReleaseRuleResult::Passed
                        } else {
                            ReleaseRuleResult::Failed
                        },
                        format!("{elapsed_days} days since publication"),
                    )
                }
                Err(_) => (
                    ReleaseRuleResult::Unknown,
                    "Published timestamp could not be parsed".to_string(),
                ),
            },
            None => (
                ReleaseRuleResult::Unknown,
                "Published timestamp unavailable".to_string(),
            ),
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: format!("At least {min_elapsed_days} elapsed days"),
            status: release_rule_status(result).to_string(),
            value,
            source: "Published GitHub Release timestamp".to_string(),
        });
    }

    if !rule.required_commit_types.is_empty() {
        let result = if release_rule_commit_type_present(commits, &rule.required_commit_types) {
            ReleaseRuleResult::Passed
        } else {
            ReleaseRuleResult::Failed
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: "Configured commit type present".to_string(),
            status: release_rule_status(result).to_string(),
            value: format!(
                "{} in {}",
                rule.required_commit_types.join(", "),
                commits.len()
            ),
            source: "Deterministic conventional-commit mapping".to_string(),
        });
    }

    (
        combine_release_rule_results(&rule.operator, &results),
        trace,
    )
}

fn evaluate_release_rule_with_quality(
    repository: &RepositorySnapshot,
    rule: &ReleaseRuleConfig,
    baseline: Option<&ReleaseSnapshot>,
    commits: &[ReleaseCommitSummary],
) -> (ReleaseRuleResult, Vec<ReleaseRuleTrace>) {
    let (base_result, mut trace) = evaluate_release_rule(rule, baseline, commits);
    let mut quality_result = ReleaseRuleResult::Passed;
    for requirement in &rule.required_quality_gates {
        let (status, freshness, detail) = quality::evaluate_requirement(repository, requirement);
        let result = match status {
            QualityGateStatus::Failed => ReleaseRuleResult::Failed,
            QualityGateStatus::Passed if freshness == QualityFreshness::Fresh => {
                ReleaseRuleResult::Passed
            }
            QualityGateStatus::Passed => ReleaseRuleResult::Blocked,
            QualityGateStatus::Blocked | QualityGateStatus::NotConfigured => {
                ReleaseRuleResult::Blocked
            }
        };
        if result == ReleaseRuleResult::Failed {
            quality_result = ReleaseRuleResult::Failed;
        } else if result == ReleaseRuleResult::Blocked
            && quality_result == ReleaseRuleResult::Passed
        {
            quality_result = ReleaseRuleResult::Blocked;
        }
        trace.push(ReleaseRuleTrace {
            label: format!(
                "Quality gate · {} · {}",
                quality::gate_label(&requirement.gate_id),
                requirement.source.as_str()
            ),
            status: format!("{} · {}", status.as_str(), freshness.as_str()),
            value: detail,
            source: format!("Imported {} evidence", requirement.source.as_str()),
        });
    }
    let result = if quality_result == ReleaseRuleResult::Passed {
        base_result
    } else {
        quality_result
    };
    (result, trace)
}

fn release_threshold_condition(
    repository: &RepositorySnapshot,
    provider_ready: bool,
    expected: &[ExpectedCondition],
    observed_at: &str,
) -> Option<Condition> {
    let rule = repository.release_rule.as_ref()?;
    if !provider_context_available(repository, provider_ready)
        && rule.required_quality_gates.is_empty()
    {
        return None;
    }
    let baseline = latest_published_release(repository);
    let base = baseline
        .as_ref()
        .and_then(|release| release.target_commit.as_deref())
        .or(repository.workspace.target_branch.as_deref());
    let commits = base
        .map(|base| {
            release_commits(
                Path::new(&repository.workspace.path),
                base,
                &repository.workspace.branch,
            )
        })
        .unwrap_or_default();
    let (result, trace) =
        evaluate_release_rule_with_quality(repository, rule, baseline.as_ref(), &commits);
    if result != ReleaseRuleResult::Passed {
        return None;
    }
    let trace_evidence = trace
        .iter()
        .map(|item| {
            evidence(
                item.label.as_str(),
                format!("{} · {}", item.status, item.value),
                "Deterministic release rule trace",
                observed_at,
            )
        })
        .collect::<Vec<_>>();
    Some(condition(
        &repository.id,
        "release-threshold",
        "Configured release threshold met",
        format!("{} passed for {}.", rule.name, repository.name),
        4,
        condition_fingerprint(
            "release-threshold",
            &[
                rule.name.clone(),
                rule.operator.clone(),
                base.unwrap_or_default().to_string(),
                repository.workspace.branch.clone(),
                commits.len().to_string(),
                rule.required_quality_gates
                    .iter()
                    .map(|requirement| {
                        format!("{}:{}", requirement.gate_id, requirement.source.as_str())
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            ],
        ),
        "A user-configured deterministic release rule evaluated true using the published baseline, committed range, and configured clauses.",
        trace_evidence,
        Vec::new(),
        Some("High"),
        repository.last_fetch_at.clone(),
        expected,
    ))
}

fn apply_release_threshold_conditions(state: &mut StoreState) {
    let observed_at = iso_now();
    let provider_ready = state.provider_status.state == "Ready";
    for repository in &mut state.repositories {
        repository
            .conditions
            .retain(|condition| condition.kind != "release-threshold");
        if let Some(threshold) = release_threshold_condition(
            repository,
            provider_ready,
            &state.expected_conditions,
            &observed_at,
        ) {
            repository.conditions.push(threshold);
            repository.conditions.sort_by_key(|item| item.priority);
        }
    }
}

fn provider_context_available(repository: &RepositorySnapshot, provider_ready: bool) -> bool {
    provider_ready && repository.provider_state.starts_with("GitHub connected")
}

fn prepare_pull_request(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    provider_available: bool,
) -> PullRequestPreparation {
    let observed_at = iso_now();
    let base_branch = workspace.target_branch.clone();
    let commit_count = unique_commits(
        Path::new(&workspace.path),
        &workspace.branch,
        base_branch.as_deref(),
    );
    let existing_pull_request = repository
        .pull_requests
        .iter()
        .filter(|pull_request| {
            pull_request.state.eq_ignore_ascii_case("open")
                && pull_request.head_branch == workspace.branch
                && base_branch
                    .as_deref()
                    .is_some_and(|base| pull_request.base_branch == base)
        })
        .max_by_key(|pull_request| pull_request.number)
        .cloned();
    let checks_state = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.checks_state.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let reviews_state = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.reviews_state.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let mergeability = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.mergeability.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let mut reasons = Vec::new();
    if base_branch.is_none() {
        reasons.push("Target branch is unknown".to_string());
    }
    if !workspace.status_available {
        reasons.push(workspace_status_unavailable_reason(workspace));
    }
    if commit_count == 0 {
        reasons.push("Branch has no unique commits relative to the target".to_string());
    }
    if workspace.dirty {
        reasons.push("Workspace has uncommitted changes".to_string());
    }
    if !provider_available {
        reasons.push(
            "GitHub provider context is unavailable; pull request creation remains blocked"
                .to_string(),
        );
    }
    let mut evidence_items = vec![
        evidence(
            "Head branch",
            workspace.branch.clone(),
            "Local workspace scan",
            &observed_at,
        ),
        evidence(
            "Base branch",
            base_branch.clone().unwrap_or_else(|| "Unknown".to_string()),
            "Workspace target inference",
            &observed_at,
        ),
        evidence(
            "Commit count",
            commit_count.to_string(),
            "git rev-list",
            &observed_at,
        ),
        evidence(
            "Workspace",
            if !workspace.status_available {
                workspace_status_unavailable_reason(workspace)
            } else if workspace.dirty {
                "Dirty · commit preparation blocked".to_string()
            } else {
                "Clean".to_string()
            },
            "git status --porcelain=v2",
            &observed_at,
        ),
        evidence(
            "Push state",
            workspace.sync_state.clone(),
            "git status --porcelain=v2",
            &observed_at,
        ),
        evidence(
            "Provider",
            repository.provider_state.clone(),
            "Local provider snapshot",
            &observed_at,
        ),
    ];
    if let Some(pull_request) = existing_pull_request.as_ref() {
        evidence_items.push(evidence(
            "Existing pull request",
            format!("#{} · {}", pull_request.number, pull_request.title),
            "Stored provider snapshot",
            &observed_at,
        ));
    }
    PullRequestPreparation {
        repository_id: repository.id.clone(),
        workspace_id: workspace.id.clone(),
        head_branch: workspace.branch.clone(),
        base_branch,
        commit_count,
        status_available: workspace.status_available,
        status_error: workspace.status_error.clone(),
        dirty: workspace.dirty,
        ahead: workspace.ahead,
        behind: workspace.behind,
        upstream: workspace.upstream.clone(),
        provider_state: repository.provider_state.clone(),
        checks_state,
        reviews_state,
        mergeability,
        status: if reasons.is_empty() {
            "Evidence ready".to_string()
        } else {
            "Blocked".to_string()
        },
        reasons,
        evidence: evidence_items,
        existing_pull_request,
    }
}

fn prepare_release(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    provider_available: bool,
) -> ReleasePreparation {
    let observed_at = iso_now();
    let target_branch = repository
        .target_branch
        .clone()
        .or_else(|| repository.default_branch.clone())
        .or_else(|| workspace.target_branch.clone());
    let connected = provider_context_available(repository, provider_available);
    let baseline = connected
        .then(|| latest_published_release(repository))
        .flatten();
    let baseline_status = if !connected {
        "Provider release data unavailable".to_string()
    } else if baseline.is_some() {
        "Published release baseline".to_string()
    } else {
        "No published release baseline".to_string()
    };
    let range_base = baseline
        .as_ref()
        .and_then(|release| release.target_commit.as_deref())
        .or(target_branch.as_deref());
    let commits_since_baseline = range_base
        .map(|base| release_commits(Path::new(&workspace.path), base, &workspace.branch))
        .unwrap_or_default();
    let candidate_bump = highest_release_bump(&commits_since_baseline);
    let candidate_version = baseline
        .as_ref()
        .and_then(|release| candidate_version(release, candidate_bump.as_deref()));
    let mut grouped = BTreeMap::<String, Vec<ReleaseCommitSummary>>::new();
    for commit in &commits_since_baseline {
        grouped
            .entry(commit.category.clone())
            .or_default()
            .push(commit.clone());
    }
    let notes = grouped
        .into_iter()
        .map(|(category, commits)| ReleaseNoteSection { category, commits })
        .collect::<Vec<_>>();
    let configured_rule = repository.release_rule.as_ref();
    let (rule_result, rule_trace) = if connected
        || configured_rule.is_some_and(|rule| !rule.required_quality_gates.is_empty())
    {
        configured_rule
            .map(|rule| {
                evaluate_release_rule_with_quality(
                    repository,
                    rule,
                    baseline.as_ref(),
                    &commits_since_baseline,
                )
            })
            .map_or((None, Vec::new()), |(result, trace)| (Some(result), trace))
    } else {
        (None, Vec::new())
    };
    let rule_status = if !connected
        && !configured_rule.is_some_and(|rule| !rule.required_quality_gates.is_empty())
    {
        "Unknown — provider release data unavailable".to_string()
    } else if let Some(result) = rule_result {
        match result {
            ReleaseRuleResult::Passed => "Configured release threshold met".to_string(),
            ReleaseRuleResult::Failed => "Configured release threshold not met".to_string(),
            ReleaseRuleResult::Blocked => "Release rule blocked by quality evidence".to_string(),
            ReleaseRuleResult::Unknown => "Release threshold evidence incomplete".to_string(),
        }
    } else {
        "Not configured — commits are shown without threshold evaluation".to_string()
    };
    let mut reasons = Vec::new();
    if target_branch.is_none() {
        reasons.push("Target branch is unknown".to_string());
    }
    if !workspace.status_available {
        reasons.push(workspace_status_unavailable_reason(workspace));
    }
    if !connected {
        reasons.push(
            "Published GitHub release data is unavailable; no release threshold is evaluated"
                .to_string(),
        );
    } else if baseline.is_none() && configured_rule.is_some_and(release_rule_needs_baseline) {
        if configured_rule.is_none() {
            reasons.push("First-release rule is not confirmed".to_string());
        } else if configured_rule.is_some_and(|rule| !rule.allow_first_release) {
            reasons.push("First-release rule is not enabled".to_string());
        }
    }
    if workspace.dirty {
        reasons.push(
            "Workspace has uncommitted changes; release preparation cannot start".to_string(),
        );
    }
    if rule_result == Some(ReleaseRuleResult::Failed) {
        reasons.push("Configured release threshold did not pass".to_string());
    }
    if rule_result == Some(ReleaseRuleResult::Blocked) {
        reasons.push(
            "Required quality evidence is blocked, stale, missing, or conflicting".to_string(),
        );
    }
    if rule_result == Some(ReleaseRuleResult::Unknown) {
        reasons.push("Release threshold evidence is incomplete".to_string());
    }
    let mut evidence_items = vec![
        evidence(
            "Target branch",
            target_branch
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            "Configured repository target or observed Git default",
            &observed_at,
        ),
        evidence(
            "Baseline",
            baseline_status.clone(),
            "Published GitHub Release snapshot",
            &observed_at,
        ),
        evidence(
            "Commits since baseline",
            commits_since_baseline.len().to_string(),
            "git log",
            &observed_at,
        ),
        evidence(
            "Rule",
            rule_status.clone(),
            "Local release configuration",
            &observed_at,
        ),
    ];
    if let Some(bump) = candidate_bump.as_ref() {
        evidence_items.push(evidence(
            "Candidate bump",
            bump.clone(),
            "Deterministic conventional-commit mapping",
            &observed_at,
        ));
    }
    if !workspace.status_available {
        evidence_items.push(evidence(
            "Workspace",
            workspace_status_unavailable_reason(workspace),
            "git status --porcelain=v2",
            &observed_at,
        ));
    } else if workspace.dirty {
        evidence_items.push(evidence(
            "Starting state",
            "Dirty · release preparation blocked".to_string(),
            "git status --porcelain=v2",
            &observed_at,
        ));
    }
    let version_status = match (
        candidate_version.as_ref(),
        repository.confirmed_release_version.as_ref(),
    ) {
        (Some(candidate), Some(confirmed)) if candidate == confirmed => {
            "Candidate version confirmed".to_string()
        }
        (Some(_), Some(_)) => "Confirmed version does not match current candidate".to_string(),
        (Some(_), None) => "Candidate requires user confirmation".to_string(),
        (None, Some(_)) => "Confirmed version has no current candidate".to_string(),
        (None, None) => {
            "Candidate unavailable until a published baseline and deterministic bump exist"
                .to_string()
        }
    };
    if candidate_version.is_some() {
        if repository.confirmed_release_version.is_none() {
            reasons.push("Candidate version requires explicit user confirmation".to_string());
        } else if repository.confirmed_release_version != candidate_version {
            reasons.push(
                "Stored release version confirmation does not match the candidate".to_string(),
            );
        }
    } else if repository.confirmed_release_version.is_some() {
        reasons.push("Stored release version confirmation has no current candidate".to_string());
    }
    evidence_items.push(evidence(
        "Version confirmation",
        version_status.clone(),
        "Deterministic candidate and local user confirmation",
        &observed_at,
    ));
    let missing_baseline =
        baseline.is_none() && configured_rule.is_some_and(release_rule_needs_baseline);
    let blocked = target_branch.is_none()
        || !connected
        || !workspace.status_available
        || workspace.dirty
        || (missing_baseline && configured_rule.is_none())
        || (missing_baseline && configured_rule.is_some_and(|rule| !rule.allow_first_release))
        || rule_result == Some(ReleaseRuleResult::Failed)
        || rule_result == Some(ReleaseRuleResult::Blocked)
        || rule_result == Some(ReleaseRuleResult::Unknown);
    ReleasePreparation {
        repository_id: repository.id.clone(),
        target_branch,
        baseline_status,
        baseline,
        commits_since_baseline,
        rule_status,
        threshold_label: configured_rule.map(|rule| rule.name.clone()),
        rule_trace,
        candidate_bump,
        candidate_version,
        version_status,
        notes,
        status: if blocked {
            if connected && missing_baseline && configured_rule.is_none() {
                "First-release rule not confirmed".to_string()
            } else {
                "Blocked".to_string()
            }
        } else if rule_result == Some(ReleaseRuleResult::Failed) {
            "Threshold not met".to_string()
        } else {
            "Evidence ready".to_string()
        },
        reasons,
        evidence: evidence_items,
    }
}

fn prepare_release_recipe(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    release: &ReleasePreparation,
) -> ReleaseRecipePreview {
    let recipe = repository
        .release_recipe
        .clone()
        .unwrap_or_else(default_release_recipe);
    let starting_state_ready = workspace.status_available
        && !workspace.dirty
        && workspace.operation.is_none()
        && workspace.activity.state != "Active";
    let release_evidence_ready = release.status == "Evidence ready";
    let version_confirmed = release.version_status == "Candidate version confirmed";
    let has_release_changes =
        !recipe.release_commands.is_empty() || !recipe.generated_paths.is_empty();
    let has_validation = !recipe.validation_commands.is_empty();
    let mut reasons = release.reasons.clone();
    if !starting_state_ready {
        if !workspace.status_available {
            reasons.push(workspace_status_unavailable_reason(workspace));
        } else if workspace.dirty {
            reasons.push("Workspace has uncommitted changes".to_string());
        }
        if let Some(operation) = workspace.operation.as_ref() {
            reasons.push(format!("Git operation is active: {operation}"));
        }
        if workspace.activity.state == "Active" {
            reasons.push("Associated agent or process activity is active".to_string());
        }
    }
    if !release_evidence_ready {
        reasons.push(format!("Release evidence is not ready: {}", release.status));
    }
    if release.candidate_version.is_none() {
        reasons.push(release.version_status.clone());
    } else if !version_confirmed {
        reasons.push(release.version_status.clone());
    }
    if !has_release_changes {
        reasons.push(
            "Release recipe has no release commands or generated paths configured".to_string(),
        );
    }
    if !has_validation {
        reasons.push("Release recipe has no validation commands configured".to_string());
    }
    reasons.push(
        "Preview only; no worktree, script, commit, push, pull request, or release publication is performed."
            .to_string(),
    );
    reasons.dedup();

    let mut steps = Vec::new();
    steps.push(ReleaseRecipeStep {
        order: 1,
        label: "Starting-state check".to_string(),
        status: if starting_state_ready {
            "Passed".to_string()
        } else {
            "Blocked".to_string()
        },
        detail: if starting_state_ready {
            format!(
                "Workspace is clean; operation is clear; activity state is {}.",
                workspace.activity.state
            )
        } else {
            "Release preparation cannot start until the workspace is safe to isolate.".to_string()
        },
    });
    steps.push(ReleaseRecipeStep {
        order: 2,
        label: "Create clean isolated release worktree".to_string(),
        status: "Deferred".to_string(),
        detail: "Preview only; no worktree was created.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 3,
        label: "Confirm candidate version".to_string(),
        status: if version_confirmed {
            "Passed".to_string()
        } else {
            "Blocked".to_string()
        },
        detail: release.version_status.clone(),
    });
    steps.push(ReleaseRecipeStep {
        order: 4,
        label: "Apply configured release changes".to_string(),
        status: if has_release_changes {
            "Configured".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: format!(
            "{} release command(s), {} generated path(s); commit message template: {}",
            recipe.release_commands.len(),
            recipe.generated_paths.len(),
            recipe.commit_message
        ),
    });
    steps.push(ReleaseRecipeStep {
        order: 5,
        label: "Run configured validation".to_string(),
        status: if has_validation {
            "Configured".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: if has_validation {
            format!(
                "{} validation command(s) would be reviewed before execution.",
                recipe.validation_commands.len()
            )
        } else {
            "Add at least one validation command before release preparation.".to_string()
        },
    });
    let blocked = !starting_state_ready
        || !release_evidence_ready
        || release.candidate_version.is_none()
        || !version_confirmed;
    steps.push(ReleaseRecipeStep {
        order: 6,
        label: "Review exact generated diff".to_string(),
        status: if blocked {
            "Blocked".to_string()
        } else if has_release_changes && has_validation {
            "Pending".to_string()
        } else {
            "Needs configuration".to_string()
        },
        detail: "A user must inspect the exact generated files before any commit.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 7,
        label: "Commit generated files only".to_string(),
        status: "Deferred".to_string(),
        detail: "No commit is created by this preview.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 8,
        label: "Push and open pull request".to_string(),
        status: "Deferred".to_string(),
        detail: "Provider mutation remains outside this local preview.".to_string(),
    });
    steps.push(ReleaseRecipeStep {
        order: 9,
        label: "Prepare draft GitHub Release".to_string(),
        status: "Deferred".to_string(),
        detail: "Release publication is not enabled in V1.".to_string(),
    });

    let status = if blocked {
        "Blocked"
    } else if !has_release_changes || !has_validation {
        "Needs configuration"
    } else {
        "Ready for user review"
    };
    ReleaseRecipePreview {
        repository_id: repository.id.clone(),
        recipe_name: recipe.name,
        candidate_version: release.candidate_version.clone(),
        version_status: release.version_status.clone(),
        status: status.to_string(),
        reasons,
        steps,
        actions_performed: false,
        generated_at: iso_now(),
    }
}

fn prepare_repository_at(
    path: &Path,
    repository_id: &str,
    workspace_id: Option<&str>,
) -> Result<RepositoryPreparation, String> {
    let state = load_store_with_quality(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    if !Path::new(&workspace.path).is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }
    let provider_available =
        provider_context_available(repository, state.provider_status.state == "Ready");
    let pull_request = prepare_pull_request(repository, workspace, provider_available);
    let release = prepare_release(repository, workspace, provider_available);
    let recipe = prepare_release_recipe(repository, workspace, &release);
    Ok(RepositoryPreparation {
        repository_id: repository.id.clone(),
        pull_request,
        release,
        recipe,
        generated_at: iso_now(),
    })
}

fn remediation_handoff_check_for_repository(
    repository: &RepositorySnapshot,
    workspace_id: Option<&str>,
) -> Result<RemediationHandoffCheck, String> {
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    let workspace_path = Path::new(&workspace.path);
    if !workspace_path.is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }

    let generated_at = iso_now();
    let live_status = run_git(
        workspace_path,
        [
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ]
        .iter(),
    )
    .and_then(parse_git_status);
    let live_head_commit = git_static(workspace_path, &["rev-parse", "HEAD"]);
    let live_operation = live_status
        .as_ref()
        .ok()
        .and_then(|_| interrupted_operation(workspace_path));
    let (
        status,
        ready,
        checkpoint_required,
        workspace_dirty,
        branch,
        operation,
        next_safe_step,
        reasons,
    ) = match live_status {
            Ok(live_status) => {
                let operation = live_operation.or_else(|| workspace.operation.clone());
                let mut reasons = Vec::new();
                if live_status.dirty {
                    reasons.push(
                        "The workspace contains uncommitted changes; create a local checkpoint commit before handoff."
                            .to_string(),
                    );
                }
                if let Some(operation) = operation.as_ref() {
                    reasons.push(format!(
                        "The workspace has an interrupted Git operation ({operation}) that must be resolved before handoff."
                    ));
                }
                if !live_status.dirty && workspace.dirty {
                    reasons.push(
                        "Live Git is clean, but the persisted Pronto snapshot still reports dirty work; run a scoped refresh before handoff."
                            .to_string(),
                    );
                }
                let ready = !live_status.dirty && operation.is_none() && !workspace.dirty;
                let next_safe_step = if live_status.dirty {
                    "Review ownership, commit the intended changes on this branch, then rerun `pronto remediation handoff-check`."
                } else if operation.is_some() {
                    "Resolve the interrupted Git operation without discarding unrelated work, then rerun `pronto remediation handoff-check`."
                } else if workspace.dirty {
                    "Run the repository-scoped `pronto refresh` after the checkpoint commit, then rerun `pronto remediation handoff-check`."
                } else {
                    "Proceed with the scoped remediation handoff; this check performed no repository mutation."
                };
                (
                    if ready { "ready" } else { "blocked" },
                    ready,
                    live_status.dirty || operation.is_some(),
                    live_status.dirty,
                    live_status.branch,
                    operation,
                    next_safe_step.to_string(),
                    reasons,
                )
            }
            Err(status_error) => {
                (
                    "unknown",
                    false,
                    true,
                    false,
                    workspace.branch.clone(),
                    workspace.operation.clone(),
                    "Restore live Git access, then rerun `pronto remediation handoff-check`; no repository mutation was attempted."
                        .to_string(),
                    vec![format!(
                        "Live Git status could not be established: {status_error} Remediation advancement is blocked until the workspace can be checked."
                    )],
                )
            }
        };

    Ok(RemediationHandoffCheck {
        schema_version: REMEDIATION_HANDOFF_SCHEMA.to_string(),
        generated_at,
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        workspace_id: workspace.id.clone(),
        workspace_path: workspace.path.clone(),
        branch,
        head_commit: live_head_commit.or_else(|| workspace.last_commit.clone()),
        status: status.to_string(),
        ready,
        checkpoint_required,
        workspace_dirty,
        persisted_snapshot_dirty: workspace.dirty,
        operation,
        reasons,
        next_safe_step,
        authorization: "Read-only live Git check; no commit, stash, merge, rebase, push, or file edit was performed."
            .to_string(),
    })
}

fn remediation_handoff_check_at(
    path: &Path,
    query: &str,
    workspace_id: Option<&str>,
) -> Result<RemediationHandoffCheck, String> {
    let state = load_store_read_only(path)?;
    let snapshot = snapshot_from_store(path, &state);
    let repository = find_cli_repository(&snapshot, query)?;
    remediation_handoff_check_for_repository(repository, workspace_id)
}

fn branch_integration_state(
    path: &Path,
    branch: &str,
    default_branch: Option<&str>,
    current_workspace: Option<&WorkspaceSummary>,
) -> String {
    if current_workspace.is_some_and(|workspace| !workspace.status_available) {
        return "Unknown".to_string();
    }
    let Some(target) = default_branch else {
        return "Target unknown".to_string();
    };
    if branch == target {
        return "No unique commits".to_string();
    }
    let unique = unique_commits(path, branch, Some(target));
    if unique == 0 {
        return "Already integrated".to_string();
    }
    if let Some(workspace) = current_workspace {
        if workspace.operation.is_some() || workspace.dirty || workspace.activity.state == "Active"
        {
            return "Blocked".to_string();
        }
    }
    "Integration eligible".to_string()
}

fn scan_workspace(
    path: &Path,
    is_primary: bool,
    default_branch: Option<&str>,
    repository_target_branch: Option<&str>,
    repository_target_confidence: &str,
    existing: Option<&RepositorySnapshot>,
) -> WorkspaceSummary {
    let (status_result, totals, operation, last_commit, last_commit_at, last_activity_at) =
        workspace_status(path);
    let (status_available, status, status_error) = match status_result {
        Ok(status) => (true, status, None),
        Err(error) => (
            false,
            ParsedStatus {
                branch: "Unknown".to_string(),
                ..ParsedStatus::default()
            },
            Some(error),
        ),
    };
    let remote_freshness = existing
        .and_then(|repository| repository.last_fetch_at.clone())
        .unwrap_or_else(|| "Not fetched by Pronto".to_string());
    let activity = collect_workspace_activity(
        path,
        status_available && status.dirty,
        if status_available { status.ahead } else { 0 },
    );
    let sync_state = if !status_available {
        "Git status unavailable".to_string()
    } else if status.upstream.is_none() {
        "No upstream".to_string()
    } else if status.ahead > 0 && status.behind > 0 {
        format!(
            "Diverged · {} ahead / {} behind",
            status.ahead, status.behind
        )
    } else if status.ahead > 0 {
        format!("Ahead by {}", status.ahead)
    } else if status.behind > 0 {
        format!("Behind by {}", status.behind)
    } else {
        "Synced".to_string()
    };
    let (role, role_confidence, target_branch, target_confidence) = if status_available {
        let (mut role, mut role_confidence) = branch_role(&status.branch, default_branch);
        let (mut target_branch, mut target_confidence) =
            target_for_branch(&status.branch, repository_target_branch);
        if target_branch.is_some() {
            target_confidence = repository_target_confidence.to_string();
        }
        if let Some(manifest) = activity.manifest.as_ref() {
            if repository_target_branch.is_none() {
                if let Some(manifest_target) = manifest.target_branch.as_ref() {
                    target_branch = Some(manifest_target.clone());
                    target_confidence = "High".to_string();
                }
            }
            if manifest.agent_type.is_some() && role != "Production" {
                role = "Agent task".to_string();
                role_confidence = "High".to_string();
            }
        }
        (role, role_confidence, target_branch, target_confidence)
    } else {
        (
            "Unknown".to_string(),
            "Unknown".to_string(),
            None,
            "Unknown".to_string(),
        )
    };
    let integration_state = if status_available {
        branch_integration_state(path, &status.branch, repository_target_branch, None)
    } else {
        "Unknown".to_string()
    };
    let mut workspace = WorkspaceSummary {
        id: path_id("workspace", path),
        path: path.to_string_lossy().to_string(),
        is_primary,
        branch: status.branch,
        status_available,
        status_error,
        dirty: status_available && status.dirty,
        added: totals.added,
        removed: totals.removed,
        line_totals_partial: totals.partial,
        sync_state,
        remote_freshness,
        ahead: status.ahead,
        behind: status.behind,
        upstream: status.upstream,
        operation,
        last_commit,
        last_commit_at,
        last_activity_at,
        integration_state,
        target_branch,
        target_confidence,
        role,
        role_confidence,
        activity,
        sync_detail: None,
    };
    if workspace.status_available {
        workspace.integration_state = branch_integration_state(
            path,
            &workspace.branch,
            repository_target_branch,
            Some(&workspace),
        );
    }
    workspace
}

fn evidence(label: &str, value: String, source: &str, observed_at: &str) -> EvidenceItem {
    EvidenceItem {
        label: label.to_string(),
        value,
        source: source.to_string(),
        observed_at: observed_at.to_string(),
    }
}

fn condition_fingerprint(kind: &str, values: &[String]) -> String {
    format!("{kind}|{}", values.join("|"))
}

fn condition(
    repository_id: &str,
    kind: &str,
    title: &str,
    summary: String,
    priority: u8,
    fingerprint: String,
    rule: &str,
    evidence: Vec<EvidenceItem>,
    missing: Vec<String>,
    confidence: Option<&str>,
    freshness: Option<String>,
    expected: &[ExpectedCondition],
) -> Condition {
    let id = format!("{repository_id}:{kind}");
    let status = expected.iter().any(|item| {
        item.repository_id == repository_id
            && item.condition_id == id
            && item.fingerprint == fingerprint
    });
    Condition {
        id,
        kind: kind.to_string(),
        title: title.to_string(),
        summary,
        priority,
        status: if status {
            "Expected".to_string()
        } else {
            "Active".to_string()
        },
        fingerprint,
        rule: rule.to_string(),
        evidence,
        missing,
        confidence: confidence.map(str::to_string),
        freshness,
    }
}

fn build_conditions(
    repository_id: &str,
    workspace: &WorkspaceSummary,
    default_branch: Option<&str>,
    expected: &[ExpectedCondition],
    observed_at: &str,
) -> Vec<Condition> {
    let mut conditions = Vec::new();
    if !workspace.status_available {
        let status_error = workspace_status_unavailable_reason(workspace);
        conditions.push(condition(
            repository_id,
            "git-status-unavailable",
            "Git status unavailable",
            status_error.clone(),
            1,
            condition_fingerprint(
                "git-status-unavailable",
                &[workspace.path.clone(), status_error.clone()],
            ),
            "Pronto could not read the workspace's local Git status; branch, cleanliness, upstream, and sync state are unknown.",
            vec![evidence(
                "Git status",
                status_error,
                "git status --porcelain=v2",
                observed_at,
            )],
            vec!["Restore local Git status access, then run a scoped refresh.".to_string()],
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.activity.state == "Active" {
        let signal_evidence = workspace
            .activity
            .signals
            .iter()
            .map(|signal| {
                evidence(
                    "Activity signal",
                    format!("{} · {}", signal.source, signal.summary),
                    "Local process and manifest metadata",
                    observed_at,
                )
            })
            .collect::<Vec<_>>();
        conditions.push(condition(
            repository_id,
            "active-agent-workspace",
            "Agent workspace active",
            "Process or manifest evidence indicates this workspace is still active.".to_string(),
            2,
            condition_fingerprint(
                "active-agent",
                &[
                    workspace.branch.clone(),
                    workspace.activity.confidence.clone(),
                ],
            ),
            "An associated process or active agent manifest was detected without capturing terminal contents, prompts, or source text.",
            signal_evidence,
            vec!["Wait for the activity signal to end before integration or cleanup.".to_string()],
            Some(&workspace.activity.confidence),
            None,
            expected,
        ));
    }
    if let Some(operation) = &workspace.operation {
        conditions.push(condition(
            repository_id,
            "interrupted-operation",
            "Interrupted Git operation",
            operation.clone(),
            1,
            condition_fingerprint("operation", std::slice::from_ref(operation)),
            "A Git operation marker exists in the workspace metadata.",
            vec![evidence(
                "Operation",
                operation.clone(),
                "Git operation marker",
                observed_at,
            )],
            vec!["The operation must be completed or resolved outside Pronto.".to_string()],
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.dirty {
        let summary = if workspace.line_totals_partial {
            "Dirty · line totals partial".to_string()
        } else {
            format!("Dirty · +{} / −{}", workspace.added, workspace.removed)
        };
        conditions.push(condition(
      repository_id,
      "dirty-workspace",
      "Dirty workspace",
      summary,
      2,
      condition_fingerprint(
        "dirty",
        &[
          workspace.branch.clone(),
          workspace.added.to_string(),
          workspace.removed.to_string(),
          workspace.line_totals_partial.to_string(),
        ],
      ),
      "The local Git status contains uncommitted work; line totals are aggregated without exposing filenames or diff content.",
      vec![
        evidence("Branch", workspace.branch.clone(), "git status --porcelain=v2", observed_at),
        evidence("Added lines", workspace.added.to_string(), "git diff --numstat", observed_at),
        evidence("Removed lines", workspace.removed.to_string(), "git diff --numstat", observed_at),
      ],
      if workspace.line_totals_partial {
        vec!["Binary, unreadable, or oversized changes were not fully countable.".to_string()]
      } else {
        Vec::new()
      },
      Some("High"),
      None,
      expected,
    ));
    }
    if workspace.upstream.is_some() && workspace.ahead > 0 && workspace.behind > 0 {
        conditions.push(condition(
      repository_id,
      "diverged-branch",
      "Diverged branch",
      workspace.sync_state.clone(),
      3,
      condition_fingerprint(
        "diverged",
        &[workspace.branch.clone(), workspace.ahead.to_string(), workspace.behind.to_string()],
      ),
      "The local branch and its tracked upstream each contain commits the other side cannot reach.",
      vec![
        evidence("Ahead", workspace.ahead.to_string(), "git status --porcelain=v2", observed_at),
        evidence("Behind", workspace.behind.to_string(), "git status --porcelain=v2", observed_at),
      ],
      Vec::new(),
      Some("High"),
      Some(workspace.remote_freshness.clone()),
      expected,
    ));
    } else if workspace.upstream.is_some() && workspace.ahead > 0 {
        conditions.push(condition(
            repository_id,
            "unpushed-commits",
            "Unpushed commits",
            workspace.sync_state.clone(),
            5,
            condition_fingerprint(
                "ahead",
                &[workspace.branch.clone(), workspace.ahead.to_string()],
            ),
            "The local branch is ahead of its tracked upstream.",
            vec![evidence(
                "Ahead",
                workspace.ahead.to_string(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    } else if workspace.upstream.is_some() && workspace.behind > 0 {
        conditions.push(condition(
            repository_id,
            "behind-remote",
            "Behind tracked branch",
            workspace.sync_state.clone(),
            6,
            condition_fingerprint(
                "behind",
                &[workspace.branch.clone(), workspace.behind.to_string()],
            ),
            "The local branch is behind its tracked upstream.",
            vec![evidence(
                "Behind",
                workspace.behind.to_string(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    }
    if workspace.upstream.is_some() && workspace.remote_freshness == "Not fetched by Pronto" {
        conditions.push(condition(
            repository_id,
            "remote-stale",
            "Remote state stale",
            "Pronto has not recorded a successful fetch for this tracked branch.".to_string(),
            8,
            condition_fingerprint("remote-stale", std::slice::from_ref(&workspace.branch)),
            "Remote comparisons remain explicitly stale until Pronto records a successful fetch.",
            vec![evidence(
                "Freshness",
                workspace.remote_freshness.clone(),
                "Local Pronto state",
                observed_at,
            )],
            vec!["Run an explicit refresh when network access is available.".to_string()],
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    }
    if workspace.status_available
        && workspace.upstream.is_none()
        && default_branch.is_some_and(|default| default != workspace.branch)
    {
        conditions.push(condition(
            repository_id,
            "no-upstream",
            "No upstream",
            format!("{} has no tracked remote branch.", workspace.branch),
            5,
            condition_fingerprint("no-upstream", std::slice::from_ref(&workspace.branch)),
            "The current non-default branch has no configured upstream.",
            vec![evidence(
                "Branch",
                workspace.branch.clone(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.integration_state == "Integration eligible" {
        conditions.push(condition(
      repository_id,
      "integration-eligible",
      "Integration eligible",
      format!("{} has unique commits relative to {} and its workspace is clean.", workspace.branch, workspace.target_branch.clone().unwrap_or_else(|| "the target".to_string())),
      7,
      condition_fingerprint(
        "integration",
        &[
          workspace.branch.clone(),
          workspace.target_branch.clone().unwrap_or_default(),
          workspace.integration_state.clone(),
        ],
      ),
      "The branch has unique commits, a known target, a clean workspace, no detected operation, and no provider requirement known locally.",
      vec![
        evidence("Branch role", workspace.role.clone(), "Local branch name and default branch", observed_at),
        evidence("Target", workspace.target_branch.clone().unwrap_or_default(), "Local default branch", observed_at),
        evidence("Workspace", "Clean".to_string(), "git status --porcelain=v2", observed_at),
      ],
      vec!["GitHub checks and pull-request permissions are not connected in this slice.".to_string()],
      Some(&workspace.target_confidence),
      None,
      expected,
    ));
    }
    conditions.sort_by_key(|item| item.priority);
    conditions
}

fn observed_fetch_at(path: &Path) -> Option<String> {
    let fetch_head = git_static(path, &["rev-parse", "--git-path", "FETCH_HEAD"])?;
    let fetch_head = PathBuf::from(fetch_head);
    let fetch_head = if fetch_head.is_absolute() {
        fetch_head
    } else {
        path.join(fetch_head)
    };
    let modified = fs::metadata(fetch_head).ok()?.modified().ok()?;
    Some(DateTime::<Utc>::from(modified).to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn scan_repository(
    path: &Path,
    existing: Option<&RepositorySnapshot>,
    expected: &[ExpectedCondition],
) -> RepositorySnapshot {
    let observed_at = iso_now();
    let repository_id = path_id("repository", path);
    let remote_url = git_static(path, &["remote", "get-url", "origin"]);
    let remote_unchanged = existing.is_some_and(|repository| repository.remote_url == remote_url);
    let provider_state = existing
        .filter(|_| remote_unchanged)
        .map(|repository| repository.provider_state.clone())
        .unwrap_or_else(|| {
            if remote_url
                .as_ref()
                .is_some_and(|url| url.contains("github.com"))
            {
                "GitHub remote detected · provider not connected".to_string()
            } else if remote_url.is_some() {
                "Remote detected · provider not connected".to_string()
            } else {
                "No remote configured".to_string()
            }
        });
    let locality = existing
        .filter(|_| remote_unchanged)
        .map(|repository| repository.locality.clone())
        .unwrap_or_else(|| {
            if remote_url.is_some() {
                "Connected".to_string()
            } else {
                "Local only".to_string()
            }
        });
    let worktree_records = parse_worktrees(path);
    let provisional_branch = git_static(path, &["branch", "--show-current"])
        .unwrap_or_else(|| "Detached HEAD".to_string());
    let default_branch = detect_default_branch(path, &provisional_branch);
    let configured_target_branch = existing
        .filter(|repository| repository.target_branch_configured)
        .and_then(|repository| repository.target_branch.clone());
    let target_branch = configured_target_branch
        .clone()
        .or_else(|| default_branch.clone());
    let target_confidence = if configured_target_branch.is_some() {
        "High"
    } else {
        "Medium"
    };
    let recorded_fetch = existing
        .filter(|_| remote_unchanged)
        .and_then(|repository| repository.last_fetch_at.clone());
    let observed_fetch = remote_url.as_ref().and_then(|_| observed_fetch_at(path));
    let existing_last_fetch = [recorded_fetch, observed_fetch].into_iter().flatten().max();
    let mut workspaces = Vec::new();
    for record in worktree_records {
        let canonical = canonical_path(&record.path).unwrap_or(record.path.clone());
        let is_primary = canonical == canonical_path(path).unwrap_or_else(|| path.to_path_buf());
        let workspace_existing = existing.and_then(|repository| {
            repository
                .workspaces
                .iter()
                .find(|workspace| workspace.path == canonical.to_string_lossy())
        });
        let mut workspace = scan_workspace(
            &canonical,
            is_primary,
            default_branch.as_deref(),
            target_branch.as_deref(),
            target_confidence,
            existing,
        );
        if existing_last_fetch.is_some() {
            workspace.remote_freshness = existing_last_fetch
                .clone()
                .unwrap_or_else(|| "Not fetched by Pronto".to_string());
        }
        if let Some(existing_workspace) = workspace_existing {
            if workspace.last_activity_at.is_none() {
                workspace.last_activity_at = existing_workspace.last_activity_at.clone();
            }
        }
        workspaces.push(workspace);
    }
    if workspaces.is_empty() {
        workspaces.push(scan_workspace(
            path,
            true,
            default_branch.as_deref(),
            target_branch.as_deref(),
            target_confidence,
            existing,
        ));
    }
    let primary_index = workspaces
        .iter()
        .position(|workspace| workspace.is_primary)
        .unwrap_or(0);
    let primary = workspaces[primary_index].clone();
    let branch_records = parse_branches(path);
    let mut branches = Vec::new();
    for record in branch_records {
        let current_workspace = workspaces
            .iter()
            .find(|workspace| workspace.branch == record.name);
        let (role, role_confidence) = current_workspace
            .map(|workspace| (workspace.role.clone(), workspace.role_confidence.clone()))
            .unwrap_or_else(|| branch_role(&record.name, default_branch.as_deref()));
        let (branch_target, branch_target_confidence) = current_workspace
            .map(|workspace| {
                (
                    workspace.target_branch.clone(),
                    workspace.target_confidence.clone(),
                )
            })
            .unwrap_or_else(|| {
                let (branch_target, mut confidence) =
                    target_for_branch(&record.name, target_branch.as_deref());
                if branch_target.is_some() {
                    confidence = target_confidence.to_string();
                }
                (branch_target, confidence)
            });
        let integration_state = if primary.status_available {
            branch_integration_state(
                path,
                &record.name,
                target_branch.as_deref(),
                current_workspace,
            )
        } else {
            "Unknown".to_string()
        };
        let ahead = current_workspace
            .map(|workspace| workspace.ahead)
            .unwrap_or(0);
        let behind = current_workspace
            .map(|workspace| workspace.behind)
            .unwrap_or(0);
        let workspace_id = current_workspace.map(|workspace| workspace.id.clone());
        branches.push(BranchSummary {
            name: record.name,
            role,
            role_confidence,
            target_branch: branch_target,
            target_confidence: branch_target_confidence,
            ahead,
            behind,
            integration_state,
            workspace_id,
            last_commit: record.last_commit,
            last_commit_at: record.last_commit_at,
        });
    }
    let mut primary_for_conditions = primary.clone();
    if primary.status_available {
        primary_for_conditions.integration_state = branch_integration_state(
            path,
            &primary.branch,
            target_branch.as_deref(),
            Some(&primary),
        );
    }
    let conditions = build_conditions(
        &repository_id,
        &primary_for_conditions,
        target_branch.as_deref(),
        expected,
        &observed_at,
    );
    let submodules = parse_submodules(path);
    let lifecycle_candidate = if primary.last_activity_at.as_ref().is_some_and(|value| {
        DateTime::parse_from_rfc3339(value)
            .map(|date| {
                Utc::now()
                    .signed_duration_since(date.with_timezone(&Utc))
                    .num_days()
                    < 90
            })
            .unwrap_or(false)
    }) {
        "Active"
    } else {
        "Maintenance"
    };
    RepositorySnapshot {
        id: repository_id,
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unnamed repository")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        locality: locality.to_string(),
        lifecycle: existing
            .map(|repository| repository.lifecycle.clone())
            .unwrap_or_else(|| "Unconfirmed".to_string()),
        lifecycle_candidate: lifecycle_candidate.to_string(),
        remote_url,
        provider_state,
        branch: primary.branch.clone(),
        default_branch,
        target_branch,
        target_branch_configured: configured_target_branch.is_some(),
        workspace: primary,
        workspaces,
        branches,
        submodules,
        pull_requests: existing
            .map(|repository| repository.pull_requests.clone())
            .unwrap_or_default(),
        releases: existing
            .map(|repository| repository.releases.clone())
            .unwrap_or_default(),
        quality: existing
            .map(|repository| repository.quality.clone())
            .unwrap_or_default(),
        project_compass: project_compass::inspect(path),
        release_rule: existing.and_then(|repository| repository.release_rule.clone()),
        release_recipe: existing.and_then(|repository| repository.release_recipe.clone()),
        confirmed_release_version: existing
            .and_then(|repository| repository.confirmed_release_version.clone()),
        ai_permission: existing
            .map(|repository| repository.ai_permission.clone())
            .unwrap_or_else(default_ai_permission),
        conditions,
        last_scan_at: observed_at,
        last_fetch_at: existing_last_fetch,
        last_activity_at: primary_for_conditions.last_activity_at.clone(),
    }
}

fn transition_fingerprint(repository: &RepositorySnapshot) -> String {
    let condition_state = repository
        .conditions
        .iter()
        .map(|condition| format!("{}:{}", condition.id, condition.fingerprint))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        repository.branch,
        repository.target_branch.as_deref().unwrap_or_default(),
        repository.target_branch_configured,
        repository.workspace.dirty,
        repository.workspace.added,
        repository.workspace.removed,
        repository.workspace.sync_state,
        repository.workspace.activity.state,
        condition_state
    )
}

fn event_summary(repository: &RepositorySnapshot) -> String {
    if repository.conditions.is_empty() {
        format!("{} has no active conditions", repository.name)
    } else {
        format!(
            "{} · {}",
            repository.name,
            repository
                .conditions
                .iter()
                .map(|condition| condition.title.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        )
    }
}

fn append_transition_event(
    state: &mut StoreState,
    old: Option<&RepositorySnapshot>,
    new: &RepositorySnapshot,
) {
    let new_fingerprint = transition_fingerprint(new);
    let changed = old
        .map(|repository| transition_fingerprint(repository) != new_fingerprint)
        .unwrap_or(true);
    if !changed {
        return;
    }
    let sequence = NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed);
    state.events.push(EventRecord {
        id: format!("event:{}:{}:{}", new.id, new.last_scan_at, sequence),
        repository_id: new.id.clone(),
        kind: if old.is_some() {
            "state-transition".to_string()
        } else {
            "repository-discovered".to_string()
        },
        summary: event_summary(new),
        fingerprint: new_fingerprint,
        created_at: new.last_scan_at.clone(),
    });
}

fn prune_events(state: &mut StoreState) {
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    state.events.retain(|event| {
        DateTime::parse_from_rfc3339(&event.created_at)
            .map(|date| date.with_timezone(&Utc) >= cutoff)
            .unwrap_or(true)
    });
    if state.events.len() > 2_000 {
        let keep_from = state.events.len() - 2_000;
        state.events = state.events.split_off(keep_from);
    }
}

fn prune_action_audits(state: &mut StoreState) {
    let cutoff = Utc::now() - chrono::Duration::days(state.retention_days.max(1));
    state.action_audits.retain(|audit| {
        DateTime::parse_from_rfc3339(&audit.created_at)
            .map(|date| date.with_timezone(&Utc) >= cutoff)
            .unwrap_or(true)
    });
    if state.action_audits.len() > 2_000 {
        let keep_from = state.action_audits.len() - 2_000;
        state.action_audits = state.action_audits.split_off(keep_from);
    }
}

fn action_audit_id(action: &str, created_at: &str) -> String {
    let sequence = NEXT_ACTION_AUDIT_ID.fetch_add(1, Ordering::Relaxed);
    format!("audit:{action}:{created_at}:{sequence}")
}

fn action_targets(
    state: &StoreState,
    action: &str,
    repository_id: Option<&str>,
) -> Result<(Vec<String>, String), String> {
    match action {
        "refresh" => {
            if repository_id.is_some() {
                return Err(
                    "Refresh preflight targets all registered discovery roots; omit repository_id"
                        .to_string(),
                );
            }
            let target_ids = state
                .roots
                .iter()
                .map(|root| root.id.clone())
                .collect::<Vec<_>>();
            let target_label = if target_ids.is_empty() {
                "No registered discovery roots".to_string()
            } else {
                "All registered discovery roots".to_string()
            };
            Ok((target_ids, target_label))
        }
        "inspect" => {
            if let Some(repository_id) = repository_id {
                let repository = state
                    .repositories
                    .iter()
                    .find(|repository| repository.id == repository_id)
                    .ok_or_else(|| "Repository is not registered".to_string())?;
                return Ok((
                    vec![repository.id.clone()],
                    format!("Repository {}", repository.name),
                ));
            }
            let target_ids = state
                .repositories
                .iter()
                .map(|repository| repository.id.clone())
                .collect::<Vec<_>>();
            let target_label = if target_ids.is_empty() {
                "No scanned repositories".to_string()
            } else {
                "All scanned repositories".to_string()
            };
            Ok((target_ids, target_label))
        }
        _ => Ok((Vec::new(), "No target selected".to_string())),
    }
}

fn build_action_preflight(
    state: &StoreState,
    action: &str,
    repository_id: Option<&str>,
) -> Result<ActionPreflight, String> {
    let normalized_action = action.trim().to_ascii_lowercase();
    let allowed = matches!(normalized_action.as_str(), "refresh" | "inspect");
    let (target_ids, target_label) = if allowed {
        action_targets(state, &normalized_action, repository_id)?
    } else {
        action_targets(state, "unsupported", None)?
    };
    let created_at = iso_now();
    let risk = if allowed { "read-only" } else { "blocked" }.to_string();
    let status = if allowed { "Preflighted" } else { "Rejected" }.to_string();
    let summary = if allowed {
        format!("Read-only {normalized_action} preflight for {target_label}.")
    } else {
        format!(
            "Action '{normalized_action}' is not enabled; Git mutation and provider writes remain blocked."
        )
    };
    let audit = ActionAudit {
        id: action_audit_id(&normalized_action, &created_at),
        action: normalized_action,
        target_ids,
        risk,
        status,
        summary,
        created_at,
        completed_at: None,
    };
    Ok(ActionPreflight {
        audit,
        allowed,
        target_label,
    })
}

fn append_action_audit(state: &mut StoreState, preflight: &ActionPreflight) {
    state.action_audits.push(preflight.audit.clone());
    prune_action_audits(state);
}

fn update_action_audit(
    state: &mut StoreState,
    audit_id: &str,
    status: &str,
    summary: String,
) -> Result<(), String> {
    let audit = state
        .action_audits
        .iter_mut()
        .find(|audit| audit.id == audit_id)
        .ok_or_else(|| "Action audit record is no longer available".to_string())?;
    audit.status = status.to_string();
    audit.summary = summary;
    audit.completed_at = Some(iso_now());
    Ok(())
}

fn preflight_action_at(
    path: &Path,
    action: &str,
    repository_id: Option<&str>,
) -> Result<ActionPreflight, String> {
    let mut state = load_store(path)?;
    let preflight = build_action_preflight(&state, action, repository_id)?;
    append_action_audit(&mut state, &preflight);
    save_store(path, &state)?;
    Ok(preflight)
}

fn audited_scan_and_persist(
    path: &Path,
    state: &mut StoreState,
) -> Result<PortfolioSnapshot, String> {
    audited_scan_and_persist_scoped(path, state, None, None)
}

fn build_targeted_refresh_preflight(
    state: &StoreState,
    target_repository_ids: &HashSet<String>,
    target_label: &str,
) -> Result<ActionPreflight, String> {
    if target_repository_ids.is_empty() {
        return Err(format!("{target_label} has no registered repositories"));
    }
    if let Some(unknown) = target_repository_ids.iter().find(|repository_id| {
        !state
            .repositories
            .iter()
            .any(|repository| &repository.id == *repository_id)
    }) {
        return Err(format!("Repository {unknown} is not registered"));
    }
    let created_at = iso_now();
    let target_ids = target_repository_ids.iter().cloned().collect::<Vec<_>>();
    let audit = ActionAudit {
        id: action_audit_id("refresh", &created_at),
        action: "refresh".to_string(),
        target_ids,
        risk: "read-only".to_string(),
        status: "Preflighted".to_string(),
        summary: format!("Read-only refresh preflight for {target_label}."),
        created_at,
        completed_at: None,
    };
    Ok(ActionPreflight {
        audit,
        allowed: true,
        target_label: target_label.to_string(),
    })
}

fn audited_scan_and_persist_scoped(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    target_label: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    let preflight = match (target_repository_ids, target_label) {
        (Some(repository_ids), Some(label)) => {
            build_targeted_refresh_preflight(state, repository_ids, label)?
        }
        (None, None) => build_action_preflight(state, "refresh", None)?,
        _ => return Err("Refresh target metadata is incomplete".to_string()),
    };
    if !preflight.allowed {
        return Err("Local refresh action is not permitted".to_string());
    }
    let audit_id = preflight.audit.id.clone();
    append_action_audit(state, &preflight);
    save_store(path, state)?;

    match scan_and_persist_scoped(path, state, target_repository_ids) {
        Ok(_) => {
            update_action_audit(
                state,
                &audit_id,
                "Completed",
                format!(
                    "Read-only refresh completed for {}.",
                    preflight.target_label
                ),
            )?;
            prune_action_audits(state);
            save_store(path, state)?;
            Ok(snapshot_from_store(path, state))
        }
        Err(error) => {
            if update_action_audit(
                state,
                &audit_id,
                "Failed",
                format!("Read-only refresh failed for {}.", preflight.target_label),
            )
            .is_ok()
            {
                let _ = save_store(path, state);
            }
            Err(error)
        }
    }
}

fn build_repository_path_refresh_preflight(
    repository_id: &str,
    repository_path: &Path,
) -> ActionPreflight {
    let created_at = iso_now();
    let target_label = format!("Repository {}", repository_path.display());
    let audit = ActionAudit {
        id: action_audit_id("refresh", &created_at),
        action: "refresh".to_string(),
        target_ids: vec![repository_id.to_string()],
        risk: "read-only".to_string(),
        status: "Preflighted".to_string(),
        summary: format!(
            "Read-only refresh preflight for {target_label}; the repository path was not previously registered."
        ),
        created_at,
        completed_at: None,
    };
    ActionPreflight {
        audit,
        allowed: true,
        target_label,
    }
}

fn audited_scan_and_persist_repository_path(
    path: &Path,
    state: &mut StoreState,
    repository_path: &Path,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock(path)?;
    let repository_path = canonical_repository_path(repository_path)
        .ok_or_else(|| "The refresh target is not an accessible Git repository".to_string())?;
    let repository_id = path_id("repository", &repository_path);
    let preflight = build_repository_path_refresh_preflight(&repository_id, &repository_path);
    if !preflight.allowed {
        return Err("Local refresh action is not permitted".to_string());
    }
    let audit_id = preflight.audit.id.clone();
    append_action_audit(state, &preflight);
    save_store(path, state)?;

    match scan_and_persist_repository_path(path, state, &repository_path) {
        Ok(_) => {
            update_action_audit(
                state,
                &audit_id,
                "Completed",
                format!(
                    "Read-only refresh completed for {}.",
                    preflight.target_label
                ),
            )?;
            prune_action_audits(state);
            save_store(path, state)?;
            Ok(snapshot_from_store(path, state))
        }
        Err(error) => {
            if update_action_audit(
                state,
                &audit_id,
                "Failed",
                format!("Read-only refresh failed for {}.", preflight.target_label),
            )
            .is_ok()
            {
                let _ = save_store(path, state);
            }
            Err(error)
        }
    }
}

fn scan_and_persist_scoped(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
) -> Result<PortfolioSnapshot, String> {
    let mut discovered = HashMap::<String, PathBuf>::new();
    for root in &state.roots {
        for repository in discover_repositories(root) {
            let repository_id = path_id("repository", &repository);
            if target_repository_ids
                .map(|targets| targets.contains(&repository_id))
                .unwrap_or(true)
            {
                discovered.insert(repository_id, repository);
            }
        }
    }
    scan_discovered_and_persist(path, state, target_repository_ids, discovered)
}

fn scan_and_persist_repository_path(
    path: &Path,
    state: &mut StoreState,
    repository_path: &Path,
) -> Result<PortfolioSnapshot, String> {
    let repository_id = path_id("repository", repository_path);
    let target_repository_ids = [repository_id.clone()].into_iter().collect::<HashSet<_>>();
    let discovered = [(repository_id, repository_path.to_path_buf())]
        .into_iter()
        .collect::<HashMap<_, _>>();
    scan_discovered_and_persist(path, state, Some(&target_repository_ids), discovered)
}

fn scan_discovered_and_persist(
    path: &Path,
    state: &mut StoreState,
    target_repository_ids: Option<&HashSet<String>>,
    discovered: HashMap<String, PathBuf>,
) -> Result<PortfolioSnapshot, String> {
    let old_by_id = state
        .repositories
        .iter()
        .map(|repository| (repository.id.clone(), repository.clone()))
        .collect::<HashMap<_, _>>();
    let mut repositories = Vec::new();
    for (id, repository_path) in discovered {
        let repository = scan_repository(
            &repository_path,
            old_by_id.get(&id),
            &state.expected_conditions,
        );
        append_transition_event(state, old_by_id.get(&id), &repository);
        repositories.push(repository);
    }
    for (id, old) in old_by_id {
        if !repositories.iter().any(|repository| repository.id == id)
            && target_repository_ids
                .map(|targets| targets.contains(&id))
                .unwrap_or(true)
        {
            if target_repository_ids.is_none()
                && repository_is_ignored_by_existing_root(state, &old)
            {
                continue;
            }
            if Path::new(&old.path).exists() {
                let repository_path = PathBuf::from(&old.path);
                let repository =
                    scan_repository(&repository_path, Some(&old), &state.expected_conditions);
                append_transition_event(state, Some(&old), &repository);
                repositories.push(repository);
            }
        } else if target_repository_ids.is_some_and(|targets| !targets.contains(&id)) {
            repositories.push(old);
        }
    }
    sort_repositories_by_name(&mut repositories);
    state.repositories = repositories;
    apply_quality_evidence_scoped(state, target_repository_ids, None);
    apply_release_threshold_conditions(state);
    prune_events(state);
    save_store(path, state)?;
    record_analytics_samples(path, state)?;
    Ok(snapshot_from_store(path, state))
}

fn mutate_expected(
    path: &Path,
    repository_id: &str,
    condition_id: &str,
    should_mark: bool,
) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    if should_mark {
        let repository = state
            .repositories
            .iter()
            .find(|repository| repository.id == repository_id)
            .ok_or_else(|| "Repository is not registered".to_string())?;
        let condition = repository
            .conditions
            .iter()
            .find(|condition| condition.id == condition_id)
            .ok_or_else(|| "Condition is no longer active".to_string())?;
        state.expected_conditions.retain(|item| {
            !(item.repository_id == repository_id && item.condition_id == condition_id)
        });
        state.expected_conditions.push(ExpectedCondition {
            repository_id: repository_id.to_string(),
            condition_id: condition_id.to_string(),
            fingerprint: condition.fingerprint.clone(),
            marked_at: iso_now(),
        });
    } else {
        state.expected_conditions.retain(|item| {
            !(item.repository_id == repository_id && item.condition_id == condition_id)
        });
    }
    for repository in &mut state.repositories {
        for condition in &mut repository.conditions {
            let expected = state.expected_conditions.iter().any(|item| {
                item.repository_id == repository.id
                    && item.condition_id == condition.id
                    && item.fingerprint == condition.fingerprint
            });
            condition.status = if expected {
                "Expected".to_string()
            } else {
                "Active".to_string()
            };
        }
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn update_root_settings_at(
    path: &Path,
    root_id: &str,
    ignore_patterns: Vec<String>,
    refresh_policy: &str,
    background_monitoring: bool,
) -> Result<PortfolioSnapshot, String> {
    let normalized_patterns = normalize_ignore_patterns(ignore_patterns)?;
    let normalized_policy = normalize_refresh_policy(refresh_policy)?;
    let mut state = load_store(path)?;
    let root = state
        .roots
        .iter_mut()
        .find(|root| root.id == root_id)
        .ok_or_else(|| "Discovery root is not registered".to_string())?;
    root.ignore_patterns = normalized_patterns;
    root.refresh_policy = normalized_policy;
    root.background_monitoring = background_monitoring;
    audited_scan_and_persist(path, &mut state)
}

fn exclude_root_patterns_at(
    path: &Path,
    root_path: &str,
    patterns: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    let canonical_root = canonical_path(Path::new(root_path))
        .ok_or_else(|| "Choose an accessible folder for repository discovery".to_string())?;
    let root_string = canonical_root.to_string_lossy().to_string();
    let mut state = load_store(path)?;
    let root = state
        .roots
        .iter_mut()
        .find(|root| root.path == root_string)
        .ok_or_else(|| format!("Discovery root '{root_string}' is not registered"))?;
    let mut combined_patterns = root.ignore_patterns.clone();
    combined_patterns.extend(patterns);
    root.ignore_patterns = normalize_ignore_patterns(combined_patterns)?;
    audited_scan_and_persist(path, &mut state)
}

fn set_repository_lifecycle_at(
    path: &Path,
    repository_id: &str,
    lifecycle: &str,
) -> Result<PortfolioSnapshot, String> {
    let normalized_lifecycle = normalize_lifecycle(lifecycle)?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.lifecycle = normalized_lifecycle;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_repository_target_branch_at(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
) -> Result<PortfolioSnapshot, String> {
    set_repository_target_branch_at_with_lock_timeout(
        path,
        repository_id,
        target_branch,
        StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS),
    )
}

fn set_repository_target_branch_at_with_lock_timeout(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
    lock_timeout: StdDuration,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock_with_timeout(path, lock_timeout)?;
    let target_branch = target_branch.trim();
    if target_branch.is_empty() || target_branch.contains('\0') {
        return Err("Choose a valid target branch".to_string());
    }
    let mut state = load_store(path)?;
    let repository_index = state
        .repositories
        .iter()
        .position(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let repository = state.repositories[repository_index].clone();
    if !repository
        .branches
        .iter()
        .any(|branch| branch.name == target_branch)
    {
        return Err(format!(
            "Target branch '{target_branch}' is not a local branch in {}",
            repository.name
        ));
    }
    let mut configured = repository.clone();
    configured.target_branch = Some(target_branch.to_string());
    configured.target_branch_configured = true;
    let rescanned = scan_repository(
        Path::new(&configured.path),
        Some(&configured),
        &state.expected_conditions,
    );
    append_transition_event(&mut state, Some(&repository), &rescanned);
    prune_events(&mut state);
    state.repositories[repository_index] = rescanned;
    if !state.remediation.id.is_empty() {
        state.remediation = remediation::rebuild_run(
            &state.repositories,
            &state.remediation,
            state.quality.latest_audit_id.as_deref(),
        );
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn refresh_repository_target_evidence_at(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
) -> Result<PortfolioSnapshot, String> {
    refresh_repository_target_evidence_at_with_lock_timeout(
        path,
        repository_id,
        target_branch,
        StdDuration::from_secs(STORE_WRITE_LOCK_WAIT_SECONDS),
    )
}

fn target_evidence_is_reusable(
    repository: &RepositorySnapshot,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    repository
        .quality
        .target_fleet_audit_root
        .as_deref()
        .is_some_and(|root| Path::new(root).is_dir())
        && quality::target_evidence_is_current(&repository.quality, target_branch, target_commit)
}

fn refresh_repository_target_evidence_at_with_lock_timeout(
    path: &Path,
    repository_id: &str,
    target_branch: &str,
    lock_timeout: StdDuration,
) -> Result<PortfolioSnapshot, String> {
    let _lock = acquire_store_write_lock_with_timeout(path, lock_timeout)?;
    let target_branch = target_branch.trim();
    if target_branch.is_empty() || target_branch.contains('\0') {
        return Err("Choose a valid target branch".to_string());
    }
    let mut state = load_store(path)?;
    let repository_index = state
        .repositories
        .iter()
        .position(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let repository = state.repositories[repository_index].clone();
    if !repository
        .branches
        .iter()
        .any(|branch| branch.name == target_branch)
    {
        return Err(format!(
            "Target branch '{target_branch}' is not a local branch in {}",
            repository.name
        ));
    }
    let repository_path = Path::new(&repository.path);
    let target_ref = format!("refs/heads/{target_branch}");
    let target_head = run_git(
        repository_path,
        vec![
            "rev-parse".to_string(),
            "--verify".to_string(),
            target_ref.clone(),
        ],
    )?;
    if !target_head.success || target_head.stdout.trim().is_empty() {
        return Err(format!(
            "Target branch '{target_branch}' could not be resolved in {}: {}",
            repository.name,
            concise_target_command_error(&target_head.stderr)
        ));
    }
    let target_head = target_head.stdout.trim().to_string();
    let reuse_target_evidence =
        target_evidence_is_reusable(&repository, target_branch, &target_head);
    let mut configured = repository.clone();
    configured.target_branch = Some(target_branch.to_string());
    configured.target_branch_configured = true;
    let mut rescanned = scan_repository(
        repository_path,
        Some(&configured),
        &state.expected_conditions,
    );
    if !reuse_target_evidence {
        rescanned.quality.target_fleet_audit_root = None;
    }
    append_transition_event(&mut state, Some(&repository), &rescanned);
    state.repositories[repository_index] = rescanned;

    if reuse_target_evidence {
        let target_ids = [repository_id.to_string()]
            .into_iter()
            .collect::<HashSet<_>>();
        apply_quality_evidence_scoped(&mut state, Some(&target_ids), None);
        apply_release_threshold_conditions(&mut state);
        if let Some(repository) = state.repositories.get_mut(repository_index) {
            let short_head = target_head.chars().take(8).collect::<String>();
            repository.quality.ingestion_message = Some(format!(
                "Reused target evidence for {target_branch} @ {short_head}; target head is unchanged."
            ));
        }
        save_store(path, &state)?;
        return Ok(snapshot_from_store(path, &state));
    }

    let run_id_prefix = target_evidence_run_prefix(&repository, target_branch);
    let target_parent = target_evidence_artifact_parent();
    fs::create_dir_all(&target_parent).map_err(|error| {
        format!(
            "Could not create Pronto target evidence workspace {}: {error}",
            target_parent.display()
        )
    })?;
    let target_worktree = target_parent.join(format!("{run_id_prefix}-worktree"));
    if target_worktree.exists() {
        return Err(format!(
            "Target evidence workspace already exists: {}",
            target_worktree.display()
        ));
    }
    let add_result = run_git(
        repository_path,
        vec![
            "worktree".to_string(),
            "add".to_string(),
            "--force".to_string(),
            "--quiet".to_string(),
            target_worktree.to_string_lossy().to_string(),
            target_ref,
        ],
    )?;
    if !add_result.success {
        return Err(format!(
            "Could not create a clean target worktree for {target_branch}: {}",
            concise_target_command_error(&add_result.stderr)
        ));
    }

    let qr_executable = resolve_qr_executable(None);
    let fleet_output_base = repository_path
        .join(".quality-runner")
        .join("fleet-audit")
        .join("target")
        .join(&run_id_prefix);
    let mut outcomes = Vec::new();
    let mut fleet_root = None;
    match run_target_qr_refresh(
        &qr_executable,
        &target_worktree,
        repository_path,
        &run_id_prefix,
        target_branch,
    ) {
        Ok(outcome) => outcomes.push(outcome),
        Err(error) => outcomes.push(format!(
            "QR evidence unavailable: {}",
            concise_target_command_error(&error)
        )),
    }
    match run_target_fleet_audit(
        &qr_executable,
        &target_worktree,
        repository_path,
        target_worktree.parent().unwrap_or(&target_parent),
        &fleet_output_base,
        target_branch,
        &repository_feed_id(&repository),
    ) {
        Ok((outcome, root)) => {
            outcomes.push(outcome);
            fleet_root = Some(root);
        }
        Err(error) => outcomes.push(format!(
            "fleet evidence unavailable: {}",
            concise_target_command_error(&error)
        )),
    }

    let cleanup = run_git(
        repository_path,
        vec![
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            target_worktree.to_string_lossy().to_string(),
        ],
    )?;
    if !cleanup.success {
        return Err(format!(
            "Target evidence refresh could not remove its temporary worktree {}: {}",
            target_worktree.display(),
            concise_target_command_error(&cleanup.stderr)
        ));
    }

    if fleet_root.is_none() {
        let _ = fs::remove_dir_all(&fleet_output_base);
    }
    state.repositories[repository_index]
        .quality
        .target_fleet_audit_root = fleet_root
        .as_ref()
        .map(|root| root.to_string_lossy().to_string());
    let target_ids = [repository_id.to_string()]
        .into_iter()
        .collect::<HashSet<_>>();
    let projection_root = fleet_root.as_deref().unwrap_or(&fleet_output_base);
    apply_quality_evidence_scoped(&mut state, Some(&target_ids), Some(projection_root));
    apply_release_threshold_conditions(&mut state);
    if let Some(repository) = state.repositories.get_mut(repository_index) {
        let short_head = target_head.chars().take(8).collect::<String>();
        let outcome = if outcomes.is_empty() {
            "No target evidence commands returned an outcome".to_string()
        } else {
            outcomes.join(" · ")
        };
        repository.quality.ingestion_message = Some(format!(
            "Target evidence refresh for {target_branch} @ {short_head}: {outcome}"
        ));
        repository.quality.target_fleet_audit_root = fleet_root
            .as_ref()
            .map(|root| root.to_string_lossy().to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_rule_at(
    path: &Path,
    repository_id: &str,
    release_rule: Option<ReleaseRuleConfig>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_rule = release_rule.map(normalize_release_rule).transpose()?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.release_rule = normalized_rule;
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_recipe_at(
    path: &Path,
    repository_id: &str,
    release_recipe: Option<ReleaseRecipeConfig>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_recipe = release_recipe.map(normalize_release_recipe).transpose()?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.release_recipe = normalized_recipe;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_release_version_at(
    path: &Path,
    repository_id: &str,
    release_version: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_version = match release_version {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(normalize_release_version(&value)?),
        None => None,
    };
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    if let Some(version) = normalized_version.as_ref() {
        let provider_available =
            provider_context_available(repository, state.provider_status.state == "Ready");
        let candidate = prepare_release(repository, &repository.workspace, provider_available)
            .candidate_version;
        if candidate.as_ref() != Some(version) {
            return Err(
                "Release version must match the current deterministic candidate".to_string(),
            );
        }
    }
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.confirmed_release_version = normalized_version;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_ai_permission_at(
    path: &Path,
    repository_id: &str,
    permission: &str,
) -> Result<PortfolioSnapshot, String> {
    let normalized_permission = normalize_ai_permission(permission)?;
    let mut state = load_store(path)?;
    let repository = state
        .repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    repository.ai_permission = normalized_permission;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn set_retention_days_at(path: &Path, retention_days: i64) -> Result<PortfolioSnapshot, String> {
    if !(1..=3_650).contains(&retention_days) {
        return Err("Retention must be between 1 and 3650 days".to_string());
    }
    let mut state = load_store(path)?;
    state.retention_days = retention_days;
    prune_events(&mut state);
    prune_action_audits(&mut state);
    save_store(path, &state)?;
    prune_analytics_samples(path, retention_days)?;
    Ok(snapshot_from_store(path, &state))
}

fn upsert_product_at(
    path: &Path,
    product_id: Option<&str>,
    name: &str,
    repository_ids: Vec<String>,
    release_mode: &str,
) -> Result<PortfolioSnapshot, String> {
    let clean_name = normalize_name(name, "Product")?;
    let clean_release_mode = normalize_release_mode(release_mode)?;
    let mut state = load_store(path)?;
    let clean_repository_ids = normalize_repository_ids(&state, repository_ids)?;
    if state.products.iter().any(|product| {
        product.name.eq_ignore_ascii_case(&clean_name) && product_id != Some(product.id.as_str())
    }) {
        return Err(format!("A product named '{clean_name}' already exists"));
    }
    let now = iso_now();
    if let Some(product_id) = product_id.filter(|value| !value.trim().is_empty()) {
        let product = state
            .products
            .iter_mut()
            .find(|product| product.id == product_id)
            .ok_or_else(|| "Product is not registered".to_string())?;
        product.name = clean_name;
        product.repository_ids = clean_repository_ids;
        product.release_mode = clean_release_mode;
        product.updated_at = now;
    } else {
        state.products.push(ProductConfig {
            id: generated_config_id("product", &clean_name),
            name: clean_name,
            repository_ids: clean_repository_ids,
            release_mode: clean_release_mode,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    state
        .products
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn upsert_group_at(
    path: &Path,
    group_id: Option<&str>,
    name: &str,
    repository_ids: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    let clean_name = normalize_name(name, "Group")?;
    let mut state = load_store(path)?;
    let clean_repository_ids = normalize_repository_ids(&state, repository_ids)?;
    if state.groups.iter().any(|group| {
        group.name.eq_ignore_ascii_case(&clean_name) && group_id != Some(group.id.as_str())
    }) {
        return Err(format!("A group named '{clean_name}' already exists"));
    }
    let now = iso_now();
    if let Some(group_id) = group_id.filter(|value| !value.trim().is_empty()) {
        let group = state
            .groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| "Group is not registered".to_string())?;
        group.name = clean_name;
        group.repository_ids = clean_repository_ids;
        group.updated_at = now;
    } else {
        state.groups.push(GroupConfig {
            id: generated_config_id("group", &clean_name),
            name: clean_name,
            repository_ids: clean_repository_ids,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    state
        .groups
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn delete_product_at(path: &Path, product_id: &str) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let original_len = state.products.len();
    state.products.retain(|product| product.id != product_id);
    if state.products.len() == original_len {
        return Err("Product is not registered".to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn delete_group_at(path: &Path, group_id: &str) -> Result<PortfolioSnapshot, String> {
    let mut state = load_store(path)?;
    let original_len = state.groups.len();
    state.groups.retain(|group| group.id != group_id);
    if state.groups.len() == original_len {
        return Err("Group is not registered".to_string());
    }
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn register_root_and_scan(path: &Path, root_path: &str) -> Result<PortfolioSnapshot, String> {
    let root = canonical_path(Path::new(root_path))
        .ok_or_else(|| "Choose an accessible folder for repository discovery".to_string())?;
    if !root.is_dir() {
        return Err("The selected repository root is not a folder".to_string());
    }
    let mut state = load_store(path)?;
    let root_string = root.to_string_lossy().to_string();
    if !state.roots.iter().any(|item| item.path == root_string) {
        state.roots.push(RootConfig {
            id: path_id("root", &root),
            path: root_string,
            label: root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Repository root")
                .to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        });
    }
    audited_scan_and_persist(path, &mut state)
}

fn set_maturity_audit_root_at(
    path: &Path,
    audit_root: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    let canonical_feed = quality::canonical_maturity_feed_path()
        .ok_or_else(|| "Pronto could not resolve the user's home directory".to_string())?;
    let requested = audit_root
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "Pronto no longer accepts clearing or choosing an audit root; Quality Runner owns {}",
                canonical_feed.display()
            )
        })?;
    let requested_path = PathBuf::from(requested);
    let matches_canonical = requested_path == canonical_feed
        || fs::canonicalize(&requested_path)
            .ok()
            .is_some_and(|path| path == canonical_feed);
    if !matches_canonical {
        return Err(format!(
            "Pronto no longer accepts arbitrary audit roots; use Quality Runner's canonical feed at {}",
            canonical_feed.display()
        ));
    }
    let mut state = load_store_with_quality(path)?;
    apply_quality_evidence(&mut state);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn open_external_tool(path: &Path, tool: &str) -> Result<(), String> {
    let tool = tool.trim().to_ascii_lowercase();
    let arguments = match tool.as_str() {
        "file_browser" => vec![path.to_string_lossy().to_string()],
        "terminal" => vec![
            "-a".to_string(),
            "Terminal".to_string(),
            path.to_string_lossy().to_string(),
        ],
        "editor" => vec![
            "-a".to_string(),
            "Visual Studio Code".to_string(),
            path.to_string_lossy().to_string(),
        ],
        "git_client" => vec![
            "-a".to_string(),
            "GitHub Desktop".to_string(),
            path.to_string_lossy().to_string(),
        ],
        _ => return Err("Choose a supported external handoff tool".to_string()),
    };

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/open")
            .args(arguments)
            .output()
            .map_err(|_| "Could not start the external handoff tool".to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "The external handoff tool could not open {}",
                path.display()
            ))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, arguments);
        Err("External handoff is currently implemented for macOS only".to_string())
    }
}

fn open_quality_report_at(path: &Path, report_path: &str) -> Result<PortfolioSnapshot, String> {
    let state = load_store_with_quality(path)?;
    let mut allowed_roots = Vec::new();
    if let Some(audit_root) = state.quality.audit_root.as_deref() {
        allowed_roots.push(PathBuf::from(audit_root));
    }
    if let Some(feed_path) = quality::canonical_maturity_feed_path() {
        allowed_roots.push(feed_path);
    }
    allowed_roots.extend(
        state
            .repositories
            .iter()
            .map(|repository| Path::new(&repository.path).join(".quality-runner")),
    );
    let report = quality::safe_report_path(Path::new(report_path), &allowed_roots)?;
    open_external_tool(&report, "file_browser")?;
    Ok(snapshot_from_store(path, &state))
}

fn open_workspace_at(
    path: &Path,
    repository_id: &str,
    workspace_id: &str,
    tool: &str,
) -> Result<PortfolioSnapshot, String> {
    let state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = repository
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| "Workspace is not registered for this repository".to_string())?;
    let workspace_path = canonical_path(Path::new(&workspace.path))
        .ok_or_else(|| "The workspace path is unavailable".to_string())?;
    if !workspace_path.is_dir() {
        return Err("The workspace path is not an accessible folder".to_string());
    }
    open_external_tool(&workspace_path, tool)?;
    Ok(snapshot_from_store(path, &state))
}

#[tauri::command]
pub fn get_snapshot() -> Result<PortfolioSnapshot, String> {
    let path = store_path();
    let state = load_store(&path)?;
    Ok(snapshot_from_store(&path, &state))
}

#[tauri::command]
pub fn get_analytics() -> Result<AnalyticsSnapshot, String> {
    load_analytics_at(&store_path())
}

#[tauri::command]
pub fn get_skills() -> Result<SkillsSnapshot, String> {
    skills::load(&store_path())
}

#[tauri::command]
pub async fn refresh_skills() -> Result<SkillsSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| skills::refresh(&store_path()))
        .await
        .map_err(|error| format!("Skills refresh task failed: {error}"))?
}

#[tauri::command]
pub fn open_skill_source(path: String) -> Result<(), String> {
    skills::open_source(&path)
}

#[tauri::command]
pub fn register_root(path: String) -> Result<PortfolioSnapshot, String> {
    register_root_and_scan(&store_path(), &path)
}

#[tauri::command]
pub async fn refresh() -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let path = store_path();
        let mut state = load_store(&path)?;
        audited_scan_and_persist(&path, &mut state)
    })
    .await
    .map_err(|error| format!("Local refresh task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_quality() -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| refresh_quality_at(&store_path()))
        .await
        .map_err(|error| format!("Quality refresh task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_repository_target_evidence(
    repository_id: String,
    target_branch: String,
) -> Result<PortfolioSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        refresh_repository_target_evidence_at(&store_path(), &repository_id, &target_branch)
    })
    .await
    .map_err(|error| format!("Target evidence refresh task failed: {error}"))?
}

#[tauri::command]
pub fn refresh_github() -> Result<PortfolioSnapshot, String> {
    refresh_github_at(&store_path())
}

#[tauri::command]
pub fn refresh_remediation() -> Result<PortfolioSnapshot, String> {
    refresh_remediation_at(
        &store_path(),
        None,
        false,
        true,
        false,
        DEFAULT_QR_AUDIT_TIMEOUT_SECONDS,
    )
}

fn remediation_dependencies_are_terminal<'a>(mut statuses: impl Iterator<Item = &'a str>) -> bool {
    statuses.all(|status| matches!(status, "verified" | "deferred"))
}

fn remediation_action_workspace_id(action: &remediation::RemediationAction) -> Option<&str> {
    [
        "branch_hygiene:activity:",
        "branch_hygiene:operation:",
        "branch_hygiene:dirty:",
        "branch_hygiene:sync:",
        "branch_hygiene:remote-freshness:",
    ]
    .iter()
    .find_map(|prefix| action.stable_key.strip_prefix(prefix))
}

#[tauri::command]
pub fn set_remediation_action_status(
    action_id: String,
    status: String,
    notes: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    let normalized_status = status.trim().to_ascii_lowercase();
    if !matches!(
        normalized_status.as_str(),
        "open" | "in_progress" | "blocked" | "deferred" | "verified"
    ) {
        return Err(
            "Remediation status must be open, in_progress, blocked, deferred, or verified."
                .to_string(),
        );
    }
    let path = store_path();
    let mut state = load_store_with_quality(&path)?;
    let mut found = false;
    for plan in &mut state.remediation.plans {
        let Some(action_index) = plan
            .actions
            .iter()
            .position(|action| action.id == action_id)
        else {
            continue;
        };
        if normalized_status == "verified" {
            let action = &plan.actions[action_index];
            if action.stable_key != remediation::GITHUB_ONLY_VERIFICATION_ACTION_KEY {
                let repository = state
                    .repositories
                    .iter()
                    .find(|repository| repository.id == plan.repository_id)
                    .ok_or_else(|| {
                        format!(
                            "Repository {} is no longer registered; remediation advancement is blocked.",
                            plan.repository_id
                        )
                    })?;
                let handoff = remediation_handoff_check_for_repository(
                    repository,
                    remediation_action_workspace_id(action),
                )?;
                if !handoff.ready {
                    return Err(format!(
                        "Remediation advancement is blocked by the handoff checkpoint ({}). {}",
                        handoff.status,
                        handoff.reasons.join(" ")
                    ));
                }
            }
            let verification_is_ready = if action.domain == "verification" {
                remediation_dependencies_are_terminal(
                    plan.actions
                        .iter()
                        .filter(|candidate| candidate.id != action.id)
                        .map(|candidate| candidate.status.as_str()),
                ) && plan
                    .actions
                    .iter()
                    .flat_map(|candidate| candidate.evidence.iter())
                    .any(|item| item.freshness.eq_ignore_ascii_case("fresh"))
            } else {
                remediation::action_has_fresh_evidence(action)
            };
            if !verification_is_ready {
                return Err(
                    "An action cannot be verified until its evidence is fresh. Refresh the source and recheck the plan first."
                        .to_string(),
                );
            }
        }
        let action = &mut plan.actions[action_index];
        action.status = normalized_status.clone();
        action.notes = notes.clone();
        action.updated_at = iso_now();
        action.completed_at = (normalized_status == "verified").then(iso_now);
        remediation::recompute_plan_derived(plan);
        found = true;
        break;
    }
    if !found {
        return Err(format!("Remediation action {action_id} was not found."));
    }
    let updated_at = iso_now();
    remediation::normalize_queue(&mut state.remediation, &updated_at);
    state.remediation.generated_at = updated_at;
    save_store(&path, &state)?;
    Ok(snapshot_from_store(&path, &state))
}

#[tauri::command]
pub fn check_remediation_handoff(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<RemediationHandoffCheck, String> {
    let path = store_path();
    let state = load_store_read_only(&path)?;
    let snapshot = snapshot_from_store(&path, &state);
    let repository = snapshot
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    remediation_handoff_check_for_repository(repository, workspace_id.as_deref())
}

#[tauri::command]
pub fn export_remediation(
    output_dir: Option<String>,
) -> Result<remediation::RemediationExport, String> {
    let path = store_path();
    let state = load_store_with_quality(&path)?;
    let mut remediation_run = state.remediation.clone();
    remediation::sync_github_only_candidates(&mut remediation_run, &state.remote_repositories);
    let root = output_dir
        .map(PathBuf::from)
        .or_else(|| {
            path.parent()
                .map(|parent| parent.join("remediation").join(&state.remediation.id))
        })
        .ok_or_else(|| "Pronto storage path has no export directory".to_string())?;
    remediation::export_run(&remediation_run, &root)
}

#[tauri::command]
pub fn set_maturity_audit_root(audit_root: Option<String>) -> Result<PortfolioSnapshot, String> {
    set_maturity_audit_root_at(&store_path(), audit_root.as_deref())
}

#[tauri::command]
pub fn open_quality_report(report_path: String) -> Result<PortfolioSnapshot, String> {
    open_quality_report_at(&store_path(), &report_path)
}

#[tauri::command]
pub fn open_workspace(
    repository_id: String,
    workspace_id: String,
    tool: String,
) -> Result<PortfolioSnapshot, String> {
    open_workspace_at(&store_path(), &repository_id, &workspace_id, &tool)
}

#[tauri::command]
pub fn prepare_repository(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<RepositoryPreparation, String> {
    prepare_repository_at(&store_path(), &repository_id, workspace_id.as_deref())
}

#[tauri::command]
pub fn preflight_action(
    action: String,
    repository_id: Option<String>,
) -> Result<ActionPreflight, String> {
    preflight_action_at(&store_path(), &action, repository_id.as_deref())
}

#[tauri::command]
pub fn mark_condition_expected(
    repository_id: String,
    condition_id: String,
) -> Result<PortfolioSnapshot, String> {
    mutate_expected(&store_path(), &repository_id, &condition_id, true)
}

#[tauri::command]
pub fn clear_condition_expected(
    repository_id: String,
    condition_id: String,
) -> Result<PortfolioSnapshot, String> {
    mutate_expected(&store_path(), &repository_id, &condition_id, false)
}

#[tauri::command]
pub fn update_root_settings(
    root_id: String,
    ignore_patterns: Vec<String>,
    refresh_policy: String,
    background_monitoring: bool,
) -> Result<PortfolioSnapshot, String> {
    update_root_settings_at(
        &store_path(),
        &root_id,
        ignore_patterns,
        &refresh_policy,
        background_monitoring,
    )
}

#[tauri::command]
pub fn set_repository_lifecycle(
    repository_id: String,
    lifecycle: String,
) -> Result<PortfolioSnapshot, String> {
    set_repository_lifecycle_at(&store_path(), &repository_id, &lifecycle)
}

#[tauri::command]
pub fn set_repository_target_branch(
    repository_id: String,
    target_branch: String,
) -> Result<PortfolioSnapshot, String> {
    set_repository_target_branch_at(&store_path(), &repository_id, &target_branch)
}

#[tauri::command]
pub fn set_release_rule(
    repository_id: String,
    release_rule: Option<ReleaseRuleConfig>,
) -> Result<PortfolioSnapshot, String> {
    set_release_rule_at(&store_path(), &repository_id, release_rule)
}

#[tauri::command]
pub fn set_release_recipe(
    repository_id: String,
    release_recipe: Option<ReleaseRecipeConfig>,
) -> Result<PortfolioSnapshot, String> {
    set_release_recipe_at(&store_path(), &repository_id, release_recipe)
}

#[tauri::command]
pub fn set_release_version(
    repository_id: String,
    release_version: Option<String>,
) -> Result<PortfolioSnapshot, String> {
    set_release_version_at(&store_path(), &repository_id, release_version)
}

#[tauri::command]
pub fn set_ai_permission(
    repository_id: String,
    permission: String,
) -> Result<PortfolioSnapshot, String> {
    set_ai_permission_at(&store_path(), &repository_id, &permission)
}

#[tauri::command]
pub fn preview_ai_summary(
    repository_id: String,
    workspace_id: Option<String>,
) -> Result<AiPayloadPreview, String> {
    preview_ai_summary_at(&store_path(), &repository_id, workspace_id.as_deref())
}

#[tauri::command]
pub fn set_retention_days(retention_days: i64) -> Result<PortfolioSnapshot, String> {
    set_retention_days_at(&store_path(), retention_days)
}

#[tauri::command]
pub fn upsert_product(
    product_id: Option<String>,
    name: String,
    repository_ids: Vec<String>,
    release_mode: String,
) -> Result<PortfolioSnapshot, String> {
    upsert_product_at(
        &store_path(),
        product_id.as_deref(),
        &name,
        repository_ids,
        &release_mode,
    )
}

#[tauri::command]
pub fn delete_product(product_id: String) -> Result<PortfolioSnapshot, String> {
    delete_product_at(&store_path(), &product_id)
}

#[tauri::command]
pub fn upsert_group(
    group_id: Option<String>,
    name: String,
    repository_ids: Vec<String>,
) -> Result<PortfolioSnapshot, String> {
    upsert_group_at(&store_path(), group_id.as_deref(), &name, repository_ids)
}

#[tauri::command]
pub fn delete_group(group_id: String) -> Result<PortfolioSnapshot, String> {
    delete_group_at(&store_path(), &group_id)
}

fn cli_option(arguments: &[String], option: &str) -> Result<Option<String>, String> {
    let mut value = None;
    let mut index = 1;
    while index < arguments.len() {
        if arguments[index] == option {
            let next = arguments
                .get(index + 1)
                .ok_or_else(|| format!("{option} requires a value"))?;
            if next.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            if value.replace(next.clone()).is_some() {
                return Err(format!("{option} may only be provided once"));
            }
            index += 1;
        }
        index += 1;
    }
    Ok(value)
}

fn cli_repeated_option(arguments: &[String], option: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut index = 1;
    while index < arguments.len() {
        if arguments[index] == option {
            let next = arguments
                .get(index + 1)
                .ok_or_else(|| format!("{option} requires a value"))?;
            if next.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            values.push(next.clone());
            index += 1;
        }
        index += 1;
    }
    Ok(values)
}

fn cli_json_option<T: DeserializeOwned>(
    arguments: &[String],
    option: &str,
) -> Result<Option<T>, String> {
    let Some(value) = cli_option(arguments, option)? else {
        return Ok(None);
    };
    let payload = if let Some(path) = value.strip_prefix('@') {
        fs::read_to_string(path)
            .map_err(|error| format!("Could not read {option} file: {error}"))?
    } else {
        value
    };
    serde_json::from_str(&payload)
        .map(Some)
        .map_err(|error| format!("{option} must contain valid JSON: {error}"))
}

fn cli_bool_option(arguments: &[String], option: &str) -> Result<Option<bool>, String> {
    cli_option(arguments, option)?
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            _ => Err(format!("{option} must be true or false")),
        })
        .transpose()
}

fn cli_positive_u64_option(arguments: &[String], option: &str) -> Result<Option<u64>, String> {
    cli_option(arguments, option)?
        .map(|value| {
            value
                .parse::<u64>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| format!("{option} must be a positive integer"))
        })
        .transpose()
}

fn append_qr_audit_runtime_arguments(
    arguments: &mut Vec<String>,
    dynamic: bool,
    changed_only: bool,
    timeout_seconds: u64,
) {
    if dynamic {
        arguments.push("--dynamic".to_string());
        if !changed_only {
            arguments.push("--no-changed-only".to_string());
        }
    }
    arguments.extend([
        "--timeout-seconds".to_string(),
        timeout_seconds.to_string(),
        "--json".to_string(),
    ]);
}

fn cli_positionals(arguments: &[String], value_options: &[&str]) -> Result<Vec<String>, String> {
    let mut positionals = Vec::new();
    let mut expecting_value = None;
    for argument in arguments.iter().skip(1) {
        if expecting_value.take().is_some() {
            if argument.starts_with("--") {
                return Err("An option value is missing".to_string());
            }
            continue;
        }
        if argument == "--json" {
            continue;
        }
        if value_options.iter().any(|option| option == argument) {
            expecting_value = Some(argument.as_str());
        } else if argument.starts_with("--") {
            return Err(format!("Unknown option {argument}"));
        } else {
            positionals.push(argument.clone());
        }
    }
    if expecting_value.is_some() {
        return Err("An option value is missing".to_string());
    }
    Ok(positionals)
}

fn cli_positionals_with_flags(
    arguments: &[String],
    value_options: &[&str],
    flags: &[&str],
) -> Result<Vec<String>, String> {
    let mut positionals = Vec::new();
    let mut expecting_value = None;
    for argument in arguments.iter().skip(1) {
        if expecting_value.take().is_some() {
            if argument.starts_with("--") {
                return Err("An option value is missing".to_string());
            }
            continue;
        }
        if argument == "--json" || flags.iter().any(|flag| flag == argument) {
            continue;
        }
        if value_options.iter().any(|option| option == argument) {
            expecting_value = Some(argument.as_str());
        } else if argument.starts_with("--") {
            return Err(format!("Unknown option {argument}"));
        } else {
            positionals.push(argument.clone());
        }
    }
    if expecting_value.is_some() {
        return Err("An option value is missing".to_string());
    }
    Ok(positionals)
}

fn repository_matches_query(repository: &RepositorySnapshot, query: &str) -> bool {
    repository.id == query
        || repository.path == query
        || repository.name.eq_ignore_ascii_case(query)
        || repository.workspaces.iter().any(|workspace| {
            workspace.path == query || workspace.id == query || workspace.path.ends_with(query)
        })
}

fn find_cli_repository<'a>(
    snapshot: &'a PortfolioSnapshot,
    query: &str,
) -> Result<&'a RepositorySnapshot, String> {
    let matches = snapshot
        .repositories
        .iter()
        .filter(|repository| repository_matches_query(repository, query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [repository] => Ok(repository),
        [] => Err(format!("Repository '{query}' is not registered")),
        _ => Err(format!("Repository query '{query}' is ambiguous")),
    }
}

fn find_cli_group<'a>(state: &'a StoreState, query: &str) -> Result<&'a GroupConfig, String> {
    let matches = state
        .groups
        .iter()
        .filter(|group| group.id == query || group.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [group] => Ok(group),
        [] => Err(format!("Group '{query}' is not registered")),
        _ => Err(format!("Group query '{query}' is ambiguous")),
    }
}

fn find_cli_product<'a>(state: &'a StoreState, query: &str) -> Result<&'a ProductConfig, String> {
    let matches = state
        .products
        .iter()
        .filter(|product| product.id == query || product.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [product] => Ok(product),
        [] => Err(format!("Product '{query}' is not registered")),
        _ => Err(format!("Product query '{query}' is ambiguous")),
    }
}

fn merge_repository_ids(existing: &[String], additions: Vec<String>) -> Vec<String> {
    existing
        .iter()
        .cloned()
        .chain(additions)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn find_repository_for_directory<'a>(
    snapshot: &'a PortfolioSnapshot,
    directory: &Path,
) -> Option<&'a RepositorySnapshot> {
    let canonical_directory = canonical_path(directory).unwrap_or_else(|| directory.to_path_buf());
    snapshot.repositories.iter().find(|repository| {
        repository
            .workspaces
            .iter()
            .any(|workspace| canonical_directory.starts_with(&workspace.path))
            || canonical_directory.starts_with(&repository.path)
    })
}

fn filter_snapshot_to_repository_ids(
    mut snapshot: PortfolioSnapshot,
    repository_ids: &HashSet<String>,
) -> PortfolioSnapshot {
    snapshot
        .repositories
        .retain(|repository| repository_ids.contains(&repository.id));
    snapshot
}

fn filter_snapshot_by_collection(
    mut snapshot: PortfolioSnapshot,
    product_name: Option<&str>,
    group_name: Option<&str>,
) -> Result<PortfolioSnapshot, String> {
    if product_name.is_some() && group_name.is_some() {
        return Err("Choose either --product or --group, not both".to_string());
    }
    if let Some(name) = product_name {
        let product = snapshot
            .products
            .iter()
            .find(|product| product.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Product '{name}' is not configured"))?;
        let repository_ids = product
            .repository_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        snapshot
            .repositories
            .retain(|repository| repository_ids.contains(&repository.id));
        snapshot.products = vec![product];
        snapshot.groups.clear();
    } else if let Some(name) = group_name {
        let group = snapshot
            .groups
            .iter()
            .find(|group| group.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Group '{name}' is not configured"))?;
        let repository_ids = group.repository_ids.iter().cloned().collect::<HashSet<_>>();
        snapshot
            .repositories
            .retain(|repository| repository_ids.contains(&repository.id));
        snapshot.groups = vec![group];
        snapshot.products.clear();
    }
    Ok(snapshot)
}

fn resolve_refresh_target(
    snapshot: &PortfolioSnapshot,
    query: &str,
) -> Result<(HashSet<String>, String), String> {
    let repository_matches = snapshot
        .repositories
        .iter()
        .filter(|repository| repository_matches_query(repository, query))
        .collect::<Vec<_>>();
    let product_matches = snapshot
        .products
        .iter()
        .filter(|product| product.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    let group_matches = snapshot
        .groups
        .iter()
        .filter(|group| group.name.eq_ignore_ascii_case(query))
        .collect::<Vec<_>>();
    let match_count = repository_matches.len() + product_matches.len() + group_matches.len();
    if match_count == 0 {
        return Err(format!(
            "Refresh target '{query}' is not a repository, product, or group"
        ));
    }
    if match_count > 1 {
        return Err(format!("Refresh target '{query}' is ambiguous"));
    }
    if let Some(repository) = repository_matches.first() {
        return Ok((
            [repository.id.clone()].into_iter().collect(),
            format!("Repository {}", repository.name),
        ));
    }
    if let Some(product) = product_matches.first() {
        return Ok((
            product.repository_ids.iter().cloned().collect(),
            format!("Product {}", product.name),
        ));
    }
    let group = group_matches
        .first()
        .expect("target count guarantees a group");
    Ok((
        group.repository_ids.iter().cloned().collect(),
        format!("Group {}", group.name),
    ))
}

#[derive(Debug)]
enum LocalRefreshTarget {
    Registered {
        repository_ids: HashSet<String>,
        label: String,
    },
    RepositoryPath(PathBuf),
}

fn resolve_local_refresh_target(
    snapshot: &PortfolioSnapshot,
    state: &StoreState,
    query: &str,
) -> Result<LocalRefreshTarget, String> {
    match resolve_refresh_target(snapshot, query) {
        Ok((repository_ids, label)) => {
            return Ok(LocalRefreshTarget::Registered {
                repository_ids,
                label,
            });
        }
        Err(error) if error.contains("ambiguous") => return Err(error),
        Err(_) => {}
    }

    let repository_path = canonical_repository_path(Path::new(query)).ok_or_else(|| {
        format!(
            "Refresh target '{query}' is not a registered repository, group, product, or repository path"
        )
    })?;
    let covered_by_root = state
        .roots
        .iter()
        .any(|root| path_is_within(Path::new(&root.path), &repository_path));
    if !covered_by_root {
        return Err(format!(
            "Repository path '{}' is not covered by a registered discovery root; register its parent root before refreshing it",
            repository_path.display()
        ));
    }
    Ok(LocalRefreshTarget::RepositoryPath(repository_path))
}

fn agent_condition_summary(condition: &Condition) -> AgentConditionSummary {
    AgentConditionSummary {
        id: condition.id.clone(),
        kind: condition.kind.clone(),
        title: condition.title.clone(),
        summary: condition.summary.clone(),
        priority: condition.priority,
        status: condition.status.clone(),
        missing: condition.missing.clone(),
        confidence: condition.confidence.clone(),
        freshness: condition.freshness.clone(),
    }
}

fn agent_workspace_summary(workspace: &WorkspaceSummary) -> AgentWorkspaceSummary {
    AgentWorkspaceSummary {
        id: workspace.id.clone(),
        path: workspace.path.clone(),
        is_primary: workspace.is_primary,
        branch: workspace.branch.clone(),
        status_available: workspace.status_available,
        status_error: workspace.status_error.clone(),
        dirty: workspace.dirty,
        sync_state: workspace.sync_state.clone(),
        ahead: workspace.ahead,
        behind: workspace.behind,
        upstream: workspace.upstream.clone(),
        operation: workspace.operation.clone(),
        integration_state: workspace.integration_state.clone(),
        target_branch: workspace.target_branch.clone(),
        target_confidence: workspace.target_confidence.clone(),
        activity_state: workspace.activity.state.clone(),
        activity_confidence: workspace.activity.confidence.clone(),
        last_commit: workspace.last_commit.clone(),
        last_commit_at: workspace.last_commit_at.clone(),
        last_activity_at: workspace.last_activity_at.clone(),
        sync_detail: workspace.sync_detail.clone(),
    }
}

fn workspace_requires_sync_attention(workspace: &WorkspaceSummary) -> bool {
    !workspace.status_available || workspace_is_unsynced(workspace)
}

fn workspace_is_unsynced(workspace: &WorkspaceSummary) -> bool {
    workspace.status_available && workspace.sync_state != "Synced"
}

fn agent_repository_summary(repository: &RepositorySnapshot) -> AgentRepositorySummary {
    let active_conditions = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .map(agent_condition_summary)
        .collect::<Vec<_>>();
    AgentRepositorySummary {
        id: repository.id.clone(),
        name: repository.name.clone(),
        path: repository.path.clone(),
        locality: repository.locality.clone(),
        lifecycle: repository.lifecycle.clone(),
        branch: repository.branch.clone(),
        default_branch: repository.default_branch.clone(),
        target_branch: repository
            .target_branch
            .clone()
            .or_else(|| repository.default_branch.clone()),
        target_branch_configured: repository.target_branch_configured,
        workspaces: repository
            .workspaces
            .iter()
            .map(agent_workspace_summary)
            .collect(),
        active_conditions,
        quality_status: repository.quality.ingestion_status.clone(),
        maturity_score: repository.quality.maturity.score,
        maturity_score_display: repository.quality.maturity.score_display.clone(),
        maturity_freshness: repository.quality.maturity.freshness.as_str().to_string(),
        ci_readiness_score: repository.quality.ci_readiness.score,
        ci_readiness_score_display: repository.quality.ci_readiness.score_display.clone(),
        ci_readiness_fresh_passing_gate_count: repository
            .quality
            .ci_readiness
            .fresh_passing_gate_ids
            .len(),
        ci_readiness_ideal_gate_count: repository.quality.ci_readiness.applicable_gate_ids.len(),
        ci_configuration_configured_gate_count: repository
            .quality
            .ci_readiness
            .configured_gate_ids
            .len(),
        ci_configuration_ideal_gate_count: repository
            .quality
            .ci_readiness
            .applicable_gate_ids
            .len(),
        findings_total: repository.quality.findings.total,
        high_severity_findings: repository.quality.findings.high_severity_total,
        project_compass: repository.project_compass.clone(),
        last_scan_at: repository.last_scan_at.clone(),
        last_activity_at: repository.last_activity_at.clone(),
    }
}

fn agent_condition_evidence(condition: &Condition) -> Vec<AgentEvidenceReference> {
    condition
        .evidence
        .iter()
        .map(|evidence| AgentEvidenceReference {
            source: evidence.source.clone(),
            label: evidence.label.clone(),
            status: None,
            freshness: condition.freshness.clone(),
            observed_at: Some(evidence.observed_at.clone()),
            value: Some(evidence.value.clone()),
            report_path: None,
        })
        .collect()
}

fn agent_gate_evidence(gate: &quality::QualityGate) -> Vec<AgentEvidenceReference> {
    gate.evidence
        .iter()
        .map(|evidence| AgentEvidenceReference {
            source: evidence.source.as_str().to_string(),
            label: evidence.source_label.clone(),
            status: Some(evidence.status.as_str().to_string()),
            freshness: Some(evidence.freshness.as_str().to_string()),
            observed_at: evidence.observed_at.clone(),
            value: Some(evidence.detail.clone()),
            report_path: evidence.report_path.clone(),
        })
        .collect()
}

fn agent_workspace_sync_evidence(workspace: &WorkspaceSummary) -> Vec<AgentEvidenceReference> {
    let Some(detail) = workspace.sync_detail.as_ref() else {
        return Vec::new();
    };
    vec![
        AgentEvidenceReference {
            source: "Local workspace scan".to_string(),
            label: if workspace.status_available {
                "Why unsynced".to_string()
            } else {
                "Why Git status unavailable".to_string()
            },
            status: Some(workspace.sync_state.clone()),
            freshness: None,
            observed_at: detail.evidence_observed_at.clone(),
            value: Some(detail.reason.clone()),
            report_path: None,
        },
        AgentEvidenceReference {
            source: "Local workspace scan".to_string(),
            label: "Evidence expires".to_string(),
            status: Some("Expiry timestamp".to_string()),
            freshness: None,
            observed_at: detail.evidence_observed_at.clone(),
            value: detail.evidence_expires_at.clone(),
            report_path: None,
        },
        AgentEvidenceReference {
            source: "Pronto scoped refresh contract".to_string(),
            label: "Next safe scoped refresh".to_string(),
            status: Some("Read-only local scan".to_string()),
            freshness: None,
            observed_at: None,
            value: Some(detail.scoped_refresh_command.clone()),
            report_path: None,
        },
    ]
}

fn agent_attention_report(snapshot: &PortfolioSnapshot) -> AgentAttentionReport {
    let mut items = Vec::new();
    for repository in &snapshot.repositories {
        for condition in repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
        {
            items.push(AgentAttentionItem {
                id: format!("{}:condition:{}", repository.id, condition.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "condition".to_string(),
                severity: format!("P{}", condition.priority),
                status: condition.status.clone(),
                freshness: condition.freshness.clone(),
                summary: condition.summary.clone(),
                evidence: agent_condition_evidence(condition),
            });
        }

        for workspace in repository
            .workspaces
            .iter()
            .filter(|workspace| workspace.dirty)
        {
            items.push(AgentAttentionItem {
                id: format!("{}:workspace-dirty:{}", repository.id, workspace.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: Some(workspace.id.clone()),
                workspace_path: Some(workspace.path.clone()),
                category: "workspace".to_string(),
                severity: "warning".to_string(),
                status: "Dirty".to_string(),
                freshness: None,
                summary: format!("Workspace {} has uncommitted changes", workspace.branch),
                evidence: Vec::new(),
            });
        }

        for workspace in repository
            .workspaces
            .iter()
            .filter(|workspace| workspace_requires_sync_attention(workspace))
        {
            let summary = if workspace.status_available {
                format!(
                    "Workspace {} is {} (ahead {}, behind {})",
                    workspace.branch, workspace.sync_state, workspace.ahead, workspace.behind
                )
            } else {
                format!(
                    "Workspace {} Git status unavailable: {}",
                    workspace.branch,
                    workspace_status_unavailable_reason(workspace)
                )
            };
            items.push(AgentAttentionItem {
                id: format!("{}:workspace-sync:{}", repository.id, workspace.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: Some(workspace.id.clone()),
                workspace_path: Some(workspace.path.clone()),
                category: "synchronization".to_string(),
                severity: "warning".to_string(),
                status: workspace.sync_state.clone(),
                freshness: workspace
                    .status_available
                    .then_some(workspace.remote_freshness.clone()),
                summary,
                evidence: agent_workspace_sync_evidence(workspace),
            });
        }

        for gate in &repository.quality.gates {
            let missing = gate.status == QualityGateStatus::NotConfigured
                && repository
                    .quality
                    .ci_readiness
                    .applicable_gate_ids
                    .iter()
                    .any(|gate_id| gate_id == &gate.id);
            let stale = gate.freshness != QualityFreshness::Fresh;
            let failed_or_blocked = matches!(
                gate.status,
                QualityGateStatus::Failed | QualityGateStatus::Blocked
            );
            if missing || stale || failed_or_blocked {
                let status = if missing {
                    "Missing".to_string()
                } else {
                    gate.status.as_str().to_string()
                };
                let severity = if failed_or_blocked {
                    "error"
                } else {
                    "warning"
                };
                items.push(AgentAttentionItem {
                    id: format!("{}:quality-gate:{}", repository.id, gate.id),
                    repository_id: repository.id.clone(),
                    repository_name: repository.name.clone(),
                    repository_path: repository.path.clone(),
                    workspace_id: None,
                    workspace_path: None,
                    category: "quality_gate".to_string(),
                    severity: severity.to_string(),
                    status,
                    freshness: Some(gate.freshness.as_str().to_string()),
                    summary: format!("{} gate requires attention", gate.label),
                    evidence: agent_gate_evidence(gate),
                });
            }
        }

        if repository.quality.findings.high_severity_total > 0 {
            items.push(AgentAttentionItem {
                id: format!("{}:quality-findings", repository.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "quality_findings".to_string(),
                severity: "error".to_string(),
                status: "Open".to_string(),
                freshness: Some(repository.quality.findings.freshness.as_str().to_string()),
                summary: format!(
                    "{} high-severity quality findings remain open",
                    repository.quality.findings.high_severity_total
                ),
                evidence: Vec::new(),
            });
        }

        if repository.quality.maturity.score.is_none()
            || repository.quality.maturity.freshness != QualityFreshness::Fresh
        {
            items.push(AgentAttentionItem {
                id: format!("{}:quality-maturity", repository.id),
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                workspace_id: None,
                workspace_path: None,
                category: "quality_maturity".to_string(),
                severity: "warning".to_string(),
                status: if repository.quality.maturity.score.is_some() {
                    "Stale".to_string()
                } else {
                    "Unknown".to_string()
                },
                freshness: Some(repository.quality.maturity.freshness.as_str().to_string()),
                summary: "Repository maturity evidence is missing or not fresh".to_string(),
                evidence: Vec::new(),
            });
        }
    }
    AgentAttentionReport {
        schema_version: AGENT_ATTENTION_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        items,
    }
}

fn agent_attention_priority(item: &AgentAttentionItem) -> u16 {
    if item.severity == "error" {
        return 0;
    }
    if let Some(priority) = item
        .severity
        .strip_prefix('P')
        .and_then(|value| value.parse::<u16>().ok())
    {
        return priority;
    }
    if item.severity == "warning" {
        return 100;
    }
    200
}

fn agent_next_action(item: &AgentAttentionItem) -> AgentNextAction {
    let (recommended_projection, next_safe_step) = match item.category.as_str() {
        "quality_gate" => (
            "quality",
            "Read the repository quality projection before changing code or recording remediation status.".to_string(),
        ),
        "quality_findings" => (
            "quality",
            "Read the repository quality projection and source report before planning remediation.".to_string(),
        ),
        "quality_maturity" => (
            "quality",
            "Inspect quality freshness and source provenance; stale or missing maturity evidence is not a pass.".to_string(),
        ),
        "synchronization" => {
            let command = item
                .evidence
                .iter()
                .find(|evidence| evidence.label == "Next safe scoped refresh")
                .and_then(|evidence| evidence.value.clone())
                .unwrap_or_else(|| {
                    format!(
                        "pronto refresh {} --json",
                        shell_quote_for_display(&item.repository_path)
                    )
                });
            (
                "repo",
                format!(
                    "Inspect this workspace's sync detail, then run `{command}` only when a fresh local comparison is needed; reopen the repository projection afterward."
                ),
            )
        }
        "workspace" => (
            "repo",
            "Inspect the repository projection; preserve dirty workspace contents and active work.".to_string(),
        ),
        "condition" => (
            "repo",
            "Inspect the repository projection and condition evidence before choosing a workflow.".to_string(),
        ),
        _ => (
            "attention",
            "Inspect the linked evidence before taking any repository, provider, remediation, or release action.".to_string(),
        ),
    };
    AgentNextAction {
        attention_id: item.id.clone(),
        repository_id: item.repository_id.clone(),
        repository_name: item.repository_name.clone(),
        workspace_id: item.workspace_id.clone(),
        category: item.category.clone(),
        severity: item.severity.clone(),
        status: item.status.clone(),
        summary: item.summary.clone(),
        recommended_projection: recommended_projection.to_string(),
        next_safe_step,
        authorization: if item.category == "synchronization" {
            "Scoped refresh is a read-only local Git scan; it persists Pronto evidence but does not pull, push, merge, rebase, or edit repository files.".to_string()
        } else {
            "Inspection only; Git, provider, remediation, and release mutations require explicit authorization.".to_string()
        },
    }
}

fn agent_next_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    scope: &str,
    limit: usize,
) -> Result<AgentNextReport, String> {
    let current_repository = query
        .map(|value| find_cli_repository(snapshot, value).map(agent_repository_summary))
        .transpose()?;
    let summary = agent_summary(snapshot, scope);
    let mut attention = agent_attention_report(snapshot).items;
    attention.sort_by(|left, right| {
        agent_attention_priority(left)
            .cmp(&agent_attention_priority(right))
            .then_with(|| left.id.cmp(&right.id))
    });
    let attention_total = attention.len();
    attention.truncate(limit);
    let actions = attention.iter().take(3).map(agent_next_action).collect();
    Ok(AgentNextReport {
        schema_version: AGENT_NEXT_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        summary,
        current_repository,
        attention_total,
        attention,
        actions,
    })
}

fn agent_fold_target(
    repository: &RepositorySnapshot,
    requested_target: Option<&str>,
) -> (Option<String>, String, String) {
    if let Some(target) = requested_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return (
            Some(target.to_string()),
            "Explicit command target".to_string(),
            "Explicit".to_string(),
        );
    }
    match repository
        .target_branch
        .clone()
        .or_else(|| repository.default_branch.clone())
    {
        Some(target) => (
            Some(target),
            if repository.target_branch_configured {
                "Pronto configured repository target".to_string()
            } else {
                "Pronto observed default branch".to_string()
            },
            "High".to_string(),
        ),
        None => (
            None,
            "No observed default branch".to_string(),
            "Unknown".to_string(),
        ),
    }
}

fn agent_fold_candidate_decision(
    branch: &BranchSummary,
    target: Option<&str>,
    workspace: Option<&WorkspaceSummary>,
) -> (String, String, String) {
    if let Some(workspace) = workspace {
        if !workspace.status_available {
            return (
                "status_unavailable".to_string(),
                workspace_status_unavailable_reason(workspace),
                "Restore live Git status access before evaluating this branch for integration or pruning."
                    .to_string(),
            );
        }
        if let Some(operation) = workspace.operation.as_deref() {
            return (
                "blocked_operation".to_string(),
                format!("Git operation in progress: {operation}"),
                "Resolve the in-progress Git operation before evaluating this branch.".to_string(),
            );
        }
        if workspace.activity.state == "Active" {
            return (
                "preserve_active".to_string(),
                "Agent activity is active for this workspace.".to_string(),
                "Wait for or hand off the active agent; do not integrate or prune this branch."
                    .to_string(),
            );
        }
        if workspace.dirty {
            return (
                "preserve_dirty".to_string(),
                "The linked workspace has uncommitted changes.".to_string(),
                "Preserve the workspace and inspect its complete diff before any fold decision."
                    .to_string(),
            );
        }
    }
    let Some(target) = target else {
        return (
            "target_unknown".to_string(),
            "No target branch was observed for this repository.".to_string(),
            "Identify the repository's canonical integration target before considering a fold."
                .to_string(),
        );
    };
    if branch.target_branch.as_deref() != Some(target) {
        return (
            "target_mismatch".to_string(),
            format!(
                "Pronto observed {} as this branch's target, not {target}.",
                branch.target_branch.as_deref().unwrap_or("an unknown branch")
            ),
            "Confirm the canonical integration target with the fold workflow before classifying this branch.".to_string(),
        );
    }
    let Some(workspace) = workspace else {
        return (
            "live_check_required".to_string(),
            "No registered workspace provides cleanliness or activity evidence for this branch."
                .to_string(),
            "Run live ref and worktree classification; do not treat this snapshot as fold authorization.".to_string(),
        );
    };
    if workspace.activity.confidence == "Low" {
        return (
            "activity_uncertain".to_string(),
            "Workspace activity evidence is explicitly uncertain.".to_string(),
            "Recheck live workspace ownership and activity before integration or pruning."
                .to_string(),
        );
    }
    match branch.integration_state.as_str() {
        "Already integrated" => (
            "prune_review".to_string(),
            "Pronto observed no unique commits relative to the observed target.".to_string(),
            "Verify remote ancestry or patch equivalence, PR/protection state, and worktree cleanliness before authorizing pruning.".to_string(),
        ),
        "Integration eligible" if workspace.upstream.is_none() => (
            "preserve_unpublished".to_string(),
            "The branch has unique commits but no tracked upstream is recorded.".to_string(),
            "Preserve the unpublished branch; stabilize and push it only with explicit task scope before integration.".to_string(),
        ),
        "Integration eligible" if workspace.sync_state != "Synced" => (
            "refresh_before_integration".to_string(),
            format!(
                "The branch has unique commits but its workspace is {}.",
                workspace.sync_state
            ),
            "Refresh scoped evidence and verify the live remote head before integration.".to_string(),
        ),
        "Integration eligible" => (
            "review_for_integration".to_string(),
            "Pronto observed a clean branch with unique commits relative to the observed target.".to_string(),
            "Run fold-feature-branches live classification and review the complete source diff before integration.".to_string(),
        ),
        "Blocked" => (
            "blocked".to_string(),
            "Pronto observed the branch as blocked for integration.".to_string(),
            "Inspect the linked workspace and condition evidence; preserve the branch until the blocker is resolved.".to_string(),
        ),
        "No unique commits" => (
            "no_unique_commits".to_string(),
            "Pronto observed no unique commits for this branch against its recorded target.".to_string(),
            "Verify live remote and worktree state; do not prune unless supersession is proven.".to_string(),
        ),
        _ => (
            "inspect".to_string(),
            format!(
                "Pronto recorded integration state: {}.",
                branch.integration_state
            ),
            "Inspect the live branch, worktree, ancestry, and remote evidence before choosing a workflow.".to_string(),
        ),
    }
}

fn agent_fold_decision_priority(decision: &str) -> u8 {
    match decision {
        "status_unavailable" | "preserve_dirty" | "preserve_active" | "blocked_operation"
        | "blocked" => 0,
        "target_unknown" | "target_mismatch" | "live_check_required" | "activity_uncertain" => 1,
        "preserve_unpublished" | "refresh_before_integration" => 2,
        "review_for_integration" => 3,
        "prune_review" => 4,
        "no_unique_commits" => 5,
        _ => 6,
    }
}

fn git_is_ancestor(path: &Path, ancestor: &str, descendant: &str) -> bool {
    run_git(
        path,
        vec![
            "merge-base".to_string(),
            "--is-ancestor".to_string(),
            ancestor.to_string(),
            descendant.to_string(),
        ],
    )
    .map(|result| result.success)
    .unwrap_or(false)
}

fn merge_tree_conflicts(output: &str) -> BTreeMap<String, u64> {
    let mut breakdown = BTreeMap::new();
    for line in output.lines() {
        let trimmed = line.trim();
        let kind = if let Some(kind) = trimmed
            .strip_prefix("CONFLICT (")
            .and_then(|value| value.split_once(')'))
            .map(|(kind, _)| kind)
        {
            Some(kind.to_string())
        } else {
            match trimmed {
                "changed in both" => Some("content".to_string()),
                "added in both" => Some("add/add".to_string()),
                "removed in local" | "removed in remote" => Some("modify/delete".to_string()),
                "removed in both" => Some("delete/delete".to_string()),
                _ => None,
            }
        };
        let Some(kind) = kind else {
            continue;
        };
        *breakdown.entry(kind).or_insert(0) += 1;
    }
    breakdown
}

fn agent_fold_merge_preview(
    path: &Path,
    source_branch: &str,
    target_branch: &str,
) -> Option<AgentFoldMergePreview> {
    let merge_base = git_owned(
        path,
        vec![
            "merge-base".to_string(),
            target_branch.to_string(),
            source_branch.to_string(),
        ],
    )?;
    let target_is_ancestor = git_is_ancestor(path, target_branch, source_branch);
    let source_is_ancestor = git_is_ancestor(path, source_branch, target_branch);
    let merge_strategy = if target_is_ancestor {
        "fast-forward"
    } else if source_is_ancestor {
        "already-integrated"
    } else {
        "three-way-merge"
    };
    let target_only_commits = git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{merge_base}..{target_branch}"),
        ],
    )?
    .parse()
    .ok()?;
    let source_only_commits = git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{merge_base}..{source_branch}"),
        ],
    )?
    .parse()
    .ok()?;
    let conflict_breakdown = if merge_strategy == "three-way-merge" {
        run_git(
            path,
            vec![
                "merge-tree".to_string(),
                merge_base.clone(),
                target_branch.to_string(),
                source_branch.to_string(),
            ],
        )
        .ok()
        .map(|result| merge_tree_conflicts(&result.stdout))
        .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let conflict_count = conflict_breakdown.values().sum();
    Some(AgentFoldMergePreview {
        merge_strategy: merge_strategy.to_string(),
        fast_forwardable: target_is_ancestor,
        target_is_ancestor,
        source_is_ancestor,
        merge_base,
        target_only_commits,
        source_only_commits,
        conflict_count,
        conflict_breakdown,
    })
}

fn agent_fold_candidate(
    repository: &RepositorySnapshot,
    branch: &BranchSummary,
    target: Option<&str>,
    target_source: &str,
    target_confidence: &str,
    include_merge_preview: bool,
) -> AgentFoldCandidate {
    let workspace = branch.workspace_id.as_deref().and_then(|workspace_id| {
        repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
    });
    let (decision, reason, next_safe_step) =
        agent_fold_candidate_decision(branch, target, workspace);
    AgentFoldCandidate {
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        source_branch: branch.name.clone(),
        target_branch: target.map(str::to_string),
        target_source: target_source.to_string(),
        target_confidence: target_confidence.to_string(),
        workspace_id: branch.workspace_id.clone(),
        workspace_path: workspace.map(|item| item.path.clone()),
        role: branch.role.clone(),
        role_confidence: branch.role_confidence.clone(),
        integration_state: branch.integration_state.clone(),
        dirty: workspace.map(|item| item.dirty),
        sync_state: workspace.map(|item| item.sync_state.clone()),
        ahead: workspace.map(|item| item.ahead).unwrap_or(branch.ahead),
        behind: workspace.map(|item| item.behind).unwrap_or(branch.behind),
        upstream: workspace.and_then(|item| item.upstream.clone()),
        operation: workspace.and_then(|item| item.operation.clone()),
        activity_state: workspace.map(|item| item.activity.state.clone()),
        activity_confidence: workspace.map(|item| item.activity.confidence.clone()),
        merge_preview: include_merge_preview.then(|| target).flatten().and_then(|target| {
            let merge_path = workspace
                .map(|item| Path::new(item.path.as_str()))
                .unwrap_or_else(|| Path::new(&repository.path));
            agent_fold_merge_preview(merge_path, &branch.name, target)
        }),
        decision,
        reason,
        next_safe_step: next_safe_step.to_string(),
        authorization: "Preview only; Git, provider, branch, worktree, merge, rebase, push, and delete mutations require explicit authorization.".to_string(),
    }
}

fn agent_fold_preview_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    requested_target: Option<&str>,
    scope: &str,
    limit: usize,
) -> Result<AgentFoldPreview, String> {
    agent_fold_preview_report_with_merge_preview(
        snapshot,
        query,
        requested_target,
        scope,
        limit,
        true,
    )
}

fn agent_fold_preview_report_with_merge_preview(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    requested_target: Option<&str>,
    scope: &str,
    limit: usize,
    include_merge_preview: bool,
) -> Result<AgentFoldPreview, String> {
    let repositories = if let Some(query) = query {
        vec![find_cli_repository(snapshot, query)?]
    } else {
        snapshot.repositories.iter().collect::<Vec<_>>()
    };
    let branch_total = repositories
        .iter()
        .map(|repository| repository.branches.len())
        .sum();
    let mut candidates = Vec::new();
    for repository in &repositories {
        let (target, target_source, target_confidence) =
            agent_fold_target(repository, requested_target);
        for branch in repository.branches.iter().filter(|branch| {
            target
                .as_deref()
                .map_or(true, |target| branch.name != target)
        }) {
            let mut candidate_branch = branch.clone();
            let explicit_local_target = requested_target
                .and_then(|_| target.as_deref())
                .filter(|target| repository.branches.iter().any(|item| &item.name == target));
            if let Some(target) = explicit_local_target {
                let workspace = branch.workspace_id.as_deref().and_then(|workspace_id| {
                    repository
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.id == workspace_id)
                });
                candidate_branch.target_branch = Some(target.to_string());
                candidate_branch.target_confidence = "Explicit".to_string();
                candidate_branch.integration_state = branch_integration_state(
                    Path::new(&repository.path),
                    &branch.name,
                    Some(target),
                    workspace,
                );
            }
            candidates.push(agent_fold_candidate(
                repository,
                &candidate_branch,
                target.as_deref(),
                &target_source,
                &target_confidence,
                include_merge_preview,
            ));
        }
    }
    candidates.sort_by(|left, right| {
        agent_fold_decision_priority(&left.decision)
            .cmp(&agent_fold_decision_priority(&right.decision))
            .then_with(|| left.repository_name.cmp(&right.repository_name))
            .then_with(|| left.source_branch.cmp(&right.source_branch))
    });
    let candidate_total = candidates.len();
    candidates.truncate(limit);
    Ok(AgentFoldPreview {
        schema_version: AGENT_FOLD_PREVIEW_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        repository_count: repositories.len(),
        branch_total,
        candidate_total,
        candidates,
        live_verification_required: true,
        authorization: "Inspection only; use fold-feature-branches for live ref classification, reviewed integration, and any authorized pruning.".to_string(),
    })
}

fn agent_doctor_check(
    id: &str,
    status: &str,
    summary: String,
    evidence: Vec<String>,
    next_safe_step: String,
) -> AgentDoctorCheck {
    AgentDoctorCheck {
        id: id.to_string(),
        status: status.to_string(),
        summary,
        evidence,
        next_safe_step,
    }
}

fn agent_doctor_relevant_roots<'a>(
    snapshot: &'a PortfolioSnapshot,
    scope: &str,
) -> Vec<&'a RootConfig> {
    if scope == "fleet" {
        return snapshot.roots.iter().collect();
    }
    snapshot
        .roots
        .iter()
        .filter(|root| {
            snapshot.repositories.iter().any(|repository| {
                Path::new(&repository.path).starts_with(Path::new(&root.path))
                    || repository.workspaces.iter().any(|workspace| {
                        Path::new(&workspace.path).starts_with(Path::new(&root.path))
                    })
            })
        })
        .collect()
}

fn agent_doctor_report(
    snapshot: &PortfolioSnapshot,
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
) -> AgentDoctorReport {
    let max_age_minutes = max_age_minutes.max(0);
    let now = Utc::now();
    let relevant_roots = agent_doctor_relevant_roots(snapshot, scope);
    let mut missing_root_paths = Vec::new();
    let mut unavailable_paths = BTreeSet::new();
    for root in &relevant_roots {
        if !Path::new(&root.path).is_dir() {
            missing_root_paths.push(root.path.clone());
            unavailable_paths.insert(root.path.clone());
        }
    }
    for repository in &snapshot.repositories {
        if !Path::new(&repository.path).is_dir() {
            unavailable_paths.insert(repository.path.clone());
        }
        for workspace in &repository.workspaces {
            if !Path::new(&workspace.path).is_dir() {
                unavailable_paths.insert(workspace.path.clone());
            }
        }
    }

    let mut stale_repository_ids = Vec::new();
    let mut invalid_scan_repository_ids = Vec::new();
    let mut oldest_scan: Option<(DateTime<Utc>, i64)> = None;
    for repository in &snapshot.repositories {
        let Ok(parsed) = DateTime::parse_from_rfc3339(&repository.last_scan_at) else {
            invalid_scan_repository_ids.push(repository.id.clone());
            continue;
        };
        let observed_at = parsed.with_timezone(&Utc);
        let age_minutes = now.signed_duration_since(observed_at).num_minutes();
        if age_minutes < 0 {
            invalid_scan_repository_ids.push(repository.id.clone());
            continue;
        }
        if age_minutes > max_age_minutes {
            stale_repository_ids.push(repository.id.clone());
        }
        if oldest_scan
            .as_ref()
            .map_or(true, |(oldest, _)| observed_at < *oldest)
        {
            oldest_scan = Some((observed_at, age_minutes));
        }
    }

    let mut checks = vec![agent_doctor_check(
        "storage",
        "Passed",
        "The Pronto database loaded through a read-only connection.".to_string(),
        vec![storage_path.display().to_string()],
        "Continue using focused Pronto projections for the requested scope.".to_string(),
    )];
    if relevant_roots.is_empty() && (scope == "fleet" || snapshot.repositories.is_empty()) {
        checks.push(agent_doctor_check(
            "roots",
            "Blocked",
            "No discovery roots are registered for this scope.".to_string(),
            Vec::new(),
            "Register an explicit discovery root and run a scoped refresh before routing work."
                .to_string(),
        ));
    } else if relevant_roots.is_empty() {
        checks.push(agent_doctor_check(
            "roots",
            "Warning",
            "No registered discovery root covers the scoped repositories.".to_string(),
            snapshot
                .repositories
                .iter()
                .map(|repository| repository.path.clone())
                .collect(),
            "Keep the repository scope explicit; register a covering root before relying on discovery refresh."
                .to_string(),
        ));
    } else if missing_root_paths.is_empty() {
        checks.push(agent_doctor_check(
            "roots",
            "Passed",
            format!(
                "{} registered discovery roots are available.",
                relevant_roots.len()
            ),
            relevant_roots
                .iter()
                .map(|root| root.path.clone())
                .collect(),
            "Keep refreshes scoped to the repository or root required by the task.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "roots",
            "Blocked",
            format!(
                "{} registered discovery root(s) are unavailable.",
                missing_root_paths.len()
            ),
            missing_root_paths.clone(),
            "Inspect the registered roots and restore or explicitly reconfigure unavailable paths before routing work.".to_string(),
        ));
    }

    if snapshot.repositories.is_empty() {
        checks.push(agent_doctor_check(
            "snapshot",
            "Blocked",
            "The persisted snapshot contains no repositories.".to_string(),
            Vec::new(),
            "Run a scoped refresh after confirming the discovery root; do not infer an empty portfolio from this report.".to_string(),
        ));
    } else if !stale_repository_ids.is_empty() || !invalid_scan_repository_ids.is_empty() {
        let mut evidence = stale_repository_ids.clone();
        evidence.extend(
            invalid_scan_repository_ids
                .iter()
                .map(|id| format!("invalid timestamp: {id}")),
        );
        checks.push(agent_doctor_check(
            "snapshot",
            "Blocked",
            format!(
                "{} repository snapshot(s) exceed the {} minute freshness window or have invalid scan timestamps.",
                stale_repository_ids.len() + invalid_scan_repository_ids.len(),
                max_age_minutes
            ),
            evidence,
            "Run a scoped `pronto refresh <repository> --json` for every stale or invalid repository, then rerun doctor.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "snapshot",
            "Passed",
            format!(
                "All {} repository snapshots are within the {} minute freshness window.",
                snapshot.repositories.len(),
                max_age_minutes
            ),
            snapshot
                .repositories
                .iter()
                .map(|repository| format!("{}: {}", repository.id, repository.last_scan_at))
                .collect(),
            "Use the focused projection that matches the task scope.".to_string(),
        ));
    }

    let unavailable_paths = unavailable_paths.into_iter().collect::<Vec<_>>();
    if unavailable_paths.is_empty() {
        checks.push(agent_doctor_check(
            "paths",
            "Passed",
            format!(
                "All {} repository and workspace paths are available.",
                snapshot.repositories.len()
                    + snapshot
                        .repositories
                        .iter()
                        .map(|repository| repository.workspaces.len())
                        .sum::<usize>()
            ),
            Vec::new(),
            "Treat the persisted paths as local evidence and still recheck live state before mutation.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "paths",
            "Blocked",
            format!(
                "{} registered repository or workspace path(s) are unavailable.",
                unavailable_paths.len()
            ),
            unavailable_paths.clone(),
            "Preserve the affected work and inspect path ownership before refreshing or folding anything.".to_string(),
        ));
    }

    if snapshot.quality.audit_status == "Ready" {
        checks.push(agent_doctor_check(
            "quality",
            "Passed",
            "Quality evidence reports Ready in the persisted portfolio snapshot.".to_string(),
            vec![snapshot.quality.audit_status.clone()],
            "Keep quality evidence separate from fresh local execution proof.".to_string(),
        ));
    } else {
        checks.push(agent_doctor_check(
            "quality",
            "Warning",
            format!(
                "Quality evidence is {} and does not block local portfolio routing.",
                snapshot.quality.audit_status
            ),
            vec![snapshot.quality.audit_status.clone()],
            "Do not treat quality or maturity as passing evidence until its source is fresh and verified.".to_string(),
        ));
    }

    let blocking = checks
        .iter()
        .any(|check| check.status == "Blocked" || check.status == "Unknown");
    let warning = checks.iter().any(|check| check.status == "Warning");
    let ready = !blocking;
    let status = if blocking {
        "Blocked"
    } else if warning {
        "Ready with warnings"
    } else {
        "Ready"
    };
    let next_safe_step = checks
        .iter()
        .find(|check| check.status == "Blocked" || check.status == "Unknown")
        .map(|check| check.next_safe_step.clone())
        .unwrap_or_else(|| {
            "Proceed with the focused Pronto projection appropriate to the task.".to_string()
        });

    AgentDoctorReport {
        schema_version: AGENT_DOCTOR_SCHEMA.to_string(),
        generated_at: iso_now(),
        scope: scope.to_string(),
        status: status.to_string(),
        ready,
        storage_path: storage_path.to_string_lossy().to_string(),
        max_age_minutes,
        root_count: relevant_roots.len(),
        repository_count: snapshot.repositories.len(),
        workspace_count: snapshot
            .repositories
            .iter()
            .map(|repository| repository.workspaces.len())
            .sum(),
        oldest_scan_at: oldest_scan
            .as_ref()
            .map(|(observed_at, _)| observed_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
        oldest_scan_age_minutes: oldest_scan.map(|(_, age_minutes)| age_minutes),
        stale_repository_ids,
        invalid_scan_repository_ids,
        unavailable_paths,
        checks,
        next_safe_step,
        authorization: "Inspection only; doctor does not refresh, write Pronto state, modify Git, access provider state, or authorize repository mutations.".to_string(),
    }
}

fn agent_doctor_error_report(
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    check_id: &str,
    error: String,
) -> AgentDoctorReport {
    let next_safe_step = if error.contains("Fresh quality projection") {
        "Rerun without `--fresh` for the cached snapshot or run `pronto quality refresh` separately before retrying a fresh projection.".to_string()
    } else if check_id == "storage" {
        "Inspect or repair the local Pronto database; do not route work from this failed state."
            .to_string()
    } else {
        "Resolve the doctor scope query and rerun doctor before routing work.".to_string()
    };
    AgentDoctorReport {
        schema_version: AGENT_DOCTOR_SCHEMA.to_string(),
        generated_at: iso_now(),
        scope: scope.to_string(),
        status: "Blocked".to_string(),
        ready: false,
        storage_path: storage_path.to_string_lossy().to_string(),
        max_age_minutes: max_age_minutes.max(0),
        root_count: 0,
        repository_count: 0,
        workspace_count: 0,
        oldest_scan_at: None,
        oldest_scan_age_minutes: None,
        stale_repository_ids: Vec::new(),
        invalid_scan_repository_ids: Vec::new(),
        unavailable_paths: Vec::new(),
        checks: vec![agent_doctor_check(
            check_id,
            "Blocked",
            error,
            vec![storage_path.display().to_string()],
            next_safe_step.clone(),
        )],
        next_safe_step,
        authorization: "Inspection only; doctor does not refresh, write Pronto state, modify Git, access provider state, or authorize repository mutations.".to_string(),
    }
}

fn agent_route_from_doctor(
    doctor: AgentDoctorReport,
    scope: &str,
    next: Option<AgentNextReport>,
    repository: Option<AgentRepositoryDetail>,
    quality: Option<AgentQualityReport>,
    fold_preview: Option<AgentFoldPreview>,
) -> AgentRouteReport {
    let change_maturity = repository.as_ref().map(|detail| {
        let maturity = &detail.repository.quality.maturity;
        let score = maturity
            .dimension_scores
            .get("change_surface_coverage")
            .copied();
        let gaps = maturity
            .gaps
            .iter()
            .filter(|gap| {
                matches!(
                    gap.dimension.as_str(),
                    "change_surface_coverage" | "skill_contract_quality"
                )
            })
            .take(3)
            .map(|gap| gap.message.clone())
            .collect::<Vec<_>>();
        AgentChangeMaturitySummary {
            score,
            status: match score {
                Some(value) if value >= 4.0 => "proven",
                Some(value) if value >= 3.0 => "validated",
                Some(value) if value > 0.0 => "attention",
                Some(_) => "missing",
                None => "unknown",
            }
            .to_string(),
            gaps,
            recommended_inspection: format!(
                "pronto change-matrix repo '{}' --json",
                detail.repository.path.replace('\'', "'\\''")
            ),
        }
    });
    let next_safe_step = if doctor.ready {
        next.as_ref()
            .and_then(|report| report.actions.first())
            .map(|action| action.next_safe_step.clone())
            .unwrap_or_else(|| {
                "No active attention action was observed; choose the next bounded inspection for this scope.".to_string()
            })
    } else {
        doctor.next_safe_step.clone()
    };
    AgentRouteReport {
        schema_version: AGENT_ROUTE_SCHEMA.to_string(),
        generated_at: doctor.generated_at.clone(),
        scope: scope.to_string(),
        status: doctor.status.clone(),
        ready: doctor.ready,
        doctor,
        next,
        repository,
        quality,
        fold_preview,
        change_maturity,
        next_safe_step,
        authorization: "Inspection only; this route does not refresh, modify Git, change provider state, update remediation status, or authorize repository or release mutations.".to_string(),
    }
}

fn agent_route_report(
    snapshot: &PortfolioSnapshot,
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    query: Option<&str>,
    limit: usize,
) -> Result<AgentRouteReport, String> {
    let doctor = agent_doctor_report(snapshot, storage_path, max_age_minutes, scope);
    if !doctor.ready {
        return Ok(agent_route_from_doctor(
            doctor, scope, None, None, None, None,
        ));
    }

    let next = agent_next_report(snapshot, query, scope, limit)?;
    let repository = query
        .map(|value| {
            find_cli_repository(snapshot, value)
                .map(|repository| agent_repository_detail(snapshot, repository))
        })
        .transpose()?;
    let quality = Some(agent_quality_report_with_scope(snapshot, None, scope)?);
    let fold_preview = Some(agent_fold_preview_report_with_merge_preview(
        snapshot, query, None, scope, limit, false,
    )?);
    Ok(agent_route_from_doctor(
        doctor,
        scope,
        Some(next),
        repository,
        quality,
        fold_preview,
    ))
}

fn agent_route_error_report(
    storage_path: &Path,
    max_age_minutes: i64,
    scope: &str,
    check_id: &str,
    error: String,
) -> AgentRouteReport {
    let doctor = agent_doctor_error_report(storage_path, max_age_minutes, scope, check_id, error);
    agent_route_from_doctor(doctor, scope, None, None, None, None)
}

fn agent_summary(snapshot: &PortfolioSnapshot, scope: &str) -> AgentSummary {
    let repositories = snapshot
        .repositories
        .iter()
        .map(agent_repository_summary)
        .collect::<Vec<_>>();
    let attention_count = agent_attention_report(snapshot).items.len();
    AgentSummary {
        schema_version: AGENT_SUMMARY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        repository_count: repositories.len(),
        active_condition_count: repositories
            .iter()
            .map(|repository| repository.active_conditions.len())
            .sum(),
        dirty_workspace_count: repositories
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .filter(|workspace| workspace.dirty)
            .count(),
        unsynced_workspace_count: repositories
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .filter(|workspace| workspace.sync_state != "Synced")
            .count(),
        attention_count,
        provider_status: snapshot.provider_status.clone(),
        quality: snapshot.quality.clone(),
        repositories,
    }
}

fn agent_repository_detail(
    snapshot: &PortfolioSnapshot,
    repository: &RepositorySnapshot,
) -> AgentRepositoryDetail {
    AgentRepositoryDetail {
        schema_version: AGENT_REPOSITORY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        repository: repository.clone(),
        products: snapshot
            .products
            .iter()
            .filter(|product| product.repository_ids.iter().any(|id| id == &repository.id))
            .cloned()
            .collect(),
        groups: snapshot
            .groups
            .iter()
            .filter(|group| group.repository_ids.iter().any(|id| id == &repository.id))
            .cloned()
            .collect(),
    }
}

fn agent_quality_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
) -> Result<AgentQualityReport, String> {
    let scope = query
        .map(|value| format!("repository:{value}"))
        .unwrap_or_else(|| "fleet".to_string());
    agent_quality_report_with_scope(snapshot, query, &scope)
}

fn agent_quality_report_with_scope(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    scope: &str,
) -> Result<AgentQualityReport, String> {
    let repositories = if let Some(query) = query {
        vec![find_cli_repository(snapshot, query)?]
    } else {
        snapshot.repositories.iter().collect::<Vec<_>>()
    };
    Ok(AgentQualityReport {
        schema_version: AGENT_QUALITY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: scope.to_string(),
        portfolio: snapshot.quality.clone(),
        repositories: repositories
            .into_iter()
            .map(|repository| AgentRepositoryQuality {
                id: repository.id.clone(),
                name: repository.name.clone(),
                path: repository.path.clone(),
                branch: repository.branch.clone(),
                quality: repository.quality.clone(),
            })
            .collect(),
    })
}

fn agent_activity_report(
    snapshot: &PortfolioSnapshot,
    query: Option<&str>,
    limit: usize,
) -> Result<AgentActivityReport, String> {
    let repository_id = query
        .map(|value| find_cli_repository(snapshot, value).map(|repository| repository.id.clone()))
        .transpose()?;
    let events = snapshot
        .events
        .iter()
        .filter(|event| {
            repository_id
                .as_deref()
                .map_or(true, |id| event.repository_id == id)
        })
        .take(limit)
        .cloned()
        .collect();
    let action_audits = snapshot
        .action_audits
        .iter()
        .filter(|audit| {
            repository_id.as_deref().map_or(true, |id| {
                audit.target_ids.iter().any(|target| target == id)
            })
        })
        .take(limit)
        .cloned()
        .collect();
    Ok(AgentActivityReport {
        schema_version: AGENT_ACTIVITY_SCHEMA.to_string(),
        generated_at: snapshot.generated_at.clone(),
        scope: query
            .map(|value| format!("repository:{value}"))
            .unwrap_or_else(|| "fleet".to_string()),
        events,
        action_audits,
    })
}

fn launch_desktop_focus(repository: Option<&RepositorySnapshot>) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err(
            "Desktop focus from the companion CLI is currently implemented for macOS bundles."
                .to_string(),
        );
    }
    let status = Command::new("open")
        .args(["-a", "Pronto"])
        .status()
        .map_err(|error| format!("Could not launch the Pronto desktop app: {error}"))?;
    if !status.success() {
        return Err("The installed Pronto desktop app could not be opened".to_string());
    }
    if let Some(repository) = repository {
        println!("Opened Pronto for {}.", repository.name);
    } else {
        println!("Opened Pronto.");
    }
    Ok(())
}

fn print_human_status(snapshot: &PortfolioSnapshot) {
    if snapshot.repositories.is_empty() {
        println!("Pronto · no repositories registered");
        println!("Add a discovery root in the desktop app, then refresh.");
        return;
    }
    println!(
        "PRONTO STATUS · {} repositories",
        snapshot.repositories.len()
    );
    for repository in &snapshot.repositories {
        let active_conditions = repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
            .map(|condition| condition.title.as_str())
            .collect::<Vec<_>>();
        let condition_text = if active_conditions.is_empty() {
            "No active conditions".to_string()
        } else {
            active_conditions.join(" · ")
        };
        println!(
            "{} · {} · {} · {}",
            repository.name, repository.locality, repository.branch, condition_text
        );
    }
}

fn print_human_groups(groups: &[GroupConfig]) {
    if groups.is_empty() {
        println!("PRONTO GROUPS · no groups registered");
        return;
    }
    println!("PRONTO GROUPS · {} groups", groups.len());
    for group in groups {
        println!(
            "{} · {} repositories · {}",
            group.name,
            group.repository_ids.len(),
            group.id
        );
    }
}

fn print_human_products(products: &[ProductConfig]) {
    if products.is_empty() {
        println!("PRONTO PRODUCTS · no products registered");
        return;
    }
    println!("PRONTO PRODUCTS · {} products", products.len());
    for product in products {
        println!(
            "{} · {} repositories · {} · {}",
            product.name,
            product.repository_ids.len(),
            product.release_mode,
            product.id
        );
    }
}

fn print_human_summary(summary: &AgentSummary) {
    println!(
        "PRONTO SUMMARY · {} repositories · {} attention items",
        summary.repository_count, summary.attention_count
    );
    println!(
        "Conditions: {} active · workspaces: {} dirty, {} unsynced",
        summary.active_condition_count,
        summary.dirty_workspace_count,
        summary.unsynced_workspace_count
    );
    println!(
        "Quality: {} · maturity {}",
        summary.quality.audit_status,
        summary
            .quality
            .maturity_score_display
            .as_deref()
            .unwrap_or("unknown")
    );
}

fn print_human_next(report: &AgentNextReport) {
    println!(
        "PRONTO NEXT · {} · {} attention items",
        report.scope, report.attention_total
    );
    if let Some(repository) = &report.current_repository {
        println!(
            "Current repository: {} · {} · {}",
            repository.name, repository.branch, repository.path
        );
    }
    for action in &report.actions {
        println!(
            "  {} · {} · {} · {}",
            action.repository_name, action.category, action.severity, action.next_safe_step
        );
    }
}

fn print_human_fold_preview(report: &AgentFoldPreview) {
    println!(
        "PRONTO FOLD PREVIEW · {} · {} candidates",
        report.scope, report.candidate_total
    );
    println!(
        "Branches: {} observed · live verification required: {}",
        report.branch_total, report.live_verification_required
    );
    for candidate in &report.candidates {
        println!(
            "  {} · {} -> {} · {} · {}",
            candidate.repository_name,
            candidate.source_branch,
            candidate
                .target_branch
                .as_deref()
                .unwrap_or("unknown target"),
            candidate.decision,
            candidate.reason
        );
        if let Some(preview) = &candidate.merge_preview {
            let breakdown = if preview.conflict_breakdown.is_empty() {
                "none".to_string()
            } else {
                preview
                    .conflict_breakdown
                    .iter()
                    .map(|(kind, count)| format!("{kind} {count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!(
                "    Merge: {} · fast-forwardable: {} · base: {} · target-only: {} · source-only: {} · conflicts: {} ({})",
                preview.merge_strategy,
                preview.fast_forwardable,
                preview.merge_base,
                preview.target_only_commits,
                preview.source_only_commits,
                preview.conflict_count,
                breakdown
            );
        }
    }
}

fn print_human_doctor(report: &AgentDoctorReport) {
    println!("PRONTO DOCTOR · {} · {}", report.status, report.scope);
    println!(
        "Snapshot: {} roots · {} repositories · {} workspaces · max age {} minutes",
        report.root_count, report.repository_count, report.workspace_count, report.max_age_minutes
    );
    if let (Some(oldest_scan_at), Some(oldest_scan_age_minutes)) = (
        report.oldest_scan_at.as_deref(),
        report.oldest_scan_age_minutes,
    ) {
        println!("Oldest scan: {oldest_scan_at} · {oldest_scan_age_minutes} minutes ago");
    }
    for check in &report.checks {
        println!("  {} · {} · {}", check.status, check.id, check.summary);
    }
    println!("Next: {}", report.next_safe_step);
}

fn print_human_route(report: &AgentRouteReport) {
    println!("PRONTO ROUTE · {} · {}", report.status, report.scope);
    println!(
        "Doctor: {} · next projection: {} · repository: {} · quality: {} · fold preview: {}",
        report.doctor.status,
        if report.next.is_some() {
            "available"
        } else {
            "blocked"
        },
        if report.repository.is_some() {
            "available"
        } else {
            "not selected"
        },
        if report.quality.is_some() {
            "available"
        } else {
            "blocked"
        },
        if report.fold_preview.is_some() {
            "available"
        } else {
            "blocked"
        },
    );
    if let Some(change) = &report.change_maturity {
        println!(
            "Change maturity: {} · {}",
            change
                .score
                .map(|score| format!("{score:.0}/4"))
                .unwrap_or_else(|| "unknown".into()),
            change.status
        );
        for gap in &change.gaps {
            println!("  gap: {gap}");
        }
        println!("Inspect: {}", change.recommended_inspection);
    }
    println!("Next: {}", report.next_safe_step);
    println!("Authorization: {}", report.authorization);
}

fn print_human_repository(detail: &AgentRepositoryDetail) {
    let repository = &detail.repository;
    println!(
        "{} · {} · {} · {}",
        repository.name, repository.lifecycle, repository.branch, repository.path
    );
    println!(
        "Quality: {} · maturity {} · {} active conditions",
        repository.quality.ingestion_status,
        repository
            .quality
            .maturity
            .score_display
            .as_deref()
            .unwrap_or("unknown"),
        repository
            .conditions
            .iter()
            .filter(|condition| condition.status == "Active")
            .count()
    );
    for workspace in &repository.workspaces {
        let cleanliness = if !workspace.status_available {
            "unavailable"
        } else if workspace.dirty {
            "dirty"
        } else {
            "clean"
        };
        println!(
            "  {} · {} · {} · {}",
            workspace.branch, cleanliness, workspace.sync_state, workspace.path
        );
        if let Some(detail) = workspace.sync_detail.as_ref() {
            println!(
                "    {}: {}",
                if workspace.status_available {
                    "why unsynced"
                } else {
                    "why Git status is unavailable"
                },
                detail.reason
            );
            println!(
                "    evidence expires: {}",
                detail
                    .evidence_expires_at
                    .as_deref()
                    .unwrap_or("unavailable")
            );
            println!(
                "    next safe scoped refresh: {}",
                detail.scoped_refresh_command
            );
            println!("    authorization: {}", detail.authorization);
        }
    }
}

fn print_human_quality(report: &AgentQualityReport) {
    println!(
        "PRONTO QUALITY · {} repositories · {}",
        report.repositories.len(),
        report.portfolio.audit_status
    );
    println!(
        "Fleet maturity: {} · CI configuration: {}/{} configured · fresh passing evidence: {}/{}",
        report
            .portfolio
            .maturity_score_display
            .as_deref()
            .unwrap_or("unknown"),
        report.portfolio.ci_configuration_configured_gate_count,
        report.portfolio.ci_configuration_ideal_gate_count,
        report.portfolio.ci_evidence_fresh_passing_gate_count,
        report.portfolio.ci_evidence_ideal_gate_count,
    );
    for repository in &report.repositories {
        println!(
            "  {} · maturity {} · {}",
            repository.name,
            repository
                .quality
                .maturity
                .score_display
                .as_deref()
                .unwrap_or("unknown"),
            repository.quality.ingestion_status
        );
    }
}

fn print_human_remediation(run: &RemediationRun) {
    println!(
        "PRONTO REMEDIATION · {} · {} active · {} closed · {} excluded · {} GitHub-only candidates",
        run.status,
        run.plans.len(),
        run.closures.len(),
        run.excluded_repositories.len(),
        run.github_only_candidates.len()
    );
    if let Some(refresh_id) = run.source_refresh_id.as_deref() {
        println!("Refresh: {refresh_id}");
    }
    if let Some(message) = run.message.as_deref() {
        println!("Message: {message}");
    }
    for step in &run.refresh_steps {
        println!(
            "  refresh · {} · {} · {}",
            step.id, step.status, step.detail
        );
    }
    for exclusion in &run.excluded_repositories {
        println!(
            "  excluded · {} · {}",
            exclusion.repository_name, exclusion.reason
        );
    }
    for candidate in &run.github_only_candidates {
        println!(
            "  github-only · {} · {} · last task {} · observed {}",
            candidate.full_name,
            candidate.status,
            candidate.last_remediation_task,
            candidate.observed_at
        );
    }
    for (index, plan) in run.plans.iter().enumerate() {
        println!(
            "  #{} · {} · goal {} ({}) · {} · {}% · {} · {} actions",
            index + 1,
            plan.repository_name,
            plan.goal.target_state,
            plan.goal.source,
            plan.status,
            plan.progress.percentage.round(),
            plan.current_stage,
            plan.actions.len()
        );
    }
    for closure in &run.closures {
        println!(
            "  closed · {} · goal {} ({}) · {} · {} · {}",
            closure.repository_name,
            closure.target_state,
            closure.goal_source,
            closure.disposition,
            closure.closed_at,
            closure.summary
        );
    }
}

fn print_human_attention(report: &AgentAttentionReport) {
    println!("PRONTO ATTENTION · {} items", report.items.len());
    for item in &report.items {
        println!(
            "{} · {} · {} · {}",
            item.repository_name, item.category, item.status, item.summary
        );
    }
}

fn print_human_activity(report: &AgentActivityReport) {
    println!(
        "PRONTO ACTIVITY · {} events · {} action audits",
        report.events.len(),
        report.action_audits.len()
    );
    for event in &report.events {
        println!("  {} · {}", event.kind, event.summary);
    }
    for audit in &report.action_audits {
        println!("  {} · {} · {}", audit.action, audit.status, audit.summary);
    }
}

fn print_human_remediation_handoff_check(check: &RemediationHandoffCheck) {
    println!(
        "PRONTO REMEDIATION HANDOFF · {} · {}",
        check.repository_name, check.status
    );
    println!(
        "Workspace: {} · branch: {} · checkpoint required: {}",
        check.workspace_path, check.branch, check.checkpoint_required
    );
    for reason in &check.reasons {
        println!("  reason: {reason}");
    }
    println!("  next: {}", check.next_safe_step);
}

fn print_human_preparation(report: &AgentPreparationReport) {
    let preparation = &report.preparation;
    println!(
        "PRONTO PREPARATION · {} · {}",
        preparation.repository_id, preparation.generated_at
    );
    println!(
        "Pull request: {} · release: {} · recipe: {}",
        preparation.pull_request.status, preparation.release.status, preparation.recipe.status
    );
}

fn print_human_release(report: &AgentReleaseReport) {
    println!(
        "PRONTO RELEASE · {} · {}",
        report.repository_id, report.release.status
    );
    println!(
        "Baseline: {} · candidate: {} · recipe: {}",
        report.release.baseline_status,
        report
            .release
            .candidate_version
            .as_deref()
            .unwrap_or("unknown"),
        report.recipe.status
    );
    for reason in &report.release.reasons {
        println!("  reason: {reason}");
    }
}

fn print_cli_usage() {
    println!(
        "Usage: pronto . | pronto skills [<skill-id>] [--json] | pronto papercuts list --json | pronto papercuts observe --stdin --json [--dry-run] | pronto papercuts digest --week current --json | pronto papercuts propose --stdin --json | pronto papercuts proposal set-status <id> <status> --json | pronto papercuts health --json | pronto change-matrix repo <repository> [--operation <add|change|remove>] [--json] | pronto change-matrix skill <skill-id> [--operation <add|change|remove>] [--json] | pronto route [<repository>] [--fresh] [--json] | pronto quality [<repository>] [--json] | pronto quality refresh [--json] | pronto remediation handoff-check <repository> [--workspace <id>] [--json] | pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json] | pronto status [--fresh] [--json] | pronto help"
    );
}

fn print_cli_json_error(command: &str, error: &str) {
    let payload = serde_json::json!({
        "schema_version": "pronto-cli-error/v1",
        "generated_at": iso_now(),
        "command": command,
        "status": "Blocked",
        "error": error,
        "next_safe_step": "Retry with the cached read path or resolve the reported storage/quality blocker."
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
            "{\"schema_version\":\"pronto-cli-error/v1\",\"status\":\"Blocked\"}".to_string()
        })
    );
}

pub fn run_cli(arguments: Vec<String>) {
    let command = arguments.first().map(String::as_str).unwrap_or("status");
    let json = arguments.iter().any(|argument| argument == "--json");
    let path = store_path();
    match command {
        "help" | "-h" | "--help" => {
            print_cli_usage();
        }
        "skills" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.first().map(String::as_str) == Some("open") && positionals.len() == 2 {
                match skills::open_source(&positionals[1]) {
                    Ok(()) => println!("Opened skill source: {}", positionals[1]),
                    Err(error) => {
                        eprintln!("Pronto could not open skill source: {error}");
                        std::process::exit(1);
                    }
                }
            } else if positionals.len() > 1 {
                eprintln!("Usage: pronto skills [<skill-id>] [--json]");
                std::process::exit(2);
            }
            match skills::load(&path) {
                Ok(mut snapshot) if json => {
                    if let Some(query) = positionals.first() {
                        snapshot.skills.retain(|skill| {
                            skill.id == *query || skill.name.eq_ignore_ascii_case(query)
                        });
                        if snapshot.skills.is_empty() {
                            eprintln!("Pronto could not find skill: {query}");
                            std::process::exit(1);
                        }
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".into())
                    )
                }
                Ok(snapshot) => {
                    if let Some(query) = positionals.first() {
                        let Some(skill) = snapshot.skills.iter().find(|skill| {
                            skill.id == *query || skill.name.eq_ignore_ascii_case(query)
                        }) else {
                            eprintln!("Pronto could not find skill: {query}");
                            std::process::exit(1);
                        };
                        println!("PRONTO SKILL · {}", skill.name);
                        println!("Description: {}", skill.description);
                        println!("Category: {} · Family: {}", skill.category, skill.family);
                        println!("Lifecycle: {}", skill.lifecycle);
                        println!(
                            "Usage: {} recent · {} all-time",
                            skill.usage.recent_count, skill.usage.all_time_count
                        );
                    } else {
                        println!(
                            "PRONTO SKILLS · {} skills · {}",
                            snapshot.skills.len(),
                            snapshot.freshness
                        );
                    }
                }
                Err(error) => {
                    eprintln!("Pronto could not read skills: {error}");
                    std::process::exit(1);
                }
            }
        }
        "papercuts" => match papercuts::run_cli(&arguments) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                if json {
                    print_cli_json_error("papercuts", &error);
                } else {
                    eprintln!("Pronto Papercuts error: {error}");
                }
                std::process::exit(1);
            }
        },
        "change-matrix" => {
            let operation = cli_option(&arguments, "--operation").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if operation
                .as_deref()
                .is_some_and(|value| !matches!(value, "add" | "change" | "remove"))
            {
                eprintln!("Pronto CLI error: --operation must be add, change, or remove");
                std::process::exit(2);
            }
            let positionals =
                cli_positionals(&arguments, &["--operation"]).unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if positionals.len() != 2 || !matches!(positionals[0].as_str(), "repo" | "skill") {
                eprintln!("Usage: pronto change-matrix repo <repository> [--operation <add|change|remove>] [--json] | pronto change-matrix skill <skill-id> [--operation <add|change|remove>] [--json]");
                std::process::exit(2);
            }
            let report = if positionals[0] == "repo" {
                let state = load_store_read_only(&path).unwrap_or_else(|error| {
                    eprintln!("Pronto could not read repository state: {error}");
                    std::process::exit(1);
                });
                let snapshot = snapshot_from_store(&path, &state);
                let repository =
                    find_cli_repository(&snapshot, &positionals[1]).unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(1);
                    });
                change_matrix::inspect_repository(
                    Path::new(&repository.path),
                    &repository.id,
                    repository.remote_url.as_deref(),
                    operation.as_deref(),
                )
            } else {
                let snapshot = skills::load(&path).unwrap_or_else(|error| {
                    eprintln!("Pronto could not read skills: {error}");
                    std::process::exit(1);
                });
                let matches = snapshot
                    .skills
                    .iter()
                    .filter(|skill| {
                        skill.id == positionals[1]
                            || skill.name.eq_ignore_ascii_case(&positionals[1])
                    })
                    .collect::<Vec<_>>();
                let skill = match matches.as_slice() {
                    [skill] => *skill,
                    [] => {
                        eprintln!("Pronto could not find skill: {}", positionals[1]);
                        std::process::exit(1);
                    }
                    _ => {
                        eprintln!("Pronto skill query is ambiguous: {}", positionals[1]);
                        std::process::exit(1);
                    }
                };
                change_matrix::inspect_skill(skill, operation.as_deref())
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
                );
            } else {
                println!(
                    "CHANGE MATRIX · {} · {} · {}",
                    report.subject_kind, report.subject_id, report.status
                );
                println!("{}", report.maturity_impact);
                println!("Expected: {}", report.expected_contract_location);
            }
        }
        "refresh-skills" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if !positionals.is_empty() {
                eprintln!("Usage: pronto refresh-skills [--json]");
                std::process::exit(2);
            }
            match skills::refresh(&path) {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".into())
                ),
                Ok(snapshot) => {
                    println!("PRONTO SKILLS · refreshed {} skills", snapshot.skills.len())
                }
                Err(error) => {
                    eprintln!("Pronto could not refresh skills: {error}");
                    std::process::exit(1);
                }
            }
        }
        "route" => {
            let fresh = arguments.iter().any(|argument| argument == "--fresh");
            let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let max_age_minutes = cli_option(&arguments, "--max-age")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    let parsed = value.parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Pronto CLI error: --max-age must be a non-negative integer");
                        std::process::exit(2);
                    });
                    if parsed < 0 || parsed > MAX_AGENT_DOCTOR_MAX_AGE_MINUTES {
                        eprintln!(
                            "Pronto CLI error: --max-age must be between 0 and {MAX_AGENT_DOCTOR_MAX_AGE_MINUTES}"
                        );
                        std::process::exit(2);
                    }
                    parsed
                })
                .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES);
            let limit = cli_option(&arguments, "--limit")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    let parsed = value.parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                        std::process::exit(2);
                    });
                    if parsed > MAX_AGENT_NEXT_LIMIT {
                        eprintln!(
                            "Pronto CLI error: --limit must be {MAX_AGENT_NEXT_LIMIT} or less"
                        );
                        std::process::exit(2);
                    }
                    parsed
                })
                .unwrap_or(DEFAULT_AGENT_NEXT_LIMIT);
            let positionals = cli_positionals_with_flags(
                &arguments,
                &["--product", "--group", "--max-age", "--limit"],
                &["--fresh"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!(
                    "Usage: pronto route [<repository>] [--product <name> | --group <name>] [--max-age <minutes>] [--limit <n>] [--fresh] [--json]"
                );
                std::process::exit(2);
            }
            let query = positionals.first().map(String::as_str);
            let base_scope = product_name
                .as_deref()
                .map(|value| format!("product:{value}"))
                .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
                .unwrap_or_else(|| "fleet".to_string());
            let scope = query
                .map(|value| format!("{base_scope}; current_repository:{value}"))
                .unwrap_or(base_scope);
            let state_result = if fresh {
                load_store_read_only_with_quality_bounded(&path)
            } else {
                load_store_read_only(&path)
            };
            let report = match state_result {
                Ok(state) => {
                    let snapshot = snapshot_from_store(&path, &state);
                    let scoped_snapshot = filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                    .and_then(|snapshot| {
                        if let Some(query) = query {
                            let repository_id = find_cli_repository(&snapshot, query)?.id.clone();
                            let repository_ids = [repository_id].into_iter().collect();
                            Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
                        } else {
                            Ok(snapshot)
                        }
                    });
                    match scoped_snapshot {
                        Ok(snapshot) => agent_route_report(
                            &snapshot,
                            &path,
                            max_age_minutes,
                            &scope,
                            query,
                            limit,
                        )
                        .unwrap_or_else(|error| {
                            agent_route_error_report(
                                &path,
                                max_age_minutes,
                                &scope,
                                "projection",
                                error,
                            )
                        }),
                        Err(error) => {
                            agent_route_error_report(&path, max_age_minutes, &scope, "scope", error)
                        }
                    }
                }
                Err(error) => {
                    agent_route_error_report(&path, max_age_minutes, &scope, "storage", error)
                }
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_human_route(&report);
            }
            if !report.ready {
                std::process::exit(1);
            }
        }
        "group" => {
            let positionals = cli_positionals(&arguments, &["--repo"]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            match positionals.first().map(String::as_str) {
                Some("list") if positionals.len() == 1 => match load_store(&path) {
                    Ok(state) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&state.groups)
                            .unwrap_or_else(|_| "[]".to_string())
                    ),
                    Ok(state) => print_human_groups(&state.groups),
                    Err(error) => {
                        eprintln!("Pronto could not read groups: {error}");
                        std::process::exit(1);
                    }
                },
                Some("create") if positionals.len() == 2 => {
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    match upsert_group_at(&path, None, &positionals[1], repository_ids)
                        .map(|snapshot| snapshot.groups)
                    {
                        Ok(groups) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&groups)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(groups) => print_human_groups(&groups),
                        Err(error) => {
                            eprintln!("Pronto could not create group: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("append") if positionals.len() == 2 => {
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    if repository_ids.is_empty() {
                        eprintln!("Usage: pronto group append <group> --repo <id>... [--json]");
                        std::process::exit(2);
                    }
                    let result = load_store(&path).and_then(|state| {
                        let group = find_cli_group(&state, &positionals[1])?;
                        let repository_ids =
                            merge_repository_ids(&group.repository_ids, repository_ids);
                        upsert_group_at(&path, Some(&group.id), &group.name, repository_ids)
                    });
                    match result.map(|snapshot| snapshot.groups) {
                        Ok(groups) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&groups)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(groups) => print_human_groups(&groups),
                        Err(error) => {
                            eprintln!("Pronto could not append to group: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("update") if positionals.len() == 3 => {
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    let result = load_store(&path).and_then(|state| {
                        let group = find_cli_group(&state, &positionals[1])?;
                        upsert_group_at(&path, Some(&group.id), &positionals[2], repository_ids)
                    });
                    match result.map(|snapshot| snapshot.groups) {
                        Ok(groups) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&groups)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(groups) => print_human_groups(&groups),
                        Err(error) => {
                            eprintln!("Pronto could not update group: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("delete") if positionals.len() == 2 => {
                    let result = load_store(&path)
                        .and_then(|state| {
                            find_cli_group(&state, &positionals[1]).map(|group| group.id.clone())
                        })
                        .and_then(|group_id| delete_group_at(&path, &group_id))
                        .map(|snapshot| snapshot.groups);
                    match result {
                        Ok(groups) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&groups)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(groups) => print_human_groups(&groups),
                        Err(error) => {
                            eprintln!("Pronto could not delete group: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!("Usage: pronto group list [--json] | pronto group create <name> [--repo <id>]... [--json] | pronto group append <group> --repo <id>... [--json] | pronto group update <group> <name> [--repo <id>]... [--json] | pronto group delete <group> [--json]");
                    std::process::exit(2);
                }
            }
        }
        "analytics" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if !positionals.is_empty() {
                eprintln!("Usage: pronto analytics [--json]");
                std::process::exit(2);
            }
            match load_analytics_at(&path) {
                Ok(analytics) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&analytics).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(analytics) => println!(
                    "PRONTO ANALYTICS · {} portfolio samples · {} repository series · {}",
                    analytics.portfolio_samples.len(),
                    analytics.repositories.len(),
                    analytics.freshness
                ),
                Err(error) => {
                    eprintln!("Pronto could not read analytics: {error}");
                    std::process::exit(1);
                }
            }
        }
        "product" => {
            let positionals = cli_positionals(&arguments, &["--repo", "--release-mode"])
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            match positionals.first().map(String::as_str) {
                Some("list") if positionals.len() == 1 => match load_store(&path) {
                    Ok(state) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&state.products)
                            .unwrap_or_else(|_| "[]".to_string())
                    ),
                    Ok(state) => print_human_products(&state.products),
                    Err(error) => {
                        eprintln!("Pronto could not read products: {error}");
                        std::process::exit(1);
                    }
                },
                Some("create") if positionals.len() == 2 => {
                    let release_mode = cli_option(&arguments, "--release-mode")
                        .unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        })
                        .unwrap_or_else(|| {
                            eprintln!("Usage: pronto product create <name> --release-mode <mode> [--repo <id>]... [--json]");
                            std::process::exit(2);
                        });
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    match upsert_product_at(
                        &path,
                        None,
                        &positionals[1],
                        repository_ids,
                        &release_mode,
                    )
                    .map(|snapshot| snapshot.products)
                    {
                        Ok(products) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&products)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(products) => print_human_products(&products),
                        Err(error) => {
                            eprintln!("Pronto could not create product: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("append") if positionals.len() == 2 => {
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    if repository_ids.is_empty() {
                        eprintln!("Usage: pronto product append <product> --repo <id>... [--json]");
                        std::process::exit(2);
                    }
                    let result = load_store(&path).and_then(|state| {
                        let product = find_cli_product(&state, &positionals[1])?;
                        let repository_ids =
                            merge_repository_ids(&product.repository_ids, repository_ids);
                        upsert_product_at(
                            &path,
                            Some(&product.id),
                            &product.name,
                            repository_ids,
                            &product.release_mode,
                        )
                    });
                    match result.map(|snapshot| snapshot.products) {
                        Ok(products) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&products)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(products) => print_human_products(&products),
                        Err(error) => {
                            eprintln!("Pronto could not append to product: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("update") if positionals.len() == 3 => {
                    let release_mode = cli_option(&arguments, "--release-mode")
                        .unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        })
                        .unwrap_or_else(|| {
                            eprintln!("Usage: pronto product update <product> <name> --release-mode <mode> [--repo <id>]... [--json]");
                            std::process::exit(2);
                        });
                    let repository_ids =
                        cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    let result = load_store(&path).and_then(|state| {
                        let product = find_cli_product(&state, &positionals[1])?;
                        upsert_product_at(
                            &path,
                            Some(&product.id),
                            &positionals[2],
                            repository_ids,
                            &release_mode,
                        )
                    });
                    match result.map(|snapshot| snapshot.products) {
                        Ok(products) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&products)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(products) => print_human_products(&products),
                        Err(error) => {
                            eprintln!("Pronto could not update product: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("delete") if positionals.len() == 2 => {
                    let result = load_store(&path)
                        .and_then(|state| {
                            find_cli_product(&state, &positionals[1])
                                .map(|product| product.id.clone())
                        })
                        .and_then(|product_id| delete_product_at(&path, &product_id))
                        .map(|snapshot| snapshot.products);
                    match result {
                        Ok(products) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&products)
                                .unwrap_or_else(|_| "[]".to_string())
                        ),
                        Ok(products) => print_human_products(&products),
                        Err(error) => {
                            eprintln!("Pronto could not delete product: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!("Usage: pronto product list [--json] | pronto product create <name> --release-mode <mode> [--repo <id>]... [--json] | pronto product append <product> --repo <id>... [--json] | pronto product update <product> <name> --release-mode <mode> [--repo <id>]... [--json] | pronto product delete <product> [--json]");
                    std::process::exit(2);
                }
            }
        }
        "workspace" => {
            let tool = cli_option(&arguments, "--tool")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .unwrap_or_else(|| {
                    eprintln!("Usage: pronto workspace open <repository> <workspace> --tool <tool> [--json]");
                    std::process::exit(2);
                });
            let positionals = cli_positionals(&arguments, &["--tool"]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() != 3 || positionals[0] != "open" {
                eprintln!(
                    "Usage: pronto workspace open <repository> <workspace> --tool <tool> [--json]"
                );
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|state| {
                let snapshot = snapshot_from_store(&path, &state);
                let repository = find_cli_repository(&snapshot, &positionals[1])?;
                let repository_id = repository.id.clone();
                open_workspace_at(&path, &repository_id, &positionals[2], &tool)
            });
            match result {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(_) => println!("Opened workspace {}.", positionals[2]),
                Err(error) => {
                    eprintln!("Pronto could not open the workspace: {error}");
                    std::process::exit(1);
                }
            }
        }
        "preflight" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if !(1..=2).contains(&positionals.len()) {
                eprintln!("Usage: pronto preflight <action> [<repository>] [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|state| {
                let repository_id = positionals
                    .get(1)
                    .map(|query| {
                        let snapshot = snapshot_from_store(&path, &state);
                        find_cli_repository(&snapshot, query)
                            .map(|repository| repository.id.clone())
                    })
                    .transpose()?;
                preflight_action_at(&path, &positionals[0], repository_id.as_deref())
            });
            match result {
                Ok(preflight) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&preflight).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(preflight) => println!(
                    "PRONTO PREFLIGHT · {} · {}",
                    preflight.target_label,
                    if preflight.allowed {
                        "allowed"
                    } else {
                        "blocked"
                    }
                ),
                Err(error) => {
                    eprintln!("Pronto could not preflight the action: {error}");
                    std::process::exit(1);
                }
            }
        }
        "condition" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() != 3 || !matches!(positionals[0].as_str(), "expect" | "clear") {
                eprintln!("Usage: pronto condition expect|clear <repository> <condition> [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|state| {
                let snapshot = snapshot_from_store(&path, &state);
                let repository = find_cli_repository(&snapshot, &positionals[1])?;
                if positionals[0] == "expect" {
                    mutate_expected(&path, &repository.id, &positionals[2], true)
                } else {
                    mutate_expected(&path, &repository.id, &positionals[2], false)
                }
            });
            match result {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(_) => println!(
                    "Condition {}.",
                    if positionals[0] == "expect" {
                        "marked expected"
                    } else {
                        "cleared"
                    }
                ),
                Err(error) => {
                    eprintln!("Pronto could not update the condition: {error}");
                    std::process::exit(1);
                }
            }
        }
        "settings" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() != 2 || positionals[0] != "retention" {
                eprintln!("Usage: pronto settings retention <days> [--json]");
                std::process::exit(2);
            }
            let retention_days = positionals[1].parse::<i64>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: retention days must be an integer");
                std::process::exit(2);
            });
            match set_retention_days_at(&path, retention_days) {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => println!("Retention: {} days", snapshot.retention_days),
                Err(error) => {
                    eprintln!("Pronto could not update settings: {error}");
                    std::process::exit(1);
                }
            }
        }
        "status" => {
            let fresh = arguments.iter().any(|argument| argument == "--fresh");
            let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let positionals =
                cli_positionals_with_flags(&arguments, &["--product", "--group"], &["--fresh"])
                    .unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    });
            if !positionals.is_empty() {
                eprintln!(
                    "Usage: pronto status [--product <name> | --group <name>] [--fresh] [--json]"
                );
                std::process::exit(2);
            }
            let state_result = if fresh {
                load_store_read_only_with_quality_bounded(&path)
            } else {
                load_store_read_only(&path)
            };
            let result = state_result
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                });
            match result {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => print_human_status(&snapshot),
                Err(error) => {
                    if json {
                        print_cli_json_error("status", &error);
                    }
                    eprintln!("Pronto could not read local state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "summary" => {
            let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let positionals = cli_positionals(&arguments, &["--product", "--group"])
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if !positionals.is_empty() {
                eprintln!("Usage: pronto summary [--product <name> | --group <name>] [--json]");
                std::process::exit(2);
            }
            let scope = product_name
                .as_deref()
                .map(|value| format!("product:{value}"))
                .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
                .unwrap_or_else(|| "fleet".to_string());
            let result = load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                })
                .map(|snapshot| agent_summary(&snapshot, &scope));
            match result {
                Ok(summary) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(summary) => print_human_summary(&summary),
                Err(error) => {
                    eprintln!("Pronto could not read local state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "doctor" => {
            let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let max_age_minutes = cli_option(&arguments, "--max-age")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    let parsed = value.parse::<i64>().unwrap_or_else(|_| {
                        eprintln!(
                            "Pronto CLI error: --max-age must be a non-negative integer"
                        );
                        std::process::exit(2);
                    });
                    if parsed < 0 {
                        eprintln!(
                            "Pronto CLI error: --max-age must be a non-negative integer"
                        );
                        std::process::exit(2);
                    }
                    if parsed > MAX_AGENT_DOCTOR_MAX_AGE_MINUTES {
                        eprintln!(
                            "Pronto CLI error: --max-age must be {MAX_AGENT_DOCTOR_MAX_AGE_MINUTES} or less"
                        );
                        std::process::exit(2);
                    }
                    parsed
                })
                .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES);
            let positionals = cli_positionals(&arguments, &["--max-age", "--product", "--group"])
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if positionals.len() > 1 {
                eprintln!(
                    "Usage: pronto doctor [<repository>] [--product <name> | --group <name>] [--max-age <minutes>] [--json]"
                );
                std::process::exit(2);
            }
            let query = positionals.first().map(String::as_str);
            let base_scope = product_name
                .as_deref()
                .map(|value| format!("product:{value}"))
                .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
                .unwrap_or_else(|| "fleet".to_string());
            let scope = query
                .map(|value| format!("{base_scope}; current_repository:{value}"))
                .unwrap_or(base_scope);
            let report = match load_store_read_only(&path) {
                Ok(state) => {
                    let snapshot = snapshot_from_store(&path, &state);
                    let scoped_snapshot = filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                    .and_then(|snapshot| {
                        if let Some(query) = query {
                            let repository_id = find_cli_repository(&snapshot, query)?.id.clone();
                            let repository_ids = [repository_id].into_iter().collect();
                            Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
                        } else {
                            Ok(snapshot)
                        }
                    });
                    match scoped_snapshot {
                        Ok(snapshot) => {
                            agent_doctor_report(&snapshot, &path, max_age_minutes, &scope)
                        }
                        Err(error) => agent_doctor_error_report(
                            &path,
                            max_age_minutes,
                            &scope,
                            "scope",
                            error,
                        ),
                    }
                }
                Err(error) => {
                    agent_doctor_error_report(&path, max_age_minutes, &scope, "storage", error)
                }
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_human_doctor(&report);
            }
            if !report.ready {
                std::process::exit(1);
            }
        }
        "next" => {
            let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let limit = cli_option(&arguments, "--limit")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    let parsed = value.parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                        std::process::exit(2);
                    });
                    if parsed > MAX_AGENT_NEXT_LIMIT {
                        eprintln!(
                            "Pronto CLI error: --limit must be {MAX_AGENT_NEXT_LIMIT} or less"
                        );
                        std::process::exit(2);
                    }
                    parsed
                })
                .unwrap_or(DEFAULT_AGENT_NEXT_LIMIT);
            let positionals = cli_positionals(&arguments, &["--product", "--group", "--limit"])
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if positionals.len() > 1 {
                eprintln!(
                    "Usage: pronto next [<repository>] [--product <name> | --group <name>] [--limit <n>] [--json]"
                );
                std::process::exit(2);
            }
            let query = positionals.first().map(String::as_str);
            let base_scope = product_name
                .as_deref()
                .map(|value| format!("product:{value}"))
                .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
                .unwrap_or_else(|| "fleet".to_string());
            let scope = query
                .map(|value| format!("{base_scope}; current_repository:{value}"))
                .unwrap_or(base_scope);
            let result = load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                })
                .and_then(|snapshot| agent_next_report(&snapshot, query, &scope, limit));
            match result {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_next(&report),
                Err(error) => {
                    eprintln!("Pronto could not read next-step state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "fold" => {
            if arguments.get(1).map(String::as_str) != Some("preview") {
                eprintln!(
                    "Usage: pronto fold preview [<repository>] [--target <branch>] [--product <name> | --group <name>] [--limit <n>] [--json]"
                );
                std::process::exit(2);
            }
            let command_arguments = &arguments[1..];
            let target = cli_option(command_arguments, "--target").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if target
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            {
                eprintln!("Pronto CLI error: --target requires a non-empty branch name");
                std::process::exit(2);
            }
            let product_name = cli_option(command_arguments, "--product").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let group_name = cli_option(command_arguments, "--group").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let limit = cli_option(command_arguments, "--limit")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    let parsed = value.parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                        std::process::exit(2);
                    });
                    if parsed > MAX_AGENT_FOLD_PREVIEW_LIMIT {
                        eprintln!(
                            "Pronto CLI error: --limit must be {MAX_AGENT_FOLD_PREVIEW_LIMIT} or less"
                        );
                        std::process::exit(2);
                    }
                    parsed
                })
                .unwrap_or(DEFAULT_AGENT_FOLD_PREVIEW_LIMIT);
            let positionals = cli_positionals(
                command_arguments,
                &["--target", "--product", "--group", "--limit"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!(
                    "Usage: pronto fold preview [<repository>] [--target <branch>] [--product <name> | --group <name>] [--limit <n>] [--json]"
                );
                std::process::exit(2);
            }
            let query = positionals.first().map(String::as_str);
            let base_scope = product_name
                .as_deref()
                .map(|value| format!("product:{value}"))
                .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
                .unwrap_or_else(|| "fleet".to_string());
            let scope = query
                .map(|value| format!("{base_scope}; current_repository:{value}"))
                .unwrap_or(base_scope);
            let result = load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    filter_snapshot_by_collection(
                        snapshot,
                        product_name.as_deref(),
                        group_name.as_deref(),
                    )
                })
                .and_then(|snapshot| {
                    agent_fold_preview_report(&snapshot, query, target.as_deref(), &scope, limit)
                });
            match result {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_fold_preview(&report),
                Err(error) => {
                    eprintln!("Pronto could not read fold preview state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "repo" => {
            let fresh = arguments.iter().any(|argument| argument == "--fresh");
            let positionals = cli_positionals_with_flags(
                &arguments,
                &["--rule-json", "--recipe-json", "--workspace"],
                &["--clear", "--fresh"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.first().map(String::as_str) == Some("set-lifecycle")
                && positionals.len() == 3
            {
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    set_repository_lifecycle_at(&path, &repository.id, &positionals[2])
                });
                match result {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("Repository lifecycle updated."),
                    Err(error) => {
                        eprintln!("Pronto could not update repository lifecycle: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            if positionals.first().map(String::as_str) == Some("set-release-rule")
                && positionals.len() == 2
            {
                let release_rule = if arguments.iter().any(|argument| argument == "--clear") {
                    None
                } else {
                    cli_json_option::<ReleaseRuleConfig>(&arguments, "--rule-json").unwrap_or_else(
                        |error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        },
                    )
                };
                if release_rule.is_none() && !arguments.iter().any(|argument| argument == "--clear")
                {
                    eprintln!("Usage: pronto repo set-release-rule <repository> --rule-json <json|@file> [--json]");
                    std::process::exit(2);
                }
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    set_release_rule_at(&path, &repository.id, release_rule)
                });
                match result {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("Release rule updated."),
                    Err(error) => {
                        eprintln!("Pronto could not update the release rule: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            if positionals.first().map(String::as_str) == Some("set-release-recipe")
                && positionals.len() == 2
            {
                let release_recipe = if arguments.iter().any(|argument| argument == "--clear") {
                    None
                } else {
                    cli_json_option::<ReleaseRecipeConfig>(&arguments, "--recipe-json")
                        .unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        })
                };
                if release_recipe.is_none()
                    && !arguments.iter().any(|argument| argument == "--clear")
                {
                    eprintln!("Usage: pronto repo set-release-recipe <repository> --recipe-json <json|@file> [--json]");
                    std::process::exit(2);
                }
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    set_release_recipe_at(&path, &repository.id, release_recipe)
                });
                match result {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("Release recipe updated."),
                    Err(error) => {
                        eprintln!("Pronto could not update the release recipe: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            if positionals.first().map(String::as_str) == Some("set-release-version")
                && (positionals.len() == 2 || positionals.len() == 3)
            {
                let release_version = if arguments.iter().any(|argument| argument == "--clear") {
                    None
                } else {
                    positionals.get(2).cloned()
                };
                if release_version.is_none()
                    && !arguments.iter().any(|argument| argument == "--clear")
                {
                    eprintln!("Usage: pronto repo set-release-version <repository> <version> [--json] | ... <repository> --clear [--json]");
                    std::process::exit(2);
                }
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    set_release_version_at(&path, &repository.id, release_version)
                });
                match result {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("Release version updated."),
                    Err(error) => {
                        eprintln!("Pronto could not update the release version: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            if positionals.first().map(String::as_str) == Some("set-ai-permission")
                && positionals.len() == 3
            {
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    set_ai_permission_at(&path, &repository.id, &positionals[2])
                });
                match result {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("AI permission updated."),
                    Err(error) => {
                        eprintln!("Pronto could not update AI permission: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            if positionals.first().map(String::as_str) == Some("preview-ai-summary")
                && positionals.len() == 2
            {
                let workspace_id = cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
                let result = load_store(&path).and_then(|state| {
                    let snapshot = snapshot_from_store(&path, &state);
                    let repository = find_cli_repository(&snapshot, &positionals[1])?;
                    preview_ai_summary_at(&path, &repository.id, workspace_id.as_deref())
                });
                match result {
                    Ok(preview) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&preview).unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(preview) => {
                        println!("AI summary: {} · {}", preview.status, preview.payload_bytes)
                    }
                    Err(error) => {
                        eprintln!("Pronto could not preview the AI summary: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            let Some(query) = positionals.first() else {
                eprintln!("Usage: pronto repo <repository> [--fresh] [--json]");
                std::process::exit(2);
            };
            if positionals.len() > 1 {
                eprintln!("Usage: pronto repo <repository> [--fresh] [--json]");
                std::process::exit(2);
            }
            let state_result = if fresh {
                load_store_read_only_with_quality_bounded(&path)
            } else {
                load_store_read_only(&path)
            };
            let result = state_result
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    let repository = find_cli_repository(&snapshot, query)?;
                    Ok(agent_repository_detail(&snapshot, repository))
                });
            match result {
                Ok(detail) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&detail).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(detail) => print_human_repository(&detail),
                Err(error) => {
                    if json {
                        print_cli_json_error("repo", &error);
                    }
                    eprintln!("Pronto could not read repository state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "attention" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if !positionals.is_empty() {
                eprintln!("Usage: pronto attention [--json]");
                std::process::exit(2);
            }
            match load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .map(|snapshot| agent_attention_report(&snapshot))
            {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_attention(&report),
                Err(error) => {
                    eprintln!("Pronto could not read attention state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "activity" => {
            let limit = cli_option(&arguments, "--limit")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .map(|value| {
                    value.parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                        std::process::exit(2);
                    })
                })
                .unwrap_or(24);
            let positionals = cli_positionals(&arguments, &["--limit"]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!("Usage: pronto activity [<repository>] [--limit <n>] [--json]");
                std::process::exit(2);
            }
            let query = positionals.first().map(String::as_str);
            let result = load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| agent_activity_report(&snapshot, query, limit));
            match result {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_activity(&report),
                Err(error) => {
                    eprintln!("Pronto could not read activity: {error}");
                    std::process::exit(1);
                }
            }
        }
        "prepare" => {
            let workspace_id = cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let positionals =
                cli_positionals(&arguments, &["--workspace"]).unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            let Some(query) = positionals.first() else {
                eprintln!("Usage: pronto prepare <repository> [--workspace <id>] [--json]");
                std::process::exit(2);
            };
            if positionals.len() > 1 {
                eprintln!("Usage: pronto prepare <repository> [--workspace <id>] [--json]");
                std::process::exit(2);
            }
            let result = load_store_with_quality(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    let repository = find_cli_repository(&snapshot, query)?;
                    prepare_repository_at(&path, &repository.id, workspace_id.as_deref())
                })
                .map(|preparation| AgentPreparationReport {
                    schema_version: AGENT_PREPARATION_SCHEMA.to_string(),
                    generated_at: preparation.generated_at.clone(),
                    preparation,
                });
            match result {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_preparation(&report),
                Err(error) => {
                    eprintln!("Pronto could not prepare the repository: {error}");
                    std::process::exit(1);
                }
            }
        }
        "release" => {
            let workspace_id = cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let positionals =
                cli_positionals(&arguments, &["--workspace"]).unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if positionals.len() != 2 || positionals[0] != "preview" {
                eprintln!("Usage: pronto release preview <repository> [--workspace <id>] [--json]");
                std::process::exit(2);
            }
            let query = &positionals[1];
            let result = load_store_with_quality(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    let repository = find_cli_repository(&snapshot, query)?;
                    prepare_repository_at(&path, &repository.id, workspace_id.as_deref())
                })
                .map(|preparation| AgentReleaseReport {
                    schema_version: AGENT_RELEASE_SCHEMA.to_string(),
                    generated_at: preparation.generated_at.clone(),
                    repository_id: preparation.repository_id,
                    release: preparation.release,
                    recipe: preparation.recipe,
                });
            match result {
                Ok(report) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(report) => print_human_release(&report),
                Err(error) => {
                    eprintln!("Pronto could not prepare the release preview: {error}");
                    std::process::exit(1);
                }
            }
        }
        "remediation" => {
            let positionals = cli_positionals_with_flags(
                &arguments,
                &["--qr-bin", "--notes", "--timeout-seconds", "--workspace"],
                &["--dynamic", "--no-changed-only", "--skip-provider"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            match positionals.first().map(String::as_str) {
                Some("handoff-check") => {
                    if positionals.len() != 2 {
                        eprintln!(
                            "Usage: pronto remediation handoff-check <repository> [--workspace <id>] [--json]"
                        );
                        std::process::exit(2);
                    }
                    let workspace_id =
                        cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        });
                    match remediation_handoff_check_at(
                        &path,
                        &positionals[1],
                        workspace_id.as_deref(),
                    ) {
                        Ok(check) if json => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&check)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                            if !check.ready {
                                std::process::exit(1);
                            }
                        }
                        Ok(check) => {
                            print_human_remediation_handoff_check(&check);
                            if !check.ready {
                                std::process::exit(1);
                            }
                        }
                        Err(error) => {
                            eprintln!("Pronto could not check the remediation handoff: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("refresh") => {
                    if positionals.len() != 1 {
                        eprintln!("Usage: pronto remediation refresh [--qr-bin <path>] [--dynamic] [--no-changed-only] [--timeout-seconds <positive-integer>] [--skip-provider] [--json]");
                        std::process::exit(2);
                    }
                    let qr_bin = cli_option(&arguments, "--qr-bin").unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    });
                    let timeout_seconds = cli_positive_u64_option(&arguments, "--timeout-seconds")
                        .unwrap_or_else(|error| {
                            eprintln!("Pronto CLI error: {error}");
                            std::process::exit(2);
                        })
                        .unwrap_or(DEFAULT_QR_AUDIT_TIMEOUT_SECONDS);
                    let result = refresh_remediation_at(
                        &path,
                        qr_bin.as_deref(),
                        arguments.iter().any(|argument| argument == "--dynamic"),
                        !arguments
                            .iter()
                            .any(|argument| argument == "--no-changed-only"),
                        arguments
                            .iter()
                            .any(|argument| argument == "--skip-provider"),
                        timeout_seconds,
                    );
                    match result {
                        Ok(snapshot) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&snapshot.remediation)
                                .unwrap_or_else(|_| "{}".to_string())
                        ),
                        Ok(snapshot) => print_human_remediation(&snapshot.remediation),
                        Err(error) => {
                            eprintln!("Pronto could not refresh remediation evidence: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("export") => {
                    if positionals.len() > 2 {
                        eprintln!("Usage: pronto remediation export [output-dir] [--json]");
                        std::process::exit(2);
                    }
                    let output_dir = positionals.get(1).cloned();
                    match export_remediation(output_dir) {
                        Ok(export) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&export)
                                .unwrap_or_else(|_| "{}".to_string())
                        ),
                        Ok(export) => println!(
                            "Remediation export: {} · {} files",
                            export.output_path,
                            export.files.len()
                        ),
                        Err(error) => {
                            eprintln!("Pronto could not export remediation plans: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                Some("set-status") => {
                    if positionals.len() != 3 {
                        eprintln!("Usage: pronto remediation set-status <action-id> <status> [--notes <text>] [--json]");
                        std::process::exit(2);
                    }
                    let notes = cli_option(&arguments, "--notes").unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    });
                    match set_remediation_action_status(
                        positionals[1].clone(),
                        positionals[2].clone(),
                        notes,
                    ) {
                        Ok(snapshot) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&snapshot.remediation)
                                .unwrap_or_else(|_| "{}".to_string())
                        ),
                        Ok(snapshot) => print_human_remediation(&snapshot.remediation),
                        Err(error) => {
                            eprintln!("Pronto could not update remediation status: {error}");
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    if positionals.len() > 1 {
                        eprintln!("Usage: pronto remediation [<repository>] [--json]");
                        std::process::exit(2);
                    }
                    let result = load_store_read_only(&path).and_then(|state| {
                        let snapshot = snapshot_from_store(&path, &state);
                        if let Some(query) = positionals.first() {
                            let plan = snapshot
                                .remediation
                                .plans
                                .iter()
                                .find(|plan| {
                                    plan.repository_id == *query
                                        || plan.repository_name.eq_ignore_ascii_case(query)
                                        || plan.repository_path == *query
                                })
                                .cloned();
                            let closures = snapshot
                                .remediation
                                .closures
                                .iter()
                                .filter(|closure| {
                                    closure.repository_id == *query
                                        || closure.repository_name.eq_ignore_ascii_case(query)
                                        || closure.repository_path == *query
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            if plan.is_none() && closures.is_empty() {
                                return Err(format!(
                                    "No active remediation plan or retained closure found for repository '{query}'."
                                ));
                            }
                            let mut run = snapshot.remediation;
                            run.plans = plan.into_iter().collect();
                            run.closures = closures;
                            Ok(run)
                        } else {
                            Ok(snapshot.remediation)
                        }
                    });
                    match result {
                        Ok(run) if json => println!(
                            "{}",
                            serde_json::to_string_pretty(&run).unwrap_or_else(|_| "{}".to_string())
                        ),
                        Ok(run) => print_human_remediation(&run),
                        Err(error) => {
                            eprintln!("Pronto could not read remediation plans: {error}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        "refresh" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!(
                    "Usage: pronto refresh [repository|group|product|repository-path] [--json]"
                );
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|mut state| {
                if let Some(target) = positionals.first() {
                    let current = snapshot_from_store(&path, &state);
                    match resolve_local_refresh_target(&current, &state, target)? {
                        LocalRefreshTarget::Registered {
                            repository_ids,
                            label,
                        } => {
                            let snapshot = audited_scan_and_persist_scoped(
                                &path,
                                &mut state,
                                Some(&repository_ids),
                                Some(&label),
                            )?;
                            Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
                        }
                        LocalRefreshTarget::RepositoryPath(repository_path) => {
                            let repository_id = path_id("repository", &repository_path);
                            let snapshot = audited_scan_and_persist_repository_path(
                                &path,
                                &mut state,
                                &repository_path,
                            )?;
                            let repository_ids = [repository_id].into_iter().collect();
                            Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
                        }
                    }
                } else {
                    audited_scan_and_persist(&path, &mut state)
                }
            });
            match result {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => print_human_status(&snapshot),
                Err(error) => {
                    eprintln!("Pronto could not refresh local state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "refresh-github" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!("Usage: pronto refresh-github [repository|group|product] [--json]");
                std::process::exit(2);
            }
            let result = if let Some(target) = positionals.first() {
                load_store(&path).and_then(|state| {
                    let current = snapshot_from_store(&path, &state);
                    let (repository_ids, _) = resolve_refresh_target(&current, target)?;
                    refresh_github_scoped_at(&path, &repository_ids).map(|snapshot| {
                        filter_snapshot_to_repository_ids(snapshot, &repository_ids)
                    })
                })
            } else {
                refresh_github_at(&path)
            };
            match result {
                Ok(snapshot) if json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    );
                    if snapshot.provider_status.state != "Ready" {
                        std::process::exit(1);
                    }
                }
                Ok(snapshot) if snapshot.provider_status.state == "Ready" => {
                    println!(
                        "GitHub provider: {} · {}",
                        snapshot.provider_status.state, snapshot.provider_status.message
                    );
                    print_human_status(&snapshot);
                }
                Ok(snapshot) => {
                    eprintln!(
                        "Pronto GitHub refresh unavailable: {}",
                        snapshot.provider_status.message
                    );
                    std::process::exit(1);
                }
                Err(error) => {
                    eprintln!("Pronto could not refresh GitHub: {error}");
                    std::process::exit(1);
                }
            }
        }
        "quality" => {
            let positionals = cli_positionals(
                &arguments,
                &["--reason", "--reviewer", "--evidence", "--expires-at"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.first().map(String::as_str) == Some("refresh") && positionals.len() == 1
            {
                match refresh_quality_at(&path) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => print_human_status(&snapshot),
                    Err(error) => {
                        if json {
                            print_cli_json_error("quality refresh", &error);
                        }
                        eprintln!("Pronto could not refresh quality evidence: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            } else if positionals.first().map(String::as_str) == Some("set-audit-root")
                && positionals.len() == 2
            {
                match set_maturity_audit_root_at(&path, Some(&positionals[1])) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => print_human_status(&snapshot),
                    Err(error) => {
                        eprintln!("Pronto could not set the maturity audit root: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            } else if positionals.first().map(String::as_str) == Some("open-report")
                && positionals.len() == 2
            {
                match open_quality_report_at(&path, &positionals[1]) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(_) => println!("Opened quality report: {}", positionals[1]),
                    Err(error) => {
                        eprintln!("Pronto could not open the quality report: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            } else if positionals.first().map(String::as_str) == Some("disposition") {
                if positionals.get(1).map(String::as_str) != Some("set") || positionals.len() != 5 {
                    eprintln!(
                        "Usage: pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json]"
                    );
                    std::process::exit(2);
                }
                let reason = cli_option(&arguments, "--reason")
                    .unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    })
                    .unwrap_or_else(|| {
                        eprintln!("Pronto CLI error: --reason is required");
                        std::process::exit(2);
                    });
                let reviewer = cli_option(&arguments, "--reviewer")
                    .unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    })
                    .unwrap_or_else(|| {
                        eprintln!("Pronto CLI error: --reviewer is required");
                        std::process::exit(2);
                    });
                let evidence =
                    cli_repeated_option(&arguments, "--evidence").unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    });
                let expires_at = cli_option(&arguments, "--expires-at").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
                let query = &positionals[2];
                let result = load_store(&path)
                    .map(|state| snapshot_from_store(&path, &state))
                    .and_then(|snapshot| {
                        let repository = find_cli_repository(&snapshot, query)?;
                        quality::set_finding_disposition(
                            Path::new(&repository.path),
                            &positionals[3],
                            &positionals[4],
                            &reason,
                            &reviewer,
                            evidence,
                            expires_at,
                        )?;
                        load_store_with_quality(&path)
                            .map(|state| snapshot_from_store(&path, &state))
                            .and_then(|snapshot| agent_quality_report(&snapshot, Some(query)))
                    });
                match result {
                    Ok(report) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(report) => print_human_quality(&report),
                    Err(error) => {
                        eprintln!("Pronto could not update the finding disposition: {error}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            let is_feed_command =
                positionals.len() == 1 && matches!(positionals[0].as_str(), "feed" | "audit-root");
            if is_feed_command {
                match load_store_read_only(&path).map(|state| snapshot_from_store(&path, &state)) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => {
                        println!(
                            "Maturity feed: {}",
                            snapshot
                                .quality
                                .audit_root
                                .as_deref()
                                .unwrap_or("Unavailable")
                        );
                        println!(
                            "Maturity feed: {} · {} matched · fleet mean {}",
                            snapshot.quality.audit_status,
                            snapshot.quality.matched_repository_count,
                            snapshot
                                .quality
                                .maturity_score_display
                                .as_deref()
                                .map(|value| format!("{value} / 4"))
                                .unwrap_or_else(|| "Not scored".to_string())
                        );
                    }
                    Err(error) => {
                        eprintln!("Pronto could not read the maturity feed: {error}");
                        std::process::exit(1);
                    }
                }
            } else if positionals.len() <= 1 {
                let query = positionals.first().map(String::as_str);
                let result = load_store_read_only(&path)
                    .map(|state| snapshot_from_store(&path, &state))
                    .and_then(|snapshot| agent_quality_report(&snapshot, query));
                match result {
                    Ok(report) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(report) => print_human_quality(&report),
                    Err(error) => {
                        eprintln!("Pronto could not read quality state: {error}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!(
                    "Usage: pronto quality [<repository>] [--json] | pronto quality refresh [--json] | pronto quality feed [--json] | pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json] (Quality Runner owns detector evidence; Pronto owns the disposition overlay)"
                );
                std::process::exit(2);
            }
        }
        "root" => {
            let positionals = cli_positionals(
                &arguments,
                &["--ignore", "--refresh-policy", "--background-monitoring"],
            )
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() == 2 && positionals[0] == "settings" {
                let refresh_policy = cli_option(&arguments, "--refresh-policy")
                    .unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    })
                    .unwrap_or_else(|| {
                        eprintln!("Usage: pronto root settings <root-id> [--ignore <pattern>]... --refresh-policy <policy> --background-monitoring <bool> [--json]");
                        std::process::exit(2);
                    });
                let background_monitoring = cli_bool_option(&arguments, "--background-monitoring")
                    .unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    })
                    .unwrap_or_else(|| {
                        eprintln!("Usage: pronto root settings <root-id> [--ignore <pattern>]... --refresh-policy <policy> --background-monitoring <bool> [--json]");
                        std::process::exit(2);
                    });
                let ignore_patterns =
                    cli_repeated_option(&arguments, "--ignore").unwrap_or_else(|error| {
                        eprintln!("Pronto CLI error: {error}");
                        std::process::exit(2);
                    });
                match update_root_settings_at(
                    &path,
                    &positionals[1],
                    ignore_patterns,
                    &refresh_policy,
                    background_monitoring,
                ) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => print_human_status(&snapshot),
                    Err(error) => {
                        eprintln!("Pronto could not update root settings: {error}");
                        std::process::exit(1);
                    }
                }
            } else if positionals.len() == 2 && positionals[0] == "add" {
                let root_path = &positionals[1];
                match register_root_and_scan(&path, root_path) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => {
                        println!("Configured discovery root: {root_path}");
                        print_human_status(&snapshot);
                    }
                    Err(error) => {
                        eprintln!("Pronto could not configure the discovery root: {error}");
                        std::process::exit(1);
                    }
                }
            } else if positionals.len() >= 3 && positionals[0] == "exclude" {
                let root_path = &positionals[1];
                let patterns = positionals[2..].to_vec();
                match exclude_root_patterns_at(&path, root_path, patterns.clone()) {
                    Ok(snapshot) if json => println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .unwrap_or_else(|_| "{}".to_string())
                    ),
                    Ok(snapshot) => {
                        println!(
                            "Excluded {} from discovery under {}.",
                            patterns.join(", "),
                            root_path
                        );
                        print_human_status(&snapshot);
                    }
                    Err(error) => {
                        eprintln!("Pronto could not exclude those discovery folders: {error}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!(
                    "Usage: pronto root add <folder> [--json] | pronto root exclude <folder> <name>... [--json]"
                );
                std::process::exit(2);
            }
        }
        "clone" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let Some(remote) = positionals.first() else {
                eprintln!("Usage: pronto clone <owner/repository> [--json]");
                std::process::exit(2);
            };
            if positionals.len() > 1 {
                eprintln!("Usage: pronto clone <owner/repository> [--json]");
                std::process::exit(2);
            }
            let result = preflight_action_at(&path, "clone", None);
            match result {
                Ok(preflight) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&preflight)
                        .unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(_) => println!(
                    "Clone of {remote} is blocked by the current local-only action policy; use the desktop confirmation flow when provider access is enabled."
                ),
                Err(error) => {
                    eprintln!("Pronto could not record the clone boundary: {error}");
                    std::process::exit(1);
                }
            }
        }
        "." => {
            let snapshot = load_store(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .unwrap_or_else(|_| PortfolioSnapshot {
                    roots: Vec::new(),
                    repositories: Vec::new(),
                    products: Vec::new(),
                    groups: Vec::new(),
                    events: Vec::new(),
                    action_audits: Vec::new(),
                    provider_identities: Vec::new(),
                    remote_repositories: Vec::new(),
                    provider_status: ProviderStatus::default(),
                    quality: QualityPortfolioSnapshot::default(),
                    remediation: remediation::empty_run(),
                    retention_days: DEFAULT_RETENTION_DAYS,
                    generated_at: iso_now(),
                    storage_path: path.to_string_lossy().to_string(),
                });
            let repository = find_repository_for_directory(
                &snapshot,
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            );
            if let Err(error) = launch_desktop_focus(repository) {
                eprintln!("Pronto could not open the desktop app: {error}");
                std::process::exit(1);
            }
        }
        "open" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let Some(query) = positionals.first() else {
                eprintln!("Usage: pronto open <repository>");
                std::process::exit(2);
            };
            if positionals.len() > 1 {
                eprintln!("Usage: pronto open <repository>");
                std::process::exit(2);
            }
            let result = load_store(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    let repository = find_cli_repository(&snapshot, query)?;
                    launch_desktop_focus(Some(repository))
                });
            if let Err(error) = result {
                eprintln!("Pronto could not open the desktop app: {error}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {command}");
            print_cli_usage();
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Output;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    fn git(path: &Path, arguments: &[&str]) -> Output {
        let output = git_process(path)
            .args(arguments)
            .output()
            .expect("git should be installed for Pronto core tests");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn fixture_root() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-core-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("test root should be creatable");
        root
    }

    fn fixture_repository(root: &Path) -> PathBuf {
        fixture_repository_named(root, "portfolio-repository")
    }

    fn fixture_repository_named(root: &Path, name: &str) -> PathBuf {
        let repository = root.join(name);
        fs::create_dir_all(&repository).expect("fixture repository should be creatable");
        git(&repository, &["init", "-b", "main"]);
        git(
            &repository,
            &["config", "user.email", "pronto-tests@example.com"],
        );
        git(&repository, &["config", "user.name", "Pronto Tests"]);
        fs::write(repository.join("tracked.txt"), "one\n")
            .expect("tracked file should be writable");
        git(&repository, &["add", "tracked.txt"]);
        git(&repository, &["commit", "-m", "Initial fixture"]);
        repository
    }

    #[test]
    fn target_qr_detached_head_provenance_is_rewritten_to_selected_branch() {
        let root = fixture_root();
        let run = root.join("run");
        fs::create_dir_all(&run).expect("target QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "git": { "branch": "HEAD", "ref": "refs/heads/HEAD" },
                "provenance": { "branch": "HEAD" }
            })
            .to_string(),
        )
        .expect("target QR manifest should be writable");

        assert_eq!(
            rewrite_target_qr_branch_provenance(&run, "dev")
                .expect("target QR provenance should be rewritten"),
            1
        );
        let payload: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(run.join("run-manifest.json"))
                .expect("rewritten target QR manifest should be readable"),
        )
        .expect("rewritten target QR manifest should remain JSON");
        assert_eq!(payload["git"]["branch"], "dev");
        assert_eq!(payload["git"]["ref"], "refs/heads/dev");
        assert_eq!(payload["provenance"]["branch"], "dev");

        fs::remove_dir_all(root).expect("target QR fixture should be removable");
    }

    #[test]
    fn verification_accepts_intentionally_deferred_terminal_dependencies() {
        assert!(remediation_dependencies_are_terminal(
            ["verified", "deferred"].into_iter()
        ));
        assert!(!remediation_dependencies_are_terminal(
            ["verified", "open"].into_iter()
        ));
        assert!(!remediation_dependencies_are_terminal(
            ["verified", "blocked"].into_iter()
        ));
    }

    #[test]
    fn registers_root_and_scans_from_cli_path() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("cli root registration should scan the folder");

        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(
            snapshot.roots[0].path,
            canonical_path(&root)
                .expect("fixture root should be canonical")
                .to_string_lossy()
        );
        assert_eq!(snapshot.repositories.len(), 1);
        assert_eq!(
            snapshot.repositories[0].path,
            canonical_path(&repository)
                .expect("fixture repository should be canonical")
                .to_string_lossy()
        );

        fs::remove_dir_all(root).expect("cli root fixture should be removable");
    }

    #[test]
    fn scoped_refresh_admits_unregistered_repository_path_under_registered_root() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: vec!["portfolio-repository".to_string()],
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        save_store(&store, &state).expect("unregistered fixture state should persist");

        let before = snapshot_from_store(&store, &state);
        let target = resolve_local_refresh_target(&before, &state, &repository.to_string_lossy())
            .expect("repository path should resolve under the registered root");
        let repository_path = match target {
            LocalRefreshTarget::RepositoryPath(path) => path,
            LocalRefreshTarget::Registered { .. } => {
                panic!("an unregistered repository path should not resolve as registered")
            }
        };
        let refreshed =
            audited_scan_and_persist_repository_path(&store, &mut state, &repository_path)
                .expect("scoped repository-path refresh should admit the repository");

        assert_eq!(refreshed.repositories.len(), 1);
        assert_eq!(
            refreshed.repositories[0].path,
            repository_path.to_string_lossy()
        );
        assert_eq!(refreshed.action_audits.len(), 1);
        assert_eq!(refreshed.action_audits[0].status, "Completed");
        assert_eq!(
            refreshed.action_audits[0].target_ids,
            vec![path_id("repository", &repository_path)]
        );
        assert_eq!(refreshed.roots.len(), 1);

        fs::remove_dir_all(root).expect("scoped refresh fixture should be removable");
    }

    #[test]
    fn scoped_refresh_rejects_repository_path_outside_registered_root() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let other_root = fixture_root();
        let other_repository = fixture_repository(&other_root);
        let store = root.join("registry.db");
        let state = StoreState {
            roots: vec![RootConfig {
                id: path_id("root", &root),
                path: root.to_string_lossy().to_string(),
                label: "fixture".to_string(),
                ignore_patterns: Vec::new(),
                refresh_policy: default_refresh_policy(),
                background_monitoring: false,
                registered_at: iso_now(),
            }],
            ..StoreState::default()
        };
        save_store(&store, &state).expect("root-only fixture state should persist");
        let snapshot = snapshot_from_store(&store, &state);

        let error =
            resolve_local_refresh_target(&snapshot, &state, &other_repository.to_string_lossy())
                .expect_err("a path outside the registered root should be rejected");
        assert!(error.contains("not covered by a registered discovery root"));

        fs::remove_dir_all(root).expect("root fixture should be removable");
        fs::remove_dir_all(other_root).expect("outside fixture should be removable");
        let _ = repository;
    }

    #[test]
    fn ordinary_refresh_retains_persisted_scoped_maturity_evidence() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = &snapshot.repositories[0];
        let audit_root = root.join("qr-audit");
        let findings_root = audit_root.join("findings");
        fs::create_dir_all(&findings_root).expect("findings root should be creatable");
        let observed_at = iso_now();
        fs::write(
            findings_root.join("repository.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-scoped",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path,
                    "checkouts": [{
                        "path": repository.workspace.path,
                        "head": repository.workspace.last_commit,
                        "branch": repository.branch
                    }]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands need work.",
                    "priority": "P1",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }]
            }))
            .expect("scoped QR finding should encode"),
        )
        .expect("scoped QR finding should be writable");

        let mut state = load_store(&store).expect("fixture store should reload");
        state.remediation.refresh_steps = vec![remediation::RemediationRefreshStep {
            id: "qr_fleet_run".to_string(),
            label: "Fresh Quality Runner fleet run".to_string(),
            status: "completed".to_string(),
            evidence_path: Some(audit_root.to_string_lossy().to_string()),
            ..remediation::RemediationRefreshStep::default()
        }];
        apply_quality_evidence_scoped(&mut state, None, None);
        assert_eq!(state.repositories[0].quality.maturity.score, Some(2.0));
        assert_eq!(
            state.repositories[0].quality.maturity.freshness,
            QualityFreshness::Fresh
        );
        save_store(&store, &state).expect("scoped maturity state should persist");

        let mut persisted = load_store(&store).expect("scoped maturity state should reload");
        let refreshed = scan_and_persist_scoped(&store, &mut persisted, None)
            .expect("ordinary refresh should succeed");
        assert_eq!(refreshed.repositories[0].quality.maturity.score, Some(2.0));
        assert_eq!(
            refreshed.repositories[0].quality.maturity.freshness,
            QualityFreshness::Fresh
        );

        fs::remove_dir_all(root).expect("scoped maturity fixture should be removable");
    }

    #[test]
    fn read_only_route_load_recovers_persisted_scoped_maturity_evidence() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = &snapshot.repositories[0];
        let audit_root = root.join("qr-audit");
        let findings_root = audit_root.join("findings");
        fs::create_dir_all(&findings_root).expect("findings root should be creatable");
        fs::write(
            findings_root.join("repository.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-route-scoped",
                "as_of": iso_now(),
                "repository": {
                    "primary_path": repository.path,
                    "checkouts": [{
                        "path": repository.workspace.path,
                        "head": repository.workspace.last_commit,
                        "branch": repository.branch
                    }]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-route-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands need work.",
                    "priority": "P1",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }]
            }))
            .expect("scoped QR finding should encode"),
        )
        .expect("scoped QR finding should be writable");

        let mut state = load_store(&store).expect("fixture store should reload");
        state.remediation.refresh_steps = vec![remediation::RemediationRefreshStep {
            id: "qr_fleet_run".to_string(),
            label: "Fresh Quality Runner fleet run".to_string(),
            status: "completed".to_string(),
            evidence_path: Some(audit_root.to_string_lossy().to_string()),
            ..remediation::RemediationRefreshStep::default()
        }];
        save_store(&store, &state).expect("scoped audit provenance should persist");

        let state = load_store_read_only_with_quality(&store)
            .expect("route should hydrate quality without opening the store for writes");
        let snapshot = snapshot_from_store(&store, &state);
        let repository_path = snapshot.repositories[0].path.clone();
        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            &format!("repository:{repository_path}"),
            Some(&repository_path),
            3,
        )
        .expect("route report should build");

        assert_eq!(
            report
                .repository
                .as_ref()
                .and_then(|detail| detail.repository.quality.maturity.score),
            Some(2.0)
        );
        assert_eq!(
            report
                .repository
                .as_ref()
                .map(|detail| &detail.repository.quality.maturity.freshness),
            Some(&QualityFreshness::Fresh)
        );

        fs::remove_dir_all(root).expect("route maturity fixture should be removable");
    }

    #[test]
    fn cached_read_does_not_ingest_quality_artifacts_until_explicit_refresh() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        assert_eq!(
            snapshot.repositories[0].quality.ingestion_status,
            "No evidence"
        );

        let observed_at = iso_now();
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-after-scan");
        fs::create_dir_all(&run).expect("quality run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "created_at": observed_at,
                "git": {
                    "branch": snapshot.repositories[0].branch,
                    "head_sha": snapshot.repositories[0].workspace.last_commit
                }
            })
            .to_string(),
        )
        .expect("quality run manifest should be writable");
        fs::write(
            run.join("gate-verification.json"),
            serde_json::json!({
                "gates": [{
                    "id": "runtime_smoke",
                    "status": "passed",
                    "capability_kind": "local_command",
                    "command": "pnpm smoke",
                    "completed_at": observed_at
                }]
            })
            .to_string(),
        )
        .expect("quality gate evidence should be writable");

        let cached = load_store_read_only(&store).expect("cached store should load");
        assert_eq!(
            cached.repositories[0].quality.ingestion_status, "No evidence",
            "read projections must not rescan quality artifacts"
        );
        let fresh = load_store_read_only_with_quality(&store)
            .expect("explicit fresh quality projection should load");
        assert_eq!(
            fresh.repositories[0].quality.ingestion_status, "Available",
            "the explicit fresh path should see newly-created quality artifacts"
        );

        fs::remove_dir_all(root).expect("cached-read fixture should be removable");
    }

    #[test]
    fn refresh_write_lock_is_single_flight() {
        let root = fixture_root();
        let store = root.join("registry.db");
        let first = acquire_store_write_lock(&store).expect("first write lock should succeed");
        let second = acquire_store_write_lock_with_timeout(&store, StdDuration::from_millis(20));
        assert!(
            second.is_err(),
            "a concurrent writer must not enter the refresh critical section"
        );
        drop(first);
        let third = acquire_store_write_lock_with_timeout(&store, StdDuration::from_millis(20));
        assert!(
            third.is_ok(),
            "the lock should be reusable after the writer exits"
        );
        drop(third);

        fs::remove_dir_all(root).expect("write-lock fixture should be removable");
    }

    #[test]
    fn maturity_checkpoint_reports_missing_and_stale_applicable_repositories() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_name = snapshot.repositories[0].name.clone();
        snapshot.repositories[0].lifecycle = "Active".to_string();
        snapshot.repositories[0].lifecycle_candidate = "Active".to_string();

        assert_eq!(
            maturity_coverage_gaps(&snapshot.repositories),
            vec![format!("{repository_name} (missing)")]
        );

        snapshot.repositories[0].quality.maturity.score = Some(2.0);
        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Stale;
        assert_eq!(
            maturity_coverage_gaps(&snapshot.repositories),
            vec![format!("{repository_name} (stale)")]
        );

        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0]
            .quality
            .mac_control_ideal_state
            .status = "Not applicable".to_string();
        snapshot.repositories[0]
            .quality
            .mac_control_ideal_state
            .freshness = "Fresh".to_string();
        assert!(maturity_coverage_gaps(&snapshot.repositories).is_empty());

        fs::remove_dir_all(root).expect("maturity coverage fixture should be removable");
    }

    #[test]
    fn cli_repeated_options_are_agent_friendly() {
        let arguments = vec![
            "group".to_string(),
            "create".to_string(),
            "Experiments".to_string(),
            "--repo".to_string(),
            "repository:one".to_string(),
            "--repo".to_string(),
            "repository:two".to_string(),
            "--json".to_string(),
        ];
        assert_eq!(
            cli_positionals(&arguments, &["--repo"]).expect("positionals should parse"),
            vec!["create", "Experiments"]
        );
        assert_eq!(
            cli_repeated_option(&arguments, "--repo").expect("repeated options should parse"),
            vec!["repository:one", "repository:two"]
        );
    }

    #[test]
    fn remediation_timeout_option_requires_a_positive_integer() {
        let valid = vec![
            "remediation".to_string(),
            "refresh".to_string(),
            "--timeout-seconds".to_string(),
            "3600".to_string(),
        ];
        assert_eq!(
            cli_positive_u64_option(&valid, "--timeout-seconds")
                .expect("positive timeout should parse"),
            Some(3600)
        );

        for invalid_value in ["0", "-1", "not-a-number"] {
            let invalid = vec![
                "remediation".to_string(),
                "refresh".to_string(),
                "--timeout-seconds".to_string(),
                invalid_value.to_string(),
            ];
            assert_eq!(
                cli_positive_u64_option(&invalid, "--timeout-seconds")
                    .expect_err("invalid timeout should fail"),
                "--timeout-seconds must be a positive integer"
            );
        }
    }

    #[test]
    fn qr_audit_runtime_arguments_apply_the_requested_timeout() {
        let mut arguments = vec!["fleet".to_string(), "audit".to_string()];
        append_qr_audit_runtime_arguments(&mut arguments, true, false, 3600);

        assert_eq!(
            arguments,
            vec![
                "fleet",
                "audit",
                "--dynamic",
                "--no-changed-only",
                "--timeout-seconds",
                "3600",
                "--json"
            ]
        );
    }

    #[test]
    fn excludes_the_scanner_process_and_its_ancestors_from_activity_candidates() {
        let rows = parse_process_activity_rows(
            "10 9 pronto_cli Tue Jul 29 13:43:00 2026\n\
             9 8 cargo Tue Jul 29 13:42:59 2026\n\
             8 7 node Tue Jul 29 13:42:58 2026\n\
             7 1 zsh Tue Jul 29 13:42:57 2026\n\
             22 1 codex Tue Jul 29 12:00:00 2026\n",
        );

        let excluded = process_ancestor_ids(&rows, 10);

        assert_eq!(
            excluded,
            [10, 9, 8, 7, 1].into_iter().collect::<HashSet<_>>()
        );
        assert!(!excluded.contains(&22));
        assert_eq!(rows[0].process_name, "pronto_cli");
        assert_eq!(
            rows[0].started_at.as_deref(),
            Some("Tue Jul 29 13:43:00 2026")
        );
    }

    #[test]
    fn ignores_idle_shells_as_agent_activity_candidates() {
        assert!(!process_name_is_activity_candidate("/bin/zsh"));
        assert!(!process_name_is_activity_candidate("bash"));
        assert!(process_name_is_activity_candidate("codex"));
        assert!(process_name_is_activity_candidate("claude-code"));
    }

    #[test]
    fn observes_git_fetch_head_freshness() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        fs::write(repository.join(".git").join("FETCH_HEAD"), "fixture\n")
            .expect("fetch marker should be writable");

        assert!(observed_fetch_at(&repository).is_some());

        fs::remove_dir_all(root).expect("fetch fixture should be removable");
    }

    #[test]
    fn appending_repository_ids_preserves_and_deduplicates_members() {
        let merged = merge_repository_ids(
            &["repository:one".to_string(), "repository:two".to_string()],
            vec!["repository:two".to_string(), "repository:three".to_string()],
        );
        let mut sorted = merged;
        sorted.sort();
        assert_eq!(
            sorted,
            vec![
                "repository:one".to_string(),
                "repository:three".to_string(),
                "repository:two".to_string()
            ]
        );
    }

    #[test]
    fn excludes_named_folders_case_insensitively_and_prunes_existing_repositories() {
        let root = fixture_root();
        let included = fixture_repository(&root);
        let excluded_parent = root.join("Not-mine");
        fs::create_dir_all(&excluded_parent).expect("excluded folder should be creatable");
        let excluded = fixture_repository(&excluded_parent);
        let store = root.join("registry.db");

        let first = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("initial scan should discover both repositories");
        assert_eq!(first.repositories.len(), 2);
        assert!(matches_ignore("Not-mine", &["not-mine".to_string()]));

        let filtered = exclude_root_patterns_at(
            &store,
            &root.to_string_lossy(),
            vec!["not-mine".to_string(), "test-fixtures".to_string()],
        )
        .expect("root exclusions should rescan the root");
        assert_eq!(filtered.repositories.len(), 1);
        assert_eq!(
            filtered.repositories[0].path,
            canonical_path(&included)
                .expect("included repository should canonicalize")
                .to_string_lossy()
        );
        assert!(!filtered.repositories.iter().any(|repository| {
            repository.path
                == canonical_path(&excluded)
                    .expect("excluded repository should canonicalize")
                    .to_string_lossy()
        }));

        let persisted = load_store(&store).expect("root exclusions should persist");
        assert_eq!(
            persisted.roots[0].ignore_patterns,
            vec!["not-mine", "test-fixtures"]
        );

        fs::remove_dir_all(root).expect("exclusion fixture should be removable");
    }

    #[test]
    fn prunes_deleted_repositories_on_full_refresh() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");

        let first = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("initial scan should discover the repository");
        assert_eq!(first.repositories.len(), 1);

        fs::remove_dir_all(&repository).expect("repository should be removable");
        let mut state = load_store(&store).expect("initial store should be readable");
        let refreshed =
            scan_and_persist_scoped(&store, &mut state, None).expect("full refresh should succeed");

        assert!(refreshed.repositories.is_empty());
        assert!(load_store(&store)
            .expect("refreshed store should be readable")
            .repositories
            .is_empty());

        fs::remove_dir_all(root).expect("deleted repository fixture should be removable");
    }

    #[test]
    fn parses_porcelain_branch_and_dirty_state() {
        let parsed = parse_status(
            "# branch.oid abc\n# branch.head feature/test\n# branch.upstream origin/feature/test\n# branch.ab +3 -2\n1 .M N... 100644 100644 100644 abc def tracked.txt\n",
        );
        assert_eq!(parsed.branch, "feature/test");
        assert_eq!(parsed.upstream.as_deref(), Some("origin/feature/test"));
        assert_eq!(parsed.ahead, 3);
        assert_eq!(parsed.behind, 2);
        assert!(parsed.dirty);
    }

    #[test]
    fn failed_git_status_is_projected_as_unavailable_not_clean_detached() {
        let root = fixture_root();
        let workspace = scan_workspace(&root, true, Some("main"), Some("main"), "Medium", None);

        assert!(!workspace.status_available);
        assert_eq!(workspace.branch, "Unknown");
        assert!(!workspace.dirty);
        assert_eq!(workspace.sync_state, "Git status unavailable");
        assert_eq!(workspace.integration_state, "Unknown");
        assert!(!workspace_is_unsynced(&workspace));
        assert!(workspace_requires_sync_attention(&workspace));
        assert!(workspace
            .status_error
            .as_deref()
            .is_some_and(|error| error.contains("Git status failed")));

        let conditions = build_conditions(
            "repository:test",
            &workspace,
            Some("main"),
            &[],
            "2026-08-08T00:00:00Z",
        );
        assert!(conditions
            .iter()
            .any(|condition| condition.kind == "git-status-unavailable"));
        assert!(!conditions
            .iter()
            .any(|condition| condition.kind == "no-upstream"));

        fs::remove_dir_all(root).expect("failed-status fixture should be removable");
    }

    #[test]
    fn remediation_handoff_requires_a_checkpoint_and_fresh_snapshot() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let clean_snapshot = scan_repository(&repository_path, None, &[]);
        let clean_check = remediation_handoff_check_for_repository(&clean_snapshot, None)
            .expect("clean fixture should be checkable");
        assert!(clean_check.ready);
        assert!(!clean_check.checkpoint_required);

        fs::write(repository_path.join("tracked.txt"), "uncommitted\n")
            .expect("dirty fixture should be writable");
        let dirty_snapshot = scan_repository(&repository_path, Some(&clean_snapshot), &[]);
        let dirty_check = remediation_handoff_check_for_repository(&dirty_snapshot, None)
            .expect("dirty fixture should be checkable");
        assert!(!dirty_check.ready);
        assert!(dirty_check.checkpoint_required);
        assert!(dirty_check.workspace_dirty);
        assert!(dirty_check
            .reasons
            .iter()
            .any(|reason| reason.contains("checkpoint commit")));

        git(&repository_path, &["add", "tracked.txt"]);
        git(&repository_path, &["commit", "-m", "Checkpoint fixture"]);
        let stale_check = remediation_handoff_check_for_repository(&dirty_snapshot, None)
            .expect("stale fixture should still be checkable");
        assert!(!stale_check.ready);
        assert!(!stale_check.workspace_dirty);
        assert!(stale_check
            .reasons
            .iter()
            .any(|reason| reason.contains("persisted Pronto snapshot")));

        let refreshed_snapshot = scan_repository(&repository_path, Some(&dirty_snapshot), &[]);
        let refreshed_check = remediation_handoff_check_for_repository(&refreshed_snapshot, None)
            .expect("refreshed fixture should be checkable");
        assert!(refreshed_check.ready);
        assert!(!refreshed_check.checkpoint_required);

        fs::remove_dir_all(root).expect("handoff fixture should be removable");
    }

    #[test]
    fn agent_sync_attention_matches_renderer_synced_state() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.sync_state = "Synced".to_string();
        assert!(!workspace_requires_sync_attention(&workspace));
        workspace.sync_state = "Ahead by 1".to_string();
        assert!(workspace_requires_sync_attention(&workspace));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn workspace_sync_detail_exposes_expiry_reason_and_scoped_refresh() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.sync_state = "Behind by 1".to_string();
        workspace.upstream = Some("origin/main".to_string());
        workspace.ahead = 0;
        workspace.behind = 1;

        let detail = workspace_sync_detail(
            &workspace,
            "/tmp/pronto with spaces",
            "2026-07-30T12:00:00Z",
        )
        .expect("unsynced workspace should have detail");

        assert_eq!(
            detail.evidence_observed_at.as_deref(),
            Some("2026-07-30T12:00:00Z")
        );
        assert_eq!(
            detail.evidence_expires_at.as_deref(),
            Some("2026-08-01T12:00:00Z")
        );
        assert!(detail.reason.contains("behind by 1 commit"));
        assert_eq!(
            detail.scoped_refresh_command,
            "pronto refresh '/tmp/pronto with spaces' --json"
        );
        assert!(detail
            .authorization
            .contains("does not pull, push, merge, rebase"));

        workspace.sync_detail = Some(detail.clone());
        let evidence = agent_workspace_sync_evidence(&workspace);
        assert!(evidence.iter().any(|item| {
            item.label == "Evidence expires"
                && item.value.as_deref() == Some("2026-08-01T12:00:00Z")
        }));
        let action = agent_next_action(&AgentAttentionItem {
            id: "workspace-sync".to_string(),
            repository_id: "repository:pronto".to_string(),
            repository_name: "pronto".to_string(),
            repository_path: "/tmp/pronto with spaces".to_string(),
            workspace_id: Some(workspace.id.clone()),
            workspace_path: Some(workspace.path.clone()),
            category: "synchronization".to_string(),
            severity: "warning".to_string(),
            status: workspace.sync_state.clone(),
            freshness: None,
            summary: "Workspace is unsynced".to_string(),
            evidence,
        });
        assert!(action
            .next_safe_step
            .contains("pronto refresh '/tmp/pronto with spaces' --json"));
        assert!(action.authorization.contains("read-only local Git scan"));

        workspace.sync_state = "Synced".to_string();
        assert!(workspace_sync_detail(&workspace, "/tmp/pronto", "2026-07-30T12:00:00Z").is_none());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn next_report_is_bounded_ranked_and_repository_aware() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();
        let repository_path = snapshot.repositories[0].path.clone();
        snapshot.repositories[0].quality.gates.clear();
        snapshot.repositories[0].quality.findings.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0].quality.maturity.freshness = QualityFreshness::Fresh;
        snapshot.repositories[0].quality.ingestion_status = "Available".to_string();
        snapshot.repositories[0].conditions = vec![
            Condition {
                id: "routine-condition".to_string(),
                kind: "branch".to_string(),
                title: "Routine condition".to_string(),
                summary: "Routine evidence needs review".to_string(),
                priority: 4,
                status: "Active".to_string(),
                fingerprint: "routine".to_string(),
                rule: "fixture".to_string(),
                evidence: Vec::new(),
                missing: Vec::new(),
                confidence: Some("High".to_string()),
                freshness: Some("Fresh".to_string()),
            },
            Condition {
                id: "urgent-condition".to_string(),
                kind: "branch".to_string(),
                title: "Urgent condition".to_string(),
                summary: "Urgent evidence needs review".to_string(),
                priority: 1,
                status: "Active".to_string(),
                fingerprint: "urgent".to_string(),
                rule: "fixture".to_string(),
                evidence: Vec::new(),
                missing: Vec::new(),
                confidence: Some("High".to_string()),
                freshness: Some("Fresh".to_string()),
            },
        ];

        let report = agent_next_report(&snapshot, Some(&repository_path), "fleet", 1)
            .expect("next report should resolve the repository");

        assert_eq!(report.schema_version, AGENT_NEXT_SCHEMA);
        assert_eq!(report.summary.repository_count, 1);
        assert_eq!(
            report.current_repository.as_ref().map(|item| &item.id),
            Some(&repository_id)
        );
        assert!(report.attention_total >= 2);
        assert_eq!(report.attention.len(), 1);
        assert_eq!(
            report.attention[0].id,
            format!("{repository_id}:condition:urgent-condition")
        );
        assert_eq!(report.actions.len(), 1);
        assert_eq!(report.actions[0].recommended_projection, "repo");
        assert!(report.actions[0]
            .authorization
            .contains("explicit authorization"));

        fs::remove_dir_all(root).expect("next report fixture should be removable");
    }

    #[test]
    fn fold_preview_preserves_unpublished_branch_and_requires_live_verification() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "dev"]);
        git(&repository, &["switch", "-c", "feature/fold-preview"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "Feature preview"]);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        snapshot.repositories[0].workspaces[0].activity.state =
            "Interrupted with unpushed commits".to_string();
        snapshot.repositories[0].workspaces[0].activity.confidence = "Medium".to_string();
        let repository_path = snapshot.repositories[0].path.clone();

        let report = agent_fold_preview_report(
            &snapshot,
            Some(&repository_path),
            Some("dev"),
            "repository:fixture",
            10,
        )
        .expect("fold preview should resolve the repository");

        assert_eq!(report.schema_version, AGENT_FOLD_PREVIEW_SCHEMA);
        assert_eq!(report.repository_count, 1);
        assert_eq!(report.branch_total, 3);
        assert_eq!(report.candidate_total, 2);
        assert_eq!(report.candidates.len(), 2);
        let candidate = report
            .candidates
            .iter()
            .find(|candidate| candidate.source_branch == "feature/fold-preview")
            .expect("feature branch should be in fold preview");
        assert_eq!(candidate.target_branch.as_deref(), Some("dev"));
        assert_eq!(candidate.decision, "preserve_unpublished");
        assert_eq!(candidate.integration_state, "Integration eligible");
        assert_eq!(candidate.dirty, Some(false));
        assert!(report.live_verification_required);
        assert!(candidate.authorization.contains("Preview only"));

        fs::remove_dir_all(root).expect("fold preview fixture should be removable");
    }

    #[test]
    fn doctor_report_blocks_stale_and_unavailable_snapshot() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository = snapshot
            .repositories
            .first_mut()
            .expect("fixture repository should be present");
        repository.last_scan_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        repository.workspaces[0].path =
            root.join("missing-workspace").to_string_lossy().to_string();

        let report = agent_doctor_report(&snapshot, &store, 60, "repository:fixture");

        assert_eq!(report.schema_version, AGENT_DOCTOR_SCHEMA);
        assert!(!report.ready);
        assert_eq!(report.status, "Blocked");
        assert_eq!(report.stale_repository_ids.len(), 1);
        assert!(report.invalid_scan_repository_ids.is_empty());
        assert!(report
            .unavailable_paths
            .iter()
            .any(|path| path.ends_with("missing-workspace")));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Blocked"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "paths" && check.status == "Blocked"));
        assert!(report.authorization.contains("does not refresh"));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }

    #[test]
    fn doctor_default_freshness_window_is_two_days() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");

        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(47)).to_rfc3339();
        let fresh = agent_doctor_report(
            &snapshot,
            &store,
            DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES,
            "repository:fixture",
        );

        assert_eq!(fresh.max_age_minutes, 2_880);
        assert!(fresh.stale_repository_ids.is_empty());
        assert!(fresh
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Passed"));

        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(49)).to_rfc3339();
        let stale = agent_doctor_report(
            &snapshot,
            &store,
            DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES,
            "repository:fixture",
        );

        assert_eq!(stale.stale_repository_ids.len(), 1);
        assert!(stale
            .checks
            .iter()
            .any(|check| check.id == "snapshot" && check.status == "Blocked"));

        fs::remove_dir_all(root).expect("doctor fixture should be removable");
    }

    #[test]
    fn scoped_doctor_ignores_unrelated_stale_repositories() {
        let root = fixture_root();
        let first_repository = fixture_repository(&root);
        let second_root = root.join("second");
        fs::create_dir_all(&second_root).expect("second repository parent should be creatable");
        fixture_repository(&second_root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        assert_eq!(snapshot.repositories.len(), 2);
        let first_repository_path = canonical_path(&first_repository)
            .expect("first repository should canonicalize")
            .to_string_lossy()
            .to_string();
        let selected_id = snapshot
            .repositories
            .iter()
            .find(|repository| repository.path == first_repository_path)
            .expect("first repository should be registered")
            .id
            .clone();
        let unrelated = snapshot
            .repositories
            .iter_mut()
            .find(|repository| repository.id != selected_id)
            .expect("second repository should be registered");
        unrelated.last_scan_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        unrelated.workspaces[0].path = root
            .join("missing-unrelated-workspace")
            .to_string_lossy()
            .to_string();
        let mut scoped_snapshot = snapshot;
        scoped_snapshot
            .repositories
            .retain(|repository| repository.id == selected_id);
        let report = agent_doctor_report(
            &scoped_snapshot,
            &store,
            60,
            "current_repository:/Users/jakyeamos/Documents/pronto",
        );

        assert!(report.ready);
        assert!(matches!(
            report.status.as_str(),
            "Ready" | "Ready with warnings"
        ));
        assert_eq!(report.repository_count, 1);
        assert_eq!(report.root_count, 1);
        assert!(report.stale_repository_ids.is_empty());
        assert!(report.unavailable_paths.is_empty());

        fs::remove_dir_all(root).expect("scoped doctor fixture should be removable");
    }

    #[test]
    fn doctor_read_only_load_does_not_create_missing_store() {
        let root = fixture_root();
        let store = root.join("missing.db");

        assert!(load_store_read_only(&store).is_err());
        assert!(!store.exists());

        fs::remove_dir_all(root).expect("doctor read-only fixture should be removable");
    }

    #[test]
    fn route_exposes_bounded_projections_after_a_ready_doctor_gate() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_path = snapshot.repositories[0].path.clone();
        let scope = format!("repository:{repository_path}");

        let report = agent_route_report(&snapshot, &store, 60, &scope, Some(&repository_path), 3)
            .expect("route report should build");

        assert_eq!(report.schema_version, AGENT_ROUTE_SCHEMA);
        assert!(report.ready);
        assert!(report.next.is_some());
        assert!(report.repository.is_some());
        assert!(report.fold_preview.is_some());
        assert!(report
            .fold_preview
            .as_ref()
            .is_some_and(|preview| preview.live_verification_required));
        assert_eq!(
            report
                .quality
                .as_ref()
                .map(|quality| quality.scope.as_str()),
            Some(scope.as_str())
        );
        assert_eq!(report.doctor.scope, scope);
        assert!(report.authorization.contains("Inspection only"));

        fs::remove_dir_all(root).expect("route fixture should be removable");
    }

    #[test]
    fn route_projection_does_not_run_live_merge_checks() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/route"]);
        fs::write(repository.join("route.txt"), "route\n")
            .expect("route fixture file should be writable");
        git(&repository, &["add", "route.txt"]);
        git(&repository, &["commit", "-m", "Route projection"]);
        let store = root.join("registry.db");
        let snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("route fixture portfolio should scan");
        let repository_path = snapshot.repositories[0].path.clone();
        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            &format!("repository:{repository_path}"),
            Some(&repository_path),
            3,
        )
        .expect("route report should build without live merge checks");
        let fold_preview = report
            .fold_preview
            .expect("route should include a fold projection");
        assert!(
            fold_preview
                .candidates
                .iter()
                .all(|candidate| candidate.merge_preview.is_none()),
            "route must keep live merge verification out of the response path"
        );
        assert!(fold_preview.live_verification_required);

        fs::remove_dir_all(root).expect("route merge-check fixture should be removable");
    }

    #[test]
    fn fresh_route_timeout_names_the_cached_fallback() {
        let root = fixture_root();
        let store = root.join("registry.db");
        let report = agent_route_error_report(
            &store,
            60,
            "fleet",
            "storage",
            "Fresh quality projection exceeded the 10 second deadline; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.".to_string(),
        );

        assert!(report
            .next_safe_step
            .contains("cached snapshot or run `pronto quality refresh`"));
        assert!(report.doctor.checks[0]
            .next_safe_step
            .contains("cached snapshot or run `pronto quality refresh`"));

        fs::remove_dir_all(root).expect("route timeout fixture should be removable");
    }

    #[test]
    fn route_withholds_follow_up_projections_when_doctor_is_blocked() {
        let root = fixture_root();
        fixture_repository(&root);
        let store = root.join("registry.db");
        let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        snapshot.repositories[0].last_scan_at =
            (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();

        let report = agent_route_report(
            &snapshot,
            &store,
            60,
            "repository:fixture",
            Some(&snapshot.repositories[0].path.clone()),
            3,
        )
        .expect("blocked route report should still build");

        assert_eq!(report.schema_version, AGENT_ROUTE_SCHEMA);
        assert!(!report.ready);
        assert_eq!(report.status, "Blocked");
        assert!(report.next.is_none());
        assert!(report.repository.is_none());
        assert!(report.quality.is_none());
        assert!(report.fold_preview.is_none());
        assert!(report.next_safe_step.contains("refresh"));

        fs::remove_dir_all(root).expect("blocked route fixture should be removable");
    }

    #[test]
    fn parses_numstat_and_marks_binary_totals_partial() {
        let totals = parse_numstat("4\t2\ttext.txt\n-\t-\timage.png\n");
        assert_eq!(totals.added, 4);
        assert_eq!(totals.removed, 2);
        assert!(totals.partial);
    }

    #[test]
    fn normalizes_github_remote_names_without_exposing_credentials() {
        assert_eq!(
            normalize_remote_name("git@github.com:Acme/Portfolio.git").as_deref(),
            Some("acme/portfolio")
        );
        assert_eq!(
            normalize_remote_name("https://github.com/Acme/Portfolio/").as_deref(),
            Some("acme/portfolio")
        );
        assert_eq!(normalize_remote_name("  ").as_deref(), None);
    }

    #[test]
    fn reads_optional_agent_manifest_with_explicit_activity_language() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "agent/manifest-task"]);
        let manifest_directory = repository.join(".pronto");
        fs::create_dir_all(&manifest_directory).expect("manifest directory should be creatable");
        fs::write(
            manifest_directory.join("agent.json"),
            serde_json::to_vec(&serde_json::json!({
                "task_id": "task-42",
                "title": "Document workspace recovery",
                "target_branch": "main",
                "agent_type": "codex",
                "start_time": "2026-07-26T12:00:00Z",
                "status": "active",
                "source_session_id": "session-42"
            }))
            .expect("manifest should encode"),
        )
        .expect("manifest should be writable");

        let workspace = scan_workspace(&repository, true, Some("main"), None, "Medium", None);
        assert_eq!(workspace.activity.state, "Active");
        assert_eq!(workspace.activity.confidence, "High");
        assert_eq!(
            workspace
                .activity
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.task_id.as_deref()),
            Some("task-42")
        );
        assert_eq!(workspace.target_branch.as_deref(), Some("main"));
        assert_eq!(workspace.target_confidence, "High");
        assert_eq!(workspace.role, "Agent task");
        assert!(workspace
            .activity
            .signals
            .iter()
            .any(|signal| signal.source == "Manifest"));

        let encoded = serde_json::to_string(&workspace).expect("workspace should serialize");
        assert!(!encoded.contains("terminal contents"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn active_workspace_activity_blocks_integration_eligibility() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/active"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "Feature commit"]);
        let mut workspace = scan_workspace(
            &repository,
            true,
            Some("main"),
            Some("main"),
            "Medium",
            None,
        );
        workspace.activity.state = "Active".to_string();
        assert_eq!(
            branch_integration_state(
                &repository,
                &workspace.branch,
                Some("main"),
                Some(&workspace),
            ),
            "Blocked"
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn stale_quality_evidence_blocks_release_rule_evaluation() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality = QualitySnapshot {
            gates: vec![quality::QualityGate {
                id: "lint".to_string(),
                label: "Lint".to_string(),
                status: QualityGateStatus::Passed,
                freshness: QualityFreshness::Stale,
                evidence: vec![quality::QualityEvidence {
                    id: "lint".to_string(),
                    source: quality::QualitySource::Ci,
                    status: QualityGateStatus::Passed,
                    freshness: QualityFreshness::Stale,
                    observed_at: Some("2026-07-10T12:00:00Z".to_string()),
                    scanned_commit: Some("old-commit".to_string()),
                    scanned_branch: Some("main".to_string()),
                    command: None,
                    source_label: "GitHub check · lint".to_string(),
                    report_path: None,
                    report_url: None,
                    report_kind: Some("GitHub check run".to_string()),
                    detail: "success".to_string(),
                }],
            }],
            ..QualitySnapshot::default()
        };
        let rule = ReleaseRuleConfig {
            name: "Fresh lint required".to_string(),
            operator: "AND".to_string(),
            min_commits: None,
            min_elapsed_days: None,
            required_commit_types: Vec::new(),
            allow_first_release: false,
            required_quality_gates: vec![QualityGateRequirement {
                gate_id: "lint".to_string(),
                source: quality::QualitySource::Ci,
            }],
        };

        let (result, trace) = evaluate_release_rule_with_quality(&repository, &rule, None, &[]);
        assert_eq!(result, ReleaseRuleResult::Blocked);
        assert!(trace.iter().any(|item| {
            item.label == "Quality gate · Lint · CI" && item.status == "Passed · Stale"
        }));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn external_handoff_requires_exact_workspace_and_supported_tool() {
        let root = fixture_root();
        let _repository = fixture_repository(&root);
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None)
            .expect("workspace fixture should persist");
        let persisted = load_store(&database).expect("workspace fixture should reload");
        let stored_repository = persisted
            .repositories
            .first()
            .expect("fixture repository should be registered");
        let workspace_id = stored_repository.workspace.id.clone();

        let missing_workspace = open_workspace_at(
            &database,
            &stored_repository.id,
            "workspace-that-is-not-registered",
            "file_browser",
        )
        .expect_err("handoff must reject an unknown workspace");
        assert_eq!(
            missing_workspace,
            "Workspace is not registered for this repository"
        );

        let unsupported_tool = open_workspace_at(
            &database,
            &stored_repository.id,
            &workspace_id,
            "unsupported",
        )
        .expect_err("handoff must reject an unknown tool");
        assert_eq!(unsupported_tool, "Choose a supported external handoff tool");
        assert_eq!(
            load_store(&database)
                .expect("handoff fixture should remain readable")
                .repositories
                .len(),
            1
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn prepares_pull_request_and_release_evidence_deterministically() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let initial_commit =
            String::from_utf8_lossy(&git(&repository, &["rev-parse", "main"]).stdout)
                .trim()
                .to_string();
        git(&repository, &["switch", "-c", "feature/release-preview"]);
        fs::write(repository.join("feature.txt"), "feature\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(&repository, &["commit", "-m", "feat: add release preview"]);
        fs::write(repository.join("feature.txt"), "feature\nfix\n")
            .expect("feature file should be writable");
        git(&repository, &["add", "feature.txt"]);
        git(
            &repository,
            &["commit", "-m", "fix: correct release preview"],
        );
        fs::write(repository.join("breaking.txt"), "breaking\n")
            .expect("breaking file should be writable");
        git(&repository, &["add", "breaking.txt"]);
        git(
            &repository,
            &["commit", "-m", "feat!: change release contract"],
        );

        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None)
            .expect("release fixture should persist");
        let mut persisted = load_store(&database).expect("release fixture should reload");
        let (repository_id, workspace_id) = {
            let stored_repository = persisted
                .repositories
                .first_mut()
                .expect("fixture repository should be registered");
            stored_repository.provider_state = "GitHub connected as github:fixture".to_string();
            stored_repository.releases = vec![ReleaseSnapshot {
                id: "github:release-1".to_string(),
                provider: "github".to_string(),
                repository_id: "github:repo-1".to_string(),
                tag: "v1.2.3".to_string(),
                name: "v1.2.3".to_string(),
                target_commit: Some(initial_commit),
                published_at: Some("2026-07-25T12:00:00Z".to_string()),
                draft: false,
                prerelease: false,
                last_refreshed_at: "2026-07-26T12:00:00Z".to_string(),
            }];
            stored_repository.pull_requests = vec![PullRequestSnapshot {
                id: "github:pr-1".to_string(),
                provider: "github".to_string(),
                repository_id: "github:repo-1".to_string(),
                number: 7,
                html_url: "https://github.com/fixture/repository/pull/7".to_string(),
                title: "Release preview".to_string(),
                head_branch: "feature/release-preview".to_string(),
                base_branch: "main".to_string(),
                state: "OPEN".to_string(),
                draft: true,
                checks_state: "Pending".to_string(),
                reviews_state: "Required review unavailable".to_string(),
                mergeability: "Unknown — provider snapshot unavailable".to_string(),
                checks: Vec::new(),
                last_refreshed_at: "2026-07-26T12:00:00Z".to_string(),
                head_commit: None,
            }];
            (
                stored_repository.id.clone(),
                stored_repository.workspace.id.clone(),
            )
        };
        persisted.provider_status = ProviderStatus {
            provider: "GitHub".to_string(),
            state: "Ready".to_string(),
            message: "fixture".to_string(),
            last_refresh_at: Some("2026-07-26T12:00:00Z".to_string()),
            identity_count: 1,
            repository_count: 1,
        };
        save_store(&database, &persisted).expect("release evidence should persist");

        let preparation = prepare_repository_at(&database, &repository_id, Some(&workspace_id))
            .expect("preparation should be deterministic");
        assert_eq!(preparation.pull_request.status, "Evidence ready");
        assert_eq!(
            preparation.pull_request.base_branch.as_deref(),
            Some("main")
        );
        assert_eq!(preparation.pull_request.commit_count, 3);
        assert_eq!(preparation.pull_request.checks_state, "Pending");
        assert_eq!(
            preparation
                .pull_request
                .existing_pull_request
                .as_ref()
                .map(|pull_request| pull_request.number),
            Some(7)
        );
        assert_eq!(
            preparation.release.baseline_status,
            "Published release baseline"
        );
        assert_eq!(preparation.release.commits_since_baseline.len(), 3);
        assert_eq!(preparation.release.candidate_bump.as_deref(), Some("major"));
        assert_eq!(
            preparation.release.candidate_version.as_deref(),
            Some("v2.0.0")
        );
        assert_eq!(
            preparation.release.rule_status,
            "Not configured — commits are shown without threshold evaluation"
        );
        assert_eq!(
            preparation
                .release
                .notes
                .iter()
                .map(|section| section.category.as_str())
                .collect::<Vec<_>>(),
            vec!["Breaking", "Features", "Fixes"]
        );
        let configured_snapshot = set_release_rule_at(
            &database,
            &repository_id,
            Some(ReleaseRuleConfig {
                name: "Two meaningful commits".to_string(),
                operator: "and".to_string(),
                min_commits: Some(2),
                min_elapsed_days: None,
                required_commit_types: vec!["FEAT".to_string()],
                allow_first_release: false,
                required_quality_gates: Vec::new(),
            }),
        )
        .expect("release rule should persist");
        assert_eq!(
            configured_snapshot.repositories[0]
                .release_rule
                .as_ref()
                .map(|rule| rule.operator.as_str()),
            Some("AND")
        );
        assert!(configured_snapshot.repositories[0]
            .conditions
            .iter()
            .any(|condition| condition.title == "Configured release threshold met"));
        let configured_preparation =
            prepare_repository_at(&database, &repository_id, Some(&workspace_id))
                .expect("configured preparation should be deterministic");
        assert_eq!(
            configured_preparation.release.rule_status,
            "Configured release threshold met"
        );
        assert_eq!(configured_preparation.release.rule_trace.len(), 3);
        assert!(configured_preparation
            .release
            .rule_trace
            .iter()
            .all(|trace| trace.status == "Passed"));
        assert_eq!(configured_preparation.recipe.status, "Blocked");
        assert_eq!(configured_preparation.recipe.steps[2].status, "Blocked");
        let mismatched_version =
            set_release_version_at(&database, &repository_id, Some("9.9.9".to_string()))
                .expect_err("stale version confirmations should be rejected");
        assert_eq!(
            mismatched_version,
            "Release version must match the current deterministic candidate"
        );
        let invalid_recipe = set_release_recipe_at(
            &database,
            &repository_id,
            Some(ReleaseRecipeConfig {
                name: "Unsafe recipe".to_string(),
                validation_commands: vec!["pnpm test\nrm -rf .".to_string()],
                release_commands: Vec::new(),
                generated_paths: vec!["../outside.txt".to_string()],
                commit_message: "chore(release): prepare {version}".to_string(),
            }),
        )
        .expect_err("unsafe recipe paths and commands should be rejected");
        assert!(invalid_recipe.contains("line breaks"));
        let confirmed_snapshot =
            set_release_version_at(&database, &repository_id, Some("2.0.0".to_string()))
                .expect("candidate version should require and accept explicit confirmation");
        assert_eq!(
            confirmed_snapshot.repositories[0]
                .confirmed_release_version
                .as_deref(),
            Some("v2.0.0")
        );
        set_release_recipe_at(
            &database,
            &repository_id,
            Some(ReleaseRecipeConfig {
                name: "Fixture release".to_string(),
                validation_commands: vec!["pnpm test".to_string()],
                release_commands: vec!["pnpm release:version".to_string()],
                generated_paths: vec!["CHANGELOG.md".to_string()],
                commit_message: "chore(release): prepare {version}".to_string(),
            }),
        )
        .expect("release recipe should persist");
        let ready_recipe_preparation =
            prepare_repository_at(&database, &repository_id, Some(&workspace_id))
                .expect("configured recipe should be readable");
        assert_eq!(
            ready_recipe_preparation.release.version_status,
            "Candidate version confirmed"
        );
        assert_eq!(
            ready_recipe_preparation.recipe.status,
            "Ready for user review"
        );
        assert!(!ready_recipe_preparation.recipe.actions_performed);
        assert_eq!(ready_recipe_preparation.recipe.steps.len(), 9);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn previews_only_allowed_committed_ai_payload_evidence() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["switch", "-c", "feature/ai-preview"]);
        fs::write(repository.join("committed.txt"), "committed evidence\n")
            .expect("committed file should be writable");
        git(&repository, &["add", "committed.txt"]);
        git(
            &repository,
            &["commit", "-m", "feat: add committed evidence"],
        );
        fs::write(
            repository.join("uncommitted.txt"),
            "private uncommitted evidence\n",
        )
        .expect("uncommitted file should be writable");

        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None)
            .expect("AI preview fixture should persist");
        let mut persisted = load_store(&database).expect("AI preview fixture should reload");
        let (repository_id, workspace_id) = {
            let stored_repository = persisted
                .repositories
                .first_mut()
                .expect("AI preview repository should be registered");
            stored_repository.workspace.dirty = true;
            (
                stored_repository.id.clone(),
                stored_repository.workspace.id.clone(),
            )
        };
        save_store(&database, &persisted).expect("AI preview state should persist");

        let disabled = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("disabled AI preview should be readable");
        assert_eq!(disabled.permission, "Disabled");
        assert_eq!(disabled.status, "AI disabled by repository policy");
        assert!(disabled.payload_text.is_empty());
        assert!(!disabled.request_performed);
        assert!(!disabled.uncommitted_included);

        set_ai_permission_at(&database, &repository_id, "Commit metadata only")
            .expect("metadata permission should persist");
        let metadata = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("metadata preview should be readable");
        assert_eq!(metadata.status, "Payload ready for user inspection");
        assert_eq!(metadata.source_references.len(), 1);
        assert_eq!(metadata.categories.len(), 1);
        assert!(metadata
            .payload_text
            .contains("feat: add committed evidence"));
        assert!(!metadata.payload_text.contains("uncommitted.txt"));
        assert!(metadata
            .reasons
            .iter()
            .any(|reason| reason.contains("Uncommitted changes are excluded")));

        set_ai_permission_at(&database, &repository_id, "Committed diff allowed")
            .expect("diff permission should persist");
        let diff = preview_ai_summary_at(&database, &repository_id, Some(&workspace_id))
            .expect("diff preview should be readable");
        assert_eq!(diff.categories.len(), 2);
        assert!(diff.payload_text.contains("committed.txt"));
        assert!(!diff.payload_text.contains("uncommitted.txt"));
        assert!(!diff.request_performed);
        assert!(!diff.uncommitted_included);
        fs::remove_dir_all(root).expect("AI preview fixture should be removable");
    }

    #[test]
    fn parses_github_repository_pages_into_remote_snapshots() {
        let payload = serde_json::json!([
            [{
                "id": 42,
                "full_name": "Acme/Portfolio",
                "name": "Portfolio",
                "owner": {"login": "Acme"},
                "html_url": "https://github.com/Acme/Portfolio",
                "default_branch": "main",
                "archived": false
            }],
            [{
                "full_name": "Acme/Archive",
                "html_url": "https://github.com/Acme/Archive",
                "archived": true
            }]
        ]);
        let repositories =
            parse_github_repositories(&payload, "github:jakyeamos", "2026-07-25T12:00:00Z")
                .expect("GitHub pages should parse");

        assert_eq!(repositories.len(), 2);
        assert_eq!(repositories[0].id, "github:42");
        assert_eq!(repositories[0].full_name, "Acme/Portfolio");
        assert_eq!(repositories[0].owner, "Acme");
        assert_eq!(repositories[0].locality, "Remote only");
        assert_eq!(repositories[1].name, "Archive");
        assert!(repositories[1].archived);
    }

    #[test]
    fn parses_github_pull_requests_and_published_release_snapshots() {
        let pull_requests = parse_github_pull_requests(
            &serde_json::json!([
                [{
                    "number": 12,
                    "html_url": "https://github.com/Acme/Portfolio/pull/12",
                    "title": "Release preview",
                    "state": "open",
                    "draft": true,
                    "head": {"ref": "feature/release"},
                    "base": {"ref": "main"},
                    "mergeable_state": "unknown"
                }]
            ]),
            "github:42",
            "2026-07-26T12:00:00Z",
        )
        .expect("pull-request pages should parse");
        assert_eq!(pull_requests.len(), 1);
        assert_eq!(pull_requests[0].number, 12);
        assert_eq!(pull_requests[0].head_branch, "feature/release");
        assert_eq!(
            pull_requests[0].checks_state,
            "Unknown — provider snapshot unavailable"
        );

        let releases = parse_github_releases(
            &serde_json::json!([
                {
                    "id": 9,
                    "tag_name": "v1.4.0",
                    "name": "Version 1.4.0",
                    "target_commitish": "abc123",
                    "published_at": "2026-07-25T12:00:00Z",
                    "draft": false,
                    "prerelease": false
                },
                {
                    "id": 10,
                    "tag_name": "v1.5.0-rc.1",
                    "published_at": "2026-07-26T12:00:00Z",
                    "draft": false,
                    "prerelease": true
                }
            ]),
            "github:42",
            "2026-07-26T12:00:00Z",
        )
        .expect("release response should parse");
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag, "v1.4.0");
        assert!(!releases[0].prerelease);
        assert!(releases[1].prerelease);
    }

    #[test]
    fn applies_provider_refresh_and_marks_local_matches() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(
            &repository,
            &[
                "remote",
                "add",
                "origin",
                "git@github.com:Acme/Portfolio-Repository.git",
            ],
        );
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None)
            .expect("local fixture should persist before provider refresh");

        let refresh = ProviderRefresh {
            identities: vec![ProviderIdentity {
                id: "github:jakyeamos".to_string(),
                provider: "github".to_string(),
                login: "jakyeamos".to_string(),
                display_name: Some("Jakye Amos".to_string()),
                organizations: Vec::new(),
                credential_state: "Authenticated".to_string(),
                updated_at: "2026-07-25T12:00:00Z".to_string(),
            }],
            repositories: vec![
                RemoteRepositorySnapshot {
                    id: "github:42".to_string(),
                    provider: "github".to_string(),
                    full_name: "acme/portfolio-repository".to_string(),
                    name: "portfolio-repository".to_string(),
                    owner: "acme".to_string(),
                    html_url: "https://github.com/acme/portfolio-repository".to_string(),
                    default_branch: Some("main".to_string()),
                    archived: false,
                    locality: "Remote only".to_string(),
                    identity_id: "github:jakyeamos".to_string(),
                    last_refreshed_at: "2026-07-25T12:00:00Z".to_string(),
                    pull_requests: Vec::new(),
                    releases: Vec::new(),
                    ci_checks: Vec::new(),
                    ci_branch: None,
                    ci_commit: None,
                },
                RemoteRepositorySnapshot {
                    id: "github:99".to_string(),
                    provider: "github".to_string(),
                    full_name: "acme/remote-only".to_string(),
                    name: "remote-only".to_string(),
                    owner: "acme".to_string(),
                    html_url: "https://github.com/acme/remote-only".to_string(),
                    default_branch: Some("main".to_string()),
                    archived: false,
                    locality: "Remote only".to_string(),
                    identity_id: "github:jakyeamos".to_string(),
                    last_refreshed_at: "2026-07-25T12:00:00Z".to_string(),
                    pull_requests: Vec::new(),
                    releases: Vec::new(),
                    ci_checks: Vec::new(),
                    ci_branch: None,
                    ci_commit: None,
                },
            ],
            pull_requests: Vec::new(),
            releases: Vec::new(),
            refreshed_at: "2026-07-25T12:00:00Z".to_string(),
        };
        let snapshot = apply_provider_refresh_at(&database, refresh, None)
            .expect("provider refresh should persist locally");

        assert_eq!(snapshot.provider_status.state, "Ready");
        assert_eq!(snapshot.provider_identities.len(), 1);
        assert_eq!(snapshot.remote_repositories.len(), 2);
        assert_eq!(
            snapshot.remote_repositories[0].full_name,
            "acme/portfolio-repository"
        );
        assert_eq!(snapshot.remote_repositories[0].locality, "Local and remote");
        assert_eq!(
            snapshot.remote_repositories[1].full_name,
            "acme/remote-only"
        );
        assert_eq!(
            snapshot.remote_repositories[1].locality,
            remediation::GITHUB_ONLY_LOCALITY
        );
        assert_eq!(snapshot.remediation.github_only_candidates.len(), 1);
        assert_eq!(
            snapshot.remediation.github_only_candidates[0].last_remediation_task,
            remediation::GITHUB_ONLY_REMEDIATION_TASK
        );
        assert_eq!(snapshot.repositories[0].locality, "Local and remote");
        assert_eq!(
            snapshot.repositories[0].provider_state,
            "GitHub connected as github:jakyeamos"
        );

        let mut persisted = load_store(&database).expect("provider snapshot should reload");
        assert_eq!(persisted.provider_status.state, "Ready");
        assert_eq!(persisted.remote_repositories.len(), 2);
        assert_eq!(persisted.remediation.github_only_candidates.len(), 1);
        let rescanned = scan_and_persist_scoped(&database, &mut persisted, None)
            .expect("local rescan should preserve provider evidence");
        assert_eq!(rescanned.repositories[0].locality, "Local and remote");
        assert_eq!(
            rescanned.repositories[0].provider_state,
            "GitHub connected as github:jakyeamos"
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn discovers_canonical_repository_and_attaches_linked_worktree() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let linked = root.join("linked-worktree");
        git(
            &repository,
            &[
                "worktree",
                "add",
                "-b",
                "agent/test",
                linked.to_str().expect("linked path should be valid UTF-8"),
                "main",
            ],
        );
        git(&repository, &["switch", "-c", "feature/test"]);
        fs::write(repository.join("tracked.txt"), "one\nupdated\n")
            .expect("tracked file should be writable");
        fs::write(repository.join("notes.md"), "first\nsecond\n")
            .expect("untracked file should be writable");

        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let discovered = discover_repositories(&root_config);
        assert_eq!(
            discovered,
            vec![canonical_path(&repository).expect("repository should canonicalize")]
        );

        let snapshot = scan_repository(&discovered[0], None, &[]);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert!(snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.is_primary));
        assert!(snapshot
            .workspaces
            .iter()
            .any(|workspace| !workspace.is_primary));
        assert!(snapshot.workspace.dirty);
        assert_eq!(snapshot.workspace.added, 3);
        assert_eq!(snapshot.workspace.removed, 0);
        assert!(!snapshot.workspace.line_totals_partial);
        assert_eq!(
            snapshot
                .conditions
                .first()
                .map(|condition| condition.kind.as_str()),
            Some("dirty-workspace")
        );
        assert!(snapshot
            .conditions
            .windows(2)
            .all(|window| window[0].priority <= window[1].priority));
        let encoded = serde_json::to_string(&snapshot).expect("snapshot should serialize");
        assert!(!encoded.contains("tracked.txt"));
        assert!(!encoded.contains("notes.md"));

        let dirty = snapshot
            .conditions
            .iter()
            .find(|condition| condition.kind == "dirty-workspace")
            .expect("dirty condition should exist");
        let expected = [ExpectedCondition {
            repository_id: snapshot.id.clone(),
            condition_id: dirty.id.clone(),
            fingerprint: dirty.fingerprint.clone(),
            marked_at: iso_now(),
        }];
        let expected_snapshot = scan_repository(&discovered[0], Some(&snapshot), &expected);
        assert_eq!(
            expected_snapshot
                .conditions
                .iter()
                .find(|condition| condition.kind == "dirty-workspace")
                .map(|condition| condition.status.as_str()),
            Some("Expected")
        );
        assert_eq!(
            transition_fingerprint(&snapshot),
            transition_fingerprint(&expected_snapshot)
        );

        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn detects_interrupted_merge_marker() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        fs::write(repository.join(".git/MERGE_HEAD"), "abc\n")
            .expect("merge marker should be writable");
        assert_eq!(
            interrupted_operation(&repository).as_deref(),
            Some("Merge in progress")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn persists_transition_events_without_duplicate_scans() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let store = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };

        let first =
            scan_and_persist_scoped(&store, &mut state, None).expect("first scan should persist");
        assert_eq!(first.repositories.len(), 1);
        assert_eq!(first.events.len(), 1);

        let second =
            scan_and_persist_scoped(&store, &mut state, None).expect("second scan should persist");
        assert_eq!(second.events.len(), 1);

        fs::write(repository.join("tracked.txt"), "one\nupdated\n")
            .expect("tracked file should be writable");
        let third =
            scan_and_persist_scoped(&store, &mut state, None).expect("changed scan should persist");
        assert_eq!(third.events.len(), 2);
        assert!(third
            .events
            .iter()
            .any(|event| event.kind == "state-transition"));

        let persisted = load_store(&store).expect("persisted state should be readable");
        assert_eq!(persisted.repositories.len(), 1);
        assert_eq!(persisted.events.len(), 2);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn records_allowed_and_rejected_action_preflights() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let state = StoreState {
            roots: vec![root_config.clone()],
            ..StoreState::default()
        };
        save_store(&database, &state).expect("preflight store should be writable");

        let allowed = preflight_action_at(&database, "refresh", None)
            .expect("refresh preflight should be recorded");
        assert!(allowed.allowed);
        assert_eq!(allowed.audit.risk, "read-only");
        assert_eq!(allowed.audit.status, "Preflighted");
        assert_eq!(allowed.audit.target_ids, vec![root_config.id]);

        let rejected = preflight_action_at(&database, "push", None)
            .expect("blocked action should be recorded");
        assert!(!rejected.allowed);
        assert_eq!(rejected.audit.risk, "blocked");
        assert_eq!(rejected.audit.status, "Rejected");
        assert!(rejected
            .audit
            .summary
            .contains("Git mutation and provider writes remain blocked"));

        let persisted = load_store(&database).expect("action audits should persist");
        assert_eq!(persisted.action_audits.len(), 2);
        assert!(persisted
            .action_audits
            .iter()
            .any(|audit| audit.id == allowed.audit.id && audit.status == "Preflighted"));
        assert!(persisted
            .action_audits
            .iter()
            .any(|audit| audit.id == rejected.audit.id && audit.status == "Rejected"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn completes_refresh_action_audit_after_read_only_scan() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };

        let snapshot = audited_scan_and_persist(&database, &mut state)
            .expect("audited refresh should scan local repositories");
        assert_eq!(snapshot.repositories.len(), 1);
        assert_eq!(snapshot.action_audits.len(), 1);
        assert_eq!(snapshot.action_audits[0].action, "refresh");
        assert_eq!(snapshot.action_audits[0].status, "Completed");
        assert!(snapshot.action_audits[0].completed_at.is_some());

        let persisted = load_store(&database).expect("completed audit should persist");
        assert_eq!(persisted.action_audits[0].status, "Completed");
        assert!(persisted.repositories[0].path.contains(
            repository
                .file_name()
                .and_then(|name| name.to_str())
                .expect("repository name should be valid UTF-8")
        ));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn migrates_schema_v1_for_action_audits() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let connection = SqliteConnection::open(&database).expect("schema fixture should open");
        connection
            .execute_batch(
                "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO metadata (key, value) VALUES ('schema_version', '1');
                 INSERT INTO metadata (key, value) VALUES ('store_version', '1');",
            )
            .expect("schema v1 fixture should be writable");
        drop(connection);

        let migrated = load_store(&database).expect("schema v1 should migrate");
        assert!(migrated.action_audits.is_empty());
        let connection = SqliteConnection::open(&database).expect("migrated database should open");
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("migrated schema version should be readable");
        assert_eq!(schema_version, SQLITE_SCHEMA_VERSION.to_string());
        let store_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'store_version'",
                [],
                |row| row.get(0),
            )
            .expect("migrated store version should be readable");
        assert_eq!(store_version, STORE_VERSION.to_string());
        let action_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'action_audits'",
                [],
                |row| row.get(0),
            )
            .expect("action audit table should exist after migration");
        assert_eq!(action_table, "action_audits");
        let provider_identity_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'provider_identities'",
                [],
                |row| row.get(0),
            )
            .expect("provider identity table should exist after migration");
        assert_eq!(provider_identity_table, "provider_identities");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn migrates_legacy_json_store_to_versioned_sqlite() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let legacy = root.join("registry.json");
        let state = StoreState {
            roots: vec![RootConfig {
                id: "root-1".to_string(),
                path: root.to_string_lossy().to_string(),
                label: "fixture".to_string(),
                ignore_patterns: vec!["target".to_string()],
                refresh_policy: default_refresh_policy(),
                background_monitoring: false,
                registered_at: iso_now(),
            }],
            ..StoreState::default()
        };
        let encoded = serde_json::to_string_pretty(&state).expect("legacy state should serialize");
        fs::write(&legacy, encoded).expect("legacy state should be writable");

        let migrated = load_store(&database).expect("legacy state should migrate");
        assert_eq!(migrated.roots.len(), 1);
        assert_eq!(migrated.version, STORE_VERSION);
        assert_eq!(migrated.roots[0].ignore_patterns, vec!["target"]);
        assert!(database.exists());
        assert!(legacy.exists());

        let connection = SqliteConnection::open(&database).expect("database should open");
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("schema version should be recorded");
        assert_eq!(schema_version, SQLITE_SCHEMA_VERSION.to_string());

        let analytics_table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'analytics_samples'",
                [],
                |row| row.get(0),
            )
            .expect("analytics table should be created");
        assert_eq!(analytics_table_count, 1);

        let analytics_views_table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'analytics_views'",
                [],
                |row| row.get(0),
            )
            .expect("analytics views table should be created");
        assert_eq!(analytics_views_table_count, 1);

        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn reloads_repositories_in_case_insensitive_name_order() {
        let root = fixture_root();
        let database = root.join("registry.db");
        let repositories = ["alpha", "Beta", "charlie", "Delta"]
            .into_iter()
            .rev()
            .map(|name| fixture_repository_named(&root, name))
            .map(|path| scan_repository(&path, None, &[]))
            .collect();
        let state = StoreState {
            repositories,
            ..StoreState::default()
        };

        save_store(&database, &state).expect("mixed-case repository state should persist");
        let reloaded = load_store(&database).expect("mixed-case repository state should reload");
        let reloaded_names = reloaded
            .repositories
            .iter()
            .map(|repository| repository.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(reloaded_names, vec!["alpha", "Beta", "charlie", "Delta"]);

        let snapshot = snapshot_from_store(&database, &state);
        let snapshot_names = snapshot
            .repositories
            .iter()
            .map(|repository| repository.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(snapshot_names, vec!["alpha", "Beta", "charlie", "Delta"]);

        fs::remove_dir_all(root).expect("mixed-case fixture should be removable");
    }

    #[test]
    fn persists_local_configuration_for_roots_products_groups_and_lifecycle() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        let database = root.join("registry.db");
        let root_config = RootConfig {
            id: path_id("root", &root),
            path: root.to_string_lossy().to_string(),
            label: "fixture".to_string(),
            ignore_patterns: Vec::new(),
            refresh_policy: default_refresh_policy(),
            background_monitoring: false,
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config.clone()],
            ..StoreState::default()
        };
        scan_and_persist_scoped(&database, &mut state, None).expect("fixture scan should persist");
        let repository_id = state.repositories[0].id.clone();

        update_root_settings_at(
            &database,
            &root_config.id,
            vec![
                "*.tmp".to_string(),
                "cache".to_string(),
                "cache".to_string(),
            ],
            "Manual",
            true,
        )
        .expect("root settings should persist");
        set_repository_lifecycle_at(&database, &repository_id, "Paused")
            .expect("lifecycle should persist");
        let product_snapshot = upsert_product_at(
            &database,
            None,
            "Public product",
            vec![repository_id.clone()],
            "Unified product version",
        )
        .expect("product should persist");
        let group_snapshot =
            upsert_group_at(&database, None, "Experiments", vec![repository_id.clone()])
                .expect("group should persist");
        set_retention_days_at(&database, 30).expect("retention should persist");

        let persisted = load_store(&database).expect("configured state should load");
        assert_eq!(persisted.roots[0].ignore_patterns, vec!["*.tmp", "cache"]);
        assert_eq!(persisted.roots[0].refresh_policy, "Manual");
        assert!(persisted.roots[0].background_monitoring);
        assert_eq!(persisted.repositories[0].lifecycle, "Paused");
        assert_eq!(persisted.products.len(), 1);
        assert_eq!(
            persisted.products[0].release_mode,
            "Unified product version"
        );
        assert_eq!(persisted.groups.len(), 1);
        assert_eq!(persisted.groups[0].name, "Experiments");
        assert_eq!(product_snapshot.products.len(), 1);
        assert_eq!(group_snapshot.groups.len(), 1);
        assert_eq!(persisted.retention_days, 30);

        let snapshot = snapshot_from_store(&database, &persisted);
        let product_status =
            filter_snapshot_by_collection(snapshot.clone(), Some("PUBLIC PRODUCT"), None)
                .expect("product status should resolve case-insensitively");
        assert_eq!(product_status.repositories.len(), 1);
        assert_eq!(product_status.products.len(), 1);
        assert!(product_status.groups.is_empty());
        let (target_ids, target_label) = resolve_refresh_target(&snapshot, "Experiments")
            .expect("group refresh target should resolve");
        assert_eq!(target_ids, [repository_id.clone()].into_iter().collect());
        assert_eq!(target_label, "Group Experiments");
        let mut refreshed_state = load_store(&database).expect("state should reload");
        let refreshed = audited_scan_and_persist_scoped(
            &database,
            &mut refreshed_state,
            Some(&target_ids),
            Some(&target_label),
        )
        .expect("targeted refresh should persist");
        assert_eq!(refreshed.repositories.len(), 1);
        assert!(refreshed.action_audits[0]
            .target_ids
            .contains(&repository_id));

        delete_product_at(&database, &persisted.products[0].id).expect("product should delete");
        delete_group_at(&database, &persisted.groups[0].id).expect("group should delete");
        assert!(load_store(&database)
            .expect("deleted configuration should load")
            .products
            .is_empty());
        assert!(load_store(&database)
            .expect("deleted configuration should load")
            .groups
            .is_empty());
        fs::remove_dir_all(root).expect("fixture root should be removable");
        let _ = repository;
    }

    #[test]
    fn repository_target_branch_override_persists_and_recomputes_tracking() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["branch", "develop"]);
        let database = root.join("registry.db");
        let snapshot = register_root_and_scan(&database, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();

        assert_eq!(
            snapshot.repositories[0].default_branch.as_deref(),
            Some("main")
        );
        assert_eq!(
            snapshot.repositories[0].target_branch.as_deref(),
            Some("main")
        );
        assert!(!snapshot.repositories[0].target_branch_configured);

        let updated = set_repository_target_branch_at(&database, &repository_id, "develop")
            .expect("a local branch should be configurable as the repository target");
        let configured = &updated.repositories[0];
        assert_eq!(configured.default_branch.as_deref(), Some("main"));
        assert_eq!(configured.target_branch.as_deref(), Some("develop"));
        assert!(configured.target_branch_configured);
        assert_eq!(
            configured.workspace.target_branch.as_deref(),
            Some("develop")
        );
        assert_eq!(configured.workspace.target_confidence, "High");
        assert_eq!(updated.events[0].kind, "state-transition");
        assert!(updated.events[0].fingerprint.contains("|develop|true|"));

        let fold_target = agent_fold_target(configured, None);
        assert_eq!(fold_target.0.as_deref(), Some("develop"));
        assert_eq!(fold_target.1, "Pronto configured repository target");

        let mut persisted = load_store(&database).expect("configured state should reload");
        let refreshed = audited_scan_and_persist(&database, &mut persisted)
            .expect("ordinary refresh should preserve the configured target");
        assert_eq!(
            refreshed.repositories[0].target_branch.as_deref(),
            Some("develop")
        );
        assert!(refreshed.repositories[0].target_branch_configured);
        assert!(set_repository_target_branch_at(&database, &repository_id, "missing").is_err());

        fs::remove_dir_all(root).expect("target branch fixture should be removable");
    }

    #[test]
    fn target_evidence_reuse_requires_matching_branch_head_and_artifacts() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let target_commit = git_static(&repository_path, &["rev-parse", "refs/heads/main"])
            .expect("fixture target head should resolve");
        let target_root = root.join("target-fleet-audit");
        fs::create_dir_all(&target_root).expect("target evidence root should be creatable");

        let mut repository = scan_repository(&repository_path, None, &[]);
        repository.quality.target_fleet_audit_root =
            Some(target_root.to_string_lossy().to_string());
        repository.quality.findings.scanned_branch = Some("main".to_string());
        repository.quality.findings.scanned_commit = Some(target_commit.clone());
        repository.quality.findings.observed_at = Some("2000-01-01T00:00:00Z".to_string());
        repository.quality.findings.freshness = quality::QualityFreshness::Stale;

        assert!(target_evidence_is_reusable(
            &repository,
            "main",
            &target_commit
        ));
        assert!(!target_evidence_is_reusable(
            &repository,
            "main",
            "different-head"
        ));
        assert!(!target_evidence_is_reusable(
            &repository,
            "develop",
            &target_commit
        ));

        repository.quality.target_fleet_audit_root = Some(
            root.join("missing-target-fleet-audit")
                .to_string_lossy()
                .to_string(),
        );
        assert!(!target_evidence_is_reusable(
            &repository,
            "main",
            &target_commit
        ));

        fs::remove_dir_all(root).expect("target evidence fixture should be removable");
    }

    #[test]
    fn repository_target_branch_override_respects_store_write_lock() {
        let root = fixture_root();
        let repository = fixture_repository(&root);
        git(&repository, &["branch", "develop"]);
        let database = root.join("registry.db");
        let snapshot = register_root_and_scan(&database, &root.to_string_lossy())
            .expect("fixture portfolio should scan");
        let repository_id = snapshot.repositories[0].id.clone();
        let _lock = acquire_store_write_lock(&database).expect("fixture lock should succeed");

        let error = set_repository_target_branch_at_with_lock_timeout(
            &database,
            &repository_id,
            "develop",
            StdDuration::from_millis(20),
        )
        .expect_err("target branch writes must not bypass an active store writer");
        assert!(error.contains("Another Pronto write is already in progress"));

        fs::remove_dir_all(root).expect("target branch lock fixture should be removable");
    }

    #[test]
    fn rejects_ignore_patterns_that_escape_repository_scope() {
        assert!(normalize_ignore_patterns(vec!["../secrets".to_string()]).is_err());
        assert!(normalize_ignore_patterns(vec!["nested/cache".to_string()]).is_err());
        assert_eq!(
            normalize_ignore_patterns(vec![
                "/target/".to_string(),
                "target".to_string(),
                "*.tmp".to_string(),
            ])
            .expect("safe patterns should normalize"),
            vec!["*.tmp", "target"]
        );
    }

    #[test]
    fn extracts_compact_analytics_metrics_and_keeps_quality_unavailable() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let repository = scan_repository(&repository_path, None, &[]);
        let observed_at = iso_now();
        let sample = analytics_repository_sample(&repository, &observed_at);

        assert_eq!(sample.repository_count, 1);
        assert_eq!(sample.workspace_count, 1);
        assert!(sample.branch_count >= 1);
        assert_eq!(sample.commits_last_30_days, Some(1));
        assert_eq!(sample.findings_total, None);
        assert_eq!(sample.high_severity_findings, None);

        let encoded = serde_json::to_string(&sample).expect("analytics sample should serialize");
        assert!(!encoded.contains(repository_path.to_string_lossy().as_ref()));
        assert!(!encoded.contains("tracked.txt"));

        let mut known_findings = repository.clone();
        known_findings.quality.findings.source = Some(quality::QualitySource::Qr);
        known_findings.quality.findings.observed_at = Some(observed_at.clone());
        let known_sample = analytics_repository_sample(&known_findings, &observed_at);
        assert_eq!(known_sample.findings_total, Some(0));
        assert_eq!(known_sample.high_severity_findings, Some(0));

        fs::remove_dir_all(root).expect("analytics metric fixture should be removable");
    }

    #[test]
    fn counts_only_local_commits_in_the_trailing_analytics_window() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        fs::write(repository_path.join("tracked.txt"), "one\ntwo\n")
            .expect("tracked file should be updated");
        git(&repository_path, &["add", "tracked.txt"]);
        git(&repository_path, &["commit", "-m", "Second fixture"]);

        let repository = scan_repository(&repository_path, None, &[]);
        let sample = analytics_repository_sample(&repository, &iso_now());
        assert_eq!(sample.commits_last_30_days, Some(2));

        fs::remove_dir_all(root).expect("commit metric fixture should be removable");
    }

    #[test]
    fn deduplicates_unchanged_samples_and_prunes_by_retention() {
        let root = fixture_root();
        let repository_path = fixture_repository(&root);
        let database = root.join("registry.db");
        let mut state = StoreState::default();
        state.repositories = vec![scan_repository(&repository_path, None, &[])];
        save_store(&database, &state).expect("analytics fixture state should persist");

        let base = Utc::now() - chrono::Duration::minutes(20);
        let first = base.to_rfc3339();
        let second = (base + chrono::Duration::minutes(5)).to_rfc3339();
        let third = (base + chrono::Duration::minutes(16)).to_rfc3339();
        record_analytics_samples_at(&database, &state, &first)
            .expect("first analytics sample should persist");
        record_analytics_samples_at(&database, &state, &second)
            .expect("unchanged analytics sample should deduplicate");

        let connection = open_store(&database).expect("analytics database should open");
        let fleet_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE repository_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("fleet sample count should be readable");
        assert_eq!(fleet_count, 1);
        let repository_id = state.repositories[0].id.clone();
        let repository_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE repository_id = ?1",
                params![analytics_scope_id(&repository_id)],
                |row| row.get(0),
            )
            .expect("repository sample count should be readable");
        assert_eq!(repository_count, 1);
        let stored_scope: String = connection
            .query_row(
                "SELECT repository_id FROM analytics_samples WHERE repository_id IS NOT NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("repository analytics scope should be readable");
        assert!(!stored_scope.contains(root.to_string_lossy().as_ref()));
        assert_ne!(stored_scope, repository_id);
        drop(connection);

        record_analytics_samples_at(&database, &state, &third)
            .expect("sample outside the deduplication window should persist");
        let analytics = load_analytics_at(&database).expect("analytics should load");
        assert_eq!(analytics.portfolio_samples.len(), 2);
        assert_eq!(analytics.repositories[0].samples.len(), 2);

        let old_observed_at = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        let old_payload = serde_json::to_string(&analytics_portfolio_sample(
            &state.repositories,
            &old_observed_at,
        ))
        .expect("old analytics sample should serialize");
        let connection = open_store(&database).expect("analytics database should reopen");
        connection
            .execute(
                "INSERT INTO analytics_samples (id, repository_id, observed_at, payload_json)
                 VALUES (?1, NULL, ?2, ?3)",
                params!["old-analytics-sample", old_observed_at, old_payload],
            )
            .expect("old analytics sample should insert");
        drop(connection);

        prune_analytics_samples(&database, 1).expect("retention pruning should succeed");
        let connection = open_store(&database).expect("pruned analytics database should open");
        let old_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM analytics_samples WHERE id = 'old-analytics-sample'",
                [],
                |row| row.get(0),
            )
            .expect("old analytics row count should be readable");
        assert_eq!(old_count, 0);

        fs::remove_dir_all(root).expect("analytics retention fixture should be removable");
    }
}
