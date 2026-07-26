use crate::quality::{
    self, QualityFreshness, QualityGateRequirement, QualityGateStatus, QualityPortfolioSnapshot,
    QualitySnapshot,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const STORE_VERSION: u8 = 4;
const SQLITE_SCHEMA_VERSION: i64 = 5;
const DEFAULT_RETENTION_DAYS: i64 = 90;
const DEFAULT_MAX_UNTRACKED_BYTES: u64 = 2_000_000;
const DEFAULT_MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_AI_DIFF_BYTES: usize = 2_000_000;

static NEXT_ACTION_AUDIT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_CONFIG_ID: AtomicU64 = AtomicU64::new(0);

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
pub struct WorkspaceSummary {
    pub id: String,
    pub path: String,
    pub is_primary: bool,
    pub branch: String,
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
    pub retention_days: i64,
    pub generated_at: String,
    pub storage_path: String,
}

#[derive(Debug)]
struct GitOutput {
    success: bool,
    stdout: String,
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

fn metadata_value(connection: &Connection, key: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("Could not read Pronto database metadata: {error}"))
}

fn table_has_column(connection: &Connection, table: &str, column: &str) -> Result<bool, String> {
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

fn initialize_store(connection: &Connection) -> Result<(), String> {
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

fn open_store(path: &Path) -> Result<Connection, String> {
    ensure_store_parent(path)?;
    let connection = Connection::open(path)
        .map_err(|error| format!("Could not open local Pronto database: {error}"))?;
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
            legacy_state.remote_repositories = filter_linked_remote_repositories(
                &legacy_state.repositories,
                legacy_state.remote_repositories,
            );
            legacy_state.provider_status.repository_count = legacy_state.remote_repositories.len();
            save_store(path, &legacy_state)?;
            return Ok(legacy_state);
        }
    }

    let connection = open_store(path)?;
    let version = metadata_value(&connection, "store_version")?
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(STORE_VERSION)
        .max(STORE_VERSION);
    let retention_days = metadata_value(&connection, "retention_days")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS);
    let mut provider_status = match metadata_value(&connection, "provider_status_json")? {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode provider status: {error}"))?,
        None => ProviderStatus::default(),
    };
    let quality = match metadata_value(&connection, "quality_summary_json")? {
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
    let repositories = repository_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode repository snapshot: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let remote_repositories = filter_linked_remote_repositories(&repositories, remote_repositories);
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
        retention_days,
    })
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

    transaction
        .commit()
        .map_err(|error| format!("Could not commit Pronto database transaction: {error}"))
}

fn snapshot_from_store(path: &Path, state: &StoreState) -> PortfolioSnapshot {
    PortfolioSnapshot {
        roots: state.roots.clone(),
        repositories: state.repositories.clone(),
        products: state.products.clone(),
        groups: state.groups.clone(),
        events: state.events.iter().rev().take(24).cloned().collect(),
        action_audits: state.action_audits.iter().rev().take(24).cloned().collect(),
        provider_identities: state.provider_identities.clone(),
        remote_repositories: state.remote_repositories.clone(),
        provider_status: state.provider_status.clone(),
        quality: state.quality.clone(),
        retention_days: state.retention_days,
        generated_at: iso_now(),
        storage_path: path.to_string_lossy().to_string(),
    }
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
}

impl Default for GitHubCliAdapter {
    fn default() -> Self {
        Self {
            executable: "gh".to_string(),
        }
    }
}

impl GitHubCliAdapter {
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
        for repository in &repositories {
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

fn filter_linked_remote_repositories(
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
            if !local_names.contains(&normalized_name) {
                return None;
            }
            remote.locality = "Local and remote".to_string();
            Some(remote)
        })
        .collect()
}

