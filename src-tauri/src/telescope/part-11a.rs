fn project_actors(
    narrative: &TelescopeNarrative,
    actions: &[TelescopeAction],
    nodes: &[TelescopeNode],
) -> Vec<TelescopeActor> {
    narrative
        .actors
        .iter()
        .map(|actor| TelescopeActor {
            id: actor.id.clone(),
            label: actor.label.clone(),
            role: actor.role.clone(),
            metaphor: actor.metaphor.clone(),
            description: actor.description.clone(),
            action_ids: actor
                .action_ids
                .iter()
                .filter(|id| actions.iter().any(|action| &action.id == *id))
                .cloned()
                .collect(),
            node_ids: actor
                .node_ids
                .iter()
                .flat_map(|id| {
                    nodes
                        .iter()
                        .filter(move |node| {
                            &node.id == id
                                || node.visual_building_id.as_deref() == Some(id.as_str())
                        })
                        .map(|node| node.id.clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            status: authored_status(&actor.status, &narrative.status),
            provenance: "authored-city-manifest".to_string(),
        })
        .collect()
}

fn project_payloads(
    narrative: &TelescopeNarrative,
    flows: &[TelescopeFlow],
) -> Vec<TelescopePayload> {
    narrative
        .payloads
        .iter()
        .map(|payload| TelescopePayload {
            id: payload.id.clone(),
            label: payload.label.clone(),
            metaphor: payload.metaphor.clone(),
            description: payload.description.clone(),
            flow_ids: payload
                .flow_ids
                .iter()
                .filter(|id| flows.iter().any(|flow| &flow.id == *id))
                .cloned()
                .collect(),
            data_shapes: payload.data_shapes.clone(),
            status: authored_status(&payload.status, &narrative.status),
            provenance: "authored-city-manifest".to_string(),
        })
        .collect()
}

fn project_scopes(
    groups: &[TelescopeGroup],
    nodes: &[TelescopeNode],
    edges: &[TelescopeEdge],
    flows: &[TelescopeFlow],
    actions: &[TelescopeAction],
) -> Vec<TelescopeScope> {
    let mut scopes = vec![TelescopeScope {
        id: "city-overview".to_string(),
        level: "overview".to_string(),
        label: "City overview".to_string(),
        purpose: "Explain the repository purpose, major boundaries, and primary story without routine implementation noise.".to_string(),
        group_ids: groups.iter().map(|group| group.id.clone()).collect(),
        node_ids: nodes
            .iter()
            .filter(|node| node.visual_building_id.is_some())
            .map(|node| node.id.clone())
            .collect(),
        edge_ids: flows
            .iter()
            .find(|flow| flow.primary)
            .map(|flow| flow.edge_ids.clone())
            .unwrap_or_default(),
        flow_ids: flows.iter().filter(|flow| flow.primary).map(|flow| flow.id.clone()).collect(),
    }];
    for group in groups {
        let node_ids = nodes
            .iter()
            .filter(|node| node.group_id == group.id)
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        scopes.push(TelescopeScope {
            id: format!("district:{}", group.id),
            level: "district".to_string(),
            label: group.label.clone(),
            purpose: format!(
                "Enter {} and inspect its responsibilities and boundary crossings.",
                group.label
            ),
            group_ids: vec![group.id.clone()],
            node_ids: node_ids.iter().cloned().collect(),
            edge_ids: edges
                .iter()
                .filter(|edge| node_ids.contains(&edge.source) || node_ids.contains(&edge.target))
                .map(|edge| edge.id.clone())
                .collect(),
            flow_ids: flows
                .iter()
                .filter(|flow| flow.node_ids.iter().any(|id| node_ids.contains(id)))
                .map(|flow| flow.id.clone())
                .collect(),
        });
    }
    for node in nodes
        .iter()
        .filter(|node| node.visual_building_id.is_some())
    {
        scopes.push(TelescopeScope {
            id: format!("building:{}", node.visual_building_id.as_deref().unwrap_or(&node.id)),
            level: "building".to_string(),
            label: node.label.clone(),
            purpose: "Inspect a local implementation map, symbols, files, evidence, and immediate handoffs.".to_string(),
            group_ids: vec![node.group_id.clone()],
            node_ids: vec![node.id.clone()],
            edge_ids: edges
                .iter()
                .filter(|edge| edge.source == node.id || edge.target == node.id)
                .map(|edge| edge.id.clone())
                .collect(),
            flow_ids: flows
                .iter()
                .filter(|flow| flow.node_ids.contains(&node.id))
                .map(|flow| flow.id.clone())
                .collect(),
        });
    }
    scopes.extend(actions.iter().map(|action| {
        TelescopeScope {
            id: format!("action:{}", action.id),
            level: "action".to_string(),
            label: action.label.clone(),
            purpose: action.what_it_does.clone(),
            group_ids: nodes
                .iter()
                .filter(|node| action.node_ids.contains(&node.id))
                .map(|node| node.group_id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            node_ids: action.node_ids.clone(),
            edge_ids: action.edge_ids.clone(),
            flow_ids: action.flow_id.clone().into_iter().collect(),
        }
    }));
    scopes
}
