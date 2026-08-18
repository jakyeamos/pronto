fn load_and_apply_narrative(
    root: &Path,
    files: &[SourceFile],
    measured_fingerprint: &str,
    groups: &mut Vec<TelescopeGroup>,
    nodes: &mut [TelescopeNode],
    edges: &mut [TelescopeEdge],
    flows: &mut Vec<TelescopeFlow>,
    warnings: &mut Vec<TelescopeWarning>,
    cancellation: Option<&AtomicBool>,
) -> Result<TelescopeNarrative, String> {
    let manifest_path = root.join(NARRATIVE_MANIFEST_PATH);
    let mut narrative = TelescopeNarrative {
        manifest_path: NARRATIVE_MANIFEST_PATH.to_string(),
        status: "missing".to_string(),
        measured_fingerprint: Some(measured_fingerprint.to_string()),
        visual_model_version: VISUAL_MODEL_VERSION.to_string(),
        ..TelescopeNarrative::default()
    };

    if !manifest_path.exists() {
        let warning = TelescopeWarning {
            code: "narrative-manifest-missing".to_string(),
            message: format!(
                "No authored architecture map was found at {NARRATIVE_MANIFEST_PATH}; the visual model is measured-only."
            ),
            scope: "narrative".to_string(),
        };
        warnings.push(warning.clone());
        narrative.drift_warnings.push(warning);
        apply_primary_flow(flows, &mut narrative.primary_flow_id);
        return Ok(narrative);
    }

    ensure_not_cancelled(cancellation)?;
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Could not read {NARRATIVE_MANIFEST_PATH}: {error}"))?;
    narrative.manifest_fingerprint = Some(stable_id("narrative", &raw));
    if raw.len() > 256 * 1024 {
        let warning = TelescopeWarning {
            code: "narrative-manifest-too-large".to_string(),
            message: "The authored architecture map exceeds the 256 KiB manifest limit and was not applied.".to_string(),
            scope: "narrative".to_string(),
        };
        warnings.push(warning.clone());
        narrative.drift_warnings.push(warning);
        narrative.status = "stale".to_string();
        apply_primary_flow(flows, &mut narrative.primary_flow_id);
        return Ok(narrative);
    }
    let manifest = match serde_json::from_str::<TelescopeManifest>(&raw) {
        Ok(manifest) => manifest,
        Err(error) => {
            let warning = TelescopeWarning {
                code: "narrative-manifest-invalid".to_string(),
                message: format!("The authored architecture map could not be applied: {error}"),
                scope: "narrative".to_string(),
            };
            warnings.push(warning.clone());
            narrative.drift_warnings.push(warning);
            narrative.status = "draft".to_string();
            apply_primary_flow(flows, &mut narrative.primary_flow_id);
            return Ok(narrative);
        }
    };

    if !manifest.schema_version.is_empty() && manifest.schema_version != "pronto-telescope-map/v1" {
        let warning = TelescopeWarning {
            code: "narrative-manifest-version-unsupported".to_string(),
            message: format!(
                "Manifest schema {} is not the supported pronto-telescope-map/v1 shape.",
                manifest.schema_version
            ),
            scope: "narrative".to_string(),
        };
        warnings.push(warning.clone());
        narrative.drift_warnings.push(warning);
    }

    let requested_status = match manifest.status.as_str() {
        "reviewed" => "reviewed",
        "draft" | "" => "draft",
        other => {
            let warning = TelescopeWarning {
                code: "narrative-status-invalid".to_string(),
                message: format!("Manifest status {other} is invalid; treating it as draft."),
                scope: "narrative".to_string(),
            };
            warnings.push(warning.clone());
            narrative.drift_warnings.push(warning);
            "draft"
        }
    };
    narrative.status = requested_status.to_string();
    if manifest
        .topology_fingerprint
        .as_deref()
        .is_some_and(|fingerprint| fingerprint != measured_fingerprint)
    {
        narrative.status = "stale".to_string();
        let warning = TelescopeWarning {
            code: "narrative-drift-detected".to_string(),
            message: "Measured source topology no longer matches the fingerprint recorded by the authored map.".to_string(),
            scope: "narrative".to_string(),
        };
        warnings.push(warning.clone());
        narrative.drift_warnings.push(warning);
    }

    validate_manifest_shape(&manifest, files, warnings, &mut narrative.drift_warnings);
    narrative.authored_groups = manifest.groups.clone();
    narrative.authored_nodes = manifest.nodes.clone();
    narrative.authored_edges = manifest.edges.clone();
    narrative.authored_actions = manifest.actions.clone();
    narrative.primary_flow_id = manifest.primary_flow_id.clone();

    let discovered_paths = files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<BTreeSet<_>>();
    let authored_paths = manifest
        .nodes
        .iter()
        .flat_map(|node| node.files.iter())
        .filter(|path| is_safe_relative_path(path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mapped_paths = authored_paths
        .intersection(&discovered_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    narrative.coverage = TelescopeNarrativeCoverage {
        authored_source_files: authored_paths.len(),
        mapped_source_files: mapped_paths.len(),
        unmapped_source_files: discovered_paths
            .difference(&mapped_paths)
            .cloned()
            .collect(),
        coverage_percent: if discovered_paths.is_empty() {
            0
        } else {
            ((mapped_paths.len() * 100) / discovered_paths.len()).min(100) as u8
        },
    };

    let measured_groups = groups.clone();
    let authored_group_ids = manifest
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<BTreeSet<_>>();
    let mut authored_group_assignments = BTreeMap::<String, String>::new();

    // A path prefix is a reviewed conceptual boundary, not a replacement for
    // measured topology. Longest-prefix matching lets a narrow neighborhood
    // win over a broad repository-wide prefix deterministically.
    for node in nodes.iter() {
        let node_paths = node_source_paths(node);
        let best_group = manifest
            .groups
            .iter()
            .flat_map(|group| {
                group
                    .path_prefixes
                    .iter()
                    .filter(|prefix| {
                        node_paths
                            .iter()
                            .any(|path| path_matches_prefix(path, prefix))
                    })
                    .map(move |prefix| (prefix.len(), group.id.clone()))
            })
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
            .map(|(_, group_id)| group_id);
        if let Some(group_id) = best_group {
            authored_group_assignments.insert(node.id.clone(), group_id);
        }
    }

    // Preserve the earlier ID/label matching behavior for manifests authored
    // against a measured export, while allowing a new conceptual ID to become
    // a real district when it has a path or building assignment.
    for authored_group in &manifest.groups {
        let measured_ids = measured_groups
            .iter()
            .filter(|group| {
                group.id == authored_group.id
                    || (!authored_group.label.is_empty()
                        && group.label.eq_ignore_ascii_case(&authored_group.label))
            })
            .map(|group| group.id.clone())
            .collect::<BTreeSet<_>>();
        for node in nodes.iter() {
            if measured_ids.contains(&node.group_id) {
                authored_group_assignments
                    .entry(node.id.clone())
                    .or_insert_with(|| authored_group.id.clone());
            }
        }
    }

    // An explicit building group is the most specific authored decision and
    // therefore wins over a prefix inferred from the neighborhood manifest.
    for authored_node in &manifest.nodes {
        if authored_node.group_id.is_empty()
            || !authored_group_ids.contains(&authored_node.group_id)
        {
            continue;
        }
        for node in nodes.iter() {
            let matches_node = node.id == authored_node.id
                || authored_node
                    .files
                    .iter()
                    .any(|file| node_source_paths(node).iter().any(|path| path == file));
            if matches_node {
                authored_group_assignments.insert(node.id.clone(), authored_node.group_id.clone());
            }
        }
    }
    for node in nodes.iter_mut() {
        if let Some(group_id) = authored_group_assignments.get(&node.id) {
            node.group_id = group_id.clone();
        }
    }

    let mut authored_groups = Vec::new();
    for authored_group in &manifest.groups {
        let Some(measured_group) = measured_groups.iter().find(|group| {
            group.id == authored_group.id
                || (!authored_group.label.is_empty()
                    && group.label.eq_ignore_ascii_case(&authored_group.label))
        }) else {
            let has_assigned_nodes = nodes.iter().any(|node| node.group_id == authored_group.id);
            if !has_assigned_nodes {
                continue;
            }
            let mut group = TelescopeGroup {
                id: authored_group.id.clone(),
                label: authored_group.label.clone(),
                kind: if authored_group.kind.is_empty() {
                    "subsystem".to_string()
                } else {
                    authored_group.kind.clone()
                },
                parent_id: None,
                summary: authored_group.summary.clone(),
                confidence: "authored".to_string(),
                provenance: vec!["authored-manifest".to_string()],
                source_file_count: 0,
                measured_lines: 0,
                visual_archetype: if authored_group.visual_archetype.is_empty() {
                    "district".to_string()
                } else {
                    authored_group.visual_archetype.clone()
                },
                visual_override_provenance: "authored-manifest".to_string(),
                narrative_status: authored_status(&authored_group.status, &narrative.status),
            };
            annotate_group_measurements(std::slice::from_mut(&mut group), nodes);
            authored_groups.push(group);
            continue;
        };
        let mut group = measured_group.clone();
        group.id = authored_group.id.clone();
        if !authored_group.label.is_empty() {
            group.label = authored_group.label.clone();
        }
        if !authored_group.kind.is_empty() {
            group.kind = authored_group.kind.clone();
        }
        if !authored_group.summary.is_empty() {
            group.summary = authored_group.summary.clone();
        }
        if !authored_group.visual_archetype.is_empty() {
            group.visual_archetype = authored_group.visual_archetype.clone();
        }
        group.visual_override_provenance = "authored-manifest".to_string();
        group.narrative_status = authored_status(&authored_group.status, &narrative.status);
        push_unique(&mut group.provenance, "authored-manifest");
        annotate_group_measurements(std::slice::from_mut(&mut group), nodes);
        authored_groups.push(group);
    }
    groups.retain(|group| {
        !authored_group_ids.contains(&group.id)
            && nodes.iter().any(|node| node.group_id == group.id)
    });
    groups.extend(authored_groups);
    groups.sort_by(|left, right| left.id.cmp(&right.id));

    let mut authored_node_to_generated = BTreeMap::<String, Vec<String>>::new();
    for authored_node in &manifest.nodes {
        let matching_ids = nodes
            .iter()
            .filter(|node| {
                node.id == authored_node.id
                    || authored_node
                        .files
                        .iter()
                        .any(|file| node_source_paths(node).iter().any(|path| path == file))
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if matching_ids.is_empty() {
            continue;
        }
        authored_node_to_generated.insert(authored_node.id.clone(), matching_ids.clone());
        for (index, node) in nodes
            .iter_mut()
            .filter(|node| matching_ids.contains(&node.id))
            .enumerate()
        {
            node.visual_building_id = Some(authored_node.id.clone());
            if !authored_node.visual_archetype.is_empty() {
                node.visual_archetype = authored_node.visual_archetype.clone();
            }
            node.visual_override_provenance = "authored-manifest".to_string();
            node.narrative_status = authored_status(&authored_node.status, &narrative.status);
            node.source_file_count = node.source_file_count.max(authored_node.files.len());
            if index == 0 {
                if !authored_node.label.is_empty() {
                    node.label = authored_node.label.clone();
                }
                if !authored_node.what_it_does.is_empty() {
                    node.semantic_summary = authored_node.what_it_does.clone();
                    node.summary_status = node.narrative_status.clone();
                }
                if !authored_node.how_its_built.is_empty() {
                    node.implementation_summary = authored_node.how_its_built.clone();
                }
            }
            push_unique(&mut node.provenance, "authored-manifest");
        }
    }

    let mut authored_edge_to_generated = BTreeMap::<String, Vec<String>>::new();
    let generated_node_for_path = nodes
        .iter()
        .flat_map(|node| {
            node_source_paths(node)
                .into_iter()
                .map(|path| (path, node.id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    for authored_edge in &manifest.edges {
        let source = generated_node_for_path.get(&authored_edge.source_file);
        let target = generated_node_for_path.get(&authored_edge.target_file);
        let matching_ids = match (source, target) {
            (Some(source), Some(target)) => edges
                .iter()
                .filter(|edge| &edge.source == source && &edge.target == target)
                .map(|edge| edge.id.clone())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        if matching_ids.is_empty() {
            continue;
        }
        authored_edge_to_generated.insert(authored_edge.id.clone(), matching_ids.clone());
        for edge in edges
            .iter_mut()
            .filter(|edge| matching_ids.contains(&edge.id))
        {
            if !authored_edge.kind.is_empty() {
                edge.kind = authored_edge.kind.clone();
            }
            if !authored_edge.label.is_empty() {
                edge.label = authored_edge.label.clone();
            }
            if !authored_edge.rail_kind.is_empty() {
                edge.rail_kind = authored_edge.rail_kind.clone();
            }
            edge.visual_override_provenance = "authored-manifest".to_string();
            edge.narrative_status = authored_status(&authored_edge.status, &narrative.status);
        }
    }

    let mut resolved_authored_flows = Vec::new();
    for authored_flow in &manifest.flows {
        let resolved_nodes = authored_flow
            .node_ids
            .iter()
            .flat_map(|id| {
                if nodes.iter().any(|node| node.id == *id) {
                    vec![id.clone()]
                } else {
                    authored_node_to_generated
                        .get(id)
                        .cloned()
                        .unwrap_or_default()
                }
            })
            .collect::<Vec<_>>();
        let resolved_edges = authored_flow
            .edge_ids
            .iter()
            .flat_map(|id| {
                if edges.iter().any(|edge| edge.id == *id) {
                    vec![id.clone()]
                } else {
                    authored_edge_to_generated
                        .get(id)
                        .cloned()
                        .unwrap_or_default()
                }
            })
            .collect::<Vec<_>>();
        let complete = resolved_nodes.len() == authored_flow.node_ids.len()
            && resolved_edges.len() == authored_flow.edge_ids.len()
            && !resolved_nodes.is_empty()
            && !resolved_edges.is_empty();
        let status = if complete {
            authored_status(&authored_flow.status, &narrative.status)
        } else {
            let warning = TelescopeWarning {
                code: "narrative-flow-unmapped".to_string(),
                message: format!(
                    "Authored flow {} references source relationships that are not present in the measured topology.",
                    authored_flow.id
                ),
                scope: "narrative".to_string(),
            };
            warnings.push(warning.clone());
            narrative.drift_warnings.push(warning);
            "partial".to_string()
        };
        let resolved_flow = TelescopeNarrativeFlow {
            id: authored_flow.id.clone(),
            label: authored_flow.label.clone(),
            kind: authored_flow.kind.clone(),
            node_ids: resolved_nodes.clone(),
            edge_ids: resolved_edges.clone(),
            data_shape: authored_flow.data_shape.clone(),
            status,
            primary: authored_flow.primary,
        };
        if complete {
            flows.push(TelescopeFlow {
                id: authored_flow.id.clone(),
                label: authored_flow.label.clone(),
                kind: authored_flow.kind.clone(),
                node_ids: resolved_nodes,
                edge_ids: resolved_edges,
                data_shape: authored_flow.data_shape.clone(),
                confidence: "authored".to_string(),
                provenance: "authored-manifest".to_string(),
                narrative_status: authored_status(&authored_flow.status, &narrative.status),
                primary: authored_flow.primary,
            });
        }
        resolved_authored_flows.push(resolved_flow);
    }
    narrative.authored_flows = resolved_authored_flows;
    if narrative.primary_flow_id.is_none() {
        narrative.primary_flow_id = manifest
            .flows
            .iter()
            .find(|flow| flow.primary)
            .map(|flow| flow.id.clone());
    }
    apply_primary_flow(flows, &mut narrative.primary_flow_id);
    narrative.visual_model_version = manifest
        .visual_model_version
        .clone()
        .unwrap_or_else(|| VISUAL_MODEL_VERSION.to_string());
    Ok(narrative)
}
