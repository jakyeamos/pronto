use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const STORE_VERSION: u8 = 1;
const SQLITE_SCHEMA_VERSION: i64 = 2;
const DEFAULT_RETENTION_DAYS: i64 = 90;
const DEFAULT_MAX_UNTRACKED_BYTES: u64 = 2_000_000;

static NEXT_ACTION_AUDIT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    pub id: String,
    pub path: String,
    pub label: String,
    pub ignore_patterns: Vec<String>,
    pub registered_at: String,
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
    pub expected_conditions: Vec<ExpectedCondition>,
    pub events: Vec<EventRecord>,
    #[serde(default)]
    pub action_audits: Vec<ActionAudit>,
    pub retention_days: i64,
}

impl Default for StoreState {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            roots: Vec::new(),
            repositories: Vec::new(),
            expected_conditions: Vec::new(),
            events: Vec::new(),
            action_audits: Vec::new(),
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub roots: Vec<RootConfig>,
    pub repositories: Vec<RepositorySnapshot>,
    pub events: Vec<EventRecord>,
    pub action_audits: Vec<ActionAudit>,
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
                 registered_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS repositories (
                 id TEXT PRIMARY KEY,
                 payload_json TEXT NOT NULL
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
             );",
        )
        .map_err(|error| format!("Could not initialize Pronto database: {error}"))?;

    let schema_version = metadata_value(connection, "schema_version")?;
    match schema_version {
        Some(value) => {
            let version = value.parse::<i64>().map_err(|error| {
                format!("Could not parse Pronto database schema version: {error}")
            })?;
            if version == 1 {
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

    if metadata_value(connection, "store_version")?.is_none() {
        connection
            .execute(
                "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
                params!["store_version", STORE_VERSION.to_string()],
            )
            .map_err(|error| format!("Could not record Pronto store version: {error}"))?;
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
        if let Some(legacy_state) = load_legacy_store(path)? {
            save_store(path, &legacy_state)?;
            return Ok(legacy_state);
        }
    }

    let connection = open_store(path)?;
    let version = metadata_value(&connection, "store_version")?
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(STORE_VERSION);
    let retention_days = metadata_value(&connection, "retention_days")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS);

    let root_rows = connection
        .prepare(
            "SELECT id, path, label, ignore_patterns_json, registered_at
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
            ))
        })
        .map_err(|error| format!("Could not read Pronto roots: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto roots: {error}"))?;
    let roots = root_rows
        .into_iter()
        .map(|(id, path, label, ignore_patterns_json, registered_at)| {
            let ignore_patterns = serde_json::from_str(&ignore_patterns_json)
                .map_err(|error| format!("Could not decode root ignore patterns: {error}"))?;
            Ok(RootConfig {
                id,
                path,
                label,
                ignore_patterns,
                registered_at,
            })
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
        expected_conditions,
        events,
        action_audits,
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
        "expected_conditions",
        "events",
        "action_audits",
    ] {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| format!("Could not clear Pronto {table} table: {error}"))?;
    }

    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["store_version", state.version.to_string()],
        )
        .map_err(|error| format!("Could not save Pronto store version: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params!["retention_days", state.retention_days.to_string()],
        )
        .map_err(|error| format!("Could not save Pronto retention setting: {error}"))?;

    for root in &state.roots {
        let ignore_patterns_json = serde_json::to_string(&root.ignore_patterns)
            .map_err(|error| format!("Could not encode root ignore patterns: {error}"))?;
        transaction
            .execute(
                "INSERT INTO roots (id, path, label, ignore_patterns_json, registered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    root.id,
                    root.path,
                    root.label,
                    ignore_patterns_json,
                    root.registered_at
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

    transaction
        .commit()
        .map_err(|error| format!("Could not commit Pronto database transaction: {error}"))
}

fn snapshot_from_store(path: &Path, state: &StoreState) -> PortfolioSnapshot {
    PortfolioSnapshot {
        roots: state.roots.clone(),
        repositories: state.repositories.clone(),
        events: state.events.iter().rev().take(24).cloned().collect(),
        action_audits: state.action_audits.iter().rev().take(24).cloned().collect(),
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
    default_ignore(name)
        || patterns.iter().any(|pattern| {
            let trimmed = pattern.trim_matches('/');
            trimmed == name
                || (trimmed.starts_with('*') && name.ends_with(trimmed.trim_start_matches('*')))
        })
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
        if workspace.operation.is_some() || workspace.dirty {
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
    let (role, role_confidence) = branch_role(&status.branch, default_branch);
    let (target_branch, target_confidence) = target_for_branch(&status.branch, default_branch);
    let integration_state = branch_integration_state(path, &status.branch, default_branch, None);
    WorkspaceSummary {
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
    }
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
        let (role, role_confidence) = branch_role(&record.name, default_branch.as_deref());
        let (target_branch, target_confidence) =
            target_for_branch(&record.name, default_branch.as_deref());
        let current_workspace = workspaces
            .iter()
            .find(|workspace| workspace.branch == record.name);
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
    primary_for_conditions.target_branch =
        target_for_branch(&primary.branch, default_branch.as_deref()).0;
    let conditions = build_conditions(
        &repository_id,
        &primary_for_conditions,
        default_branch.as_deref(),
        expected,
        &observed_at,
    );
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
        "{}|{}|{}|{}|{}|{}",
        repository.branch,
        repository.workspace.dirty,
        repository.workspace.added,
        repository.workspace.removed,
        repository.workspace.sync_state,
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

fn unavailable_repository(old: &RepositorySnapshot) -> RepositorySnapshot {
    let mut repository = old.clone();
    let now = iso_now();
    repository.locality = "Unavailable".to_string();
    repository.provider_state = "Local path unavailable".to_string();
    repository.last_scan_at = now.clone();
    repository.conditions = vec![Condition {
        id: format!("{}:unavailable", repository.id),
        kind: "unavailable".to_string(),
        title: "Local path unavailable".to_string(),
        summary: "The registered workspace path could not be scanned.".to_string(),
        priority: 1,
        status: "Active".to_string(),
        fingerprint: "unavailable".to_string(),
        rule: "A registered local path no longer exists or is inaccessible.".to_string(),
        evidence: vec![evidence(
            "Path",
            repository.path.clone(),
            "Local registry",
            &now,
        )],
        missing: vec!["Restore access to the path or remove the root from settings.".to_string()],
        confidence: Some("High".to_string()),
        freshness: None,
    }];
    repository
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
    let preflight = build_action_preflight(state, "refresh", None)?;
    if !preflight.allowed {
        return Err("Local refresh action is not permitted".to_string());
    }
    let audit_id = preflight.audit.id.clone();
    append_action_audit(state, &preflight);
    save_store(path, state)?;

    match scan_and_persist(path, state) {
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

fn scan_and_persist(path: &Path, state: &mut StoreState) -> Result<PortfolioSnapshot, String> {
    let mut discovered = HashMap::<String, PathBuf>::new();
    for root in &state.roots {
        for repository in discover_repositories(root) {
            discovered.insert(path_id("repository", &repository), repository);
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
            && !Path::new(&old.path).exists()
        {
            let repository = unavailable_repository(&old);
            append_transition_event(state, Some(&old), &repository);
            repositories.push(repository);
        }
    }
    repositories.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    state.repositories = repositories;
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
            registered_at: iso_now(),
        });
    }
    audited_scan_and_persist(path, &mut state)
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
pub fn refresh() -> Result<PortfolioSnapshot, String> {
    let path = store_path();
    let mut state = load_store(&path)?;
    audited_scan_and_persist(&path, &mut state)
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
    let result = match command {
        "status" => load_store(&path).map(|state| snapshot_from_store(&path, &state)),
        "refresh" => {
            load_store(&path).and_then(|mut state| audited_scan_and_persist(&path, &mut state))
        }
        "." | "open" => {
            println!("Pronto desktop focus is available from an installed bundle.");
            return;
        }
        _ => {
            eprintln!("Usage: pronto status [--json] | pronto refresh | pronto .");
            return;
        }
    };
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
            registered_at: iso_now(),
        };
        let mut state = StoreState {
            roots: vec![root_config],
            ..StoreState::default()
        };

        let first = scan_and_persist(&store, &mut state).expect("first scan should persist");
        assert_eq!(first.repositories.len(), 1);
        assert_eq!(first.events.len(), 1);

        let second = scan_and_persist(&store, &mut state).expect("second scan should persist");
        assert_eq!(second.events.len(), 1);

        fs::write(repository.join("tracked.txt"), "one\nupdated\n")
            .expect("tracked file should be writable");
        let third = scan_and_persist(&store, &mut state).expect("changed scan should persist");
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
        let action_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'action_audits'",
                [],
                |row| row.get(0),
            )
            .expect("action audit table should exist after migration");
        assert_eq!(action_table, "action_audits");
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
                registered_at: iso_now(),
            }],
            ..StoreState::default()
        };
        let encoded = serde_json::to_string_pretty(&state).expect("legacy state should serialize");
        fs::write(&legacy, encoded).expect("legacy state should be writable");

        let migrated = load_store(&database).expect("legacy state should migrate");
        assert_eq!(migrated.roots.len(), 1);
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
}
