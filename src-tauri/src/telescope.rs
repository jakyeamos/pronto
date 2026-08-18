use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LazyLock, Mutex,
};

pub const SCHEMA_VERSION: &str = "pronto-telescope/v1";
const MAX_SOURCE_FILES: usize = 2_500;
const MAX_SOURCE_BYTES: u64 = 512 * 1024;
const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "go", "java", "kt", "swift", "rb", "php",
    "cs", "cpp", "cc", "c", "h", "vue", "svelte",
];
static ACTIVE_REFRESHES: LazyLock<Mutex<BTreeMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeProjection {
    pub schema_version: String,
    pub repository_id: String,
    pub repository_name: String,
    pub binding: TelescopeBinding,
    pub freshness: TelescopeFreshness,
    pub coverage: TelescopeCoverage,
    pub groups: Vec<TelescopeGroup>,
    pub nodes: Vec<TelescopeNode>,
    pub edges: Vec<TelescopeEdge>,
    pub flows: Vec<TelescopeFlow>,
    pub warnings: Vec<TelescopeWarning>,
    pub enrichment: TelescopeEnrichment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeBinding {
    pub workspace_id: String,
    pub branch: String,
    pub commit: Option<String>,
    pub dirty: bool,
    pub dirty_state_fingerprint: String,
    pub workspace_fingerprint: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeFreshness {
    pub state: String,
    pub cache: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeCoverage {
    pub discovered_source_files: usize,
    pub examined_source_files: usize,
    pub supported_source_files: usize,
    pub partial_source_files: usize,
    pub skipped_large_files: usize,
    pub truncated: bool,
    pub resolved_relationships: usize,
    pub inferred_relationships: usize,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeGroup {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub summary: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeAnchor {
    pub path: String,
    pub line: Option<usize>,
    pub symbol: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeNode {
    pub id: String,
    pub group_id: String,
    pub label: String,
    pub kind: String,
    pub technology: String,
    pub semantic_summary: String,
    pub implementation_summary: String,
    pub summary_status: String,
    pub confidence: String,
    pub provenance: Vec<String>,
    pub source_anchors: Vec<TelescopeAnchor>,
    pub symbols: Vec<String>,
    pub data_shapes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub direction: String,
    pub label: String,
    pub confidence: String,
    pub provenance: String,
    pub inferred: bool,
    pub source_anchor: Option<TelescopeAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeFlow {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub data_shape: Option<String>,
    pub confidence: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeWarning {
    pub code: String,
    pub message: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeEnrichment {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub source_content_transmitted: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct TelescopeRequest<'a> {
    pub repository_id: &'a str,
    pub repository_name: &'a str,
    pub workspace_id: &'a str,
    pub workspace_path: &'a Path,
    pub branch: &'a str,
    pub known_commit: Option<&'a str>,
    pub known_dirty: bool,
}

#[derive(Debug, Clone)]
struct SourceFile {
    relative_path: String,
    absolute_path: PathBuf,
    language: String,
    supported: bool,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct ImportRecord {
    source_path: String,
    line: usize,
    specifier: String,
    kind: String,
    confidence: String,
}

pub fn get_or_generate(
    store_path: &Path,
    request: &TelescopeRequest<'_>,
    force_refresh: bool,
) -> Result<TelescopeProjection, String> {
    get_or_generate_cancellable(store_path, request, force_refresh, None)
}

pub fn begin_refresh(repository_id: &str) -> Arc<AtomicBool> {
    let cancellation = Arc::new(AtomicBool::new(false));
    if let Ok(mut active) = ACTIVE_REFRESHES.lock() {
        if let Some(previous) = active.insert(repository_id.to_string(), cancellation.clone()) {
            previous.store(true, Ordering::Release);
        }
    }
    cancellation
}

pub fn cancel_refresh(repository_id: &str) -> bool {
    ACTIVE_REFRESHES
        .lock()
        .ok()
        .and_then(|active| active.get(repository_id).cloned())
        .is_some_and(|cancellation| {
            cancellation.store(true, Ordering::Release);
            true
        })
}

pub fn finish_refresh(repository_id: &str, cancellation: &Arc<AtomicBool>) {
    if let Ok(mut active) = ACTIVE_REFRESHES.lock() {
        let owns_entry = active
            .get(repository_id)
            .is_some_and(|current| Arc::ptr_eq(current, cancellation));
        if owns_entry {
            active.remove(repository_id);
        }
    }
}

pub fn get_or_generate_cancellable(
    store_path: &Path,
    request: &TelescopeRequest<'_>,
    force_refresh: bool,
    cancellation: Option<&AtomicBool>,
) -> Result<TelescopeProjection, String> {
    ensure_not_cancelled(cancellation)?;
    let commit = git_output(request.workspace_path, &["rev-parse", "HEAD"])
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| request.known_commit.map(str::to_string));
    let (dirty_payload, live_status_available) = match git_output(
        request.workspace_path,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    ) {
        Ok(status) => (status, true),
        Err(_) => (
            if request.known_dirty {
                "dirty"
            } else {
                "clean"
            }
            .to_string(),
            false,
        ),
    };
    let dirty = if live_status_available {
        !dirty_payload.trim().is_empty()
    } else {
        request.known_dirty
    };
    let dirty_fingerprint = dirty_state_fingerprint(request.workspace_path, &dirty_payload);
    let fingerprint_input = format!(
        "{}\n{}\n{}\n{}\n{}",
        SCHEMA_VERSION,
        request.workspace_id,
        request.branch,
        commit.as_deref().unwrap_or("unknown"),
        dirty_fingerprint
    );
    let workspace_fingerprint = stable_id("workspace", &fingerprint_input);
    let cache_repository_id = public_identifier("repository", request.repository_id);

    ensure_cache_table(store_path)?;
    if !force_refresh {
        if let Some(mut cached) =
            load_cached(store_path, &cache_repository_id, &workspace_fingerprint)?
        {
            cached.freshness.cache = "hit".to_string();
            cached.freshness.reason =
                "The cached projection matches the active workspace fingerprint and schema."
                    .to_string();
            return Ok(cached);
        }
    }

    let mut projection = extract_projection(
        request,
        commit,
        dirty,
        dirty_fingerprint,
        workspace_fingerprint,
        cancellation,
    )?;
    ensure_not_cancelled(cancellation)?;
    projection.freshness.cache = if force_refresh { "refreshed" } else { "miss" }.to_string();
    save_cached(store_path, &projection)?;
    Ok(projection)
}

fn extract_projection(
    request: &TelescopeRequest<'_>,
    commit: Option<String>,
    dirty: bool,
    dirty_state_fingerprint: String,
    workspace_fingerprint: String,
    cancellation: Option<&AtomicBool>,
) -> Result<TelescopeProjection, String> {
    let generated_at = Utc::now().to_rfc3339();
    let mut warnings = Vec::new();
    let (files, discovered, skipped_large, truncated) =
        discover_source_files(request.workspace_path, cancellation)?;
    let supported_count = files.iter().filter(|file| file.supported).count();
    let partial_count = files.len().saturating_sub(supported_count);
    if partial_count > 0 {
        warnings.push(TelescopeWarning {
            code: "partial-language-coverage".to_string(),
            message: format!(
                "{partial_count} source files use generic topology because no semantic adapter is available."
            ),
            scope: "extraction".to_string(),
        });
    }
    if truncated {
        warnings.push(TelescopeWarning {
            code: "source-limit-reached".to_string(),
            message: format!("Extraction stopped at {MAX_SOURCE_FILES} source files."),
            scope: "extraction".to_string(),
        });
    }
    if dirty {
        warnings.push(TelescopeWarning {
            code: "dirty-workspace".to_string(),
            message: "The map includes the active dirty worktree and is bound to its dirty-state fingerprint.".to_string(),
            scope: "freshness".to_string(),
        });
    }
    if commit.is_none() {
        warnings.push(TelescopeWarning {
            code: "commit-binding-unavailable".to_string(),
            message: "The active workspace commit could not be confirmed; topology is available with partial freshness.".to_string(),
            scope: "freshness".to_string(),
        });
    }

    let mut groups = build_groups(&files);
    let mut nodes = Vec::new();
    let mut imports = Vec::new();
    let mut node_by_path = BTreeMap::new();
    let mut external_specifiers = BTreeSet::new();

    for file in &files {
        ensure_not_cancelled(cancellation)?;
        let group_id = group_id_for_path(&file.relative_path);
        let node_id = stable_id("node", &file.relative_path);
        node_by_path.insert(file.relative_path.clone(), node_id.clone());
        let content = if file.bytes <= MAX_SOURCE_BYTES {
            fs::read_to_string(&file.absolute_path).unwrap_or_default()
        } else {
            String::new()
        };
        let kind = classify_node(&file.relative_path, &content);
        let (symbols, data_shapes) = if file.supported {
            extract_symbols_and_shapes(&content, &file.language)
        } else {
            (Vec::new(), Vec::new())
        };
        if file.supported {
            let discovered_imports = extract_imports(&file.relative_path, &content, &file.language);
            for import in &discovered_imports {
                if !import.specifier.starts_with('.')
                    && !import.specifier.starts_with("crate::")
                    && !import.specifier.starts_with("self::")
                    && !import.specifier.starts_with("super::")
                {
                    external_specifiers.insert(package_root(&import.specifier));
                }
            }
            imports.extend(discovered_imports);
        }
        let label = display_label(&file.relative_path);
        nodes.push(TelescopeNode {
            id: node_id,
            group_id,
            label: label.clone(),
            kind: kind.clone(),
            technology: file.language.clone(),
            semantic_summary: semantic_summary(&label, &kind),
            implementation_summary: format!(
                "Derived from {} using the {} extractor.",
                file.relative_path,
                if file.supported {
                    &file.language
                } else {
                    "generic topology"
                }
            ),
            summary_status: "derived".to_string(),
            confidence: if file.supported { "high" } else { "partial" }.to_string(),
            provenance: vec![
                "workspace-source".to_string(),
                if file.supported {
                    "language-adapter"
                } else {
                    "directory-topology"
                }
                .to_string(),
            ],
            source_anchors: vec![TelescopeAnchor {
                path: file.relative_path.clone(),
                line: Some(1),
                symbol: symbols.first().cloned(),
                provenance: "source".to_string(),
            }],
            symbols,
            data_shapes,
        });
    }

    if !external_specifiers.is_empty() {
        groups.push(TelescopeGroup {
            id: "group-external".to_string(),
            label: "External integrations".to_string(),
            kind: "external".to_string(),
            parent_id: None,
            summary: "Imported packages and services outside this repository.".to_string(),
            confidence: "high".to_string(),
        });
        for specifier in external_specifiers {
            let id = stable_id("external", &specifier);
            nodes.push(TelescopeNode {
                id,
                group_id: "group-external".to_string(),
                label: specifier.clone(),
                kind: "integration".to_string(),
                technology: "external".to_string(),
                semantic_summary: format!("External integration referenced as {specifier}."),
                implementation_summary:
                    "Observed through a static import; runtime behavior was not inspected."
                        .to_string(),
                summary_status: "derived".to_string(),
                confidence: "medium".to_string(),
                provenance: vec!["static-import".to_string()],
                source_anchors: Vec::new(),
                symbols: Vec::new(),
                data_shapes: Vec::new(),
            });
        }
    }

    let node_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    let mut resolved_relationships = 0;
    let mut inferred_relationships = 0;
    let mut seen_edges = BTreeSet::new();
    for import in imports {
        let Some(source) = node_by_path.get(&import.source_path).cloned() else {
            continue;
        };
        let (target, inferred) = resolve_import(&import, &node_by_path)
            .map(|target| (target, false))
            .unwrap_or_else(|| {
                (
                    stable_id("external", &package_root(&import.specifier)),
                    true,
                )
            });
        if !node_ids.contains(&target) || source == target {
            continue;
        }
        let edge_key = format!("{source}|{target}|{}", import.kind);
        if !seen_edges.insert(edge_key.clone()) {
            continue;
        }
        if inferred {
            inferred_relationships += 1;
        } else {
            resolved_relationships += 1;
        }
        edges.push(TelescopeEdge {
            id: stable_id("edge", &edge_key),
            source,
            target,
            kind: import.kind.clone(),
            direction: "forward".to_string(),
            label: relationship_label(&import.kind),
            confidence: if inferred {
                "medium"
            } else {
                &import.confidence
            }
            .to_string(),
            provenance: if inferred {
                "static-import-package"
            } else {
                "resolved-static-import"
            }
            .to_string(),
            inferred,
            source_anchor: Some(TelescopeAnchor {
                path: import.source_path,
                line: Some(import.line),
                symbol: None,
                provenance: "source".to_string(),
            }),
        });
    }
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    groups.sort_by(|left, right| left.id.cmp(&right.id));
    let flows = build_flows(&nodes, &edges);
    let confidence = if files.is_empty() {
        "unavailable"
    } else if partial_count == 0 && !truncated {
        "high"
    } else {
        "partial"
    };
    let freshness_state = if commit.is_some() { "fresh" } else { "partial" };

    Ok(TelescopeProjection {
        schema_version: SCHEMA_VERSION.to_string(),
        repository_id: public_identifier("repository", request.repository_id),
        repository_name: request.repository_name.to_string(),
        binding: TelescopeBinding {
            workspace_id: public_identifier("workspace", request.workspace_id),
            branch: request.branch.to_string(),
            commit,
            dirty,
            dirty_state_fingerprint,
            workspace_fingerprint,
            generated_at,
        },
        freshness: TelescopeFreshness {
            state: freshness_state.to_string(),
            cache: "miss".to_string(),
            reason: "Generated from the active workspace fingerprint.".to_string(),
        },
        coverage: TelescopeCoverage {
            discovered_source_files: discovered,
            examined_source_files: files.len(),
            supported_source_files: supported_count,
            partial_source_files: partial_count,
            skipped_large_files: skipped_large,
            truncated,
            resolved_relationships,
            inferred_relationships,
            confidence: confidence.to_string(),
        },
        groups,
        nodes,
        edges,
        flows,
        warnings,
        enrichment: TelescopeEnrichment {
            enabled: false,
            provider: None,
            model: None,
            source_content_transmitted: false,
            status: "disabled-by-default".to_string(),
        },
    })
}

fn discover_source_files(
    root: &Path,
    cancellation: Option<&AtomicBool>,
) -> Result<(Vec<SourceFile>, usize, usize, bool), String> {
    let mut queue = VecDeque::from([root.to_path_buf()]);
    let mut files = Vec::new();
    let mut discovered = 0;
    let mut skipped_large = 0;
    let mut truncated = false;
    while let Some(directory) = queue.pop_front() {
        ensure_not_cancelled(cancellation)?;
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("Could not inspect {}: {error}", directory.display()))?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            ensure_not_cancelled(cancellation)?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !is_ignored_directory(&name)
                    && !entry
                        .file_type()
                        .map(|kind| kind.is_symlink())
                        .unwrap_or(true)
                {
                    queue.push_back(path);
                }
                continue;
            }
            let Some(extension) = path
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if !SOURCE_EXTENSIONS.contains(&extension.as_str()) {
                continue;
            }
            discovered += 1;
            if files.len() >= MAX_SOURCE_FILES {
                truncated = true;
                continue;
            }
            let bytes = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
            if bytes > MAX_SOURCE_BYTES {
                skipped_large += 1;
                continue;
            }
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let supported = matches!(
                extension.as_str(),
                "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "rs"
            );
            files.push(SourceFile {
                relative_path,
                absolute_path: path,
                language: language_label(&extension).to_string(),
                supported,
                bytes,
            });
        }
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok((files, discovered, skipped_large, truncated))
}

fn ensure_not_cancelled(cancellation: Option<&AtomicBool>) -> Result<(), String> {
    if cancellation.is_some_and(|value| value.load(Ordering::Acquire)) {
        Err("Telescope refresh cancelled.".to_string())
    } else {
        Ok(())
    }
}

fn is_ignored_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | "dist"
            | "out"
            | "build"
            | "coverage"
            | ".next"
            | ".turbo"
            | "vendor"
    )
}

fn language_label(extension: &str) -> &str {
    match extension {
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "rs" => "Rust",
        other => other,
    }
}

fn build_groups(files: &[SourceFile]) -> Vec<TelescopeGroup> {
    let mut labels = BTreeMap::new();
    for file in files {
        let group_id = group_id_for_path(&file.relative_path);
        labels
            .entry(group_id)
            .or_insert_with(|| group_label_for_path(&file.relative_path));
    }
    labels
        .into_iter()
        .map(|(id, label)| TelescopeGroup {
            id,
            label: label.clone(),
            kind: "subsystem".to_string(),
            parent_id: None,
            summary: format!("Source modules grouped under {label}."),
            confidence: "high".to_string(),
        })
        .collect()
}

fn group_id_for_path(path: &str) -> String {
    stable_id("group", &group_label_for_path(path))
}

fn group_label_for_path(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"src") && parts.len() > 2 {
        return format!("src / {}", parts[1]);
    }
    parts.first().copied().unwrap_or("root").to_string()
}

fn display_label(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .replace(['_', '-'], " ")
}

fn classify_node(path: &str, content: &str) -> String {
    let normalized = path.to_ascii_lowercase();
    if normalized.contains("route") || normalized.contains("router") || normalized.contains("/api/")
    {
        "route"
    } else if normalized.contains("service")
        || normalized.contains("client")
        || normalized.contains("controller")
    {
        "service"
    } else if normalized.contains("store")
        || normalized.contains("state")
        || normalized.contains("reducer")
        || normalized.contains("database")
    {
        "store"
    } else if normalized.contains("component")
        || normalized.ends_with(".tsx")
        || normalized.ends_with(".jsx")
    {
        "interface"
    } else if content.contains("fn main(") || content.contains("function main(") {
        "entrypoint"
    } else {
        "module"
    }
    .to_string()
}

fn semantic_summary(label: &str, kind: &str) -> String {
    match kind {
        "route" => format!("Routes requests or navigation through {label}."),
        "service" => format!("Coordinates service behavior for {label}."),
        "store" => format!("Owns state or persistence concerns for {label}."),
        "interface" => format!("Presents or adapts an interface through {label}."),
        "entrypoint" => format!("Starts an application boundary through {label}."),
        _ => format!("Defines the {label} module."),
    }
}

fn extract_symbols_and_shapes(content: &str, language: &str) -> (Vec<String>, Vec<String>) {
    let mut symbols = BTreeSet::new();
    let mut shapes = BTreeSet::new();
    for line in content.lines().take(8_000) {
        let trimmed = line.trim_start();
        let symbol_prefixes: &[&str] = if language == "Rust" {
            &[
                "pub fn ",
                "fn ",
                "pub struct ",
                "struct ",
                "pub enum ",
                "enum ",
                "pub trait ",
                "trait ",
            ]
        } else {
            &[
                "export function ",
                "export class ",
                "export const ",
                "function ",
                "class ",
                "const ",
            ]
        };
        for prefix in symbol_prefixes {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if let Some(name) = identifier(rest) {
                    symbols.insert(name);
                }
            }
        }
        for prefix in [
            "export interface ",
            "interface ",
            "export type ",
            "type ",
            "pub struct ",
            "struct ",
            "pub enum ",
            "enum ",
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if let Some(name) = identifier(rest) {
                    shapes.insert(name);
                }
            }
        }
    }
    (
        symbols.into_iter().take(12).collect(),
        shapes.into_iter().take(12).collect(),
    )
}

fn identifier(value: &str) -> Option<String> {
    let name = value
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn extract_imports(path: &str, content: &str, language: &str) -> Vec<ImportRecord> {
    let mut imports = Vec::new();
    for (index, line) in content.lines().take(12_000).enumerate() {
        let trimmed = line.trim();
        if language == "Rust" {
            if let Some(rest) = trimmed.strip_prefix("use ") {
                let specifier = rest
                    .trim_end_matches(';')
                    .split('{')
                    .next()
                    .unwrap_or(rest)
                    .trim_end_matches("::")
                    .to_string();
                imports.push(ImportRecord {
                    source_path: path.to_string(),
                    line: index + 1,
                    specifier,
                    kind: "uses".to_string(),
                    confidence: "high".to_string(),
                });
            } else if let Some(rest) = trimmed.strip_prefix("mod ") {
                let specifier = format!("self::{}", rest.trim_end_matches(';').trim());
                imports.push(ImportRecord {
                    source_path: path.to_string(),
                    line: index + 1,
                    specifier,
                    kind: "contains".to_string(),
                    confidence: "high".to_string(),
                });
            }
            continue;
        }
        let looks_like_import = trimmed.starts_with("import ")
            || trimmed.starts_with("export ") && trimmed.contains(" from ")
            || trimmed.contains("require(")
            || trimmed.contains("import(");
        if !looks_like_import {
            continue;
        }
        if let Some(specifier) = quoted_literal(trimmed) {
            let kind = if trimmed.contains("import(") {
                "dynamic"
            } else {
                "imports"
            };
            imports.push(ImportRecord {
                source_path: path.to_string(),
                line: index + 1,
                specifier,
                kind: kind.to_string(),
                confidence: if kind == "dynamic" { "medium" } else { "high" }.to_string(),
            });
        }
    }
    imports
}

fn quoted_literal(line: &str) -> Option<String> {
    for quote in ['\'', '"'] {
        let Some(end) = line.rfind(quote) else {
            continue;
        };
        let Some(start) = line[..end].rfind(quote) else {
            continue;
        };
        if start < end {
            return Some(line[start + 1..end].to_string());
        }
    }
    None
}

fn package_root(specifier: &str) -> String {
    if specifier.starts_with('@') {
        specifier.split('/').take(2).collect::<Vec<_>>().join("/")
    } else {
        specifier
            .split(['/', ':'])
            .next()
            .unwrap_or(specifier)
            .to_string()
    }
}

fn resolve_import(import: &ImportRecord, nodes: &BTreeMap<String, String>) -> Option<String> {
    if import.specifier.starts_with('.') {
        let parent = Path::new(&import.source_path)
            .parent()
            .unwrap_or(Path::new(""));
        let candidate = normalize_relative(&parent.join(&import.specifier));
        for path in module_candidates(&candidate) {
            if let Some(id) = nodes.get(&path) {
                return Some(id.clone());
            }
        }
    } else if import.specifier.starts_with("crate::") {
        let candidate = format!(
            "src/{}",
            import
                .specifier
                .trim_start_matches("crate::")
                .replace("::", "/")
        );
        for path in rust_module_candidates(&candidate) {
            if let Some(id) = nodes.get(&path) {
                return Some(id.clone());
            }
        }
    } else if import.specifier.starts_with("self::") {
        let parent = Path::new(&import.source_path)
            .parent()
            .unwrap_or(Path::new(""));
        let candidate = normalize_relative(
            &parent.join(
                import
                    .specifier
                    .trim_start_matches("self::")
                    .replace("::", "/"),
            ),
        );
        for path in rust_module_candidates(&candidate) {
            if let Some(id) = nodes.get(&path) {
                return Some(id.clone());
            }
        }
    } else if import.specifier.starts_with("super::") {
        let mut parent = Path::new(&import.source_path)
            .parent()
            .unwrap_or(Path::new(""));
        let mut specifier = import.specifier.as_str();
        while let Some(rest) = specifier.strip_prefix("super::") {
            parent = parent.parent().unwrap_or(Path::new(""));
            specifier = rest;
        }
        let candidate = normalize_relative(&parent.join(specifier.replace("::", "/")));
        for path in rust_module_candidates(&candidate) {
            if let Some(id) = nodes.get(&path) {
                return Some(id.clone());
            }
        }
    }
    None
}

fn rust_module_candidates(base: &str) -> Vec<String> {
    let mut current = base.to_string();
    let mut candidates = Vec::new();
    loop {
        candidates.extend(module_candidates(&current));
        let Some((parent, _)) = current.rsplit_once('/') else {
            break;
        };
        current = parent.to_string();
    }
    candidates
}

fn normalize_relative(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            _ => {}
        }
    }
    parts.join("/")
}

fn module_candidates(base: &str) -> Vec<String> {
    let without_extension = Path::new(base).extension().is_some();
    let mut candidates = vec![base.to_string()];
    if !without_extension {
        for extension in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs"] {
            candidates.push(format!("{base}.{extension}"));
            candidates.push(format!("{base}/index.{extension}"));
            if extension == "rs" {
                candidates.push(format!("{base}/mod.rs"));
            }
        }
    }
    candidates
}

