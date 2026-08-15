use chrono::{Datelike, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const SCHEMA_VERSION: &str = "pronto-papercuts/v2";
const OBSERVATION_CONTRACT_VERSION: &str = "pronto-papercuts-observation/v1";
const FAMILY: &str = "design-audit";
const EXCERPT_RETENTION_DAYS: i64 = 90;
const EXCERPT_MAX_CHARS: usize = 240;
const FINGERPRINT_VERSION: &str = "v1";
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

const SIGNAL_KINDS: &[&str] = &[
    "dissatisfaction",
    "correction",
    "boundary_correction",
    "failure_report",
    "failed_verification",
    "repeated_failure",
    "agent_suggestion",
    "capability_gap",
    "manual_handoff",
    "legacy_manual",
];

const TARGET_KINDS: &[&str] = &[
    "agent_answer",
    "workflow",
    "tool",
    "repository",
    "artifact",
    "user_preference_model",
    "other",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PapercutObservationInput {
    pub event_key: String,
    pub scope_id: String,
    pub scope_kind: String,
    pub domain: String,
    pub signal_kind: String,
    pub target_kind: String,
    pub summary: String,
    pub excerpt: Option<String>,
    pub source: String,
    pub evidence_refs: Vec<String>,
    pub phenomenon_key: String,
    pub failure_mode: String,
    pub priority: String,
    pub urgent: bool,
    pub verified: bool,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PapercutObservationContract {
    pub schema_version: String,
    pub signal_kinds: Vec<String>,
    pub target_kinds: Vec<String>,
    pub minimal_input: serde_json::Value,
}

fn observation_contract() -> PapercutObservationContract {
    let mut signal_kinds = SIGNAL_KINDS
        .iter()
        .filter(|kind| **kind != "legacy_manual")
        .map(|kind| (*kind).to_string())
        .collect::<Vec<_>>();
    signal_kinds.sort();
    let mut target_kinds = TARGET_KINDS
        .iter()
        .map(|kind| (*kind).to_string())
        .collect::<Vec<_>>();
    target_kinds.sort();
    PapercutObservationContract {
        schema_version: OBSERVATION_CONTRACT_VERSION.to_string(),
        signal_kinds,
        target_kinds,
        minimal_input: serde_json::json!({
            "event_key": "v1:example:opaque-event",
            "scope_id": "opaque:v1:example-scope",
            "signal_kind": "capability_gap",
            "target_kind": "tool",
            "summary": "Sanitized factual summary",
            "failure_mode": "stable-failure-mode",
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PapercutObservation {
    pub id: String,
    pub event_key: String,
    pub scope_id: String,
    pub scope_kind: String,
    pub domain: String,
    pub signal_kind: String,
    pub target_kind: String,
    pub summary: String,
    pub excerpt: Option<String>,
    pub excerpt_hash: String,
    pub excerpt_expires_at: Option<String>,
    pub source: String,
    pub evidence_refs: Vec<String>,
    pub phenomenon_key: String,
    pub failure_mode: String,
    pub priority: String,
    pub urgent: bool,
    pub verified: bool,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PapercutPattern {
    pub id: String,
    pub fingerprint: String,
    pub fingerprint_version: String,
    pub scope_kind: String,
    pub scope_id: Option<String>,
    pub title: String,
    pub detail: String,
    pub domain: String,
    pub target_kind: String,
    pub phenomenon_key: String,
    pub failure_mode: String,
    pub surface: String,
    pub source: String,
    pub evidence_refs: Vec<String>,
    pub impact: String,
    pub priority: String,
    pub status: String,
    pub next_action: String,
    pub evidence_tier: String,
    pub occurrence_count: usize,
    pub scope_count: usize,
    pub first_observed_at: String,
    pub last_observed_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MultiplierProposalInput {
    pub pattern_ids: Vec<String>,
    pub title: String,
    pub hypothesis: String,
    pub root_cause: String,
    pub multiplier: String,
    pub evidence_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultiplierProposal {
    pub id: String,
    pub pattern_ids: Vec<String>,
    pub title: String,
    pub hypothesis: String,
    pub root_cause: String,
    pub multiplier: String,
    pub evidence_tier: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PapercutDigest {
    pub id: String,
    pub week_start: String,
    pub week_end: String,
    pub generated_at: String,
    pub observation_count: usize,
    pub local_pattern_count: usize,
    pub cross_scope_pattern_count: usize,
    pub draft_proposal_count: usize,
    pub top_patterns: Vec<PapercutPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PapercutCaptureDiagnostic {
    pub error_code: String,
    pub failure_kind: String,
    pub stage: String,
    pub message: String,
    pub operation: String,
    pub observed_at: String,
    pub retryable: bool,
    pub recovery_command: String,
    pub attempt: usize,
    pub timeout_seconds: Option<u64>,
    pub exit_code: Option<i32>,
}

impl Default for PapercutCaptureDiagnostic {
    fn default() -> Self {
        Self {
            error_code: String::new(),
            failure_kind: String::new(),
            stage: String::new(),
            message: String::new(),
            operation: String::new(),
            observed_at: String::new(),
            retryable: false,
            recovery_command: String::new(),
            attempt: 0,
            timeout_seconds: None,
            exit_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PapercutCaptureHealth {
    pub status: String,
    pub database_writable: bool,
    pub consecutive_failures: usize,
    pub spooled_events: usize,
    pub quarantined_events: usize,
    pub oldest_spool_at: Option<String>,
    pub last_success_at: Option<String>,
    pub warning: Option<String>,
    pub last_error: Option<PapercutCaptureDiagnostic>,
    pub excerpt_retention_days: i64,
}

impl Default for PapercutCaptureHealth {
    fn default() -> Self {
        Self {
            status: "healthy".to_string(),
            database_writable: true,
            consecutive_failures: 0,
            spooled_events: 0,
            quarantined_events: 0,
            oldest_spool_at: None,
            last_success_at: None,
            warning: None,
            last_error: None,
            excerpt_retention_days: EXCERPT_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Papercut {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub family: String,
    pub surface: String,
    pub source: String,
    pub evidence_refs: Vec<String>,
    pub impact: String,
    pub priority: String,
    pub status: String,
    pub next_action: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PapercutCounts {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub deferred: usize,
    pub resolved: usize,
    pub observations: usize,
    pub local_patterns: usize,
    pub cross_scope_patterns: usize,
    pub draft_proposals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PapercutBacklog {
    pub schema_version: String,
    pub family: String,
    pub generated_at: String,
    pub papercuts: Vec<Papercut>,
    pub counts: PapercutCounts,
    pub observations: Vec<PapercutObservation>,
    pub patterns: Vec<PapercutPattern>,
    pub proposals: Vec<MultiplierProposal>,
    pub digests: Vec<PapercutDigest>,
    pub health: PapercutCaptureHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PapercutObserveResult {
    pub schema_version: String,
    pub status: String,
    pub deduplicated: bool,
    pub observation: PapercutObservation,
    pub promoted_patterns: Vec<PapercutPattern>,
}

fn iso_now() -> String {
    Utc::now().to_rfc3339()
}

fn stable_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn compact_id(prefix: &str, value: &str) -> String {
    format!("{prefix}:{}", &stable_hash(value)[..24])
}

fn required(value: String, label: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(format!("Papercut {label} is required."))
    } else {
        Ok(value)
    }
}

fn normalized_choice(value: String, label: &str, allowed: &[&str]) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(format!(
            "Papercut {label} must be one of: {}.",
            allowed.join(", ")
        ))
    }
}

fn normalize_priority(value: String) -> Result<String, String> {
    let value = if value.trim().is_empty() {
        "P2".to_string()
    } else {
        value.trim().to_ascii_uppercase()
    };
    if matches!(value.as_str(), "P0" | "P1" | "P2" | "P3") {
        Ok(value)
    } else {
        Err("Papercut priority must be P0, P1, P2, or P3.".to_string())
    }
}

fn normalize_status(value: String) -> Result<String, String> {
    normalized_choice(
        value,
        "status",
        &["open", "in_progress", "deferred", "resolved"],
    )
}

fn sanitize_excerpt(value: &str) -> String {
    let mut sanitized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    for marker in ["/Users/", "/private/", "/tmp/", "sk-", "ghp_", "Bearer"] {
        while let Some(start) = sanitized.find(marker) {
            let end = sanitized[start..]
                .find(char::is_whitespace)
                .map(|offset| start + offset)
                .unwrap_or(sanitized.len());
            let replacement = if marker.starts_with('/') {
                "[path]"
            } else {
                "[secret]"
            };
            sanitized.replace_range(start..end, replacement);
        }
    }
    let mut chars = sanitized.chars();
    let clipped = chars
        .by_ref()
        .take(EXCERPT_MAX_CHARS.saturating_sub(1))
        .collect::<String>();
    if chars.next().is_some() {
        format!("{}…", clipped.trim_end())
    } else {
        sanitized
    }
}

fn sanitize_identifier(value: String, label: &str) -> Result<String, String> {
    let value = required(value, label)?;
    if value.contains("/Users/")
        || value.contains("/private/")
        || value.contains("/tmp/")
        || value.contains("\\Users\\")
    {
        Ok(format!("opaque:v1:{}", &stable_hash(&value)[..24]))
    } else {
        Ok(value)
    }
}

fn normalize_phenomenon(value: &str) -> String {
    let normalized = value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join("-");
    if normalized.is_empty() {
        "uncategorized-friction".to_string()
    } else {
        normalized
    }
}

fn normalize_observation_input(
    mut input: PapercutObservationInput,
) -> Result<PapercutObservationInput, String> {
    input.event_key = sanitize_identifier(input.event_key, "event key")?;
    input.scope_id = sanitize_identifier(input.scope_id, "scope id")?;
    input.scope_kind = normalized_choice(
        if input.scope_kind.trim().is_empty() {
            "project".to_string()
        } else {
            input.scope_kind
        },
        "scope kind",
        &["repository", "project", "global"],
    )?;
    input.domain = if input.domain.trim().is_empty() {
        "general".to_string()
    } else {
        input.domain.trim().to_ascii_lowercase()
    };
    input.signal_kind = normalized_choice(input.signal_kind, "signal kind", SIGNAL_KINDS)?;
    input.target_kind = normalized_choice(input.target_kind, "target kind", TARGET_KINDS)?;
    input.summary = sanitize_excerpt(&required(input.summary, "summary")?);
    input.excerpt = input
        .excerpt
        .as_deref()
        .map(sanitize_excerpt)
        .filter(|value| !value.is_empty());
    input.source = if input.source.trim().is_empty() {
        "codex-agent".to_string()
    } else {
        sanitize_excerpt(&input.source).to_ascii_lowercase()
    };
    input.evidence_refs = input
        .evidence_refs
        .into_iter()
        .map(|value| sanitize_excerpt(&value))
        .filter(|value| !value.is_empty())
        .collect();
    input.phenomenon_key = normalize_phenomenon(if input.phenomenon_key.trim().is_empty() {
        &input.summary
    } else {
        &input.phenomenon_key
    });
    input.failure_mode = normalize_phenomenon(if input.failure_mode.trim().is_empty() {
        &input.signal_kind
    } else {
        &input.failure_mode
    });
    input.priority = normalize_priority(input.priority)?;
    Ok(input)
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect Pronto papercut storage: {error}"))
}

fn ensure_schema(connection: &mut Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS papercut_observations (
                 id TEXT PRIMARY KEY,
                 event_key TEXT NOT NULL UNIQUE,
                 scope_id TEXT NOT NULL,
                 scope_kind TEXT NOT NULL,
                 domain TEXT NOT NULL,
                 signal_kind TEXT NOT NULL,
                 target_kind TEXT NOT NULL,
                 summary TEXT NOT NULL,
                 excerpt TEXT,
                 excerpt_hash TEXT NOT NULL,
                 excerpt_expires_at TEXT,
                 source TEXT NOT NULL,
                 evidence_refs_json TEXT NOT NULL,
                 phenomenon_key TEXT NOT NULL,
                 failure_mode TEXT NOT NULL,
                 priority TEXT NOT NULL,
                 urgent INTEGER NOT NULL,
                 verified INTEGER NOT NULL,
                 observed_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_papercut_observations_scope_phenomenon
                 ON papercut_observations (scope_id, target_kind, phenomenon_key, failure_mode, observed_at DESC);
             CREATE TABLE IF NOT EXISTS papercut_patterns (
                 id TEXT PRIMARY KEY,
                 fingerprint TEXT NOT NULL UNIQUE,
                 fingerprint_version TEXT NOT NULL,
                 scope_kind TEXT NOT NULL,
                 scope_id TEXT,
                 title TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 domain TEXT NOT NULL,
                 target_kind TEXT NOT NULL,
                 phenomenon_key TEXT NOT NULL,
                 failure_mode TEXT NOT NULL,
                 surface TEXT NOT NULL,
                 source TEXT NOT NULL,
                 evidence_refs_json TEXT NOT NULL,
                 impact TEXT NOT NULL,
                 priority TEXT NOT NULL,
                 status TEXT NOT NULL,
                 next_action TEXT NOT NULL,
                 evidence_tier TEXT NOT NULL,
                 occurrence_count INTEGER NOT NULL,
                 scope_count INTEGER NOT NULL,
                 first_observed_at TEXT NOT NULL,
                 last_observed_at TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 resolved_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_papercut_patterns_status_updated
                 ON papercut_patterns (status, updated_at DESC);
             CREATE TABLE IF NOT EXISTS papercut_pattern_observations (
                 pattern_id TEXT NOT NULL,
                 observation_id TEXT NOT NULL,
                 PRIMARY KEY (pattern_id, observation_id),
                 FOREIGN KEY (pattern_id) REFERENCES papercut_patterns(id) ON DELETE CASCADE,
                 FOREIGN KEY (observation_id) REFERENCES papercut_observations(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS papercut_multiplier_proposals (
                 id TEXT PRIMARY KEY,
                 pattern_ids_json TEXT NOT NULL,
                 title TEXT NOT NULL,
                 hypothesis TEXT NOT NULL,
                 root_cause TEXT NOT NULL,
                 multiplier TEXT NOT NULL,
                 evidence_tier TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 reviewed_at TEXT
             );
             CREATE TABLE IF NOT EXISTS papercut_digests (
                 id TEXT PRIMARY KEY,
                 week_start TEXT NOT NULL,
                 week_end TEXT NOT NULL,
                 digest_json TEXT NOT NULL,
                 generated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS papercut_meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )
        .map_err(|error| format!("Could not initialize Pronto papercut storage: {error}"))?;

    migrate_v1(connection)
}

#[derive(Debug)]
struct LegacyPapercut {
    id: String,
    title: String,
    detail: String,
    surface: String,
    source: String,
    evidence_refs_json: String,
    impact: String,
    priority: String,
    status: String,
    next_action: String,
    created_at: String,
    updated_at: String,
    resolved_at: Option<String>,
}

fn migrate_v1(connection: &mut Connection) -> Result<(), String> {
    if !table_exists(connection, "papercuts")? {
        return Ok(());
    }
    let migrated = connection
        .query_row(
            "SELECT value FROM papercut_meta WHERE key = 'v1_migrated'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Could not inspect Papercuts migration state: {error}"))?
        .is_some();
    if migrated {
        return Ok(());
    }

    let legacy = {
        let mut statement = connection
            .prepare(
                "SELECT id, title, detail, surface, source, evidence_refs_json, impact,
                        priority, status, next_action, created_at, updated_at, resolved_at
                 FROM papercuts",
            )
            .map_err(|error| format!("Could not prepare Papercuts v1 migration: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(LegacyPapercut {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    detail: row.get(2)?,
                    surface: row.get(3)?,
                    source: row.get(4)?,
                    evidence_refs_json: row.get(5)?,
                    impact: row.get(6)?,
                    priority: row.get(7)?,
                    status: row.get(8)?,
                    next_action: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    resolved_at: row.get(12)?,
                })
            })
            .map_err(|error| format!("Could not read Papercuts v1 rows: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Could not decode Papercuts v1 rows: {error}"))?;
        rows
    };

    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not start Papercuts v1 migration: {error}"))?;
    for item in legacy {
        let observation_id = format!("observation:legacy:{}", item.id);
        let event_key = format!("legacy:{}", item.id);
        let excerpt = sanitize_excerpt(&item.detail);
        let excerpt_hash = stable_hash(&excerpt);
        let phenomenon_key = normalize_phenomenon(&item.title);
        let expires_at = chrono::DateTime::parse_from_rfc3339(&item.created_at)
            .map(|value| {
                (value.with_timezone(&Utc) + Duration::days(EXCERPT_RETENTION_DAYS)).to_rfc3339()
            })
            .unwrap_or_else(|_| (Utc::now() + Duration::days(EXCERPT_RETENTION_DAYS)).to_rfc3339());
        transaction
            .execute(
                "INSERT OR IGNORE INTO papercut_observations
                 (id, event_key, scope_id, scope_kind, domain, signal_kind, target_kind,
                  summary, excerpt, excerpt_hash, excerpt_expires_at, source, evidence_refs_json,
                  phenomenon_key, failure_mode, priority, urgent, verified, observed_at)
                 VALUES (?1, ?2, 'global-agent', 'global', 'design', 'legacy_manual', 'artifact',
                         ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'legacy-manual', ?10, 1, 1, ?11)",
                params![
                    observation_id,
                    event_key,
                    sanitize_excerpt(&item.title),
                    excerpt,
                    excerpt_hash,
                    expires_at,
                    item.source,
                    item.evidence_refs_json,
                    phenomenon_key,
                    item.priority,
                    item.created_at,
                ],
            )
            .map_err(|error| format!("Could not migrate Papercuts v1 observation: {error}"))?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO papercut_patterns
                 (id, fingerprint, fingerprint_version, scope_kind, scope_id, title, detail,
                  domain, target_kind, phenomenon_key, failure_mode, surface, source,
                  evidence_refs_json, impact, priority, status, next_action, evidence_tier,
                  occurrence_count, scope_count, first_observed_at, last_observed_at,
                  created_at, updated_at, resolved_at)
                 VALUES (?1, ?2, ?3, 'local', 'global-agent', ?4, ?5, 'design', 'artifact',
                         ?6, 'legacy-manual', ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         'legacy_manual', 1, 1, ?14, ?15, ?14, ?15, ?16)",
                params![
                    item.id,
                    format!("legacy:{}", item.id),
                    FINGERPRINT_VERSION,
                    sanitize_excerpt(&item.title),
                    sanitize_excerpt(&item.detail),
                    phenomenon_key,
                    item.surface,
                    item.source,
                    item.evidence_refs_json,
                    item.impact,
                    item.priority,
                    item.status,
                    item.next_action,
                    item.created_at,
                    item.updated_at,
                    item.resolved_at,
                ],
            )
            .map_err(|error| format!("Could not migrate Papercuts v1 pattern: {error}"))?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO papercut_pattern_observations (pattern_id, observation_id)
                 VALUES (?1, ?2)",
                params![item.id, observation_id],
            )
            .map_err(|error| format!("Could not link migrated Papercuts v1 evidence: {error}"))?;
    }
    transaction
        .execute(
            "INSERT OR REPLACE INTO papercut_meta (key, value) VALUES ('v1_migrated', ?1)",
            [iso_now()],
        )
        .map_err(|error| format!("Could not record Papercuts v1 migration: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Could not commit Papercuts v1 migration: {error}"))
}

fn prune_expired_excerpts(connection: &Connection) -> Result<usize, String> {
    connection
        .execute(
            "UPDATE papercut_observations SET excerpt = NULL
             WHERE excerpt IS NOT NULL AND excerpt_expires_at <= ?1",
            [iso_now()],
        )
        .map_err(|error| format!("Could not prune expired Papercut excerpts: {error}"))
}

fn observation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PapercutObservation> {
    let evidence_json: String = row.get(12)?;
    Ok(PapercutObservation {
        id: row.get(0)?,
        event_key: row.get(1)?,
        scope_id: row.get(2)?,
        scope_kind: row.get(3)?,
        domain: row.get(4)?,
        signal_kind: row.get(5)?,
        target_kind: row.get(6)?,
        summary: row.get(7)?,
        excerpt: row.get(8)?,
        excerpt_hash: row.get(9)?,
        excerpt_expires_at: row.get(10)?,
        source: row.get(11)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        phenomenon_key: row.get(13)?,
        failure_mode: row.get(14)?,
        priority: row.get(15)?,
        urgent: row.get::<_, i64>(16)? != 0,
        verified: row.get::<_, i64>(17)? != 0,
        observed_at: row.get(18)?,
    })
}

fn pattern_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PapercutPattern> {
    let evidence_json: String = row.get(13)?;
    Ok(PapercutPattern {
        id: row.get(0)?,
        fingerprint: row.get(1)?,
        fingerprint_version: row.get(2)?,
        scope_kind: row.get(3)?,
        scope_id: row.get(4)?,
        title: row.get(5)?,
        detail: row.get(6)?,
        domain: row.get(7)?,
        target_kind: row.get(8)?,
        phenomenon_key: row.get(9)?,
        failure_mode: row.get(10)?,
        surface: row.get(11)?,
        source: row.get(12)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        impact: row.get(14)?,
        priority: row.get(15)?,
        status: row.get(16)?,
        next_action: row.get(17)?,
        evidence_tier: row.get(18)?,
        occurrence_count: row.get::<_, i64>(19)?.max(0) as usize,
        scope_count: row.get::<_, i64>(20)?.max(0) as usize,
        first_observed_at: row.get(21)?,
        last_observed_at: row.get(22)?,
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
        resolved_at: row.get(25)?,
    })
}

const OBSERVATION_COLUMNS: &str = "id, event_key, scope_id, scope_kind, domain, signal_kind,
    target_kind, summary, excerpt, excerpt_hash, excerpt_expires_at, source,
    evidence_refs_json, phenomenon_key, failure_mode, priority, urgent, verified, observed_at";
const PATTERN_COLUMNS: &str = "id, fingerprint, fingerprint_version, scope_kind, scope_id, title,
    detail, domain, target_kind, phenomenon_key, failure_mode, surface, source,
    evidence_refs_json, impact, priority, status, next_action, evidence_tier,
    occurrence_count, scope_count, first_observed_at, last_observed_at, created_at,
    updated_at, resolved_at";

fn load_observation(
    connection: &Connection,
    observation_id: &str,
) -> Result<PapercutObservation, String> {
    connection
        .query_row(
            &format!("SELECT {OBSERVATION_COLUMNS} FROM papercut_observations WHERE id = ?1"),
            [observation_id],
            observation_from_row,
        )
        .map_err(|error| format!("Could not read Papercut observation: {error}"))
}

fn load_pattern(connection: &Connection, pattern_id: &str) -> Result<PapercutPattern, String> {
    connection
        .query_row(
            &format!("SELECT {PATTERN_COLUMNS} FROM papercut_patterns WHERE id = ?1"),
            [pattern_id],
            pattern_from_row,
        )
        .map_err(|error| format!("Could not read Papercut pattern: {error}"))
}

fn upsert_pattern(
    connection: &Connection,
    observation: &PapercutObservation,
    scope_kind: &str,
    scope_id: Option<&str>,
    evidence_tier: &str,
    matching_observations: &[String],
) -> Result<PapercutPattern, String> {
    let fingerprint = if let Some(scope_id) = scope_id {
        format!(
            "{FINGERPRINT_VERSION}|local|{scope_id}|{}|{}|{}",
            observation.target_kind, observation.phenomenon_key, observation.failure_mode
        )
    } else {
        format!(
            "{FINGERPRINT_VERSION}|cross|{}|{}|{}",
            observation.target_kind, observation.phenomenon_key, observation.failure_mode
        )
    };
    let id = compact_id("pattern", &fingerprint);
    let now = iso_now();
    let first_observed_at = connection
        .query_row(
            "SELECT MIN(observed_at) FROM papercut_observations
             WHERE target_kind = ?1 AND phenomenon_key = ?2 AND failure_mode = ?3
               AND (?4 IS NULL OR scope_id = ?4)",
            params![
                observation.target_kind,
                observation.phenomenon_key,
                observation.failure_mode,
                scope_id,
            ],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|error| format!("Could not find first Papercut occurrence: {error}"))?
        .unwrap_or_else(|| observation.observed_at.clone());
    let scope_count = connection
        .query_row(
            "SELECT COUNT(DISTINCT scope_id) FROM papercut_observations
             WHERE target_kind = ?1 AND phenomenon_key = ?2 AND failure_mode = ?3
               AND (?4 IS NULL OR scope_id = ?4)",
            params![
                observation.target_kind,
                observation.phenomenon_key,
                observation.failure_mode,
                scope_id,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Could not count Papercut scopes: {error}"))?;
    let occurrence_count = matching_observations.len();
    connection
        .execute(
            "INSERT INTO papercut_patterns
             (id, fingerprint, fingerprint_version, scope_kind, scope_id, title, detail,
              domain, target_kind, phenomenon_key, failure_mode, surface, source,
              evidence_refs_json, impact, priority, status, next_action, evidence_tier,
              occurrence_count, scope_count, first_observed_at, last_observed_at,
              created_at, updated_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                     '', ?15, 'open', 'Review the evidence and choose the smallest reusable prevention.',
                     ?16, ?17, ?18, ?19, ?20, ?21, ?21, NULL)
             ON CONFLICT(fingerprint) DO UPDATE SET
                 title = excluded.title,
                 detail = excluded.detail,
                 evidence_refs_json = excluded.evidence_refs_json,
                 priority = CASE
                     WHEN excluded.priority < papercut_patterns.priority THEN excluded.priority
                     ELSE papercut_patterns.priority END,
                 evidence_tier = excluded.evidence_tier,
                 occurrence_count = excluded.occurrence_count,
                 scope_count = excluded.scope_count,
                 first_observed_at = excluded.first_observed_at,
                 last_observed_at = excluded.last_observed_at,
                 updated_at = excluded.updated_at",
            params![
                id,
                fingerprint,
                FINGERPRINT_VERSION,
                scope_kind,
                scope_id,
                observation.summary,
                observation.excerpt.clone().unwrap_or_else(|| observation.summary.clone()),
                observation.domain,
                observation.target_kind,
                observation.phenomenon_key,
                observation.failure_mode,
                observation.target_kind,
                observation.source,
                serde_json::to_string(&observation.evidence_refs).unwrap_or_else(|_| "[]".to_string()),
                observation.priority,
                evidence_tier,
                occurrence_count as i64,
                scope_count,
                first_observed_at,
                observation.observed_at,
                now,
            ],
        )
        .map_err(|error| format!("Could not promote Papercut pattern: {error}"))?;
    for observation_id in matching_observations {
        connection
            .execute(
                "INSERT OR IGNORE INTO papercut_pattern_observations (pattern_id, observation_id)
                 VALUES (?1, ?2)",
                params![id, observation_id],
            )
            .map_err(|error| format!("Could not attach Papercut evidence: {error}"))?;
    }
    load_pattern(connection, &id)
}

fn matching_observation_ids(
    connection: &Connection,
    observation: &PapercutObservation,
    scope_id: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id FROM papercut_observations
             WHERE target_kind = ?1 AND phenomenon_key = ?2 AND failure_mode = ?3
               AND (?4 IS NULL OR scope_id = ?4)
             ORDER BY observed_at ASC, id ASC",
        )
        .map_err(|error| format!("Could not prepare Papercut grouping query: {error}"))?;
    let rows = statement
        .query_map(
            params![
                observation.target_kind,
                observation.phenomenon_key,
                observation.failure_mode,
                scope_id,
            ],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not group Papercut observations: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Papercut grouping result: {error}"))?;
    Ok(rows)
}

fn promote_observation(
    connection: &Connection,
    observation: &PapercutObservation,
    force_local: bool,
) -> Result<Vec<PapercutPattern>, String> {
    let mut promoted = Vec::new();
    let local_ids = matching_observation_ids(connection, observation, Some(&observation.scope_id))?;
    let urgent_verified = observation.urgent && observation.verified;
    if force_local || urgent_verified || local_ids.len() >= 2 {
        promoted.push(upsert_pattern(
            connection,
            observation,
            "local",
            Some(&observation.scope_id),
            if urgent_verified {
                "urgent"
            } else if force_local {
                "manual"
            } else {
                "local_recurring"
            },
            &local_ids,
        )?);
    }
    let cross_ids = matching_observation_ids(connection, observation, None)?;
    let scope_count = connection
        .query_row(
            "SELECT COUNT(DISTINCT scope_id) FROM papercut_observations
             WHERE target_kind = ?1 AND phenomenon_key = ?2 AND failure_mode = ?3",
            params![
                observation.target_kind,
                observation.phenomenon_key,
                observation.failure_mode
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Could not count cross-scope Papercut evidence: {error}"))?;
    if cross_ids.len() >= 3 && scope_count >= 2 {
        promoted.push(upsert_pattern(
            connection,
            observation,
            "cross_scope",
            None,
            "cross_scope",
            &cross_ids,
        )?);
    }
    Ok(promoted)
}

fn observe_at(
    path: &Path,
    input: PapercutObservationInput,
    dry_run: bool,
    force_local: bool,
) -> Result<PapercutObserveResult, String> {
    let input = normalize_observation_input(input)?;
    let excerpt_hash = stable_hash(input.excerpt.as_deref().unwrap_or(""));
    let observed_at = input.observed_at.clone().unwrap_or_else(iso_now);
    let excerpt_expires_at = input.excerpt.as_ref().map(|_| {
        chrono::DateTime::parse_from_rfc3339(&observed_at)
            .map(|value| {
                (value.with_timezone(&Utc) + Duration::days(EXCERPT_RETENTION_DAYS)).to_rfc3339()
            })
            .unwrap_or_else(|_| (Utc::now() + Duration::days(EXCERPT_RETENTION_DAYS)).to_rfc3339())
    });
    let observation = PapercutObservation {
        id: compact_id("observation", &input.event_key),
        event_key: input.event_key,
        scope_id: input.scope_id,
        scope_kind: input.scope_kind,
        domain: input.domain,
        signal_kind: input.signal_kind,
        target_kind: input.target_kind,
        summary: input.summary,
        excerpt: input.excerpt,
        excerpt_hash,
        excerpt_expires_at,
        source: input.source,
        evidence_refs: input.evidence_refs,
        phenomenon_key: input.phenomenon_key,
        failure_mode: input.failure_mode,
        priority: input.priority,
        urgent: input.urgent,
        verified: input.verified,
        observed_at,
    };
    if dry_run {
        return Ok(PapercutObserveResult {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "dry_run".to_string(),
            deduplicated: false,
            observation,
            promoted_patterns: Vec::new(),
        });
    }

    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        prune_expired_excerpts(connection)?;
        let existing_id = connection
            .query_row(
                "SELECT id FROM papercut_observations WHERE event_key = ?1",
                [&observation.event_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Could not deduplicate Papercut observation: {error}"))?;
        if let Some(existing_id) = existing_id {
            let existing = load_observation(connection, &existing_id)?;
            let patterns = patterns_for_observation(connection, &existing_id)?;
            return Ok(PapercutObserveResult {
                schema_version: SCHEMA_VERSION.to_string(),
                status: "deduplicated".to_string(),
                deduplicated: true,
                observation: existing,
                promoted_patterns: patterns,
            });
        }
        connection
            .execute(
                "INSERT INTO papercut_observations
                 (id, event_key, scope_id, scope_kind, domain, signal_kind, target_kind,
                  summary, excerpt, excerpt_hash, excerpt_expires_at, source, evidence_refs_json,
                  phenomenon_key, failure_mode, priority, urgent, verified, observed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    observation.id,
                    observation.event_key,
                    observation.scope_id,
                    observation.scope_kind,
                    observation.domain,
                    observation.signal_kind,
                    observation.target_kind,
                    observation.summary,
                    observation.excerpt,
                    observation.excerpt_hash,
                    observation.excerpt_expires_at,
                    observation.source,
                    serde_json::to_string(&observation.evidence_refs)
                        .map_err(|error| format!("Could not encode Papercut evidence: {error}"))?,
                    observation.phenomenon_key,
                    observation.failure_mode,
                    observation.priority,
                    i64::from(observation.urgent),
                    i64::from(observation.verified),
                    observation.observed_at,
                ],
            )
            .map_err(|error| format!("Could not save Papercut observation: {error}"))?;
        let promoted_patterns = promote_observation(connection, &observation, force_local)?;
        Ok(PapercutObserveResult {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "captured".to_string(),
            deduplicated: false,
            observation,
            promoted_patterns,
        })
    })
}

fn patterns_for_observation(
    connection: &Connection,
    observation_id: &str,
) -> Result<Vec<PapercutPattern>, String> {
    let mut statement = connection
        .prepare(
            "SELECT p.id FROM papercut_patterns p
             JOIN papercut_pattern_observations po ON po.pattern_id = p.id
             WHERE po.observation_id = ?1 ORDER BY p.updated_at DESC",
        )
        .map_err(|error| format!("Could not prepare Papercut promotion query: {error}"))?;
    let pattern_ids = statement
        .query_map([observation_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read promoted Papercut patterns: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode promoted Papercut patterns: {error}"))?;
    pattern_ids
        .iter()
        .map(|pattern_id| load_pattern(connection, pattern_id))
        .collect()
}

fn load_health_from_home(home: &Path) -> PapercutCaptureHealth {
    let paths = [
        home.join("Library/Application Support/Pronto/papercuts-hook/health.json"),
        home.join(".codex/papercuts/health.json"),
    ];
    let mut health: PapercutCaptureHealth = paths
        .iter()
        .find_map(|path| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|value| serde_json::from_str(&value).ok())
        })
        .unwrap_or_default();
    if health.status == "healthy" {
        health.last_error = None;
    } else if let Some(diagnostic) = health.last_error.as_mut() {
        if diagnostic.failure_kind.is_empty() {
            diagnostic.failure_kind = match diagnostic.error_code.as_str() {
                "PAPERCUTS-E4001" => "child_process_timeout",
                "PAPERCUTS-E4002" => "child_process_failure",
                "PAPERCUTS-E4003" => "pronto_cli_unavailable",
                "PAPERCUTS-E4004" => "pronto_output_invalid",
                "PAPERCUTS-E5001" => "contract_invalid",
                "PAPERCUTS-E5002" => "spooled_contract_invalid",
                "PAPERCUTS-E5003" => "downstream_contract_invalid",
                _ => "legacy_failure",
            }
            .to_string();
        }
        diagnostic.retryable = !matches!(
            diagnostic.error_code.as_str(),
            "PAPERCUTS-E5001" | "PAPERCUTS-E5002" | "PAPERCUTS-E5003"
        );
        if diagnostic.recovery_command.is_empty() {
            diagnostic.recovery_command = format!(
                "{} papercuts health --json",
                home.join(".codex/bin/pronto-papercuts").display()
            );
        }
    }
    health
}

fn load_health() -> PapercutCaptureHealth {
    dirs::home_dir()
        .as_deref()
        .map(load_health_from_home)
        .unwrap_or_default()
}

fn backlog_from_connection(connection: &Connection) -> Result<PapercutBacklog, String> {
    let observations = {
        let mut statement = connection
            .prepare(&format!(
                "SELECT {OBSERVATION_COLUMNS} FROM papercut_observations
                 ORDER BY observed_at DESC, id DESC LIMIT 1000"
            ))
            .map_err(|error| format!("Could not prepare Papercut observations query: {error}"))?;
        let rows = statement
            .query_map([], observation_from_row)
            .map_err(|error| format!("Could not read Papercut observations: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Could not decode Papercut observations: {error}"))?;
        rows
    };
    let patterns = {
        let mut statement = connection
            .prepare(&format!(
                "SELECT {PATTERN_COLUMNS} FROM papercut_patterns
                 ORDER BY CASE status WHEN 'open' THEN 0 WHEN 'in_progress' THEN 1
                                      WHEN 'deferred' THEN 2 WHEN 'resolved' THEN 3 ELSE 4 END,
                          updated_at DESC, id DESC"
            ))
            .map_err(|error| format!("Could not prepare Papercut patterns query: {error}"))?;
        let rows = statement
            .query_map([], pattern_from_row)
            .map_err(|error| format!("Could not read Papercut patterns: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Could not decode Papercut patterns: {error}"))?;
        rows
    };
    let proposals = load_proposals(connection)?;
    let digests = load_digests(connection)?;
    let papercuts = patterns
        .iter()
        .map(|pattern| Papercut {
            id: pattern.id.clone(),
            title: pattern.title.clone(),
            detail: pattern.detail.clone(),
            family: FAMILY.to_string(),
            surface: pattern.surface.clone(),
            source: pattern.source.clone(),
            evidence_refs: pattern.evidence_refs.clone(),
            impact: pattern.impact.clone(),
            priority: pattern.priority.clone(),
            status: pattern.status.clone(),
            next_action: pattern.next_action.clone(),
            created_at: pattern.created_at.clone(),
            updated_at: pattern.updated_at.clone(),
            resolved_at: pattern.resolved_at.clone(),
        })
        .collect::<Vec<_>>();
    let mut counts = PapercutCounts {
        total: papercuts.len(),
        observations: observations.len(),
        local_patterns: patterns
            .iter()
            .filter(|item| item.scope_kind == "local")
            .count(),
        cross_scope_patterns: patterns
            .iter()
            .filter(|item| item.scope_kind == "cross_scope")
            .count(),
        draft_proposals: proposals
            .iter()
            .filter(|item| item.status == "draft")
            .count(),
        ..PapercutCounts::default()
    };
    for item in &papercuts {
        match item.status.as_str() {
            "open" => counts.open += 1,
            "in_progress" => counts.in_progress += 1,
            "deferred" => counts.deferred += 1,
            "resolved" => counts.resolved += 1,
            _ => {}
        }
    }
    Ok(PapercutBacklog {
        schema_version: SCHEMA_VERSION.to_string(),
        family: FAMILY.to_string(),
        generated_at: iso_now(),
        papercuts,
        counts,
        observations,
        patterns,
        proposals,
        digests,
        health: load_health(),
    })
}

fn load_at(path: &Path) -> Result<PapercutBacklog, String> {
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        prune_expired_excerpts(connection)?;
        backlog_from_connection(connection)
    })
}

fn create_at(
    path: &Path,
    title: String,
    detail: String,
    surface: String,
    source: String,
    priority: String,
    evidence_refs: Vec<String>,
    impact: String,
    next_action: String,
) -> Result<PapercutBacklog, String> {
    let title = required(title, "title")?;
    let detail = required(detail, "detail")?;
    let now = iso_now();
    let event_key = format!("manual:{now}:{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    let result = observe_at(
        path,
        PapercutObservationInput {
            event_key,
            scope_id: "global-agent".to_string(),
            scope_kind: "global".to_string(),
            domain: "design".to_string(),
            signal_kind: "dissatisfaction".to_string(),
            target_kind: "artifact".to_string(),
            summary: title,
            excerpt: Some(detail),
            source,
            evidence_refs,
            phenomenon_key: String::new(),
            failure_mode: "manual-friction".to_string(),
            priority,
            urgent: false,
            verified: true,
            observed_at: Some(now),
        },
        false,
        true,
    )?;
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        if let Some(pattern) = result.promoted_patterns.first() {
            connection
                .execute(
                    "UPDATE papercut_patterns SET surface = ?1, impact = ?2, next_action = ?3
                     WHERE id = ?4",
                    params![
                        if surface.trim().is_empty() {
                            "Pronto UI"
                        } else {
                            surface.trim()
                        },
                        sanitize_excerpt(&impact),
                        if next_action.trim().is_empty() {
                            "Define the next validation step.".to_string()
                        } else {
                            sanitize_excerpt(&next_action)
                        },
                        pattern.id,
                    ],
                )
                .map_err(|error| format!("Could not finish manual Papercut capture: {error}"))?;
        }
        backlog_from_connection(connection)
    })
}

fn set_status_at(
    path: &Path,
    pattern_id: String,
    status: String,
) -> Result<PapercutBacklog, String> {
    let pattern_id = required(pattern_id, "id")?;
    let status = normalize_status(status)?;
    let now = iso_now();
    let resolved_at = (status == "resolved").then(|| now.clone());
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        let updated = connection
            .execute(
                "UPDATE papercut_patterns SET status = ?1, updated_at = ?2, resolved_at = ?3
                 WHERE id = ?4",
                params![status, now, resolved_at, pattern_id],
            )
            .map_err(|error| format!("Could not update Papercut pattern status: {error}"))?;
        if updated == 0 {
            return Err("That Papercut pattern is no longer available.".to_string());
        }
        backlog_from_connection(connection)
    })
}

fn load_proposals(connection: &Connection) -> Result<Vec<MultiplierProposal>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, pattern_ids_json, title, hypothesis, root_cause, multiplier,
                    evidence_tier, status, created_at, updated_at, reviewed_at
             FROM papercut_multiplier_proposals
             ORDER BY CASE status WHEN 'draft' THEN 0 WHEN 'accepted' THEN 1
                                  WHEN 'deferred' THEN 2 ELSE 3 END,
                      updated_at DESC",
        )
        .map_err(|error| format!("Could not prepare multiplier proposals query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            let pattern_ids_json: String = row.get(1)?;
            Ok(MultiplierProposal {
                id: row.get(0)?,
                pattern_ids: serde_json::from_str(&pattern_ids_json).unwrap_or_default(),
                title: row.get(2)?,
                hypothesis: row.get(3)?,
                root_cause: row.get(4)?,
                multiplier: row.get(5)?,
                evidence_tier: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                reviewed_at: row.get(10)?,
            })
        })
        .map_err(|error| format!("Could not read multiplier proposals: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode multiplier proposals: {error}"))?;
    Ok(rows)
}

fn propose_at(path: &Path, input: MultiplierProposalInput) -> Result<MultiplierProposal, String> {
    let title = sanitize_excerpt(&required(input.title, "proposal title")?);
    let hypothesis = sanitize_excerpt(&required(input.hypothesis, "proposal hypothesis")?);
    let root_cause = sanitize_excerpt(&required(input.root_cause, "proposal root cause")?);
    let multiplier = sanitize_excerpt(&required(input.multiplier, "proposal multiplier")?);
    if input.pattern_ids.is_empty() {
        return Err("A multiplier proposal must reference at least one pattern.".to_string());
    }
    let mut pattern_ids = input.pattern_ids;
    pattern_ids.sort();
    pattern_ids.dedup();
    let evidence_tier = normalized_choice(
        input.evidence_tier,
        "proposal evidence tier",
        &["single", "local_recurring", "cross_scope", "urgent"],
    )?;
    let identity = format!("{}|{}|{}", pattern_ids.join("|"), root_cause, multiplier);
    let id = compact_id("proposal", &identity);
    let now = iso_now();
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        for pattern_id in &pattern_ids {
            let exists = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM papercut_patterns WHERE id = ?1)",
                    [pattern_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(|error| format!("Could not validate proposal evidence: {error}"))?;
            if !exists {
                return Err(format!(
                    "Multiplier proposal pattern does not exist: {pattern_id}"
                ));
            }
        }
        connection
            .execute(
                "INSERT INTO papercut_multiplier_proposals
                 (id, pattern_ids_json, title, hypothesis, root_cause, multiplier,
                  evidence_tier, status, created_at, updated_at, reviewed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', ?8, ?8, NULL)
                 ON CONFLICT(id) DO UPDATE SET title = excluded.title,
                     hypothesis = excluded.hypothesis, updated_at = excluded.updated_at",
                params![
                    id,
                    serde_json::to_string(&pattern_ids)
                        .map_err(|error| format!("Could not encode proposal patterns: {error}"))?,
                    title,
                    hypothesis,
                    root_cause,
                    multiplier,
                    evidence_tier,
                    now,
                ],
            )
            .map_err(|error| format!("Could not save multiplier proposal: {error}"))?;
        load_proposals(connection)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "Saved multiplier proposal could not be reloaded.".to_string())
    })
}

fn set_proposal_status_at(
    path: &Path,
    proposal_id: String,
    status: String,
) -> Result<MultiplierProposal, String> {
    let proposal_id = required(proposal_id, "proposal id")?;
    let status = normalized_choice(
        status,
        "proposal status",
        &["draft", "accepted", "deferred", "rejected"],
    )?;
    let now = iso_now();
    let reviewed_at = (status != "draft").then(|| now.clone());
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        let updated = connection
            .execute(
                "UPDATE papercut_multiplier_proposals
                 SET status = ?1, updated_at = ?2, reviewed_at = ?3 WHERE id = ?4",
                params![status, now, reviewed_at, proposal_id],
            )
            .map_err(|error| format!("Could not update multiplier proposal: {error}"))?;
        if updated == 0 {
            return Err("That multiplier proposal is no longer available.".to_string());
        }
        load_proposals(connection)?
            .into_iter()
            .find(|item| item.id == proposal_id)
            .ok_or_else(|| "Updated multiplier proposal could not be reloaded.".to_string())
    })
}

fn digest_at(path: &Path) -> Result<PapercutDigest, String> {
    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        prune_expired_excerpts(connection)?;
        let today = Utc::now().date_naive();
        let week_start_date = today - Duration::days(today.weekday().num_days_from_monday() as i64);
        let week_end_date = week_start_date + Duration::days(7);
        let week_start = format!("{week_start_date}T00:00:00Z");
        let week_end = format!("{week_end_date}T00:00:00Z");
        let observation_count = connection
            .query_row(
                "SELECT COUNT(*) FROM papercut_observations WHERE observed_at >= ?1 AND observed_at < ?2",
                params![week_start, week_end],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("Could not count weekly Papercut observations: {error}"))?
            .max(0) as usize;
        let backlog = backlog_from_connection(connection)?;
        let mut top_patterns = backlog
            .patterns
            .iter()
            .filter(|item| item.last_observed_at >= week_start && item.last_observed_at < week_end)
            .cloned()
            .collect::<Vec<_>>();
        top_patterns.sort_by(|left, right| {
            right
                .occurrence_count
                .cmp(&left.occurrence_count)
                .then_with(|| right.scope_count.cmp(&left.scope_count))
                .then_with(|| left.id.cmp(&right.id))
        });
        top_patterns.truncate(10);
        let generated_at = iso_now();
        let digest = PapercutDigest {
            id: format!("digest:{week_start_date}"),
            week_start: week_start.clone(),
            week_end: week_end.clone(),
            generated_at: generated_at.clone(),
            observation_count,
            local_pattern_count: backlog.counts.local_patterns,
            cross_scope_pattern_count: backlog.counts.cross_scope_patterns,
            draft_proposal_count: backlog.counts.draft_proposals,
            top_patterns,
        };
        connection
            .execute(
                "INSERT INTO papercut_digests (id, week_start, week_end, digest_json, generated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET digest_json = excluded.digest_json,
                     generated_at = excluded.generated_at",
                params![
                    digest.id,
                    digest.week_start,
                    digest.week_end,
                    serde_json::to_string(&digest)
                        .map_err(|error| format!("Could not encode Papercut digest: {error}"))?,
                    generated_at,
                ],
            )
            .map_err(|error| format!("Could not save Papercut digest: {error}"))?;
        Ok(digest)
    })
}

fn load_digests(connection: &Connection) -> Result<Vec<PapercutDigest>, String> {
    let mut statement = connection
        .prepare("SELECT digest_json FROM papercut_digests ORDER BY week_start DESC LIMIT 12")
        .map_err(|error| format!("Could not prepare Papercut digest query: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read Papercut digests: {error}"))?
        .map(|row| {
            let value =
                row.map_err(|error| format!("Could not decode Papercut digest row: {error}"))?;
            serde_json::from_str(&value)
                .map_err(|error| format!("Could not decode Papercut digest: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(rows)
}

fn read_stdin_json<T: for<'de> Deserialize<'de>>() -> Result<T, String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("Could not read Papercuts JSON from stdin: {error}"))?;
    serde_json::from_str(&input)
        .map_err(|error| format!("Papercuts stdin must be valid JSON: {error}"))
}

fn cli_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|error| format!("Could not encode Papercuts JSON: {error}"))
}

pub fn run_cli(arguments: &[String]) -> Result<String, String> {
    let subcommand = arguments.get(1).map(String::as_str).unwrap_or("list");
    let path = crate::core::local_store_path();
    match subcommand {
        "list" => cli_json(&load_at(&path)?),
        "observe" => {
            if !arguments.iter().any(|item| item == "--stdin") {
                return Err("Usage: pronto papercuts observe --stdin --json [--dry-run]".to_string());
            }
            let input: PapercutObservationInput = read_stdin_json()?;
            cli_json(&observe_at(
                &path,
                input,
                arguments.iter().any(|item| item == "--dry-run"),
                false,
            )?)
        }
        "contract" => cli_json(&observation_contract()),
        "digest" => {
            if let Some(index) = arguments.iter().position(|item| item == "--week") {
                if arguments.get(index + 1).map(String::as_str) != Some("current") {
                    return Err("Papercuts currently supports only --week current.".to_string());
                }
            }
            cli_json(&digest_at(&path)?)
        }
        "propose" => {
            if !arguments.iter().any(|item| item == "--stdin") {
                return Err("Usage: pronto papercuts propose --stdin --json".to_string());
            }
            let input: MultiplierProposalInput = read_stdin_json()?;
            cli_json(&propose_at(&path, input)?)
        }
        "proposal" if arguments.get(2).map(String::as_str) == Some("set-status") => {
            let proposal_id = arguments
                .get(3)
                .cloned()
                .ok_or_else(|| "Papercuts proposal id is required.".to_string())?;
            let status = arguments
                .get(4)
                .cloned()
                .ok_or_else(|| "Papercuts proposal status is required.".to_string())?;
            cli_json(&set_proposal_status_at(&path, proposal_id, status)?)
        }
        "health" => {
            let mut health = load_health();
            health.database_writable = load_at(&path).is_ok();
            if !health.database_writable {
                health.status = "degraded".to_string();
                health.warning = Some("Pronto's Papercuts database is not writable.".to_string());
            }
            cli_json(&health)
        }
        _ => Err("Usage: pronto papercuts list --json | pronto papercuts observe --stdin --json [--dry-run] | pronto papercuts contract --json | pronto papercuts digest --week current --json | pronto papercuts propose --stdin --json | pronto papercuts proposal set-status <id> <status> --json | pronto papercuts health --json".to_string()),
    }
}

#[tauri::command]
pub fn get_papercut_backlog() -> Result<PapercutBacklog, String> {
    load_at(&crate::core::local_store_path())
}

#[tauri::command]
pub fn create_papercut(
    title: String,
    detail: String,
    surface: String,
    source: String,
    priority: String,
    evidence_refs: Vec<String>,
    impact: String,
    next_action: String,
) -> Result<PapercutBacklog, String> {
    create_at(
        &crate::core::local_store_path(),
        title,
        detail,
        surface,
        source,
        priority,
        evidence_refs,
        impact,
        next_action,
    )
}

#[tauri::command]
pub fn observe_papercut(input: PapercutObservationInput) -> Result<PapercutObserveResult, String> {
    observe_at(&crate::core::local_store_path(), input, false, false)
}

#[tauri::command]
pub fn generate_papercut_digest() -> Result<PapercutDigest, String> {
    digest_at(&crate::core::local_store_path())
}

#[tauri::command]
pub fn create_multiplier_proposal(
    input: MultiplierProposalInput,
) -> Result<MultiplierProposal, String> {
    propose_at(&crate::core::local_store_path(), input)
}

#[tauri::command]
pub fn set_multiplier_proposal_status(
    proposal_id: String,
    status: String,
) -> Result<MultiplierProposal, String> {
    set_proposal_status_at(&crate::core::local_store_path(), proposal_id, status)
}

#[tauri::command]
pub fn set_papercut_status(papercut_id: String, status: String) -> Result<PapercutBacklog, String> {
    set_status_at(&crate::core::local_store_path(), papercut_id, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn observation_contract_matches_the_agent_hook_surface() {
        let contract = observation_contract();
        assert_eq!(contract.schema_version, OBSERVATION_CONTRACT_VERSION);
        assert!(contract
            .signal_kinds
            .contains(&"boundary_correction".to_string()));
        assert!(!contract.signal_kinds.contains(&"legacy_manual".to_string()));
        assert_eq!(
            contract.minimal_input["signal_kind"],
            serde_json::json!("capability_gap")
        );
        let input: PapercutObservationInput = serde_json::from_value(contract.minimal_input)
            .expect("minimal contract input should decode");
        normalize_observation_input(input).expect("minimal contract input should validate");
    }

    fn test_database() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "pronto-papercuts-{}-{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("test storage should be created");
        (root.join("registry.db"), root)
    }

    #[test]
    fn capture_health_prefers_pronto_runtime_and_falls_back_to_legacy_state() {
        let (_, root) = test_database();
        let primary = root.join("Library/Application Support/Pronto/papercuts-hook");
        let legacy = root.join(".codex/papercuts");
        fs::create_dir_all(&legacy).expect("legacy health directory should exist");
        fs::write(
            legacy.join("health.json"),
            r#"{"status":"degraded","database_writable":false}"#,
        )
        .expect("legacy health should persist");
        assert_eq!(load_health_from_home(&root).status, "degraded");

        fs::create_dir_all(&primary).expect("primary health directory should exist");
        fs::write(
            primary.join("health.json"),
            r#"{"status":"failing","database_writable":false,"last_error":{"error_code":"PAPERCUTS-E4001","failure_kind":"child_process_timeout","stage":"pronto_process","message":"the Pronto capture process timed out","operation":"drain","observed_at":"2026-08-14T23:00:00Z","retryable":true,"recovery_command":"pronto-papercuts papercuts health --json","attempt":3,"timeout_seconds":3}}"#,
        )
        .expect("primary health should persist");
        let health = load_health_from_home(&root);
        assert_eq!(health.status, "failing");
        assert!(!health.database_writable);
        let diagnostic = health.last_error.expect("last error should load");
        assert_eq!(diagnostic.error_code, "PAPERCUTS-E4001");
        assert_eq!(diagnostic.timeout_seconds, Some(3));
        assert_eq!(diagnostic.attempt, 3);
        assert!(diagnostic.retryable);

        fs::write(
            primary.join("health.json"),
            r#"{"status":"degraded","database_writable":false,"last_error":{"error_code":"PAPERCUTS-E6003","stage":"io","message":"the collector could not remove a flushed spool file","operation":"drain","observed_at":"2026-08-14T22:06:27Z"}}"#,
        )
        .expect("legacy partial health should persist");
        let legacy_diagnostic = load_health_from_home(&root)
            .last_error
            .expect("legacy error should normalize");
        assert_eq!(legacy_diagnostic.failure_kind, "legacy_failure");
        assert!(legacy_diagnostic.retryable);
        assert_eq!(
            legacy_diagnostic.recovery_command,
            format!(
                "{} papercuts health --json",
                root.join(".codex/bin/pronto-papercuts").display()
            )
        );
        let _ = fs::remove_dir_all(root);
    }

    fn observation(event_key: &str, scope_id: &str) -> PapercutObservationInput {
        PapercutObservationInput {
            event_key: event_key.to_string(),
            scope_id: scope_id.to_string(),
            scope_kind: "repository".to_string(),
            domain: "software".to_string(),
            signal_kind: "correction".to_string(),
            target_kind: "agent_answer".to_string(),
            summary: "Agent claims success before forward verification".to_string(),
            excerpt: Some("That does not work in the installed app".to_string()),
            source: "codex-hook".to_string(),
            evidence_refs: vec!["verification:installed-app".to_string()],
            phenomenon_key: "premature success claim".to_string(),
            failure_mode: "missing forward verification".to_string(),
            priority: "P1".to_string(),
            urgent: false,
            verified: true,
            observed_at: None,
        }
    }

    #[test]
    fn promotes_local_then_cross_scope_patterns_and_deduplicates() {
        let (database, root) = test_database();
        let first = observe_at(&database, observation("turn-a:0", "repo-a"), false, false)
            .expect("first observation should persist");
        assert!(first.promoted_patterns.is_empty());
        let duplicate = observe_at(&database, observation("turn-a:0", "repo-a"), false, false)
            .expect("duplicate should be safe");
        assert!(duplicate.deduplicated);
        let second = observe_at(&database, observation("turn-b:0", "repo-a"), false, false)
            .expect("second observation should persist");
        assert!(second
            .promoted_patterns
            .iter()
            .any(|item| item.scope_kind == "local"));
        let third = observe_at(&database, observation("turn-c:0", "repo-b"), false, false)
            .expect("third observation should persist");
        assert!(third
            .promoted_patterns
            .iter()
            .any(|item| item.scope_kind == "cross_scope" && item.scope_count == 2));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn verified_urgent_failure_promotes_locally_but_not_cross_scope() {
        let (database, root) = test_database();
        let mut input = observation("urgent:0", "repo-a");
        input.signal_kind = "failed_verification".to_string();
        input.urgent = true;
        let result =
            observe_at(&database, input, false, false).expect("urgent signal should persist");
        assert_eq!(result.promoted_patterns.len(), 1);
        assert_eq!(result.promoted_patterns[0].evidence_tier, "urgent");
        assert_eq!(result.promoted_patterns[0].scope_kind, "local");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sanitizes_and_expires_short_excerpts() {
        let (database, root) = test_database();
        let mut input = observation("sanitize:0", "repo-a");
        input.excerpt = Some(format!(
            "See /Users/person/private.txt with sk-secret {}",
            "x".repeat(300)
        ));
        let result = observe_at(&database, input, false, false).expect("signal should persist");
        let excerpt = result
            .observation
            .excerpt
            .expect("excerpt should remain during retention");
        assert!(!excerpt.contains("/Users/"));
        assert!(!excerpt.contains("sk-secret"));
        assert!(excerpt.chars().count() <= EXCERPT_MAX_CHARS);
        assert!(result.observation.excerpt_expires_at.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sanitizes_path_bearing_scope_and_event_identifiers() {
        let (database, root) = test_database();
        let mut input = observation(
            "codex:/Users/person/private/session",
            "repository:/Users/person/private/repo",
        );
        input.source = "/Users/person/private/hook".to_string();
        let result = observe_at(&database, input, false, false).expect("signal should persist");
        assert!(result.observation.event_key.starts_with("opaque:v1:"));
        assert!(result.observation.scope_id.starts_with("opaque:v1:"));
        assert!(!result.observation.source.contains("/Users/"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn expired_excerpt_text_is_deleted_but_hash_and_summary_remain() {
        let (database, root) = test_database();
        let mut old = observation("expired:0", "repo-a");
        old.observed_at = Some("2020-01-01T00:00:00Z".to_string());
        let old_result =
            observe_at(&database, old, false, false).expect("old signal should persist");
        observe_at(&database, observation("fresh:0", "repo-b"), false, false)
            .expect("a later write should run retention pruning");
        let backlog = load_at(&database).expect("backlog should load");
        let retained = backlog
            .observations
            .iter()
            .find(|item| item.id == old_result.observation.id)
            .expect("structured observation should remain");
        assert!(retained.excerpt.is_none());
        assert!(!retained.excerpt_hash.is_empty());
        assert!(!retained.summary.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn transactionally_migrates_v1_rows_and_preserves_compatibility_fields() {
        let (database, root) = test_database();
        let connection = Connection::open(&database).expect("legacy database should open");
        connection
            .execute_batch(
                "CREATE TABLE papercuts (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL, detail TEXT NOT NULL,
                    family TEXT NOT NULL, surface TEXT NOT NULL, source TEXT NOT NULL,
                    evidence_refs_json TEXT NOT NULL, impact TEXT NOT NULL,
                    priority TEXT NOT NULL, status TEXT NOT NULL, next_action TEXT NOT NULL,
                    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, resolved_at TEXT
                );
                INSERT INTO papercuts VALUES (
                    'legacy-id', 'Legacy title', 'Legacy detail', 'design-audit',
                    'Pronto UI', 'manual', '[\"screen:legacy\"]', 'Legacy impact',
                    'P1', 'deferred', 'Review later', '2026-01-02T03:04:05Z',
                    '2026-02-03T04:05:06Z', NULL
                );",
            )
            .expect("legacy fixture should be created");
        drop(connection);

        let backlog = load_at(&database).expect("legacy database should migrate");
        assert_eq!(backlog.papercuts.len(), 1);
        assert_eq!(backlog.papercuts[0].id, "legacy-id");
        assert_eq!(backlog.papercuts[0].status, "deferred");
        assert_eq!(backlog.papercuts[0].evidence_refs, vec!["screen:legacy"]);
        assert_eq!(backlog.patterns[0].created_at, "2026-01-02T03:04:05Z");
        assert_eq!(backlog.patterns[0].updated_at, "2026-02-03T04:05:06Z");
        assert_eq!(backlog.observations[0].signal_kind, "legacy_manual");

        let second = load_at(&database).expect("migration should be idempotent");
        assert_eq!(second.patterns.len(), 1);
        assert_eq!(second.observations.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_capture_remains_visible_through_the_v1_projection() {
        let (database, root) = test_database();
        let backlog = create_at(
            &database,
            "The empty state hides the next action".to_string(),
            "A first-time user has to infer where to begin.".to_string(),
            "Pronto UI".to_string(),
            "manual".to_string(),
            "P2".to_string(),
            vec!["screen:portfolio-empty".to_string()],
            "Adds avoidable orientation cost.".to_string(),
            "Exercise the empty-state flow.".to_string(),
        )
        .expect("manual papercut should persist");
        assert_eq!(backlog.papercuts.len(), 1);
        assert_eq!(backlog.patterns.len(), 1);
        assert_eq!(backlog.observations.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn proposals_require_human_review_and_digest_is_idempotent() {
        let (database, root) = test_database();
        let mut input = observation("urgent-proposal:0", "repo-a");
        input.urgent = true;
        let observed = observe_at(&database, input, false, false).expect("pattern should exist");
        let pattern_id = observed.promoted_patterns[0].id.clone();
        let proposal = propose_at(
            &database,
            MultiplierProposalInput {
                pattern_ids: vec![pattern_id],
                title: "Require installed-surface verification".to_string(),
                hypothesis: "Source checks are mistaken for live behavior.".to_string(),
                root_cause: "Evidence states are collapsed.".to_string(),
                multiplier: "Add a forward-surface verification gate.".to_string(),
                evidence_tier: "urgent".to_string(),
            },
        )
        .expect("proposal should persist");
        assert_eq!(proposal.status, "draft");
        let first = digest_at(&database).expect("digest should persist");
        let second = digest_at(&database).expect("digest rerun should update deterministically");
        assert_eq!(first.id, second.id);
        let _ = fs::remove_dir_all(root);
    }
}
