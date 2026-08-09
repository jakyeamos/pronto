use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const SCHEMA_VERSION: &str = "pronto-papercuts/v1";
const FAMILY: &str = "design-audit";
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PapercutBacklog {
    pub schema_version: String,
    pub family: String,
    pub generated_at: String,
    pub papercuts: Vec<Papercut>,
    pub counts: PapercutCounts,
}

fn iso_now() -> String {
    Utc::now().to_rfc3339()
}

fn empty_backlog() -> PapercutBacklog {
    PapercutBacklog {
        schema_version: SCHEMA_VERSION.to_string(),
        family: FAMILY.to_string(),
        generated_at: iso_now(),
        papercuts: Vec::new(),
        counts: PapercutCounts::default(),
    }
}

fn ensure_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS papercuts (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 family TEXT NOT NULL,
                 surface TEXT NOT NULL,
                 source TEXT NOT NULL,
                 evidence_refs_json TEXT NOT NULL,
                 impact TEXT NOT NULL,
                 priority TEXT NOT NULL,
                 status TEXT NOT NULL,
                 next_action TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 resolved_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_papercuts_status_updated
                 ON papercuts (status, updated_at DESC);",
        )
        .map_err(|error| format!("Could not initialize Pronto papercut storage: {error}"))
}

fn table_exists(connection: &Connection) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'papercuts'
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect Pronto papercut storage: {error}"))
}

fn backlog_from_connection(connection: &Connection) -> Result<PapercutBacklog, String> {
    let rows = connection
        .prepare(
            "SELECT id, title, detail, family, surface, source,
                    evidence_refs_json, impact, priority, status, next_action,
                    created_at, updated_at, resolved_at
             FROM papercuts
             ORDER BY
                CASE status
                    WHEN 'open' THEN 0
                    WHEN 'in_progress' THEN 1
                    WHEN 'deferred' THEN 2
                    WHEN 'resolved' THEN 3
                    ELSE 4
                END,
                updated_at DESC,
                id DESC",
        )
        .map_err(|error| format!("Could not prepare Pronto papercuts query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto papercuts: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto papercuts: {error}"))?;

    let papercuts = rows
        .into_iter()
        .map(
            |(
                id,
                title,
                detail,
                family,
                surface,
                source,
                evidence_refs_json,
                impact,
                priority,
                status,
                next_action,
                created_at,
                updated_at,
                resolved_at,
            )| {
                let evidence_refs = serde_json::from_str(&evidence_refs_json).map_err(|error| {
                    format!("Could not decode papercut evidence references: {error}")
                })?;
                Ok(Papercut {
                    id,
                    title,
                    detail,
                    family,
                    surface,
                    source,
                    evidence_refs,
                    impact,
                    priority,
                    status,
                    next_action,
                    created_at,
                    updated_at,
                    resolved_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let mut counts = PapercutCounts {
        total: papercuts.len(),
        ..PapercutCounts::default()
    };
    for papercut in &papercuts {
        match papercut.status.as_str() {
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
    })
}

fn load_at(path: &Path) -> Result<PapercutBacklog, String> {
    if !path.is_file() {
        return Ok(empty_backlog());
    }
    let connection = crate::core::open_store_read_only_for_extension(path)?;
    if !table_exists(&connection)? {
        return Ok(empty_backlog());
    }
    backlog_from_connection(&connection)
}

fn required(value: String, label: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("Papercut {label} is required."));
    }
    Ok(value)
}

fn normalize_source(source: String) -> Result<String, String> {
    let source = source.trim().to_ascii_lowercase();
    if matches!(source.as_str(), "manual" | "design-friction") {
        Ok(source)
    } else {
        Err("Papercut source must be manual or design-friction.".to_string())
    }
}

fn normalize_priority(priority: String) -> Result<String, String> {
    let priority = priority.trim().to_ascii_uppercase();
    if matches!(priority.as_str(), "P0" | "P1" | "P2" | "P3") {
        Ok(priority)
    } else {
        Err("Papercut priority must be P0, P1, P2, or P3.".to_string())
    }
}

fn normalize_status(status: String) -> Result<String, String> {
    let status = status.trim().to_ascii_lowercase();
    if matches!(
        status.as_str(),
        "open" | "in_progress" | "deferred" | "resolved"
    ) {
        Ok(status)
    } else {
        Err("Papercut status must be open, in_progress, deferred, or resolved.".to_string())
    }
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
    let source = normalize_source(source)?;
    let priority = normalize_priority(priority)?;
    let surface = {
        let value = surface.trim().to_string();
        if value.is_empty() {
            "Pronto UI".to_string()
        } else {
            value
        }
    };
    let evidence_refs = evidence_refs
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let impact = impact.trim().to_string();
    let next_action = {
        let value = next_action.trim().to_string();
        if value.is_empty() {
            "Define the next validation step.".to_string()
        } else {
            value
        }
    };
    let created_at = iso_now();
    let id = format!(
        "papercut:{}:{}",
        created_at,
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    );
    let evidence_refs_json = serde_json::to_string(&evidence_refs)
        .map_err(|error| format!("Could not encode papercut evidence references: {error}"))?;

    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        connection
            .execute(
                "INSERT INTO papercuts
                 (id, title, detail, family, surface, source, evidence_refs_json,
                  impact, priority, status, next_action, created_at, updated_at, resolved_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'open', ?10, ?11, ?11, NULL)",
                params![
                    id,
                    title,
                    detail,
                    FAMILY,
                    surface,
                    source,
                    evidence_refs_json,
                    impact,
                    priority,
                    next_action,
                    created_at,
                ],
            )
            .map_err(|error| format!("Could not save Pronto papercut: {error}"))?;
        backlog_from_connection(connection)
    })
}

