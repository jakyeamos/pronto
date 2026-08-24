fn resolve_behavior_sources(
    behavior: &BehaviorContractBehavior,
    nodes: &[TelescopeNode],
) -> (Vec<String>, Vec<TelescopeAnchor>) {
    let mut node_ids = Vec::new();
    let mut source_anchors = Vec::new();
    for path in &behavior.change_triggers {
        if !is_safe_relative_path(path) {
            continue;
        }
        let matching_nodes = nodes
            .iter()
            .filter(|node| {
                node_source_paths(node)
                    .iter()
                    .any(|candidate| candidate == path)
            })
            .collect::<Vec<_>>();
        for node in matching_nodes {
            push_unique(&mut node_ids, &node.id);
            for anchor in &node.source_anchors {
                push_unique_anchor(&mut source_anchors, anchor);
            }
        }
        push_unique_anchor(
            &mut source_anchors,
            &TelescopeAnchor {
                path: path.clone(),
                line: None,
                symbol: None,
                provenance: "behavior-change-trigger".to_string(),
            },
        );
    }
    (node_ids, source_anchors)
}

fn resolve_action_nodes(
    action: &TelescopeNarrativeAction,
    nodes: &[TelescopeNode],
) -> (Vec<String>, bool) {
    let mut node_ids = Vec::new();
    let mut complete = true;
    for requested_id in &action.node_ids {
        let matches = nodes
            .iter()
            .filter(|node| {
                node.id == *requested_id
                    || node.visual_building_id.as_deref() == Some(requested_id.as_str())
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if matches.is_empty() {
            complete = false;
        }
        for node_id in matches {
            push_unique(&mut node_ids, &node_id);
        }
    }
    for path in &action.files {
        let matches = nodes
            .iter()
            .filter(|node| {
                node_source_paths(node)
                    .iter()
                    .any(|candidate| candidate == path)
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if matches.is_empty() {
            complete = false;
        }
        for node_id in matches {
            push_unique(&mut node_ids, &node_id);
        }
    }
    (node_ids, complete)
}

fn action_status(authored_status: &str, narrative_status: &str, complete: bool) -> String {
    if !complete {
        return "partial".to_string();
    }
    if narrative_status == "stale" {
        return "stale".to_string();
    }
    if authored_status == "reviewed" && narrative_status == "reviewed" {
        "reviewed".to_string()
    } else {
        "draft".to_string()
    }
}

fn push_unique_anchor(anchors: &mut Vec<TelescopeAnchor>, anchor: &TelescopeAnchor) {
    if !anchors.iter().any(|candidate| {
        candidate.path == anchor.path
            && candidate.line == anchor.line
            && candidate.symbol == anchor.symbol
    }) {
        anchors.push(anchor.clone());
    }
}
