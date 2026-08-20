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
    #[serde(default)]
    pub workspace_warnings: Vec<String>,
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
    pub developer_legibility: Option<AgentMaturityGateSummary>,
    pub change_surface_hotspots: Option<AgentMaturityGateSummary>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMaturityGateSummary {
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
