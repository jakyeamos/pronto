use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub const COLLECTOR_ADDRESS: &str = "127.0.0.1:43180";
pub const COLLECTOR_ENDPOINT: &str = "http://127.0.0.1:43180/v1/metrics";
const METRIC_NAME: &str = "codex.skill.injected";
const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
const HEARTBEAT_INTERVAL_SECONDS: u64 = 30;
const HEALTHY_HEARTBEAT_SECONDS: i64 = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtlpSkillUsage {
    pub all_time_count: u64,
    pub recent_count: u64,
    pub last_seen_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtlpUsageSnapshot {
    pub usage: HashMap<String, OtlpSkillUsage>,
    pub coverage_started_at_ms: i64,
    pub last_heartbeat_at_ms: i64,
    pub healthy: bool,
}

pub fn usage_database_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Pronto")
        .join("codex-skill-metrics.db")
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Skill metric database has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create the skill metric storage directory {}: {error}",
            parent.display()
        )
    })?;
    let connection = Connection::open(path).map_err(|error| {
        format!(
            "Could not open skill metric database {}: {error}",
            path.display()
        )
    })?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| format!("Could not configure skill metric storage: {error}"))?;
    connection
        .execute_batch(
            r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS collector_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS codex_skill_metric_series (
    series_key TEXT PRIMARY KEY,
    last_value INTEGER NOT NULL,
    last_point_time_unix_nano TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS codex_skill_metric_events (
    event_key TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL,
    invocation_type TEXT NOT NULL CHECK (invocation_type IN ('explicit', 'implicit')),
    status TEXT NOT NULL CHECK (status IN ('ok', 'error')),
    delta INTEGER NOT NULL CHECK (delta > 0),
    occurred_at_ms INTEGER NOT NULL
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_codex_skill_metric_events_name_time
    ON codex_skill_metric_events (skill_name, occurred_at_ms);
"#,
        )
        .map_err(|error| format!("Could not initialize skill metric storage: {error}"))?;
    Ok(connection)
}

fn record_heartbeat(path: &Path, now_ms: i64) -> Result<(), String> {
    let connection = open_database(path)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO collector_metadata (key, value) VALUES ('coverage_started_at_ms', ?1)",
            params![now_ms.to_string()],
        )
        .map_err(|error| format!("Could not record skill metric coverage start: {error}"))?;
    connection
        .execute(
            "INSERT INTO collector_metadata (key, value) VALUES ('last_heartbeat_at_ms', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now_ms.to_string()],
        )
        .map_err(|error| format!("Could not record skill metric collector heartbeat: {error}"))?;
    Ok(())
}

fn metadata_i64(connection: &Connection, key: &str) -> Result<Option<i64>, String> {
    let value = connection
        .query_row(
            "SELECT value FROM collector_metadata WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Could not read skill metric metadata {key}: {error}"))?;
    value
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("Skill metric metadata {key} is invalid: {error}"))
        })
        .transpose()
}

pub fn read_usage_snapshot(
    path: &Path,
    cutoff_ms: i64,
    now_ms: i64,
) -> Result<OtlpUsageSnapshot, String> {
    if !path.is_file() {
        return Err(format!(
            "The Codex OTLP compatibility feed has not been created at {}.",
            path.display()
        ));
    }
    let connection = open_database(path)?;
    let coverage_started_at_ms = metadata_i64(&connection, "coverage_started_at_ms")?
        .ok_or_else(|| "The Codex OTLP compatibility feed has no coverage start.".to_string())?;
    let last_heartbeat_at_ms = metadata_i64(&connection, "last_heartbeat_at_ms")?
        .ok_or_else(|| "The Codex OTLP compatibility collector has no heartbeat.".to_string())?;
    let healthy = now_ms.saturating_sub(last_heartbeat_at_ms)
        <= HEALTHY_HEARTBEAT_SECONDS.saturating_mul(1_000);
    let mut statement = connection
        .prepare(
            r#"
SELECT
    lower(skill_name),
    SUM(delta),
    SUM(CASE WHEN occurred_at_ms >= ?1 THEN delta ELSE 0 END),
    MAX(occurred_at_ms)
FROM codex_skill_metric_events
WHERE status = 'ok'
GROUP BY lower(skill_name)
"#,
        )
        .map_err(|error| format!("Could not prepare the Codex OTLP usage query: {error}"))?;
    let rows = statement
        .query_map(params![cutoff_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| format!("Could not query Codex OTLP skill usage: {error}"))?;
    let mut usage = HashMap::new();
    for row in rows {
        let (skill_name, all_time_count, recent_count, last_seen_at_ms) =
            row.map_err(|error| format!("Could not decode Codex OTLP skill usage: {error}"))?;
        usage.insert(
            skill_name,
            OtlpSkillUsage {
                all_time_count: all_time_count.max(0) as u64,
                recent_count: recent_count.max(0) as u64,
                last_seen_at_ms,
            },
        );
    }
    Ok(OtlpUsageSnapshot {
        usage,
        coverage_started_at_ms,
        last_heartbeat_at_ms,
        healthy,
    })
}

fn string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn unsigned_value(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::String(value) => value.parse().ok(),
        Value::Number(value) => value.as_u64(),
        _ => None,
    }
}

