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
                narrative_status: "derived".to_string(),
                primary: flows.is_empty(),
            });
        }
    }
    flows
}
