use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "pronto-skills/v2";
const RECENT_DAYS: i64 = 30;
const MAX_FILES: usize = 3_000;
const MAX_FILE_BYTES: u64 = 256 * 1024;
const MAX_TELEMETRY_FILES: usize = 10;
const MAX_SESSION_FILE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillUsage {
    pub recent_count: u64,
    pub all_time_count: u64,
    pub by_provider: BTreeMap<String, u64>,
    pub last_seen_at: Option<String>,
    pub telemetry_source: String,
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
}

fn default_skill_category() -> String {
    "Other".into()
}

fn default_skill_family() -> String {
    "Standalone".into()
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

fn invocation_candidates(
    root: &Path,
    provider: &str,
    names: &HashSet<String>,
    counts: &mut HashMap<String, SkillUsage>,
) {
    let mut files = Vec::new();
    collect_jsonl_files(root, 0, &mut files);
    for path in files.into_iter().take(MAX_TELEMETRY_FILES) {
        if fs::metadata(&path)
            .map(|metadata| metadata.len() > MAX_SESSION_FILE_BYTES)
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for line in contents
            .lines()
            .filter(|line| line.to_ascii_lowercase().contains("skill"))
        {
            let timestamp = serde_json::from_str::<Value>(line).ok().and_then(|value| {
                value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
            for name in names {
                let marker = format!("${name}");
                if !line.contains(&marker) && !line.contains(&format!("\"{name}\"")) {
                    continue;
                }
                let entry = counts.entry(name.clone()).or_default();
                entry.all_time_count += 1;
                *entry.by_provider.entry(provider.to_string()).or_default() += 1;
                let is_recent = timestamp
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|date| {
                        date.with_timezone(&Utc) >= Utc::now() - Duration::days(RECENT_DAYS)
                    })
                    .unwrap_or(false);
                if is_recent {
                    entry.recent_count += 1;
                }
                if timestamp
                    .as_ref()
                    .is_some_and(|value| entry.last_seen_at.as_ref().is_none_or(|old| value > old))
                {
                    entry.last_seen_at = timestamp.clone();
                }
                entry.telemetry_source =
                    "Local session records; prompts and bodies are not retained".into();
            }
        }
    }
}

fn collect_jsonl_files(root: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if output.len() >= MAX_FILES || depth > 5 {
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
        if fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
        {
            continue;
        }
        if path.is_dir() {
            collect_jsonl_files(&path, depth + 1, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            output.push(path);
        }
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
    let names = grouped
        .values()
        .flat_map(|items| items.iter().map(|item| item.name.clone()))
        .collect::<HashSet<_>>();
    let mut usage = HashMap::new();
    let home = home();
    invocation_candidates(&home.join(".codex/sessions"), "codex", &names, &mut usage);
    invocation_candidates(&home.join(".claude/projects"), "claude", &names, &mut usage);
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
                id,
                name: candidates
                    .first()
                    .map(|candidate| candidate.name.clone())
                    .unwrap_or_default(),
                description,
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
                usage: usage
                    .remove(
                        &candidates
                            .first()
                            .map(|candidate| candidate.name.clone())
                            .unwrap_or_default(),
                    )
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    skills.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    SkillsSnapshot { schema_version: SCHEMA.into(), generated_at: iso_now(), refreshed_at: Some(iso_now()), freshness: format!("Observed through {}", iso_now()), source: "Local skill roots and local session records".into(), recent_days: RECENT_DAYS, roots, skills, telemetry_gap: "Invocation evidence is best-effort: only recognizable local session records are counted; missing or provider-blocked telemetry remains unknown.".into() }
}

fn classify_snapshot(snapshot: &mut SkillsSnapshot) {
    let mut family_counts = HashMap::new();
    for skill in &snapshot.skills {
        *family_counts
            .entry(skill_family_seed(&skill.name))
            .or_insert(0usize) += 1;
    }
    for skill in &mut snapshot.skills {
        let family_seed = skill_family_seed(&skill.name);
        skill.family = if family_counts.get(&family_seed).copied().unwrap_or(0) > 1 {
            skill_family_label(&family_seed)
        } else {
            default_skill_family()
        };
        skill.category = classify_skill_category(&skill.name, &skill.description);
    }
    snapshot.schema_version = SCHEMA.into();
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
            source: "Local skill roots and local session records".into(),
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
    SkillsSnapshot {
        schema_version: SCHEMA.into(),
        generated_at: iso_now(),
        refreshed_at: None,
        freshness: "Unavailable until the first skills refresh".into(),
        source: "Local skill roots and local session records".into(),
        recent_days: RECENT_DAYS,
        roots: Vec::new(),
        skills: Vec::new(),
        telemetry_gap: "No skills refresh has been recorded.".into(),
    }
}

pub fn open_source(path: &str) -> Result<(), String> {
    let requested = PathBuf::from(path);
    let allowed = root_specs()
        .into_iter()
        .map(|(_, path, _, _)| path)
        .any(|root| requested.starts_with(root));
    if !allowed || requested.file_name().is_none_or(|name| name != "SKILL.md") {
        return Err("Skill source is outside the discovered skill roots".into());
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
}
