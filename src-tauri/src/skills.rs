use crate::skill_usage_collector;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "pronto-skills/v4";
const QUALITY_RUNNER_CAPABILITY_SCHEMA: &str = "quality-runner-skill-capability/v1";
const QUALITY_RUNNER_CAPABILITY_FEED: &str =
    ".quality-runner/skill-capabilities/current/capabilities.json";
const BUILT_IN_PAPERCUTS_ID: &str = "papercuts";
const RECENT_DAYS: i64 = 30;
const MAX_FILES: usize = 3_000;
const MAX_FILE_BYTES: u64 = 256 * 1024;
const CODEX_USAGE_DATABASE: &str = "state_5.sqlite";
const CODEX_USAGE_SOURCE: &str =
    "Codex structured skill-invocation state (~/.codex/state_5.sqlite)";
const CODEX_OTLP_USAGE_SOURCE: &str = "Codex OTLP skill metric compatibility feed (localhost)";
const STRUCTURED_USAGE_UNAVAILABLE_REASON: &str =
    "The installed Codex state database does not expose the structured skill_invocations feed yet.";
const STRUCTURED_USAGE_SOURCE: &str =
    "Unavailable; catalog, prompt, and transcript text are never counted as invocations.";
const PARTIAL_USAGE_GAP: &str = "Codex usage is observed from runtime events; Claude, Gemini, Cursor, and pre-instrumentation history remain unavailable.";

fn default_usage_state() -> String {
    "unavailable".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillUsage {
    #[serde(default = "default_usage_state")]
    pub state: String,
    #[serde(default)]
    pub recent_count: u64,
    #[serde(default)]
    pub all_time_count: u64,
    #[serde(default)]
    pub by_provider: BTreeMap<String, u64>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub telemetry_source: String,
    #[serde(default)]
    pub reason: String,
}

impl Default for SkillUsage {
    fn default() -> Self {
        Self {
            state: default_usage_state(),
            recent_count: 0,
            all_time_count: 0,
            by_provider: BTreeMap::new(),
            last_seen_at: None,
            telemetry_source: STRUCTURED_USAGE_SOURCE.into(),
            reason: STRUCTURED_USAGE_UNAVAILABLE_REASON.into(),
        }
    }
}

#[derive(Debug)]
enum CodexUsageFeed {
    Observed {
        usage: HashMap<String, SkillUsage>,
        empty_usage: SkillUsage,
    },
    Unavailable(String),
}

fn codex_usage_database_path() -> PathBuf {
    std::env::var_os("CODEX_SQLITE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex"))
        .join(CODEX_USAGE_DATABASE)
}

fn read_codex_usage_feed(path: &Path) -> CodexUsageFeed {
    if !path.is_file() {
        return CodexUsageFeed::Unavailable(format!(
            "Codex state database was not found at {}.",
            path.display()
        ));
    }
    let connection = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(connection) => connection,
        Err(error) => {
            return CodexUsageFeed::Unavailable(format!(
                "Could not open Codex state database at {}: {error}",
                path.display()
            ));
        }
    };
    let has_table = match connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'skill_invocations')",
        [],
        |row| row.get::<_, bool>(0),
    ) {
        Ok(has_table) => has_table,
        Err(error) => {
            return CodexUsageFeed::Unavailable(format!(
                "Could not inspect Codex skill invocation storage: {error}"
            ));
        }
    };
    if !has_table {
        return CodexUsageFeed::Unavailable(STRUCTURED_USAGE_UNAVAILABLE_REASON.into());
    }

    let cutoff_ms = (Utc::now() - Duration::days(RECENT_DAYS)).timestamp_millis();
    let mut statement = match connection.prepare(
        r#"
SELECT
    lower(skill_name),
    COUNT(*),
    SUM(CASE WHEN occurred_at_ms >= ?1 THEN 1 ELSE 0 END),
    MAX(occurred_at_ms)
FROM skill_invocations
WHERE status = 'ok'
GROUP BY lower(skill_name)
"#,
    ) {
        Ok(statement) => statement,
        Err(error) => {
            return CodexUsageFeed::Unavailable(format!(
                "Could not prepare the Codex skill usage query: {error}"
            ));
        }
    };
    let rows = match statement.query_map(params![cutoff_ms], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            return CodexUsageFeed::Unavailable(format!(
                "Could not query Codex skill invocation storage: {error}"
            ));
        }
    };

    let mut usage = HashMap::new();
    for row in rows {
        let (name, all_time_count, recent_count, last_seen_at_ms) = match row {
            Ok(row) => row,
            Err(error) => {
                return CodexUsageFeed::Unavailable(format!(
                    "Could not decode a Codex skill invocation row: {error}"
                ));
            }
        };
        let Some(last_seen_at) = DateTime::<Utc>::from_timestamp_millis(last_seen_at_ms) else {
            return CodexUsageFeed::Unavailable(format!(
                "Codex skill invocation storage contains an invalid timestamp: {last_seen_at_ms}"
            ));
        };
        let all_time_count = all_time_count.max(0) as u64;
        let recent_count = recent_count.max(0) as u64;
        usage.insert(
            name,
            SkillUsage {
                state: "observed".into(),
                recent_count,
                all_time_count,
                by_provider: BTreeMap::from([("codex".into(), all_time_count)]),
                last_seen_at: Some(last_seen_at.to_rfc3339()),
                telemetry_source: CODEX_USAGE_SOURCE.into(),
                reason:
                    "Recorded by the Codex runtime; failed loads and transcript text are excluded."
                        .into(),
            },
        );
    }
    CodexUsageFeed::Observed {
        usage,
        empty_usage: SkillUsage {
            state: "observed".into(),
            recent_count: 0,
            all_time_count: 0,
            by_provider: BTreeMap::from([("codex".into(), 0)]),
            last_seen_at: None,
            telemetry_source: CODEX_USAGE_SOURCE.into(),
            reason: "No successful Codex runtime invocation has been recorded for this skill since the structured feed was installed.".into(),
        },
    }
}

