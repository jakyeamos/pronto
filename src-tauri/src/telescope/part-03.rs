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
            provenance: vec!["workspace-source".to_string()],
            source_file_count: 0,
            measured_lines: 0,
            visual_archetype: "district".to_string(),
            visual_override_provenance: "measured-default".to_string(),
            narrative_status: "derived".to_string(),
        })
        .collect()
}

fn annotate_group_measurements(groups: &mut [TelescopeGroup], nodes: &[TelescopeNode]) {
    let mut measurements = BTreeMap::<String, (usize, usize)>::new();
    for node in nodes {
        let entry = measurements.entry(node.group_id.clone()).or_default();
        entry.0 += node.source_file_count;
        entry.1 += node.measured_lines;
    }
    for group in groups {
        let (source_file_count, measured_lines) =
            measurements.get(&group.id).copied().unwrap_or_default();
        group.source_file_count = source_file_count;
        group.measured_lines = measured_lines;
    }
}

fn visual_archetype_for_kind(kind: &str) -> &'static str {
    match kind {
        "route" | "entrypoint" => "tower",
        "store" => "slab-stack",
        "interface" => "fin-row",
        "integration" => "low-slab",
        "service" => "cube",
        _ => "cube",
    }
}

fn rail_kind_for_relationship(kind: &str) -> &'static str {
    match kind {
        "dynamic" => "event",
        "contains" | "uses" => "control",
        _ => "import",
    }
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