fn apply_provider_refresh_at(
    path: &Path,
    refresh: ProviderRefresh,
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
    let remote_repositories =
        filter_linked_remote_repositories(&state.repositories, refresh.repositories);
    state.provider_identities = refresh.identities;
    state.remote_repositories = remote_repositories;
    state.provider_status = ProviderStatus {
        provider: "GitHub".to_string(),
        state: "Ready".to_string(),
        message: "Read-only GitHub context refreshed for connected local repositories.".to_string(),
        last_refresh_at: Some(refresh.refreshed_at),
        identity_count: state.provider_identities.len(),
        repository_count: state.remote_repositories.len(),
    };
    apply_quality_evidence(&mut state);
    apply_release_threshold_conditions(&mut state);
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

fn apply_quality_evidence(state: &mut StoreState) {
    let audit = quality::audit_import(
        state.quality.audit_root.as_deref().map(Path::new),
        &state.repositories,
    );
    state.quality = audit.portfolio;
    let remote_by_name = state
        .remote_repositories
        .iter()
        .filter_map(|remote| normalize_remote_name(&remote.full_name).map(|name| (name, remote)))
        .collect::<HashMap<_, _>>();
    for repository in &mut state.repositories {
        let remote = repository
            .remote_url
            .as_deref()
            .and_then(normalize_remote_name)
            .and_then(|name| remote_by_name.get(&name).copied());
        let maturity = audit.maturities.get(&repository.id).cloned();
        repository.quality = quality::ingest_repository_quality(repository, remote, maturity);
    }
    quality::update_ci_readiness_summary(&mut state.quality, &state.repositories);
}

fn refresh_github_at(path: &Path) -> Result<PortfolioSnapshot, String> {
    let adapter = GitHubCliAdapter::default();
    match adapter.refresh() {
        Ok(refresh) => apply_provider_refresh_at(path, refresh),
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
    ParsedStatus,
    DiffTotals,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let status_output = run_git(
        path,
        [
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ]
        .iter(),
    )
    .map(|result| result.stdout)
    .unwrap_or_default();
    let status = parse_status(&status_output);
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
        "bash",
        "zsh",
        "fish",
        "sh",
        "pwsh",
        "powershell",
        "cmd",
        "terminal",
        "iterm",
        "warp",
        "alacritty",
        "kitty",
        "wezterm",
        "codex",
        "claude",
        "aider",
        "cursor",
        "continue",
        "opencode",
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
            .args(["-axo", "pid=,comm=,lstart="])
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
        let mut signals = Vec::new();
        let mut unresolved_candidate = false;
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut fields = line.split_whitespace();
            let Some(process_id) = fields.next().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };
            let Some(process_name) = fields.next() else {
                continue;
            };
            if !process_name_is_activity_candidate(process_name) {
                continue;
            }
            let started_at = fields.collect::<Vec<_>>().join(" ");
            let started_at = (!started_at.is_empty()).then_some(started_at);
            let Some(working_directory) = process_working_directory(process_id) else {
                unresolved_candidate = true;
                continue;
            };
            if workspace_contains(path, &working_directory) {
                signals.push(activity_signal(
                    "Process",
                    "Process evidence found",
                    "Medium",
                    Some(process_name),
                    Some(process_id),
                    started_at.as_deref(),
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
            if workspace.dirty {
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
        .default_branch
        .clone()
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
            "Local default branch and workspace target",
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
    if workspace.dirty {
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
    let starting_state_ready =
        !workspace.dirty && workspace.operation.is_none() && workspace.activity.state != "Active";
    let release_evidence_ready = release.status == "Evidence ready";
    let version_confirmed = release.version_status == "Candidate version confirmed";
    let has_release_changes =
        !recipe.release_commands.is_empty() || !recipe.generated_paths.is_empty();
    let has_validation = !recipe.validation_commands.is_empty();
    let mut reasons = release.reasons.clone();
    if !starting_state_ready {
        if workspace.dirty {
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

fn branch_integration_state(
    path: &Path,
    branch: &str,
    default_branch: Option<&str>,
    current_workspace: Option<&WorkspaceSummary>,
) -> String {
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
    existing: Option<&RepositorySnapshot>,
) -> WorkspaceSummary {
    let (status, totals, operation, last_commit, last_commit_at, last_activity_at) =
        workspace_status(path);
    let remote_freshness = existing
        .and_then(|repository| repository.last_fetch_at.clone())
        .unwrap_or_else(|| "Not fetched by Pronto".to_string());
    let activity = collect_workspace_activity(path, status.dirty, status.ahead);
    let sync_state = if status.upstream.is_none() {
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
    let (mut role, mut role_confidence) = branch_role(&status.branch, default_branch);
    let (mut target_branch, mut target_confidence) =
        target_for_branch(&status.branch, default_branch);
    if let Some(manifest) = activity.manifest.as_ref() {
        if let Some(manifest_target) = manifest.target_branch.as_ref() {
            target_branch = Some(manifest_target.clone());
            target_confidence = "High".to_string();
        }
        if manifest.agent_type.is_some() && role != "Production" {
            role = "Agent task".to_string();
            role_confidence = "High".to_string();
        }
    }
    let integration_state = branch_integration_state(path, &status.branch, default_branch, None);
    let mut workspace = WorkspaceSummary {
        id: path_id("workspace", path),
        path: path.to_string_lossy().to_string(),
        is_primary,
        branch: status.branch,
        dirty: status.dirty,
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
    };
    workspace.integration_state =
        branch_integration_state(path, &workspace.branch, default_branch, Some(&workspace));
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
    if workspace.upstream.is_none()
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

fn scan_repository(
    path: &Path,
    existing: Option<&RepositorySnapshot>,
    expected: &[ExpectedCondition],
) -> RepositorySnapshot {
    let observed_at = iso_now();
    let repository_id = path_id("repository", path);
    let remote_url = git_static(path, &["remote", "get-url", "origin"]);
    let provider_state = if remote_url
        .as_ref()
        .is_some_and(|url| url.contains("github.com"))
    {
        "GitHub remote detected · provider not connected".to_string()
    } else if remote_url.is_some() {
        "Remote detected · provider not connected".to_string()
    } else {
        "No remote configured".to_string()
    };
    let locality = if remote_url.is_some() {
        "Connected"
    } else {
        "Local only"
    };
    let worktree_records = parse_worktrees(path);
    let provisional_branch = git_static(path, &["branch", "--show-current"])
        .unwrap_or_else(|| "Detached HEAD".to_string());
    let default_branch = detect_default_branch(path, &provisional_branch);
    let existing_last_fetch = existing.and_then(|repository| repository.last_fetch_at.clone());
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
        let mut workspace =
            scan_workspace(&canonical, is_primary, default_branch.as_deref(), existing);
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
        let (target_branch, target_confidence) = current_workspace
            .map(|workspace| {
                (
                    workspace.target_branch.clone(),
                    workspace.target_confidence.clone(),
                )
            })
            .unwrap_or_else(|| target_for_branch(&record.name, default_branch.as_deref()));
        let integration_state = branch_integration_state(
            path,
            &record.name,
            default_branch.as_deref(),
            current_workspace,
        );
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
            target_branch,
            target_confidence,
            ahead,
            behind,
            integration_state,
            workspace_id,
            last_commit: record.last_commit,
            last_commit_at: record.last_commit_at,
        });
    }
    let mut primary_for_conditions = primary.clone();
    primary_for_conditions.integration_state = branch_integration_state(
        path,
        &primary.branch,
        default_branch.as_deref(),
        Some(&primary),
    );
    let conditions = build_conditions(
        &repository_id,
        &primary_for_conditions,
        default_branch.as_deref(),
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
        "{}|{}|{}|{}|{}|{}|{}",
        repository.branch,
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
    repositories.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    state.repositories = repositories;
    apply_quality_evidence(state);
    apply_release_threshold_conditions(state);
    prune_events(state);
    save_store(path, state)?;
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
    let normalized_root = match audit_root.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => {
            let root = fs::canonicalize(value)
                .map_err(|error| format!("Could not access the maturity audit root: {error}"))?;
            if !root.is_dir() {
                return Err("The maturity audit root is not a folder".to_string());
            }
            Some(root.to_string_lossy().to_string())
        }
        None => None,
    };
    let mut state = load_store(path)?;
    state.quality.audit_root = normalized_root;
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
    let state = load_store(path)?;
    let mut allowed_roots = Vec::new();
    if let Some(audit_root) = state.quality.audit_root.as_deref() {
        allowed_roots.push(PathBuf::from(audit_root));
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
pub fn refresh_github() -> Result<PortfolioSnapshot, String> {
    refresh_github_at(&store_path())
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

pub fn run_cli(arguments: Vec<String>) {
    let command = arguments.first().map(String::as_str).unwrap_or("status");
    let json = arguments.iter().any(|argument| argument == "--json");
    let path = store_path();
    match command {
        "status" => {
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
                eprintln!("Usage: pronto status [--product <name> | --group <name>] [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path)
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
                    eprintln!("Pronto could not read local state: {error}");
                    std::process::exit(1);
                }
            }
        }
        "refresh" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() > 1 {
                eprintln!("Usage: pronto refresh [repository|group|product] [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|mut state| {
                if let Some(target) = positionals.first() {
                    let current = snapshot_from_store(&path, &state);
                    let (repository_ids, label) = resolve_refresh_target(&current, target)?;
                    let snapshot = audited_scan_and_persist_scoped(
                        &path,
                        &mut state,
                        Some(&repository_ids),
                        Some(&label),
                    )?;
                    Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
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
            if !positionals.is_empty() {
                eprintln!("Usage: pronto refresh-github [--json]");
                std::process::exit(2);
            }
            match refresh_github_at(&path) {
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
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() != 2 || positionals[0] != "audit-root" {
                eprintln!("Usage: pronto quality audit-root <folder|clear> [--json]");
                std::process::exit(2);
            }
            let audit_root = if positionals[1].eq_ignore_ascii_case("clear") {
                None
            } else {
                Some(positionals[1].as_str())
            };
            match set_maturity_audit_root_at(&path, audit_root) {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => {
                    println!(
                        "Maturity audit root: {}",
                        snapshot
                            .quality
                            .audit_root
                            .as_deref()
                            .unwrap_or("Not configured")
                    );
                    println!(
                        "Maturity audit: {} · {} matched · fleet mean {}",
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
                    eprintln!("Pronto could not configure the maturity audit root: {error}");
                    std::process::exit(1);
                }
            }
        }
        "root" => {
            let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            if positionals.len() == 2 && positionals[0] == "add" {
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
            eprintln!(
                "Usage: pronto . | pronto root add <folder> [--json] | pronto root exclude <folder> <name>... [--json] | pronto status [--product <name> | --group <name>] [--json] | pronto open <repository> | pronto refresh [repository|group|product] [--json] | pronto refresh-github [--json] | pronto quality audit-root <folder|clear> [--json] | pronto clone <owner/repository> [--json]"
            );
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
        let repository = root.join("portfolio-repository");
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

        let workspace = scan_workspace(&repository, true, Some("main"), None);
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
        let mut workspace = scan_workspace(&repository, true, Some("main"), None);
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
        let snapshot = apply_provider_refresh_at(&database, refresh)
            .expect("provider refresh should persist locally");

        assert_eq!(snapshot.provider_status.state, "Ready");
        assert_eq!(snapshot.provider_identities.len(), 1);
        assert_eq!(snapshot.remote_repositories.len(), 1);
        assert_eq!(
            snapshot.remote_repositories[0].full_name,
            "acme/portfolio-repository"
        );
        assert_eq!(snapshot.remote_repositories[0].locality, "Local and remote");
        assert_eq!(snapshot.repositories[0].locality, "Local and remote");
        assert_eq!(
            snapshot.repositories[0].provider_state,
            "GitHub connected as github:jakyeamos"
        );

        let persisted = load_store(&database).expect("provider snapshot should reload");
        assert_eq!(persisted.provider_status.state, "Ready");
        assert_eq!(persisted.remote_repositories.len(), 1);
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
        let connection = Connection::open(&database).expect("schema fixture should open");
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
        let connection = Connection::open(&database).expect("migrated database should open");
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

        let connection = Connection::open(&database).expect("database should open");
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("schema version should be recorded");
        assert_eq!(schema_version, SQLITE_SCHEMA_VERSION.to_string());

        fs::remove_dir_all(root).expect("fixture root should be removable");
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
}