fn relationship_label(kind: &str) -> String {
    match kind {
        "dynamic" => "loads at runtime",
        "uses" => "uses",
        "contains" => "contains",
        _ => "imports",
    }
    .to_string()
}

fn build_flows(nodes: &[TelescopeNode], edges: &[TelescopeEdge]) -> Vec<TelescopeFlow> {
    let mut flows = Vec::new();
    for entry in nodes
        .iter()
        .filter(|node| matches!(node.kind.as_str(), "route" | "entrypoint"))
        .take(8)
    {
        let mut node_ids = vec![entry.id.clone()];
        let mut edge_ids = Vec::new();
        let mut current = entry.id.clone();
        let mut visited = BTreeSet::from([entry.id.clone()]);
        for _ in 0..5 {
            let Some(edge) = edges
                .iter()
                .find(|edge| edge.source == current && !visited.contains(&edge.target))
            else {
                break;
            };
            edge_ids.push(edge.id.clone());
            node_ids.push(edge.target.clone());
            visited.insert(edge.target.clone());
            current = edge.target.clone();
        }
        if !edge_ids.is_empty() {
            let data_shape = nodes
                .iter()
                .find(|node| node.id == *node_ids.last().unwrap())
                .and_then(|node| node.data_shapes.first())
                .cloned();
            flows.push(TelescopeFlow {
                id: stable_id("flow", &entry.id),
                label: format!("{} flow", entry.label),
                kind: if entry.kind == "route" {
                    "request"
                } else {
                    "control"
                }
                .to_string(),
                node_ids,
                edge_ids,
                data_shape,
                confidence: "derived".to_string(),
                provenance: "static-relationship-walk".to_string(),
            });
        }
    }
    flows
}

