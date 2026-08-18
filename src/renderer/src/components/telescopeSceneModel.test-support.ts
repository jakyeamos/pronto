import type {
  TelescopeEdge,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";

export function makeTelescopeSceneProjection(count = 8): TelescopeProjection {
  const groups = Array.from({ length: 4 }, (_, index) => ({
    id: "group-" + index,
    label: "District " + index,
    kind: "subsystem",
    summary: "Measured district " + index,
    confidence: "high",
    measured_lines: 100 + index,
    source_file_count: 2,
  }));
  const kinds = ["route", "interface", "store", "integration", "service"];
  const nodes: TelescopeNode[] = Array.from({ length: count }, (_, index) => {
    const kind = kinds[index % kinds.length];
    return {
      id: "node-" + index,
      group_id: groups[index % groups.length].id,
      label: "Building " + index,
      kind,
      technology: index % 2 ? "Rust" : "TypeScript",
      semantic_summary: "Explains building " + index,
      implementation_summary: "Implements building " + index,
      summary_status: "derived",
      confidence: "high",
      provenance: ["static-source"],
      source_anchors: [
        {
          path: "src/building-" + index + ".ts",
          line: 10 + index,
          provenance: "symbol",
        },
      ],
      symbols: ["building" + index],
      data_shapes: ["Payload" + index],
      source_paths: ["src/building-" + index + ".ts"],
      measured_lines: 40 + index,
      source_file_count: 1,
      visual_building_id: "authored-building-" + index,
      visual_archetype: kind === "route" ? "tower" : undefined,
      visual_override_provenance: "authored-manifest",
      narrative_status: "reviewed",
    };
  });
  const edges: TelescopeEdge[] = nodes.slice(1).map((node, index) => ({
    id: "edge-" + index,
    source: nodes[index].id,
    target: node.id,
    kind: index % 2 ? "uses" : "import",
    direction: "forward",
    label: index % 2 ? "calls" : "imports",
    confidence: "high",
    provenance: "static-import",
    inferred: false,
    rail_kind: index % 2 ? "control" : "import",
  }));
  return {
    schema_version: "pronto-telescope/v1",
    repository_id: "repo-scene",
    repository_name: "scene-fixture",
    binding: {
      workspace_id: "workspace-scene",
      branch: "codex/scene",
      commit: "0123456789abcdef",
      dirty: false,
      dirty_state_fingerprint: "clean",
      workspace_fingerprint: "workspace-scene",
      generated_at: "2026-08-18T16:00:00Z",
    },
    freshness: { state: "fresh", cache: "miss", reason: "fixture" },
    coverage: {
      discovered_source_files: count,
      examined_source_files: count,
      supported_source_files: count,
      partial_source_files: 0,
      skipped_large_files: 0,
      truncated: false,
      resolved_relationships: edges.length,
      inferred_relationships: 0,
      confidence: "high",
    },
    groups,
    nodes,
    edges,
    flows: [
      {
        id: "flow-primary",
        label: "Primary story",
        kind: "data",
        node_ids: nodes.map((node) => node.id),
        edge_ids: edges.map((edge) => edge.id),
        data_shape: "Repository payload",
        confidence: "high",
        provenance: "authored-manifest",
        primary: true,
      },
    ],
    actions: [],
    action_coverage: {
      inventory_status: "missing",
      total: 0,
      authored: 0,
      inferred: 0,
      partial: 0,
      mapped: 0,
      unmapped: 0,
      behavior_backed: 0,
      unprofiled: 0,
    },
    warnings: [],
    enrichment: {
      enabled: false,
      source_content_transmitted: false,
      status: "disabled",
    },
    narrative: {
      status: "reviewed",
      primary_flow_id: "flow-primary",
      visual_model_version: "pronto-telescope-city/v1",
    },
  };
}