fn attribute_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|attribute| {
            let key = attribute.get("key")?.as_str()?.to_string();
            let encoded = attribute.get("value")?;
            let value = string_value(encoded.get("stringValue"))
                .or_else(|| string_value(encoded.get("intValue")))
                .or_else(|| string_value(encoded.get("boolValue")))
                .or_else(|| string_value(encoded.get("doubleValue")))?;
            Some((key, value))
        })
        .collect()
}

fn sha256(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

fn resource_fingerprint(resource: Option<&Value>) -> String {
    let attributes = attribute_map(resource.and_then(|resource| resource.get("attributes")));
    let canonical = attributes
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    sha256(&[canonical.as_str()])
}

fn persist_point(
    transaction: &rusqlite::Transaction<'_>,
    resource_id: &str,
    temporality: u64,
    point: &Value,
) -> Result<u64, String> {
    let attributes = attribute_map(point.get("attributes"));
    let skill_name = attributes
        .get("skill")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or_else(|| "Codex skill metric point has no bounded skill attribute.".to_string())?;
    let invocation_type = attributes
        .get("invoke_type")
        .map(String::as_str)
        .filter(|value| matches!(*value, "explicit" | "implicit"))
        .ok_or_else(|| "Codex skill metric point has an invalid invoke_type.".to_string())?;
    let status = attributes
        .get("status")
        .map(String::as_str)
        .filter(|value| matches!(*value, "ok" | "error"))
        .ok_or_else(|| "Codex skill metric point has an invalid status.".to_string())?;
    let start_time = string_value(point.get("startTimeUnixNano"))
        .filter(|value| value.parse::<u64>().is_ok())
        .ok_or_else(|| "Codex skill metric point has no valid startTimeUnixNano.".to_string())?;
    let point_time = unsigned_value(point.get("timeUnixNano"))
        .ok_or_else(|| "Codex skill metric point has no valid timeUnixNano.".to_string())?;
    let value = unsigned_value(point.get("asInt"))
        .or_else(|| unsigned_value(point.get("asDouble")))
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or_else(|| "Codex skill metric point has no bounded integer value.".to_string())?;
    let series_key = sha256(&[
        resource_id,
        skill_name,
        invocation_type,
        status,
        start_time.as_str(),
    ]);
    let prior = transaction
        .query_row(
            "SELECT last_value, last_point_time_unix_nano FROM codex_skill_metric_series WHERE series_key = ?1",
            params![series_key],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| format!("Could not read Codex skill metric series state: {error}"))?;
    if let Some((_, prior_time)) = &prior {
        if prior_time.parse::<u64>().unwrap_or_default() >= point_time {
            return Ok(0);
        }
    }
    let delta = match temporality {
        1 => value,
        2 => prior
            .map(|(prior_value, _)| {
                let prior_value = prior_value.max(0) as u64;
                value.checked_sub(prior_value).unwrap_or(value)
            })
            .unwrap_or(value),
        _ => {
            return Err(format!(
                "Unsupported Codex metric temporality {temporality}."
            ))
        }
    };
    transaction
        .execute(
            "INSERT INTO codex_skill_metric_series (series_key, last_value, last_point_time_unix_nano) VALUES (?1, ?2, ?3) ON CONFLICT(series_key) DO UPDATE SET last_value = excluded.last_value, last_point_time_unix_nano = excluded.last_point_time_unix_nano",
            params![series_key, value as i64, point_time.to_string()],
        )
        .map_err(|error| format!("Could not update Codex skill metric series state: {error}"))?;
    if delta == 0 {
        return Ok(0);
    }
    let event_key = sha256(&[
        series_key.as_str(),
        point_time.to_string().as_str(),
        value.to_string().as_str(),
        temporality.to_string().as_str(),
    ]);
    let inserted = transaction
        .execute(
            "INSERT OR IGNORE INTO codex_skill_metric_events (event_key, skill_name, invocation_type, status, delta, occurred_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event_key,
                skill_name,
                invocation_type,
                status,
                delta as i64,
                (point_time / 1_000_000) as i64
            ],
        )
        .map_err(|error| format!("Could not persist Codex skill metric delta: {error}"))?;
    Ok(if inserted == 1 { delta } else { 0 })
}

