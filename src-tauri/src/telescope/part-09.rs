fn validate_manifest_shape(
    manifest: &TelescopeManifest,
    files: &[SourceFile],
    warnings: &mut Vec<TelescopeWarning>,
    drift_warnings: &mut Vec<TelescopeWarning>,
) {
    if manifest.groups.len() > 7 || manifest.groups.len() < 4 {
        push_manifest_warning(
            warnings,
            drift_warnings,
            "narrative-group-range",
            format!(
                "Authored architecture maps should use 4–7 neighborhoods; this map declares {}.",
                manifest.groups.len()
            ),
        );
    }
    let discovered = files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<BTreeSet<_>>();
    let mut group_ids = BTreeSet::new();
    for group in &manifest.groups {
        if group.id.trim().is_empty() {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-group-id-missing",
                "Authored neighborhoods require a stable id.".to_string(),
            );
        } else if !group_ids.insert(group.id.clone()) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-group-duplicate",
                format!(
                    "Authored neighborhood {} is declared more than once.",
                    group.id
                ),
            );
        }
        validate_authored_status(
            &group.status,
            &format!("neighborhood {}", group.id),
            warnings,
            drift_warnings,
        );
        for prefix in &group.path_prefixes {
            if !is_safe_relative_path(prefix) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-prefix-invalid",
                    format!(
                        "Authored neighborhood {} has an unsafe source prefix.",
                        group.id
                    ),
                );
            } else if !discovered
                .iter()
                .any(|path| path_matches_prefix(path, prefix))
            {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-prefix-unmapped",
                    format!(
                        "Authored neighborhood {} has no measured source under {}.",
                        group.id, prefix
                    ),
                );
            }
        }
    }

    let mut node_ids = BTreeSet::new();
    let mut authored_source_paths = BTreeSet::new();
    for node in &manifest.nodes {
        if node.id.trim().is_empty() {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-building-id-missing",
                "Authored buildings require a stable id.".to_string(),
            );
        } else if !node_ids.insert(node.id.clone()) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-building-duplicate",
                format!("Authored building {} is declared more than once.", node.id),
            );
        }
        if !node.group_id.is_empty() && !group_ids.contains(&node.group_id) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-building-group-missing",
                format!(
                    "Authored building {} references an undeclared neighborhood {}.",
                    node.id, node.group_id
                ),
            );
        }
        if node.files.is_empty() {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-building-files-missing",
                format!(
                    "Authored building {} has no representative source file.",
                    node.id
                ),
            );
        }
        validate_authored_status(
            &node.status,
            &format!("building {}", node.id),
            warnings,
            drift_warnings,
        );
        for path in &node.files {
            if !authored_source_paths.insert(path.clone()) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-source-duplicate",
                    format!(
                        "Authored source path {} is assigned to more than one building.",
                        path
                    ),
                );
            }
            if !is_safe_relative_path(path) || !discovered.contains(path.as_str()) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-source-unmapped",
                    format!(
                        "Authored building {} references an unmapped source path.",
                        node.id
                    ),
                );
            }
        }
    }

    let mut edge_ids = BTreeSet::new();
    for edge in &manifest.edges {
        if edge.id.trim().is_empty() {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-rail-id-missing",
                "Authored rails require a stable id.".to_string(),
            );
        } else if !edge_ids.insert(edge.id.clone()) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-rail-duplicate",
                format!("Authored rail {} is declared more than once.", edge.id),
            );
        }
        for path in [&edge.source_file, &edge.target_file] {
            if !is_safe_relative_path(path) || !discovered.contains(path.as_str()) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-rail-unmapped",
                    format!(
                        "Authored rail {} references an unmapped source path.",
                        edge.id
                    ),
                );
            }
        }
        validate_authored_status(
            &edge.status,
            &format!("rail {}", edge.id),
            warnings,
            drift_warnings,
        );
    }

    let mut flow_ids = BTreeSet::new();
    for flow in &manifest.flows {
        if !flow_ids.insert(flow.id.clone()) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-flow-duplicate",
                format!("Authored flow {} is declared more than once.", flow.id),
            );
        }
        for node_id in &flow.node_ids {
            if !node_ids.contains(node_id) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-flow-node-invalid",
                    format!(
                        "Authored flow {} references an unknown building {}.",
                        flow.id, node_id
                    ),
                );
            }
        }
        for edge_id in &flow.edge_ids {
            if !edge_ids.contains(edge_id) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-flow-rail-invalid",
                    format!(
                        "Authored flow {} references an unknown rail {}.",
                        flow.id, edge_id
                    ),
                );
            }
        }
        validate_authored_status(
            &flow.status,
            &format!("flow {}", flow.id),
            warnings,
            drift_warnings,
        );
    }
    let mut action_ids = BTreeSet::new();
    for action in &manifest.actions {
        if action.id.trim().is_empty() {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-action-id-missing",
                "Authored actions require a stable id or behavior id.".to_string(),
            );
        } else if !action_ids.insert(action.id.clone()) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-action-duplicate",
                format!("Authored action {} is declared more than once.", action.id),
            );
        }
        for path in &action.files {
            if !is_safe_relative_path(path) || !discovered.contains(path.as_str()) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-action-source-unmapped",
                    format!(
                        "Authored action {} references an unmapped source path.",
                        action.id
                    ),
                );
            }
        }
        for node_id in &action.node_ids {
            if !node_ids.contains(node_id) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-action-node-invalid",
                    format!(
                        "Authored action {} references an unknown building {}.",
                        action.id, node_id
                    ),
                );
            }
        }
        for edge_id in &action.edge_ids {
            if !edge_ids.contains(edge_id) {
                push_manifest_warning(
                    warnings,
                    drift_warnings,
                    "narrative-action-rail-invalid",
                    format!(
                        "Authored action {} references an unknown rail {}.",
                        action.id, edge_id
                    ),
                );
            }
        }
        validate_authored_status(
            &action.status,
            &format!("action {}", action.id),
            warnings,
            drift_warnings,
        );
    }
    if let Some(primary_flow_id) = &manifest.primary_flow_id {
        if !flow_ids.contains(primary_flow_id) {
            push_manifest_warning(
                warnings,
                drift_warnings,
                "narrative-primary-flow-invalid",
                format!(
                    "Primary authored flow {} is not declared in flows.",
                    primary_flow_id
                ),
            );
        }
    }
}