fn set_status_at(
    path: &Path,
    papercut_id: String,
    status: String,
) -> Result<PapercutBacklog, String> {
    let papercut_id = required(papercut_id, "id")?;
    let status = normalize_status(status)?;
    let updated_at = iso_now();
    let resolved_at = (status == "resolved").then(|| updated_at.clone());

    crate::core::with_store_write_for_extension(path, |connection| {
        ensure_schema(connection)?;
        let updated = connection
            .execute(
                "UPDATE papercuts
                 SET status = ?1, updated_at = ?2, resolved_at = ?3
                 WHERE id = ?4",
                params![status, updated_at, resolved_at, papercut_id],
            )
            .map_err(|error| format!("Could not update Pronto papercut status: {error}"))?;
        if updated == 0 {
            return Err("That papercut is no longer in the backlog.".to_string());
        }
        backlog_from_connection(connection)
    })
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
pub fn set_papercut_status(papercut_id: String, status: String) -> Result<PapercutBacklog, String> {
    set_status_at(&crate::core::local_store_path(), papercut_id, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_database() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "pronto-papercuts-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("test storage should be created");
        (root.join("registry.db"), root)
    }

    #[test]
    fn captures_design_friction_as_a_durable_open_papercut() {
        let (database, root) = test_database();
        let backlog = create_at(
            &database,
            "The empty state hides the next action".to_string(),
            "A first-time user has to infer where to begin.".to_string(),
            "Pronto UI".to_string(),
            "design-friction".to_string(),
            "P1".to_string(),
            vec!["screen:portfolio-empty".to_string()],
            "Adds avoidable orientation cost.".to_string(),
            "Exercise the empty-state flow after the copy change.".to_string(),
        )
        .expect("papercut should persist");

        assert_eq!(backlog.family, FAMILY);
        assert_eq!(backlog.counts.open, 1);
        assert_eq!(backlog.papercuts[0].source, "design-friction");
        assert_eq!(
            load_at(&database)
                .expect("papercut should reload")
                .counts
                .total,
            1
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_change_keeps_the_item_and_records_resolution() {
        let (database, root) = test_database();
        let created = create_at(
            &database,
            "Repeated navigation".to_string(),
            "The same context must be reconstructed across surfaces.".to_string(),
            "Pronto UI".to_string(),
            "manual".to_string(),
            "P2".to_string(),
            Vec::new(),
            String::new(),
            String::new(),
        )
        .expect("papercut should persist");
        let id = created.papercuts[0].id.clone();
        let updated = set_status_at(&database, id, "resolved".to_string())
            .expect("papercut status should persist");

        assert_eq!(updated.counts.resolved, 1);
        assert!(updated.papercuts[0].resolved_at.is_some());
        let _ = fs::remove_dir_all(root);
    }
}