fn read_preferred_codex_usage_feed(
    sqlite_path: &Path,
    otlp_path: &Path,
    now: DateTime<Utc>,
) -> CodexUsageFeed {
    let sqlite_feed = read_codex_usage_feed(sqlite_path);
    if matches!(sqlite_feed, CodexUsageFeed::Observed { .. }) {
        return sqlite_feed;
    }
    let sqlite_reason = match sqlite_feed {
        CodexUsageFeed::Unavailable(reason) => reason,
        CodexUsageFeed::Observed { .. } => unreachable!(),
    };
    let cutoff_ms = (now - Duration::days(RECENT_DAYS)).timestamp_millis();
    let otlp = match skill_usage_collector::read_usage_snapshot(
        otlp_path,
        cutoff_ms,
        now.timestamp_millis(),
    ) {
        Ok(otlp) => otlp,
        Err(otlp_reason) => {
            return CodexUsageFeed::Unavailable(format!(
                "{sqlite_reason} The Codex OTLP compatibility feed is also unavailable: {otlp_reason}"
            ));
        }
    };
    let coverage_started_at = DateTime::<Utc>::from_timestamp_millis(otlp.coverage_started_at_ms)
        .map(|time| time.to_rfc3339())
        .unwrap_or_else(|| otlp.coverage_started_at_ms.to_string());
    let last_heartbeat_at = DateTime::<Utc>::from_timestamp_millis(otlp.last_heartbeat_at_ms)
        .map(|time| time.to_rfc3339())
        .unwrap_or_else(|| otlp.last_heartbeat_at_ms.to_string());
    let health_note = if otlp.healthy {
        format!("The localhost collector is healthy as of {last_heartbeat_at}.")
    } else {
        format!(
            "Recorded counts remain valid, but the localhost collector heartbeat is stale at {last_heartbeat_at}; current coverage may be interrupted."
        )
    };
    let reason = format!(
        "Recorded from Codex OTLP metric deltas since {coverage_started_at}; earlier history and non-Codex providers remain unavailable. {health_note}"
    );
    let usage = otlp
        .usage
        .into_iter()
        .filter_map(|(skill_name, usage)| {
            DateTime::<Utc>::from_timestamp_millis(usage.last_seen_at_ms).map(|last_seen_at| {
                (
                    skill_name,
                    SkillUsage {
                        state: "observed".into(),
                        recent_count: usage.recent_count,
                        all_time_count: usage.all_time_count,
                        by_provider: BTreeMap::from([("codex".into(), usage.all_time_count)]),
                        last_seen_at: Some(last_seen_at.to_rfc3339()),
                        telemetry_source: CODEX_OTLP_USAGE_SOURCE.into(),
                        reason: reason.clone(),
                    },
                )
            })
        })
        .collect();
    CodexUsageFeed::Observed {
        usage,
        empty_usage: SkillUsage {
            state: "observed".into(),
            recent_count: 0,
            all_time_count: 0,
            by_provider: BTreeMap::from([("codex".into(), 0)]),
            last_seen_at: None,
            telemetry_source: CODEX_OTLP_USAGE_SOURCE.into(),
            reason,
        },
    }
}

fn usage_for_skill(skill_id: &str, feed: &CodexUsageFeed) -> SkillUsage {
    match feed {
        CodexUsageFeed::Observed { usage, empty_usage } => usage
            .get(&skill_id.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| empty_usage.clone()),
        CodexUsageFeed::Unavailable(reason) => SkillUsage {
            reason: reason.clone(),
            ..SkillUsage::default()
        },
    }
}

