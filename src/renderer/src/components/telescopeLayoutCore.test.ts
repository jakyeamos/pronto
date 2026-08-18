import { describe, expect, it } from "vitest";
import type { TelescopeProjection } from "../types/telescope";
import { computeTelescopeLayout } from "./telescopeLayoutCore";

const projection: TelescopeProjection = {
  schema_version: "pronto-telescope/v1",
  repository_id: "repository",
  repository_name: "Fixture",
  binding: {
    workspace_id: "workspace",
    branch: "dev",
    commit: "abc123",
    dirty: false,
    dirty_state_fingerprint: "clean",
    workspace_fingerprint: "workspace-1",
    generated_at: "2026-08-18T00:00:00Z",
  },
  freshness: { state: "fresh", cache: "miss", reason: "fixture" },
  coverage: {
    discovered_source_files: 2,
    examined_source_files: 2,
    supported_source_files: 2,
    partial_source_files: 0,
    skipped_large_files: 0,
    truncated: false,
    resolved_relationships: 1,
    inferred_relationships: 0,
    confidence: "high",
  },
  groups: [
    {
      id: "group-app",
      label: "Application",
      kind: "subsystem",
      summary: "Application",
      confidence: "high",
    },
  ],
  nodes: ["source", "target"].map((id) => ({
    id,
    group_id: "group-app",
    label: id,
    kind: "module",
    technology: "TypeScript",
    semantic_summary: id,
    implementation_summary: id,
    summary_status: "derived",
    confidence: "high",
    provenance: ["source"],
    source_anchors: [{ path: `src/${id}.ts`, provenance: "source" }],
    symbols: [],
    data_shapes: [],
  })),
  edges: [
    {
      id: "edge",
      source: "source",
      target: "target",
      kind: "imports",
      direction: "forward",
      label: "imports",
      confidence: "high",
      provenance: "resolved-static-import",
      inferred: false,
    },
  ],
  flows: [],
  actions: [],
  action_coverage: {
    inventory_status: "missing",
    total: 0,
    authored: 0,
    inferred: 0,
    partial: 0,
    mapped: 0,
    unmapped: 0,
  },
  warnings: [],
  enrichment: {
    enabled: false,
    source_content_transmitted: false,
    status: "disabled-by-default",
  },
};

describe("computeTelescopeLayout", () => {
  it("lays out compound groups and their child entities with stable graph ids", async () => {
    const result = await computeTelescopeLayout(projection);

    expect(result.nodes.map((node) => node.id).sort()).toEqual([
      "group-app",
      "source",
      "target",
    ]);
    expect(result.nodes.find((node) => node.id === "source")?.parentId).toBe(
      "group-app",
    );
    expect(
      result.nodes.every(
        (node) =>
          Number.isFinite(node.position.x) && Number.isFinite(node.position.y),
      ),
    ).toBe(true);
    expect(result.edges).toMatchObject([
      { id: "edge", source: "source", target: "target" },
    ]);
  });

  it("compacts collapsed groups while preserving the selectable group", async () => {
    const result = await computeTelescopeLayout(projection, ["group-app"]);

    expect(result.nodes.map((node) => node.id)).toEqual(["group-app"]);
    expect(result.edges).toEqual([]);
    expect(result.nodes[0].style).toMatchObject({ width: 260, height: 180 });
  });
});