fn ensure_cache_table(store_path: &Path) -> Result<(), String> {
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create Pronto data directory: {error}"))?;
    }
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS telescope_cache (
            repository_id TEXT NOT NULL,
            schema_version TEXT NOT NULL,
            workspace_fingerprint TEXT NOT NULL,
            generated_at TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY(repository_id, schema_version, workspace_fingerprint)
        );
        DELETE FROM telescope_cache
        WHERE instr(repository_id, '/') > 0
           OR instr(repository_id, char(92)) > 0
           OR instr(COALESCE(json_extract(payload_json, '$.repository_id'), ''), '/') > 0
           OR instr(COALESCE(json_extract(payload_json, '$.binding.workspace_id'), ''), '/') > 0
           OR instr(COALESCE(json_extract(payload_json, '$.repository_id'), ''), char(92)) > 0
           OR instr(COALESCE(json_extract(payload_json, '$.binding.workspace_id'), ''), char(92)) > 0;",
        )
        .map_err(|error| format!("Could not initialize Telescope cache: {error}"))
}

fn load_cached(
    store_path: &Path,
    repository_id: &str,
    workspace_fingerprint: &str,
) -> Result<Option<TelescopeProjection>, String> {
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    let result = connection.query_row(
        "SELECT payload_json FROM telescope_cache WHERE repository_id = ?1 AND schema_version = ?2 AND workspace_fingerprint = ?3",
        params![repository_id, SCHEMA_VERSION, workspace_fingerprint],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(payload) => serde_json::from_str(&payload)
            .map(Some)
            .map_err(|error| format!("Could not decode Telescope cache: {error}")),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Could not read Telescope cache: {error}")),
    }
}

