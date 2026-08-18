fn load_behavior_contract(root: &Path, warnings: &mut Vec<TelescopeWarning>) -> BehaviorContract {
    let path = root.join(CONTRACT_PATH);
    if !path.exists() {
        return BehaviorContract::default();
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) => {
            warnings.push(TelescopeWarning {
                code: "behavior-contract-unavailable".to_string(),
                message: format!(
                    "The behavior assurance contract could not be read; action mapping remains narrative or inferred: {error}"
                ),
                scope: "actions".to_string(),
            });
            return BehaviorContract::default();
        }
    };
    let contract = match serde_json::from_str::<BehaviorContract>(&raw) {
        Ok(contract) => contract,
        Err(error) => {
            warnings.push(TelescopeWarning {
                code: "behavior-contract-invalid".to_string(),
                message: format!(
                    "The behavior assurance contract could not be parsed; action mapping remains narrative or inferred: {error}"
                ),
                scope: "actions".to_string(),
            });
            return BehaviorContract::default();
        }
    };
    if contract.schema != CONTRACT_SCHEMA {
        warnings.push(TelescopeWarning {
            code: "behavior-contract-legacy".to_string(),
            message: format!(
                "Behavior action mapping uses {} evidence; v2 behavior invariants remain the canonical contract.",
                if contract.schema.is_empty() {
                    "an unknown schema"
                } else {
                    contract.schema.as_str()
                }
            ),
            scope: "actions".to_string(),
        });
    }
    if contract.applicability == "not_applicable" {
        return BehaviorContract::default();
    }
    contract
}

