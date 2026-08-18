use crate::change_matrix;

use crate::mac_control_maturity;

use crate::papercuts;

use crate::project_compass::{self, ProjectCompassSummary};

use crate::quality::{
    self, QualityFreshness, QualityGateRequirement, QualityGateStatus, QualityPortfolioSnapshot,
    QualitySnapshot,
};

use crate::remediation::{self, RemediationRun};

use crate::showcase::{self, ShowcasePortfolioSnapshot};

use crate::skills::{self, SkillsSnapshot};

use crate::task_lanes::{self, TaskLaneReport};

use chrono::{DateTime, Duration, SecondsFormat, Utc};

use rusqlite::{params, Connection as SqliteConnection, OpenFlags, OptionalExtension, Row};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use serde_json::Value;

use sha2::{Digest, Sha256};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use std::ffi::{OsStr, OsString};

use std::fs::{self, OpenOptions};

use std::io::{Read, Write};

use std::path::{Path, PathBuf};

use std::process::{Command, Stdio};

use std::sync::atomic::{AtomicU64, Ordering};

use std::sync::mpsc;

use std::sync::{Arc, Mutex};

use std::thread;

use std::time::{Duration as StdDuration, Instant};

const STORE_VERSION: u8 = 5;

const SQLITE_SCHEMA_VERSION: i64 = 11;

const DEFAULT_RETENTION_DAYS: i64 = 90;

const DEFAULT_MAX_UNTRACKED_BYTES: u64 = 2_000_000;

const DEFAULT_MAX_MANIFEST_BYTES: u64 = 64 * 1024;

const MAX_AI_DIFF_BYTES: usize = 2_000_000;

const ANALYTICS_SCHEMA: &str = "pronto-analytics/v2";

const ANALYTICS_RANGE_DAYS: i64 = 30;

const ANALYTICS_MIN_RANGE_DAYS: i64 = 1;

const ANALYTICS_DEDUP_MINUTES: i64 = 15;

const DEFAULT_QR_AUDIT_TIMEOUT_SECONDS: u64 = 120;

const STORE_WRITE_LOCK_WAIT_SECONDS: u64 = 5;

const STORE_WRITE_LOCK_STALE_SECONDS: u64 = 1_800;

// A full fleet detector import can legitimately take tens of seconds to project.
// Keep the route bounded, but leave enough headroom for the registered fleet.
const QUALITY_READ_TIMEOUT_SECONDS: u64 = 60;

const DEFAULT_REFRESH_BATCH_PARALLELISM: usize = 4;

const MAX_REFRESH_BATCH_PARALLELISM: usize = 32;

const MAX_REFRESH_BATCH_CONFLICT_RETRIES: usize = 1;

const RELEASE_GIT_TIMEOUT_SECONDS: u64 = 10;

const RELEASE_COMMIT_LIMIT: usize = 1_000;

const TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS: u64 = 120;

const TARGET_EVIDENCE_TOTAL_TIMEOUT_SECONDS: u64 = 600;

const DEFAULT_FLEET_DETECTOR_TIMEOUT_SECONDS: u64 = 600;

const WORKSPACE_ROLE_MAP_SCHEMA: &str = "workspace-role-map/v1";

const WORKSPACE_FLEET_MANIFEST_SCHEMA: &str = "workspace-fleet-manifest/v1";

const WORKSPACE_POLICY_GENERATION_SCHEMA: &str = "workspace-policy-generation/v1";

const CI_RUN_LIMIT: usize = 20;

const CI_ARTIFACT_LOOKUP_LIMIT: usize = 8;

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

/// Execute a local store read-modify-write operation under one file lock.
///
/// The state must be loaded after the lock is acquired. Loading it before the
/// lock allows two writers to persist stale snapshots after the first writer
/// has already committed a newer audit, event, or repository projection.
fn with_store_write_state<T, F>(path: &Path, operation: F) -> Result<T, String>
where
    F: FnOnce(&mut StoreState) -> Result<T, String>,
{
    let _lock = acquire_store_write_lock(path)?;
    let mut state = load_store(path)?;
    operation(&mut state)
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
    #[serde(default)]
    pub ci_runs: Vec<CiRunSnapshot>,
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
pub struct CiJobSnapshot {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: Option<String>,
    #[serde(default)]
    pub failed_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CiPromptArtifactSnapshot {
    pub id: u64,
    pub name: String,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CiRunSnapshot {
    pub id: u64,
    pub workflow_name: String,
    pub workflow_path: Option<String>,
    pub display_title: String,
    pub run_number: u64,
    pub run_attempt: u64,
    pub event: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: String,
    pub html_url: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub pull_request_number: Option<u64>,
    pub is_fork: bool,
    #[serde(default)]
    pub jobs: Vec<CiJobSnapshot>,
    pub failure_summary: Option<String>,
    pub failure_signature: Option<String>,
    pub prompt_artifact: Option<CiPromptArtifactSnapshot>,
    pub last_refreshed_at: String,
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

fn default_workspace_provenance_kind() -> String {
    "unknown".to_string()
}

fn default_workspace_cleanup_state() -> String {
    "unknown".to_string()
}