pub fn ingest_otlp_json(path: &Path, body: &[u8], received_at_ms: i64) -> Result<u64, String> {
    let payload: Value = serde_json::from_slice(body)
        .map_err(|error| format!("Could not decode OTLP JSON metrics: {error}"))?;
    let mut connection = open_database(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not start Codex skill metric transaction: {error}"))?;
    let mut accepted_delta = 0_u64;
    let resource_metrics = payload
        .get("resourceMetrics")
        .and_then(Value::as_array)
        .ok_or_else(|| "OTLP JSON payload has no resourceMetrics array.".to_string())?;
    for resource_metric in resource_metrics {
        let resource_id = resource_fingerprint(resource_metric.get("resource"));
        let Some(scope_metrics) = resource_metric
            .get("scopeMetrics")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for scope_metric in scope_metrics {
            let Some(metrics) = scope_metric.get("metrics").and_then(Value::as_array) else {
                continue;
            };
            for metric in metrics {
                if metric.get("name").and_then(Value::as_str) != Some(METRIC_NAME) {
                    continue;
                }
                let Some(sum) = metric.get("sum") else {
                    continue;
                };
                let temporality =
                    unsigned_value(sum.get("aggregationTemporality")).ok_or_else(|| {
                        "Codex skill metric has no aggregation temporality.".to_string()
                    })?;
                let Some(data_points) = sum.get("dataPoints").and_then(Value::as_array) else {
                    continue;
                };
                for point in data_points.iter().take(10_000) {
                    accepted_delta = accepted_delta.saturating_add(persist_point(
                        &transaction,
                        resource_id.as_str(),
                        temporality,
                        point,
                    )?);
                }
            }
        }
    }
    transaction
        .execute(
            "INSERT OR IGNORE INTO collector_metadata (key, value) VALUES ('coverage_started_at_ms', ?1)",
            params![received_at_ms.to_string()],
        )
        .map_err(|error| format!("Could not record Codex OTLP coverage start: {error}"))?;
    transaction
        .execute(
            "INSERT INTO collector_metadata (key, value) VALUES ('last_received_at_ms', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![received_at_ms.to_string()],
        )
        .map_err(|error| format!("Could not record Codex OTLP receipt time: {error}"))?;
    transaction
        .execute(
            "INSERT INTO collector_metadata (key, value) VALUES ('last_heartbeat_at_ms', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![received_at_ms.to_string()],
        )
        .map_err(|error| format!("Could not record Codex OTLP collector health: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Could not commit Codex skill metrics: {error}"))?;
    Ok(accepted_delta)
}

fn json_response(status: u16, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes("Content-Type", "application/json") {
        response.add_header(header);
    }
    response
}

fn handle_request(mut request: Request, path: &Path) {
    if request.method() != &Method::Post || request.url() != "/v1/metrics" {
        let _ = request.respond(json_response(404, r#"{"error":"not found"}"#));
        return;
    }
    if request
        .body_length()
        .is_some_and(|length| length > MAX_REQUEST_BYTES)
    {
        let _ = request.respond(json_response(413, r#"{"error":"payload too large"}"#));
        return;
    }
    let content_type_is_json = request.headers().iter().any(|header| {
        header.field.equiv("Content-Type") && header.value.as_str().starts_with("application/json")
    });
    if !content_type_is_json {
        let _ = request.respond(json_response(415, r#"{"error":"OTLP JSON required"}"#));
        return;
    }
    let mut body = Vec::new();
    if request
        .as_reader()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .is_err()
        || body.len() > MAX_REQUEST_BYTES
    {
        let _ = request.respond(json_response(400, r#"{"error":"invalid request body"}"#));
        return;
    }
    match ingest_otlp_json(path, &body, Utc::now().timestamp_millis()) {
        Ok(_) => {
            let _ = request.respond(json_response(200, "{}"));
        }
        Err(error) => {
            log::warn!("rejected Codex OTLP skill metrics: {error}");
            let _ = request.respond(json_response(
                400,
                r#"{"error":"invalid OTLP metrics payload"}"#,
            ));
        }
    }
}

pub fn run() -> Result<(), String> {
    let path = usage_database_path();
    record_heartbeat(&path, Utc::now().timestamp_millis())?;
    let server = Server::http(COLLECTOR_ADDRESS)
        .map_err(|error| format!("Could not bind {COLLECTOR_ADDRESS}: {error}"))?;
    loop {
        match server.recv_timeout(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS)) {
            Ok(Some(request)) => handle_request(request, &path),
            Ok(None) => {}
            Err(error) => return Err(format!("Skill usage collector receive failed: {error}")),
        }
        record_heartbeat(&path, Utc::now().timestamp_millis())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    fn fixture_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "pronto-codex-otlp-{}-{}-{}.sqlite",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn payload(value: u64, time: u64, status: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "resourceMetrics": [{
                "resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "codex"}}]},
                "scopeMetrics": [{
                    "metrics": [{
                        "name": METRIC_NAME,
                        "sum": {
                            "aggregationTemporality": 2,
                            "isMonotonic": true,
                            "dataPoints": [{
                                "attributes": [
                                    {"key": "skill", "value": {"stringValue": "Example"}},
                                    {"key": "status", "value": {"stringValue": status}},
                                    {"key": "invoke_type", "value": {"stringValue": "explicit"}}
                                ],
                                "startTimeUnixNano": "1000000000",
                                "timeUnixNano": time.to_string(),
                                "asInt": value.to_string()
                            }]
                        }
                    }]
                }]
            }]
        }))
        .expect("encode OTLP fixture")
    }

    #[test]
    fn cumulative_points_are_idempotent_and_store_only_deltas() {
        let path = fixture_path();
        assert_eq!(
            ingest_otlp_json(&path, &payload(2, 2_000_000_000, "ok"), 2_000).unwrap(),
            2
        );
        assert_eq!(
            ingest_otlp_json(&path, &payload(2, 2_000_000_000, "ok"), 2_000).unwrap(),
            0
        );
        assert_eq!(
            ingest_otlp_json(&path, &payload(5, 3_000_000_000, "ok"), 3_000).unwrap(),
            3
        );
        record_heartbeat(&path, 3_000).unwrap();

        let snapshot = read_usage_snapshot(&path, 0, 3_000).unwrap();
        let usage = snapshot.usage.get("example").expect("example usage");
        assert_eq!(usage.all_time_count, 5);
        assert_eq!(usage.recent_count, 5);
        assert!(snapshot.healthy);
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn failed_metrics_are_retained_but_excluded_from_usage() {
        let path = fixture_path();
        ingest_otlp_json(&path, &payload(4, 2_000_000_000, "error"), 2_000).unwrap();
        record_heartbeat(&path, 2_000).unwrap();

        let snapshot = read_usage_snapshot(&path, 0, 2_000).unwrap();
        assert!(snapshot.usage.is_empty());
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn stale_heartbeat_preserves_recorded_counts_but_marks_feed_unhealthy() {
        let path = fixture_path();
        ingest_otlp_json(&path, &payload(1, 2_000_000_000, "ok"), 2_000).unwrap();
        record_heartbeat(&path, 2_000).unwrap();

        let snapshot = read_usage_snapshot(&path, 0, 200_000).unwrap();
        assert_eq!(snapshot.usage["example"].all_time_count, 1);
        assert!(!snapshot.healthy);
        fs::remove_file(path).expect("remove fixture");
    }
}