fn validate_authored_status(
    status: &str,
    subject: &str,
    warnings: &mut Vec<TelescopeWarning>,
    drift_warnings: &mut Vec<TelescopeWarning>,
) {
    if !status.is_empty() && status != "draft" && status != "reviewed" {
        push_manifest_warning(
            warnings,
            drift_warnings,
            "narrative-status-invalid",
            format!("Authored {subject} has unsupported review status {status}."),
        );
    }
}

fn push_manifest_warning(
    warnings: &mut Vec<TelescopeWarning>,
    drift_warnings: &mut Vec<TelescopeWarning>,
    code: &str,
    message: String,
) {
    let warning = TelescopeWarning {
        code: code.to_string(),
        message,
        scope: "narrative".to_string(),
    };
    warnings.push(warning.clone());
    drift_warnings.push(warning);
}

fn authored_status(authored_status: &str, manifest_status: &str) -> String {
    match authored_status {
        "reviewed" => "reviewed".to_string(),
        "draft" => "draft".to_string(),
        _ if manifest_status == "stale" => "stale".to_string(),
        _ => "draft".to_string(),
    }
}

fn node_source_paths(node: &TelescopeNode) -> Vec<String> {
    if !node.source_paths.is_empty() {
        return node.source_paths.clone();
    }
    node.source_anchors
        .iter()
        .map(|anchor| anchor.path.clone())
        .collect()
}

fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') || Path::new(path).is_absolute() {
        return false;
    }
    Path::new(path)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
}

fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    let normalized = prefix.trim_matches('/');
    path == normalized || path.starts_with(&format!("{normalized}/"))
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|candidate| candidate == value) {
        values.push(value.to_string());
    }
}

fn measured_topology_fingerprint(files: &[SourceFile], edges: &[TelescopeEdge]) -> String {
    let mut input = String::new();
    for file in files {
        input.push_str(&file.relative_path);
        input.push(':');
        input.push_str(&file.bytes.to_string());
        input.push('\n');
    }
    for edge in edges {
        input.push_str(&edge.source);
        input.push('|');
        input.push_str(&edge.target);
        input.push('|');
        input.push_str(&edge.kind);
        input.push('\n');
    }
    stable_id("topology", &input)
}

fn apply_primary_flow(flows: &mut [TelescopeFlow], primary_flow_id: &mut Option<String>) {
    if primary_flow_id.is_none() {
        *primary_flow_id = flows.first().map(|flow| flow.id.clone());
    }
    let primary = primary_flow_id.clone();
    for flow in flows {
        flow.primary = primary.as_deref() == Some(flow.id.as_str());
    }
}