fn build_telescope_actions(
    narrative: &TelescopeNarrative,
    behavior_contract: &BehaviorContract,
    nodes: &[TelescopeNode],
    edges: &[TelescopeEdge],
    flows: &[TelescopeFlow],
) -> (Vec<TelescopeAction>, TelescopeActionCoverage) {
    let mut actions = Vec::new();
    let mut covered_flow_ids = BTreeSet::new();
    let mut covered_behavior_ids = BTreeSet::new();

    for authored in &narrative.authored_actions {
        let (mut node_ids, nodes_complete) = resolve_action_nodes(authored, nodes);
        let flow = authored
            .flow_id
            .as_ref()
            .and_then(|flow_id| flows.iter().find(|flow| &flow.id == flow_id));
        if let Some(flow) = flow {
            covered_flow_ids.insert(flow.id.clone());
            for node_id in &flow.node_ids {
                push_unique(&mut node_ids, node_id);
            }
        }

        let mut edge_ids = authored
            .edge_ids
            .iter()
            .filter(|edge_id| edges.iter().any(|edge| &edge.id == *edge_id))
            .cloned()
            .collect::<Vec<_>>();
        if let Some(flow) = flow {
            for edge_id in &flow.edge_ids {
                push_unique(&mut edge_ids, edge_id);
            }
        }
        for edge in edges {
            if node_ids.contains(&edge.source) && node_ids.contains(&edge.target) {
                push_unique(&mut edge_ids, &edge.id);
            }
        }

        let mut source_anchors = Vec::new();
        for node in nodes.iter().filter(|node| node_ids.contains(&node.id)) {
            for anchor in &node.source_anchors {
                push_unique_anchor(&mut source_anchors, anchor);
            }
        }
        for path in &authored.files {
            if is_safe_relative_path(path) {
                push_unique_anchor(
                    &mut source_anchors,
                    &TelescopeAnchor {
                        path: path.clone(),
                        line: None,
                        symbol: None,
                        provenance: "authored-action".to_string(),
                    },
                );
            }
        }

        let files_complete = authored.files.iter().all(|path| {
            is_safe_relative_path(path)
                && nodes.iter().any(|node| {
                    node_source_paths(node)
                        .iter()
                        .any(|candidate| candidate == path)
                })
        });
        let flow_complete = authored.flow_id.is_none() || flow.is_some();
        let behavior = authored.behavior_id.as_ref().and_then(|behavior_id| {
            behavior_contract
                .behaviors
                .iter()
                .find(|candidate| &candidate.id == behavior_id)
        });
        if let Some(behavior_id) = &authored.behavior_id {
            if behavior.is_some() {
                covered_behavior_ids.insert(behavior_id.clone());
            }
        }
        let behavior_link_complete = authored.behavior_id.is_none() || behavior.is_some();
        let complete = nodes_complete
            && files_complete
            && flow_complete
            && behavior_link_complete
            && !node_ids.is_empty();
        let status = action_status(&authored.status, &narrative.status, complete);
        let confidence = if status == "reviewed" {
            "high"
        } else if status == "partial" || status == "stale" {
            "partial"
        } else {
            "medium"
        };
        actions.push(TelescopeAction {
            id: authored.id.clone(),
            label: if authored.label.is_empty() {
                format!("Action: {}", authored.id)
            } else {
                authored.label.clone()
            },
            verb: if authored.verb.is_empty() {
                "Inspect".to_string()
            } else {
                authored.verb.clone()
            },
            category: if authored.category.is_empty() {
                "workflow".to_string()
            } else {
                authored.category.clone()
            },
            what_it_does: if authored.what_it_does.is_empty() {
                "A reviewed action meaning has not been written yet.".to_string()
            } else {
                authored.what_it_does.clone()
            },
            how_its_built: if authored.how_its_built.is_empty() {
                "Implementation evidence is limited to the mapped source anchors.".to_string()
            } else {
                authored.how_its_built.clone()
            },
            node_ids,
            edge_ids,
            flow_id: flow
                .map(|flow| flow.id.clone())
                .or_else(|| authored.flow_id.clone()),
            behavior_id: authored.behavior_id.clone(),
            scenario_ids: if authored.scenario_ids.is_empty() {
                behavior
                    .map(|behavior| {
                        behavior
                            .scenarios
                            .iter()
                            .map(|scenario| scenario.id.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                authored.scenario_ids.clone()
            },
            behavior_state: if behavior.is_some() {
                "declared".to_string()
            } else if authored.behavior_id.is_some() {
                "unresolved".to_string()
            } else {
                "unprofiled".to_string()
            },
            behavior_verification: behavior
                .map(|behavior| {
                    if behavior.scenarios.is_empty() {
                        "no-scenarios".to_string()
                    } else {
                        behavior
                            .scenarios
                            .iter()
                            .map(|scenario| scenario.verification_level.as_str())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                })
                .or_else(|| {
                    authored
                        .behavior_id
                        .as_ref()
                        .map(|_| "behavior-not-found".to_string())
                })
                .unwrap_or_else(|| "not-profiled".to_string()),
            source_anchors,
            status,
            confidence: confidence.to_string(),
            provenance: "authored-action-inventory".to_string(),
            read_only: authored.read_only,
            guarded: authored.guarded,
        });
    }

    for behavior in &behavior_contract.behaviors {
        if behavior.id.is_empty() || covered_behavior_ids.contains(&behavior.id) {
            continue;
        }
        let (node_ids, source_anchors) = resolve_behavior_sources(behavior, nodes);
        let scenario_ids = behavior
            .scenarios
            .iter()
            .map(|scenario| scenario.id.clone())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        let verification = behavior
            .scenarios
            .iter()
            .map(|scenario| scenario.verification_level.as_str())
            .filter(|level| !level.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let invariant = behavior.invariants.first().cloned().unwrap_or_else(|| {
            "The behavior contract does not yet declare an invariant.".to_string()
        });
        actions.push(TelescopeAction {
            id: behavior.id.clone(),
            label: if behavior.title.is_empty() {
                format!("Review behavior {}", behavior.id)
            } else {
                behavior.title.clone()
            },
            verb: "Review".to_string(),
            category: format!("behavior-tier-{}", behavior.tier),
            what_it_does: format!("{} {}", behavior.title, invariant),
            how_its_built: format!(
                "Declared in the v2 behavior contract as a Tier {} {} behavior with {} scenario(s).",
                behavior.tier,
                if behavior.automation.is_empty() {
                    "manual"
                } else {
                    behavior.automation.as_str()
                },
                behavior.scenarios.len()
            ),
            node_ids,
            edge_ids: Vec::new(),
            flow_id: None,
            behavior_id: Some(behavior.id.clone()),
            scenario_ids,
            behavior_state: "declared".to_string(),
            behavior_verification: if verification.is_empty() {
                "not-profiled".to_string()
            } else {
                verification.join(", ")
            },
            source_anchors,
            status: "draft".to_string(),
            confidence: "medium".to_string(),
            provenance: "behavior-assurance-contract".to_string(),
            read_only: true,
            guarded: false,
        });
    }

    for flow in flows {
        if covered_flow_ids.contains(&flow.id) {
            continue;
        }
        let source_anchors = nodes
            .iter()
            .filter(|node| flow.node_ids.contains(&node.id))
            .flat_map(|node| node.source_anchors.clone())
            .collect::<Vec<_>>();
        actions.push(TelescopeAction {
            id: stable_id("action", &flow.id),
            label: format!("Trace {}", flow.label),
            verb: "Trace".to_string(),
            category: "flow".to_string(),
            what_it_does: format!(
                "Follows the {} path across {} entities and {} handoffs.",
                flow.label.to_lowercase(),
                flow.node_ids.len(),
                flow.edge_ids.len()
            ),
            how_its_built:
                "Inferred from a bounded static relationship walk; it is not a runtime trace."
                    .to_string(),
            node_ids: flow.node_ids.clone(),
            edge_ids: flow.edge_ids.clone(),
            flow_id: Some(flow.id.clone()),
            behavior_id: None,
            scenario_ids: Vec::new(),
            behavior_state: "unprofiled".to_string(),
            behavior_verification: "not-profiled".to_string(),
            source_anchors,
            status: "inferred".to_string(),
            confidence: "medium".to_string(),
            provenance: "static-relationship-walk".to_string(),
            read_only: true,
            guarded: false,
        });
    }

    if actions.is_empty() {
        for node in nodes
            .iter()
            .filter(|node| matches!(node.kind.as_str(), "route" | "entrypoint"))
            .take(8)
        {
            actions.push(TelescopeAction {
                id: stable_id("action", &format!("inspect:{}", node.id)),
                label: format!("Inspect {}", node.label),
                verb: "Inspect".to_string(),
                category: "inspect".to_string(),
                what_it_does: node.semantic_summary.clone(),
                how_its_built: node.implementation_summary.clone(),
                node_ids: vec![node.id.clone()],
                edge_ids: Vec::new(),
                flow_id: None,
                behavior_id: None,
                scenario_ids: Vec::new(),
                behavior_state: "unprofiled".to_string(),
                behavior_verification: "not-profiled".to_string(),
                source_anchors: node.source_anchors.clone(),
                status: "inferred".to_string(),
                confidence: "medium".to_string(),
                provenance: "static-entrypoint-inference".to_string(),
                read_only: true,
                guarded: false,
            });
        }
    }

    actions.sort_by(|left, right| {
        let left_authored = left.provenance == "authored-action-inventory";
        let right_authored = right.provenance == "authored-action-inventory";
        right_authored
            .cmp(&left_authored)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.id.cmp(&right.id))
    });

    let authored = actions
        .iter()
        .filter(|action| action.provenance == "authored-action-inventory")
        .count();
    let behavior_backed = actions
        .iter()
        .filter(|action| action.behavior_state == "declared")
        .count();
    let unprofiled = actions
        .iter()
        .filter(|action| action.behavior_state != "declared")
        .count();
    let inferred = actions.len().saturating_sub(authored);
    let partial = actions
        .iter()
        .filter(|action| matches!(action.status.as_str(), "partial" | "stale"))
        .count();
    let mapped = actions
        .iter()
        .filter(|action| !action.node_ids.is_empty())
        .count();
    let inventory_status = if actions.is_empty() {
        "missing"
    } else if partial > 0 {
        "partial"
    } else if authored > 0 {
        if actions.iter().all(|action| action.status == "reviewed") {
            "reviewed"
        } else {
            "draft"
        }
    } else {
        "inferred"
    };
    (
        actions,
        TelescopeActionCoverage {
            inventory_status: inventory_status.to_string(),
            total: authored + inferred,
            authored,
            inferred,
            partial,
            mapped,
            unmapped: authored + inferred - mapped,
            behavior_backed,
            unprofiled,
        },
    )
}
