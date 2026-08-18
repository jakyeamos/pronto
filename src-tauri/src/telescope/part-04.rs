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