fn save_cached(store_path: &Path, projection: &TelescopeProjection) -> Result<(), String> {
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    let payload = serde_json::to_string(projection)
        .map_err(|error| format!("Could not encode Telescope projection: {error}"))?;
    connection.execute(
        "INSERT INTO telescope_cache (repository_id, schema_version, workspace_fingerprint, generated_at, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(repository_id, schema_version, workspace_fingerprint) DO UPDATE SET generated_at=excluded.generated_at, payload_json=excluded.payload_json",
        params![projection.repository_id, projection.schema_version, projection.binding.workspace_fingerprint, projection.binding.generated_at, payload],
    ).map_err(|error| format!("Could not cache Telescope projection: {error}"))?;
    connection
        .execute(
            "DELETE FROM telescope_cache
         WHERE repository_id = ?1 AND schema_version = ?2 AND workspace_fingerprint NOT IN (
             SELECT workspace_fingerprint FROM telescope_cache
             WHERE repository_id = ?1 AND schema_version = ?2
             ORDER BY generated_at DESC LIMIT 4
         )",
            params![projection.repository_id, projection.schema_version],
        )
        .map_err(|error| format!("Could not prune Telescope cache: {error}"))?;
    Ok(())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = git_output_bytes(root, arguments)?;
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}

fn git_output_bytes(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("Could not run Git for Telescope: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(output.stdout)
}

fn dirty_state_fingerprint(root: &Path, status: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(status.as_bytes());
    for arguments in [
        ["diff", "--no-ext-diff", "--binary"].as_slice(),
        ["diff", "--no-ext-diff", "--binary", "--cached"].as_slice(),
    ] {
        if let Ok(output) = git_output_bytes(root, arguments) {
            digest.update(output);
        }
    }
    if let Ok(untracked) =
        git_output_bytes(root, &["ls-files", "--others", "--exclude-standard", "-z"])
    {
        for path_bytes in untracked.split(|byte| *byte == 0).take(MAX_SOURCE_FILES) {
            if path_bytes.is_empty() {
                continue;
            }
            let relative = String::from_utf8_lossy(path_bytes);
            let path = root.join(relative.as_ref());
            digest.update(path_bytes);
            if let Ok(metadata) = fs::metadata(&path) {
                digest.update(metadata.len().to_le_bytes());
                if metadata.len() <= MAX_SOURCE_BYTES {
                    if let Ok(content) = fs::read(path) {
                        digest.update(content);
                    }
                } else if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.duration_since(std::time::UNIX_EPOCH) {
                        digest.update(elapsed.as_nanos().to_le_bytes());
                    }
                }
            }
        }
    }
    format!(
        "dirty-{}",
        digest
            .finalize()
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn stable_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    format!(
        "{prefix}-{}",
        digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn public_identifier(kind: &str, value: &str) -> String {
    if Path::new(value).is_absolute() || value.contains('/') || value.contains('\\') {
        stable_id(kind, value)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pronto-telescope-{name}-{nonce}"));
        fs::create_dir_all(root.join("src/services")).unwrap();
        root
    }

    fn initialize_git_index(root: &Path) {
        for arguments in [vec!["init"], vec!["add", "."]] {
            let status = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(arguments)
                .status()
                .unwrap();
            assert!(status.success());
        }
    }

    #[test]
    fn extracts_typescript_topology_without_source_bodies_or_absolute_paths() {
        let root = fixture_root("typescript");
        fs::write(root.join("src/main.ts"), "import { load } from './services/load';\nexport interface RequestShape { id: string }\nexport function main() { return load(); }").unwrap();
        fs::write(
            root.join("src/services/load.ts"),
            "export function load() { return true; }",
        )
        .unwrap();
        let database = root.join("registry.db");
        let repository_id = format!("repository:{}", root.display());
        let workspace_id = format!("workspace:{}", root.display());
        let request = TelescopeRequest {
            repository_id: &repository_id,
            repository_name: "Fixture",
            workspace_id: &workspace_id,
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let projection = get_or_generate(&database, &request, true).unwrap();
        assert_eq!(projection.schema_version, SCHEMA_VERSION);
        assert_eq!(projection.coverage.supported_source_files, 2);
        assert_eq!(projection.edges.len(), 1);
        assert!(projection
            .nodes
            .iter()
            .any(|node| node.data_shapes.contains(&"RequestShape".to_string())));
        let json = serde_json::to_string(&projection).unwrap();
        assert!(!json.contains(root.to_string_lossy().as_ref()));
        assert!(!json.contains("return true"));
        let connection = Connection::open(&database).unwrap();
        let cached: (String, String) = connection
            .query_row(
                "SELECT repository_id, payload_json FROM telescope_cache LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(!cached.0.contains('/'));
        assert!(!cached.1.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unsupported_languages_are_visible_as_partial_generic_topology() {
        let root = fixture_root("partial");
        fs::write(root.join("src/main.py"), "print('hello')").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: false,
        };
        let projection = get_or_generate(&database, &request, true).unwrap();
        assert_eq!(projection.coverage.partial_source_files, 1);
        assert_eq!(projection.coverage.confidence, "partial");
        assert_eq!(projection.nodes[0].summary_status, "derived");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn matching_workspace_fingerprint_reuses_cache() {
        let root = fixture_root("cache");
        fs::write(root.join("src/main.ts"), "export const ready = true;").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let first = get_or_generate(&database, &request, true).unwrap();
        let second = get_or_generate(&database, &request, false).unwrap();
        assert_eq!(
            first.binding.workspace_fingerprint,
            second.binding.workspace_fingerprint
        );
        assert_eq!(second.freshness.cache, "hit");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dynamic_double_quoted_imports_and_cycles_remain_inspectable() {
        let root = fixture_root("dynamic-cycle");
        fs::write(
            root.join("src/main.ts"),
            "export async function main() { return import(\"./services/load\"); }",
        )
        .unwrap();
        fs::write(
            root.join("src/services/load.ts"),
            "import { main } from \"../main\";\nexport const load = main;",
        )
        .unwrap();
        let projection = get_or_generate(
            &root.join("registry.db"),
            &TelescopeRequest {
                repository_id: "repo",
                repository_name: "Fixture",
                workspace_id: "workspace",
                workspace_path: &root,
                branch: "dev",
                known_commit: None,
                known_dirty: false,
            },
            true,
        )
        .unwrap();
        assert_eq!(projection.edges.len(), 2);
        assert!(projection.edges.iter().any(|edge| edge.kind == "dynamic"));
        assert!(projection.flows.iter().all(|flow| flow.node_ids.len() <= 6));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn identifiers_are_deterministic_across_forced_regeneration() {
        let root = fixture_root("deterministic");
        fs::create_dir_all(root.join("packages/client/src")).unwrap();
        fs::write(
            root.join("packages/client/src/index.ts"),
            "export const client = true;",
        )
        .unwrap();
        fs::write(root.join("src/main.py"), "print('partial')").unwrap();
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let first = get_or_generate(&root.join("registry.db"), &request, true).unwrap();
        let second = get_or_generate(&root.join("registry.db"), &request, true).unwrap();
        assert_eq!(
            first.nodes.iter().map(|node| &node.id).collect::<Vec<_>>(),
            second.nodes.iter().map(|node| &node.id).collect::<Vec<_>>()
        );
        assert_eq!(
            first
                .groups
                .iter()
                .map(|group| &group.id)
                .collect::<Vec<_>>(),
            second
                .groups
                .iter()
                .map(|group| &group.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(first.coverage.partial_source_files, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dirty_content_changes_invalidate_a_matching_status_shape() {
        let root = fixture_root("dirty-cache");
        fs::write(root.join("src/main.ts"), "export const value = 1;").unwrap();
        initialize_git_index(&root);
        fs::write(root.join("src/main.ts"), "export const value = 2;").unwrap();
        let database = std::env::temp_dir().join(format!(
            "pronto-telescope-cache-{}.db",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: true,
        };
        let first = get_or_generate(&database, &request, false).unwrap();
        fs::write(root.join("src/main.ts"), "export const value = 3;").unwrap();
        let second = get_or_generate(&database, &request, false).unwrap();
        assert_ne!(
            first.binding.dirty_state_fingerprint,
            second.binding.dirty_state_fingerprint
        );
        assert_eq!(second.freshness.cache, "miss");
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancelled_refresh_does_not_generate_or_cache_a_partial_projection() {
        let root = fixture_root("cancelled-refresh");
        fs::write(root.join("src/main.ts"), "export const value = 1;").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: false,
        };
        let cancellation = AtomicBool::new(true);

        let error = get_or_generate_cancellable(&database, &request, true, Some(&cancellation))
            .unwrap_err();

        assert_eq!(error, "Telescope refresh cancelled.");
        assert!(!database.exists());
        let _ = fs::remove_dir_all(root);
    }
}
