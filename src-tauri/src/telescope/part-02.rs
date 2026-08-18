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
        let measured_lines = content.lines().count();
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
            source_paths: vec![file.relative_path.clone()],
            measured_lines,
            source_file_count: 1,
            visual_building_id: None,
            visual_archetype: visual_archetype_for_kind(&kind).to_string(),
            visual_override_provenance: "measured-default".to_string(),
            narrative_status: "derived".to_string(),
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
            provenance: vec!["static-import".to_string()],
            source_file_count: 0,
            measured_lines: 0,
            visual_archetype: "low-slab".to_string(),
            visual_override_provenance: "measured-default".to_string(),
            narrative_status: "derived".to_string(),
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
                source_paths: Vec::new(),
                measured_lines: 0,
                source_file_count: 0,
                visual_building_id: None,
                visual_archetype: "low-slab".to_string(),
                visual_override_provenance: "measured-default".to_string(),
                narrative_status: "derived".to_string(),
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
            rail_kind: rail_kind_for_relationship(&import.kind).to_string(),
            visual_override_provenance: "measured-default".to_string(),
            narrative_status: "derived".to_string(),
        });
    }
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    groups.sort_by(|left, right| left.id.cmp(&right.id));
    annotate_group_measurements(&mut groups, &nodes);
    let mut flows = build_flows(&nodes, &edges);
    let measured_fingerprint = measured_topology_fingerprint(&files, &edges);
    let narrative = load_and_apply_narrative(
        request.workspace_path,
        &files,
        &measured_fingerprint,
        &mut groups,
        &mut nodes,
        &mut edges,
        &mut flows,
        &mut warnings,
        cancellation,
    )?;
    let behavior_contract = load_behavior_contract(request.workspace_path, &mut warnings);
    let (actions, action_coverage) =
        build_telescope_actions(&narrative, &behavior_contract, &nodes, &edges, &flows);
    for action in &actions {
        if let Some(behavior_id) = &action.behavior_id {
            if action.behavior_state != "declared" {
                warnings.push(TelescopeWarning {
                    code: "action-behavior-unresolved".to_string(),
                    message: format!(
                        "Action {} references behavior {} but that ID is not present in the canonical behavior contract; it remains unprofiled.",
                        action.id, behavior_id
                    ),
                    scope: "actions".to_string(),
                });
            }
        }
    }
    if matches!(
        action_coverage.inventory_status.as_str(),
        "missing" | "inferred" | "partial"
    ) {
        warnings.push(TelescopeWarning {
            code: "action-inventory-partial".to_string(),
            message: match action_coverage.inventory_status.as_str() {
                "missing" => "No user-action inventory was found; Telescope is showing only source-derived flow actions.".to_string(),
                "inferred" => "The action inventory is inferred from static flows; review .pronto/telescope-map.json before treating actions as confirmed product behavior.".to_string(),
                _ => "Some catalogued actions do not have complete source or relationship mapping and are shown as partial.".to_string(),
            },
            scope: "actions".to_string(),
        });
    }
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
        actions,
        action_coverage,
        warnings,
        enrichment: TelescopeEnrichment {
            enabled: false,
            provider: None,
            model: None,
            source_content_transmitted: false,
            status: "disabled-by-default".to_string(),
        },
        narrative,
    })
}