fn update_snapshot_usage_summary(snapshot: &mut SkillsSnapshot) {
    if snapshot
        .skills
        .iter()
        .any(|skill| skill.usage.state == "observed")
    {
        let observed = snapshot
            .skills
            .iter()
            .find(|skill| skill.usage.state == "observed")
            .expect("observed usage exists");
        snapshot.source = format!("Local skill roots and {}", observed.usage.telemetry_source);
        snapshot.telemetry_gap = if observed.usage.telemetry_source == CODEX_USAGE_SOURCE {
            PARTIAL_USAGE_GAP.into()
        } else {
            observed.usage.reason.clone()
        };
    } else {
        snapshot.source = "Local skill roots; usage requires structured provider telemetry".into();
        snapshot.telemetry_gap = snapshot
            .skills
            .iter()
            .find_map(|skill| {
                (!skill.usage.reason.trim().is_empty()).then(|| skill.usage.reason.clone())
            })
            .unwrap_or_else(|| STRUCTURED_USAGE_UNAVAILABLE_REASON.into());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillProviderState {
    pub state: String,
    pub reason: String,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSource {
    pub path: String,
    pub root: String,
    pub provenance: String,
    pub sha256: String,
    pub hosted_in_jakye_agent_setup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillFindingClass {
    pub id: String,
    pub label: String,
    pub state: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillBackfillPhase {
    pub id: String,
    pub state: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillBackfillCapability {
    pub mode: String,
    pub phases: Vec<SkillBackfillPhase>,
    pub safety: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillQualityRunnerCoverage {
    pub rule_count: u64,
    pub finding_count: u64,
    pub statuses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillQualityRunnerRepresentation {
    pub status: String,
    pub adapter: String,
    pub finding_categories: Vec<String>,
    pub coverage: SkillQualityRunnerCoverage,
    pub evidence: Vec<String>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillFindingCapability {
    pub finding_expectation: String,
    pub finding_expectation_reason: String,
    pub finding_classes: Vec<SkillFindingClass>,
    pub backfill: SkillBackfillCapability,
    pub quality_runner: SkillQualityRunnerRepresentation,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_skill_category")]
    pub category: String,
    #[serde(default = "default_skill_family")]
    pub family: String,
    pub lifecycle: String,
    pub hosted_in_jakye_agent_setup: bool,
    pub sources: Vec<SkillSource>,
    pub providers: BTreeMap<String, SkillProviderState>,
    pub parity_score: Option<f64>,
    pub parity_evidence: Vec<String>,
    pub usage: SkillUsage,
    #[serde(default)]
    pub finding_capability: SkillFindingCapability,
}

fn default_skill_category() -> String {
    "Other".into()
}

fn default_skill_family() -> String {
    "Standalone".into()
}

fn backfill_phase(id: &str, state: &str, evidence: &str) -> SkillBackfillPhase {
    SkillBackfillPhase {
        id: id.into(),
        state: state.into(),
        evidence: evidence.into(),
    }
}

fn report_only_backfill() -> SkillBackfillCapability {
    SkillBackfillCapability {
        mode: "report_and_plan".into(),
        phases: vec![
            backfill_phase(
                "detect",
                "native",
                "Quality Runner evaluates the configured skill rules.",
            ),
            backfill_phase(
                "report",
                "native",
                "Findings are emitted with evidence, risk, and expected improvement.",
            ),
            backfill_phase(
                "plan",
                "available",
                "Findings can enter existing remediation and handoff projections.",
            ),
            backfill_phase(
                "apply",
                "unsupported",
                "Quality Runner does not apply code changes from a skill finding.",
            ),
            backfill_phase(
                "verify",
                "required",
                "A later gate or owner-controlled check must verify the remediation result.",
            ),
        ],
        safety: "Report-only; no automatic code changes are authorized by this capability record."
            .into(),
    }
}

fn unknown_backfill() -> SkillBackfillCapability {
    SkillBackfillCapability {
        mode: "not_evidenced".into(),
        phases: vec![
            backfill_phase(
                "detect",
                "not_evidenced",
                "No reviewed Quality Runner representation was found.",
            ),
            backfill_phase(
                "report",
                "not_evidenced",
                "No finding-producing adapter was found.",
            ),
            backfill_phase(
                "plan",
                "not_evidenced",
                "Backfill planning cannot be inferred from skill inventory alone.",
            ),
            backfill_phase(
                "apply",
                "unsupported",
                "No automatic application path is implied.",
            ),
            backfill_phase(
                "verify",
                "not_evidenced",
                "Verification evidence is unavailable.",
            ),
        ],
        safety: "Inventory evidence only; review is required before treating this skill as a finding source."
            .into(),
    }
}

fn quality_runner_representation(
    status: &str,
    adapter: &str,
    categories: &[&str],
    evidence: &[&str],
    gaps: &[&str],
) -> SkillQualityRunnerRepresentation {
    SkillQualityRunnerRepresentation {
        status: status.into(),
        adapter: adapter.into(),
        finding_categories: categories.iter().map(|value| (*value).into()).collect(),
        coverage: SkillQualityRunnerCoverage::default(),
        evidence: evidence.iter().map(|value| (*value).into()).collect(),
        gaps: gaps.iter().map(|value| (*value).into()).collect(),
    }
}

fn papercuts_finding_capability() -> SkillFindingCapability {
    SkillFindingCapability {
        finding_expectation: "required".into(),
        finding_expectation_reason:
            "Papercuts is a native Pronto design-audit surface that turns durable friction into backlog findings."
                .into(),
        finding_classes: vec![SkillFindingClass {
            id: "papercut".into(),
            label: "Durable friction finding".into(),
            state: "native".into(),
            evidence: "Native Pronto Papercuts backlog record.".into(),
        }],
        backfill: SkillBackfillCapability {
            mode: "capture_and_triage".into(),
            phases: vec![
                backfill_phase("detect", "native", "Design-audit friction can be captured."),
                backfill_phase("report", "native", "Papercuts are persisted as reviewable backlog records."),
                backfill_phase("plan", "available", "Status and notes support owner-controlled triage."),
                backfill_phase("apply", "unsupported", "Papercuts do not apply code changes."),
                backfill_phase("verify", "required", "Resolution must be checked against the original friction."),
            ],
            safety: "Backlog capture is owner-controlled and does not imply a code change.".into(),
        },
        quality_runner: quality_runner_representation(
            "not_applicable",
            "pronto_native_backlog",
            &[],
            &["Native Pronto finding surface; Quality Runner representation is not expected."],
            &[],
        ),
        gaps: vec!["Quality Runner is not the owner of this native Pronto finding surface.".into()],
    }
}

fn debloat_finding_capability() -> SkillFindingCapability {
    SkillFindingCapability {
        finding_expectation: "required".into(),
        finding_expectation_reason:
            "Debloat is an audit skill: it should produce reviewable structural candidate findings rather than silently deleting code."
                .into(),
        finding_classes: vec![
            SkillFindingClass {
                id: "large-source-file".into(),
                label: "Large source file".into(),
                state: "native".into(),
                evidence: "Native Quality Runner debloat rule.".into(),
            },
            SkillFindingClass {
                id: "fat-router".into(),
                label: "Fat router".into(),
                state: "native".into(),
                evidence: "Native Quality Runner debloat rule.".into(),
            },
            SkillFindingClass {
                id: "ownership-pressure-review".into(),
                label: "Ownership-pressure review".into(),
                state: "review_required".into(),
                evidence: "Structural signals are candidate triggers; ownership and deletion decisions require review."
                    .into(),
            },
        ],
        backfill: report_only_backfill(),
        quality_runner: quality_runner_representation(
            "adapter_defined",
            "native_debloat_category",
            &["debloat"],
            &["Quality Runner defines native debloat structural signals: large-source-file and fat-router."],
            &["No current Quality Runner scan was supplied, so repository-specific coverage is unknown."],
        ),
        gaps: vec![
            "File size and router size are triggers for a read-only audit, not proof of bloat or authorization to delete code."
                .into(),
            "Apply and final verification remain owner-controlled.".into(),
        ],
    }
}

fn unknown_finding_capability() -> SkillFindingCapability {
    SkillFindingCapability {
        finding_expectation: "review_required".into(),
        finding_expectation_reason:
            "No reviewed finding profile or Quality Runner capability evidence was found for this skill."
                .into(),
        finding_classes: Vec::new(),
        backfill: unknown_backfill(),
        quality_runner: quality_runner_representation(
            "unknown",
            "",
            &[],
            &[],
            &["No Quality Runner capability feed or reviewed adapter was found."],
        ),
        gaps: vec![
            "Review whether this skill should produce findings before adding it to the Quality Runner representation."
                .into(),
        ],
    }
}

fn is_debloat_skill(id: &str, name: &str, description: &str) -> bool {
    format!("{id} {name} {description}")
        .to_ascii_lowercase()
        .contains("debloat")
}

fn fallback_finding_capability(id: &str, name: &str, description: &str) -> SkillFindingCapability {
    if id.eq_ignore_ascii_case(BUILT_IN_PAPERCUTS_ID) {
        return papercuts_finding_capability();
    }
    if is_debloat_skill(id, name, description) {
        return debloat_finding_capability();
    }
    unknown_finding_capability()
}

#[derive(Debug, Clone, Deserialize)]
struct QualityRunnerCapabilityFeed {
    schema: String,
    #[serde(default)]
    generated_at: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    skills: Vec<QualityRunnerCapabilityRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct QualityRunnerCapabilityRecord {
    id: String,
    #[serde(flatten)]
    capability: SkillFindingCapability,
}

fn read_quality_runner_capability_feed() -> Option<QualityRunnerCapabilityFeed> {
    let path = home().join(QUALITY_RUNNER_CAPABILITY_FEED);
    let payload = fs::read_to_string(path).ok()?;
    let feed = serde_json::from_str::<QualityRunnerCapabilityFeed>(&payload).ok()?;
    (feed.schema == QUALITY_RUNNER_CAPABILITY_SCHEMA).then_some(feed)
}

fn apply_finding_capabilities(snapshot: &mut SkillsSnapshot) {
    let feed = read_quality_runner_capability_feed();
    for skill in &mut snapshot.skills {
        let fallback = fallback_finding_capability(&skill.id, &skill.name, &skill.description);
        skill.finding_capability = feed
            .as_ref()
            .and_then(|value| {
                value
                    .skills
                    .iter()
                    .find(|candidate| candidate.id.eq_ignore_ascii_case(&skill.id))
            })
            .map(|candidate| candidate.capability.clone())
            .unwrap_or(fallback);
        if let Some(value) = feed.as_ref() {
            if let Some(generated_at) = value.generated_at.as_deref() {
                skill
                    .finding_capability
                    .quality_runner
                    .evidence
                    .push(format!("Capability feed generated at {generated_at}."));
            }
            if let Some(run_id) = value.run_id.as_deref() {
                skill
                    .finding_capability
                    .quality_runner
                    .evidence
                    .push(format!("Source Quality Runner run: {run_id}."));
            }
        }
    }
}

fn built_in_papercuts_skill() -> SkillRecord {
    let mut providers = BTreeMap::new();
    providers.insert(
        "pronto".into(),
        SkillProviderState {
            state: "native".into(),
            reason: "Native Pronto design-audit backlog surface".into(),
            source_path: None,
        },
    );

    SkillRecord {
        id: BUILT_IN_PAPERCUTS_ID.into(),
        name: "Papercuts".into(),
        description: "Capture and triage durable small hurts from the design-audit family.".into(),
        category: "UI & Design".into(),
        family: "Design Audit".into(),
        lifecycle: "canonical".into(),
        hosted_in_jakye_agent_setup: false,
        sources: Vec::new(),
        providers,
        parity_score: None,
        parity_evidence: vec![
            "Native Pronto skill surface; provider parity is not applicable.".into(),
        ],
        usage: SkillUsage {
            reason: "Papercuts activity is tracked in its backlog; skill invocation telemetry is not recorded.".into(),
            ..SkillUsage::default()
        },
        finding_capability: papercuts_finding_capability(),
    }
}

fn ensure_builtin_skills(snapshot: &mut SkillsSnapshot) {
    if !snapshot.skills.iter().any(|skill| {
        skill.id.eq_ignore_ascii_case(BUILT_IN_PAPERCUTS_ID)
            || skill.name.eq_ignore_ascii_case("Papercuts")
    }) {
        snapshot.skills.push(built_in_papercuts_skill());
    }
    snapshot.skills.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillsSnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub refreshed_at: Option<String>,
    pub freshness: String,
    pub source: String,
    pub recent_days: i64,
    pub roots: Vec<String>,
    pub skills: Vec<SkillRecord>,
    pub telemetry_gap: String,
}

#[derive(Debug, Clone)]
struct Candidate {
    name: String,
    description: String,
    path: PathBuf,
    root: String,
    provenance: String,
    provider: Option<String>,
    hosted: bool,
    hash: String,
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn root_specs() -> Vec<(String, PathBuf, String, Option<String>)> {
    let root = home();
    vec![
        (
            "canonical".into(),
            root.join(".agents/skills"),
            "Provider-neutral canonical".into(),
            None,
        ),
        (
            "claude".into(),
            root.join(".claude/skills"),
            "Claude provider-native".into(),
            Some("claude".into()),
        ),
        (
            "codex".into(),
            root.join(".codex/skills"),
            "Codex provider-native".into(),
            Some("codex".into()),
        ),
        (
            "codex-plugin-cache".into(),
            root.join(".codex/plugins/cache"),
            "Codex plugin cache".into(),
            Some("codex".into()),
        ),
        (
            "cursor".into(),
            root.join(".cursor/skills"),
            "Cursor provider-native".into(),
            Some("cursor".into()),
        ),
        (
            "cursor-managed".into(),
            root.join(".cursor/skills-cursor"),
            "Cursor managed".into(),
            Some("cursor".into()),
        ),
        (
            "gemini-legacy".into(),
            root.join(".gemini/config/plugins/claude-commands/skills"),
            "Gemini legacy".into(),
            Some("gemini".into()),
        ),
        (
            "jakye-agent-setup".into(),
            root.join("projects/jakyeamos-agent-skills/skills"),
            "Hosted in jakye-agent-setup".into(),
            None,
        ),
    ]
}

fn read_skill_metadata(path: &Path) -> Option<(String, String)> {
    let contents = fs::read_to_string(path).ok()?;
    let mut name = path.parent()?.file_name()?.to_string_lossy().to_string();
    let mut description = "No description recorded".to_string();
    for line in contents.lines().take(48) {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("name:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                name = value.to_string();
            }
        }
        if let Some(value) = trimmed.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                description = value.to_string();
            }
        }
    }
    Some((name, description))
}

fn hash_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut digest = Sha256::new();
    digest.update(bytes);
    Some(format!("{:x}", digest.finalize()))
}

fn collect_skill_files(root: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if output.len() >= MAX_FILES || depth > 6 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if output.len() >= MAX_FILES {
            break;
        }
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|name| name == ".system" || name == "codex-primary-runtime")
        {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            let skill_file = path.join("SKILL.md");
            if skill_file.is_file() {
                output.push(skill_file);
            }
            continue;
        }
        if path.is_dir() {
            collect_skill_files(&path, depth + 1, output);
        } else if path.file_name().is_some_and(|name| name == "SKILL.md") {
            output.push(path);
        }
    }
}

fn discover_candidates() -> (Vec<Candidate>, Vec<String>) {
    let hosted_root = home().join("projects/jakyeamos-agent-skills");
    let mut candidates = Vec::new();
    let mut roots = Vec::new();
    for (root_name, root_path, provenance, provider) in root_specs() {
        if !root_path.is_dir() {
            continue;
        }
        roots.push(format!("{} · {}", root_name, root_path.display()));
        let mut files = Vec::new();
        collect_skill_files(&root_path, 0, &mut files);
        for path in files {
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if metadata.len() > MAX_FILE_BYTES {
                continue;
            }
            let Some((name, description)) = read_skill_metadata(&path) else {
                continue;
            };
            let Some(hash) = hash_file(&path) else {
                continue;
            };
            candidates.push(Candidate {
                name,
                description,
                path: path.clone(),
                root: root_name.clone(),
                provenance: provenance.clone(),
                provider: provider.clone(),
                hosted: path.starts_with(&hosted_root),
                hash,
            });
        }
    }
    (candidates, roots)
}

fn provider_state(
    provider: &str,
    candidates: &[Candidate],
    canonical: Option<&Candidate>,
) -> SkillProviderState {
    let variant = candidates
        .iter()
        .find(|candidate| candidate.provider.as_deref() == Some(provider));
    let (state, reason, source_path) = match (provider, variant, canonical) {
        ("cursor", None, _) => (
            "blocked",
            "Cursor global loader/runtime is not behavior-verified".to_string(),
            None,
        ),
        ("gemini", None, Some(source)) => (
            "native",
            "Provider consumes the canonical skill root directly".to_string(),
            Some(source.path.display().to_string()),
        ),
        (_, Some(candidate), Some(source)) if candidate.hash == source.hash => (
            "projected",
            "Provider payload matches the canonical source hash".to_string(),
            Some(candidate.path.display().to_string()),
        ),
        (_, Some(candidate), Some(_)) => (
            "divergent",
            "Provider payload differs from the canonical source hash".to_string(),
            Some(candidate.path.display().to_string()),
        ),
        (_, Some(candidate), None) => (
            "native",
            "Provider-native skill has no canonical source match".to_string(),
            Some(candidate.path.display().to_string()),
        ),
        (_, None, _) => (
            "unsupported",
            "No provider payload or verified trigger evidence was found".to_string(),
            None,
        ),
    };
    SkillProviderState {
        state: state.into(),
        reason,
        source_path,
    }
}

fn contains_any(value: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| value.contains(term))
}

fn classify_skill_category(name: &str, description: &str) -> String {
    let haystack = format!("{name} {description}").to_ascii_lowercase();
    if contains_any(&haystack, &["security", "secret", "vulnerab", "threat"]) {
        return "Quality & Security".into();
    }
    if contains_any(
        &haystack,
        &["career", "outreach", "application runner", "job search"],
    ) {
        return "Career".into();
    }
    if contains_any(
        &haystack,
        &[
            "design",
            "visual",
            "browser",
            "screenshot",
            "animation",
            "color",
            "typograph",
            "tailwind",
            "react",
            "component",
            "frontend",
            "appkit",
        ],
    ) {
        return "UI & Design".into();
    }
    if contains_any(
        &haystack,
        &[
            "research",
            "writing",
            "paper",
            "document",
            "docs",
            "humaniz",
            "citation",
            "literature",
        ],
    ) {
        return "Research & Writing".into();
    }
    if contains_any(
        &haystack,
        &[
            "spreadsheet",
            "excel",
            "crawl",
            "scrape",
            "automation",
            "workflow",
            "pipeline",
            "data extraction",
        ],
    ) {
        return "Automation & Data".into();
    }
    if contains_any(&haystack, &["audit", "quality", "lint", "test", "verify"]) {
        return "Quality & Security".into();
    }
    if contains_any(
        &haystack,
        &[
            "devops",
            "deploy",
            "release",
            "build",
            "docker",
            "kubernetes",
            "infrastructure",
            "continuous integration",
            "ci/cd",
            "git",
            "github",
            "repository",
            "repo",
            "dev server",
        ],
    ) {
        return "DevOps".into();
    }
    if contains_any(
        &haystack,
        &[
            "agent",
            "skill",
            "task",
            "execution",
            "project",
            "pronto",
            "tmcp",
            "gsd",
        ],
    ) {
        return "Agent Operations".into();
    }
    "Other".into()
}

fn skill_family_seed(name: &str) -> String {
    let normalized = name.to_ascii_lowercase().replace(':', "-");
    const KNOWN_FAMILIES: &[&str] = &[
        "quality-runner",
        "project-compass",
        "skill-harvest",
        "career-ops",
        "agent-browser",
        "google-drive",
        "firecrawl",
        "opencli",
        "browser",
        "chrome",
        "github",
        "react",
        "spreadsheets",
        "sites",
        "impeccable",
        "gsd",
        "tmcp",
        "rdw",
    ];
    KNOWN_FAMILIES
        .iter()
        .find(|prefix| normalized == **prefix || normalized.starts_with(&format!("{prefix}-")))
        .map(|prefix| (*prefix).into())
        .unwrap_or_else(|| {
            normalized
                .split('-')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("standalone")
                .into()
        })
}

fn skill_family_label(seed: &str) -> String {
    match seed {
        "gsd" => "GSD".into(),
        "tmcp" => "TMCP".into(),
        "rdw" => "RDW".into(),
        _ => seed
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn build_snapshot() -> SkillsSnapshot {
    let (candidates, roots) = discover_candidates();
    let codex_usage = read_preferred_codex_usage_feed(
        &codex_usage_database_path(),
        &skill_usage_collector::usage_database_path(),
        Utc::now(),
    );
    let mut grouped: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for candidate in candidates {
        grouped
            .entry(candidate.name.to_ascii_lowercase())
            .or_default()
            .push(candidate);
    }
    let mut family_counts = HashMap::new();
    for name in grouped.keys() {
        *family_counts
            .entry(skill_family_seed(name))
            .or_insert(0usize) += 1;
    }
    let mut skills = grouped
        .into_iter()
        .map(|(id, candidates)| {
            let canonical = candidates
                .iter()
                .find(|candidate| candidate.provider.is_none() && candidate.root == "canonical");
            let mut providers = BTreeMap::new();
            for provider in ["codex", "claude", "gemini", "cursor"] {
                providers.insert(
                    provider.into(),
                    provider_state(provider, &candidates, canonical),
                );
            }
            let hosted = candidates.iter().any(|candidate| candidate.hosted);
            let description = canonical
                .or_else(|| candidates.first())
                .map(|candidate| candidate.description.clone())
                .unwrap_or_else(|| "No description recorded".into());
            let family_seed = skill_family_seed(
                candidates
                    .first()
                    .map(|candidate| candidate.name.as_str())
                    .unwrap_or_default(),
            );
            let family = if family_counts.get(&family_seed).copied().unwrap_or(0) > 1 {
                skill_family_label(&family_seed)
            } else {
                default_skill_family()
            };
            let parity_evidence = vec![
                "Behavioral trigger/output fixtures are not available in the local runtime".into(),
            ];
            SkillRecord {
                id: id.clone(),
                name: candidates
                    .first()
                    .map(|candidate| candidate.name.clone())
                    .unwrap_or_default(),
                description: description.clone(),
                category: classify_skill_category(
                    candidates
                        .first()
                        .map(|candidate| candidate.name.as_str())
                        .unwrap_or_default(),
                    canonical
                        .or_else(|| candidates.first())
                        .map(|candidate| candidate.description.as_str())
                        .unwrap_or_default(),
                ),
                family,
                lifecycle: if canonical.is_some() {
                    "canonical".into()
                } else {
                    "provider-native".into()
                },
                hosted_in_jakye_agent_setup: hosted,
                sources: candidates
                    .iter()
                    .map(|candidate| SkillSource {
                        path: candidate.path.display().to_string(),
                        root: candidate.root.clone(),
                        provenance: candidate.provenance.clone(),
                        sha256: candidate.hash.clone(),
                        hosted_in_jakye_agent_setup: candidate.hosted,
                    })
                    .collect(),
                providers,
                parity_score: None,
                parity_evidence,
                usage: usage_for_skill(&id, &codex_usage),
                finding_capability: fallback_finding_capability(
                    &id,
                    candidates
                        .first()
                        .map(|candidate| candidate.name.as_str())
                        .unwrap_or_default(),
                    description.as_str(),
                ),
            }
        })
        .collect::<Vec<_>>();
    skills.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    let mut snapshot = SkillsSnapshot {
        schema_version: SCHEMA.into(),
        generated_at: iso_now(),
        refreshed_at: Some(iso_now()),
        freshness: format!("Observed through {}", iso_now()),
        source: "Local skill roots; usage requires structured provider telemetry".into(),
        recent_days: RECENT_DAYS,
        roots,
        skills,
        telemetry_gap: STRUCTURED_USAGE_UNAVAILABLE_REASON.into(),
    };
    ensure_builtin_skills(&mut snapshot);
    update_snapshot_usage_summary(&mut snapshot);
    apply_finding_capabilities(&mut snapshot);
    snapshot
}

fn classify_snapshot(snapshot: &mut SkillsSnapshot) {
    ensure_builtin_skills(snapshot);
    let mut family_counts = HashMap::new();
    for skill in &snapshot.skills {
        *family_counts
            .entry(skill_family_seed(&skill.name))
            .or_insert(0usize) += 1;
    }
    for skill in &mut snapshot.skills {
        if !observed_usage_is_valid(&skill.usage) {
            skill.usage = SkillUsage::default();
        }
        if skill.id.eq_ignore_ascii_case(BUILT_IN_PAPERCUTS_ID) {
            continue;
        }
        let family_seed = skill_family_seed(&skill.name);
        skill.family = if family_counts.get(&family_seed).copied().unwrap_or(0) > 1 {
            skill_family_label(&family_seed)
        } else {
            default_skill_family()
        };
        skill.category = classify_skill_category(&skill.name, &skill.description);
    }
    snapshot.schema_version = SCHEMA.into();
    update_snapshot_usage_summary(snapshot);
    apply_finding_capabilities(snapshot);
}

fn observed_usage_is_valid(usage: &SkillUsage) -> bool {
    if usage.state != "observed"
        || usage.telemetry_source.trim().is_empty()
        || usage.by_provider.is_empty()
        || usage.recent_count > usage.all_time_count
    {
        return false;
    }
    let Some(provider_total) = usage
        .by_provider
        .values()
        .try_fold(0_u64, |total, count| total.checked_add(*count))
    else {
        return false;
    };
    if provider_total != usage.all_time_count {
        return false;
    }
    match (usage.all_time_count, usage.last_seen_at.as_deref()) {
        (0, None) => true,
        (0, Some(_)) | (_, None) => false,
        (_, Some(timestamp)) => DateTime::parse_from_rfc3339(timestamp).is_ok(),
    }
}

fn iso_now() -> String {
    Utc::now().to_rfc3339()
}

pub fn refresh(path: &Path) -> Result<SkillsSnapshot, String> {
    let snapshot = build_snapshot();
    let connection = Connection::open(path)
        .map_err(|error| format!("Could not open Pronto database: {error}"))?;
    connection.execute("CREATE TABLE IF NOT EXISTS skills_snapshots (id INTEGER PRIMARY KEY CHECK (id = 1), payload_json TEXT NOT NULL)", []).map_err(|error| format!("Could not initialize skills storage: {error}"))?;
    let payload = serde_json::to_string(&snapshot)
        .map_err(|error| format!("Could not encode skills snapshot: {error}"))?;
    connection
        .execute(
            "INSERT OR REPLACE INTO skills_snapshots (id, payload_json) VALUES (1, ?1)",
            params![payload],
        )
        .map_err(|error| format!("Could not save skills snapshot: {error}"))?;
    Ok(snapshot)
}

pub fn load(path: &Path) -> Result<SkillsSnapshot, String> {
    if !path.is_file() {
        return Ok(SkillsSnapshot {
            schema_version: SCHEMA.into(),
            generated_at: iso_now(),
            refreshed_at: None,
            freshness: "Unavailable until the first skills refresh".into(),
            source: "Local skill roots; usage requires structured provider telemetry".into(),
            recent_days: RECENT_DAYS,
            roots: Vec::new(),
            skills: Vec::new(),
            telemetry_gap: "No skills refresh has been recorded.".into(),
        });
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Could not read Pronto database: {error}"))?;
    let has_table: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'skills_snapshots')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect skills storage: {error}"))?;
    if !has_table {
        return Ok(empty_snapshot());
    }
    let payload = connection
        .query_row(
            "SELECT payload_json FROM skills_snapshots WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Could not read skills snapshot: {error}"))?;
    payload
        .map(|value| -> Result<SkillsSnapshot, String> {
            let mut snapshot = serde_json::from_str(&value)
                .map_err(|error| format!("Could not decode skills snapshot: {error}"))?;
            classify_snapshot(&mut snapshot);
            Ok(snapshot)
        })
        .transpose()
        .map(|value| value.unwrap_or_else(empty_snapshot))
}

fn empty_snapshot() -> SkillsSnapshot {
    let mut snapshot = SkillsSnapshot {
        schema_version: SCHEMA.into(),
        generated_at: iso_now(),
        refreshed_at: None,
        freshness: "Unavailable until the first skills refresh".into(),
        source: "Local skill roots; usage requires structured provider telemetry".into(),
        recent_days: RECENT_DAYS,
        roots: Vec::new(),
        skills: Vec::new(),
        telemetry_gap: "No skills refresh has been recorded.".into(),
    };
    ensure_builtin_skills(&mut snapshot);
    snapshot
}

fn is_discovered_source(requested: &Path, candidates: &[Candidate]) -> bool {
    requested.file_name().is_some_and(|name| name == "SKILL.md")
        && candidates
            .iter()
            .any(|candidate| candidate.path == requested)
}

pub fn open_source(path: &str) -> Result<(), String> {
    let requested = PathBuf::from(path);
    let (candidates, _) = discover_candidates();
    if !requested.is_file() || !is_discovered_source(&requested, &candidates) {
        return Err("Skill source is not an exact currently discovered source".into());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(&requested)
            .status()
            .map_err(|error| format!("Could not open skill source: {error}"))?
            .success()
            .then_some(())
            .ok_or_else(|| "The skill source could not be opened".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Opening skill sources is currently supported on macOS only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(provider: Option<&str>, hash: &str) -> Candidate {
        Candidate {
            name: "example".into(),
            description: "Example".into(),
            path: PathBuf::from("/tmp/example/SKILL.md"),
            root: "test".into(),
            provenance: "Test".into(),
            provider: provider.map(str::to_string),
            hosted: false,
            hash: hash.into(),
        }
    }

    #[test]
    fn matching_provider_payload_is_projected() {
        let source = candidate(None, "same");
        let variant = candidate(Some("codex"), "same");
        let state = provider_state("codex", &[variant], Some(&source));
        assert_eq!(state.state, "projected");
    }

    #[test]
    fn changed_provider_payload_is_divergent() {
        let source = candidate(None, "source");
        let variant = candidate(Some("claude"), "different");
        let state = provider_state("claude", &[variant], Some(&source));
        assert_eq!(state.state, "divergent");
    }

    #[test]
    fn cursor_without_payload_is_blocked() {
        let state = provider_state("cursor", &[], None);
        assert_eq!(state.state, "blocked");
    }

    #[test]
    fn gemini_consumes_the_canonical_skill_root_directly() {
        let source = candidate(None, "same");
        let state = provider_state("gemini", &[], Some(&source));
        assert_eq!(state.state, "native");
        assert_eq!(state.source_path, Some("/tmp/example/SKILL.md".to_string()));
    }

    #[test]
    fn source_opening_requires_an_exact_discovered_path() {
        let discovered = vec![candidate(None, "same")];
        assert!(is_discovered_source(
            Path::new("/tmp/example/SKILL.md"),
            &discovered
        ));
        assert!(!is_discovered_source(
            Path::new("/tmp/example/../outside/SKILL.md"),
            &discovered
        ));
        assert!(!is_discovered_source(
            Path::new("/tmp/example-copy/SKILL.md"),
            &discovered
        ));
    }

    #[cfg(unix)]
    #[test]
    fn skill_discovery_reads_direct_skill_file_from_symlinked_directory() {
        use std::os::unix::fs::symlink;

        let test_root = std::env::temp_dir().join(format!(
            "pronto-symlinked-skill-test-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let canonical = test_root.join("canonical/example");
        let provider_root = test_root.join("provider");
        fs::create_dir_all(&canonical).expect("create canonical skill directory");
        fs::create_dir_all(&provider_root).expect("create provider root");
        fs::write(
            canonical.join("SKILL.md"),
            "---\nname: example\ndescription: Example\n---\n",
        )
        .expect("write canonical skill");
        symlink(&canonical, provider_root.join("example")).expect("project skill directory");

        let mut files = Vec::new();
        collect_skill_files(&provider_root, 0, &mut files);

        assert_eq!(files, vec![provider_root.join("example/SKILL.md")]);
        fs::remove_dir_all(&test_root).expect("remove skill discovery fixture");
    }

    #[test]
    fn classification_groups_known_family_names() {
        assert_eq!(skill_family_seed("firecrawl-search"), "firecrawl");
        assert_eq!(
            skill_family_seed("browser:control-in-app-browser"),
            "browser"
        );
        assert_eq!(skill_family_label("quality-runner"), "Quality Runner");
        assert_eq!(skill_family_label("tmcp"), "TMCP");
    }

    #[test]
    fn classification_uses_name_and_description_for_categories() {
        assert_eq!(
            classify_skill_category("deploy-preview", "Deploy a preview build"),
            "DevOps"
        );
        assert_eq!(
            classify_skill_category("visual-interface", "Review the visual UI"),
            "UI & Design"
        );
        assert_eq!(
            classify_skill_category("security-audit", "Audit the security posture"),
            "Quality & Security"
        );
        assert_eq!(
            classify_skill_category("career-runner", "Prepare a job application"),
            "Career"
        );
    }

    #[test]
    fn built_in_papercuts_skill_is_available_without_a_skill_file() {
        let skill = built_in_papercuts_skill();
        assert_eq!(skill.id, "papercuts");
        assert_eq!(skill.category, "UI & Design");
        assert_eq!(skill.family, "Design Audit");
        assert_eq!(skill.providers["pronto"].state, "native");

        let mut snapshot = empty_snapshot();
        ensure_builtin_skills(&mut snapshot);
        assert_eq!(
            snapshot
                .skills
                .iter()
                .filter(|candidate| candidate.id == "papercuts")
                .count(),
            1
        );
    }

    #[test]
    fn legacy_unstructured_usage_is_invalidated_on_load() {
        let mut snapshot = empty_snapshot();
        let skill = snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
            .expect("built-in skill");
        skill.usage = serde_json::from_value(serde_json::json!({
            "recent_count": 2,
            "all_time_count": 2,
            "by_provider": { "claude": 2 },
            "last_seen_at": "2026-07-08T03:52:57.943Z",
            "telemetry_source": "Local session records"
        }))
        .expect("legacy usage");

        classify_snapshot(&mut snapshot);

        let usage = &snapshot
            .skills
            .iter()
            .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
            .expect("built-in skill")
            .usage;
        assert_eq!(snapshot.schema_version, SCHEMA);
        assert_eq!(usage.state, "unavailable");
        assert_eq!(usage.recent_count, 0);
        assert_eq!(usage.all_time_count, 0);
        assert!(usage.by_provider.is_empty());
        assert!(usage.last_seen_at.is_none());
    }

    #[test]
    fn structured_observed_usage_survives_snapshot_classification() {
        let mut snapshot = empty_snapshot();
        let skill = snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
            .expect("built-in skill");
        skill.usage = SkillUsage {
            state: "observed".into(),
            recent_count: 3,
            all_time_count: 7,
            by_provider: BTreeMap::from([("codex".into(), 7)]),
            last_seen_at: Some("2026-08-10T20:00:00Z".into()),
            telemetry_source: "Structured test provider feed".into(),
            reason: "Structured usage evidence observed.".into(),
        };

        classify_snapshot(&mut snapshot);

        let usage = &snapshot
            .skills
            .iter()
            .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
            .expect("built-in skill")
            .usage;
        assert_eq!(usage.state, "observed");
        assert_eq!(usage.recent_count, 3);
        assert_eq!(usage.all_time_count, 7);
        assert_eq!(usage.by_provider.get("codex"), Some(&7));
    }

    #[test]
    fn malformed_observed_usage_fails_closed_on_snapshot_classification() {
        for invalid_usage in [
            SkillUsage {
                state: "observed".into(),
                recent_count: 8,
                all_time_count: 7,
                by_provider: BTreeMap::from([("codex".into(), 7)]),
                last_seen_at: Some("2026-08-10T20:00:00Z".into()),
                telemetry_source: "Structured test provider feed".into(),
                reason: "Structured usage evidence observed.".into(),
            },
            SkillUsage {
                state: "observed".into(),
                recent_count: 3,
                all_time_count: 7,
                by_provider: BTreeMap::from([("codex".into(), 6)]),
                last_seen_at: Some("not-a-timestamp".into()),
                telemetry_source: "".into(),
                reason: "Structured usage evidence observed.".into(),
            },
        ] {
            let mut snapshot = empty_snapshot();
            let skill = snapshot
                .skills
                .iter_mut()
                .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
                .expect("built-in skill");
            skill.usage = invalid_usage;

            classify_snapshot(&mut snapshot);

            let usage = &snapshot
                .skills
                .iter()
                .find(|skill| skill.id == BUILT_IN_PAPERCUTS_ID)
                .expect("built-in skill")
                .usage;
            assert_eq!(usage.state, "unavailable");
            assert_eq!(usage.all_time_count, 0);
            assert!(usage.by_provider.is_empty());
        }
    }

    fn usage_test_database() -> PathBuf {
        std::env::temp_dir().join(format!(
            "pronto-codex-skill-usage-{}-{}.sqlite",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn codex_usage_feed_counts_only_successful_structured_events() {
        let path = usage_test_database();
        let connection = Connection::open(&path).expect("open usage fixture");
        connection
            .execute_batch(
                r#"
CREATE TABLE skill_invocations (
    thread_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    skill_path TEXT NOT NULL,
    skill_scope TEXT NOT NULL,
    invocation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL,
    PRIMARY KEY (thread_id, turn_id, skill_path, invocation_type)
);
"#,
            )
            .expect("create usage fixture");
        let recent = Utc::now().timestamp_millis();
        let old = (Utc::now() - Duration::days(RECENT_DAYS + 1)).timestamp_millis();
        for (turn, invocation_type, status, timestamp) in [
            ("turn-1", "explicit", "ok", old),
            ("turn-2", "implicit", "ok", recent),
            ("turn-3", "explicit", "error", recent),
        ] {
            connection
                .execute(
                    "INSERT INTO skill_invocations VALUES (?1, ?2, 'Example', '/skills/example/SKILL.md', 'user', ?3, ?4, ?5)",
                    params!["thread-1", turn, invocation_type, status, timestamp],
                )
                .expect("insert usage fixture");
        }
        drop(connection);

        let CodexUsageFeed::Observed { usage, .. } = read_codex_usage_feed(&path) else {
            panic!("expected observed Codex usage feed");
        };
        let example = usage.get("example").expect("example usage");
        assert_eq!(example.recent_count, 1);
        assert_eq!(example.all_time_count, 2);
        assert_eq!(example.by_provider.get("codex"), Some(&2));
        assert!(example.last_seen_at.is_some());
        fs::remove_file(path).expect("remove usage fixture");
    }

    #[test]
    fn empty_structured_feed_is_observed_zero_not_unavailable() {
        let path = usage_test_database();
        let connection = Connection::open(&path).expect("open usage fixture");
        connection
            .execute_batch(
                r#"
CREATE TABLE skill_invocations (
    thread_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    skill_path TEXT NOT NULL,
    skill_scope TEXT NOT NULL,
    invocation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL
);
"#,
            )
            .expect("create usage fixture");
        drop(connection);

        let feed = read_codex_usage_feed(&path);
        let usage = usage_for_skill("not-yet-used", &feed);
        assert_eq!(usage.state, "observed");
        assert_eq!(usage.all_time_count, 0);
        assert_eq!(usage.by_provider.get("codex"), Some(&0));
        fs::remove_file(path).expect("remove usage fixture");
    }

    #[test]
    fn missing_structured_table_fails_closed() {
        let path = usage_test_database();
        drop(Connection::open(&path).expect("open usage fixture"));

        let feed = read_codex_usage_feed(&path);
        let usage = usage_for_skill("example", &feed);
        assert_eq!(usage.state, "unavailable");
        assert!(usage.reason.contains("skill_invocations"));
        fs::remove_file(path).expect("remove usage fixture");
    }

    fn otlp_usage_payload(skill: &str, count: u64, timestamp_ms: i64) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "resourceMetrics": [{
                "resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "codex"}}]},
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "codex.skill.injected",
                        "sum": {
                            "aggregationTemporality": 2,
                            "dataPoints": [{
                                "attributes": [
                                    {"key": "skill", "value": {"stringValue": skill}},
                                    {"key": "status", "value": {"stringValue": "ok"}},
                                    {"key": "invoke_type", "value": {"stringValue": "explicit"}}
                                ],
                                "startTimeUnixNano": ((timestamp_ms - 1_000) * 1_000_000).to_string(),
                                "timeUnixNano": (timestamp_ms * 1_000_000).to_string(),
                                "asInt": count.to_string()
                            }]
                        }
                    }]
                }]
            }]
        }))
        .expect("encode OTLP usage fixture")
    }

    #[test]
    fn otlp_compatibility_feed_is_used_when_structured_state_is_missing() {
        let sqlite_path = usage_test_database();
        let otlp_path = sqlite_path.with_extension("otlp.sqlite");
        let now = Utc::now();
        skill_usage_collector::ingest_otlp_json(
            &otlp_path,
            &otlp_usage_payload("Example", 3, now.timestamp_millis()),
            now.timestamp_millis(),
        )
        .expect("ingest OTLP fixture");

        let feed = read_preferred_codex_usage_feed(&sqlite_path, &otlp_path, now);
        let usage = usage_for_skill("example", &feed);
        assert_eq!(usage.state, "observed");
        assert_eq!(usage.all_time_count, 3);
        assert_eq!(usage.telemetry_source, CODEX_OTLP_USAGE_SOURCE);
        assert!(usage.reason.contains("earlier history"));
        fs::remove_file(otlp_path).expect("remove OTLP fixture");
    }

    #[test]
    fn structured_state_remains_preferred_over_otlp_compatibility_feed() {
        let sqlite_path = usage_test_database();
        let otlp_path = sqlite_path.with_extension("otlp.sqlite");
        let connection = Connection::open(&sqlite_path).expect("open structured fixture");
        connection
            .execute_batch(
                r#"
CREATE TABLE skill_invocations (
    thread_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    skill_path TEXT NOT NULL,
    skill_scope TEXT NOT NULL,
    invocation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL
);
"#,
            )
            .expect("create structured fixture");
        drop(connection);
        let now = Utc::now();
        skill_usage_collector::ingest_otlp_json(
            &otlp_path,
            &otlp_usage_payload("Example", 5, now.timestamp_millis()),
            now.timestamp_millis(),
        )
        .expect("ingest OTLP fixture");

        let feed = read_preferred_codex_usage_feed(&sqlite_path, &otlp_path, now);
        let usage = usage_for_skill("example", &feed);
        assert_eq!(usage.all_time_count, 0);
        assert_eq!(usage.telemetry_source, CODEX_USAGE_SOURCE);
        fs::remove_file(sqlite_path).expect("remove structured fixture");
        fs::remove_file(otlp_path).expect("remove OTLP fixture");
    }
}
