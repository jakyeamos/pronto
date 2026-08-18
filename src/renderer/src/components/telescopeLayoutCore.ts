import * as ElkApiModule from "elkjs/lib/elk-api.js";
import type { ElkNode } from "elkjs/lib/elk-api";
import ElkAlgorithmWorker from "elkjs/lib/elk-worker.min.js?worker";
import type { Edge, Node } from "@xyflow/react";
import type { TelescopeProjection } from "../types/telescope";
import type { TelescopeLayout } from "./telescopeLayout";

const NODE_WIDTH = 184;
const NODE_HEIGHT = 78;

type ElkConstructor = new (options: { workerFactory: () => Worker }) => {
  layout: (graph: ElkNode) => Promise<ElkNode>;
};

function resolveElkConstructor(): ElkConstructor {
  // elk-api is CommonJS and can arrive as `default`, `default.default`, or the
  // namespace itself depending on the Node/Vite/WebKit boundary.
  const namespace = ElkApiModule as unknown as {
    default?: unknown;
  };
  const firstDefault = namespace.default;
  const nestedDefault =
    firstDefault && typeof firstDefault === "object"
      ? (firstDefault as { default?: unknown }).default
      : undefined;
  const candidate = [firstDefault, nestedDefault, namespace].find(
    (value) => typeof value === "function",
  );
  if (!candidate) {
    throw new TypeError("ELK API constructor is unavailable.");
  }
  return candidate as ElkConstructor;
}

interface TelescopeNodeData extends Record<string, unknown> {
  telescopeId: string;
  label: string;
  kind: string;
  technology: string;
  confidence: string;
}

export async function computeTelescopeLayout(
  projection: TelescopeProjection,
  collapsedGroupIds: string[] = [],
): Promise<TelescopeLayout> {
  const collapsedGroups = new Set(collapsedGroupIds);
  const visibleNodeIds = new Set(
    projection.nodes
      .filter((node) => !collapsedGroups.has(node.group_id))
      .map((node) => node.id),
  );
  const graph = {
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "RIGHT",
      "elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "elk.edgeRouting": "ORTHOGONAL",
      "elk.spacing.nodeNode": "42",
      "elk.layered.spacing.nodeNodeBetweenLayers": "86",
      "elk.padding": "[top=42,left=42,bottom=42,right=42]",
    },
    children: projection.groups.map((group) => ({
      id: group.id,
      width: 260,
      height: 180,
      layoutOptions: {
        "elk.algorithm": "layered",
        "elk.direction": "DOWN",
        "elk.padding": "[top=48,left=24,bottom=24,right=24]",
        "elk.spacing.nodeNode": "24",
      },
      children: projection.nodes
        .filter(
          (node) =>
            node.group_id === group.id && !collapsedGroups.has(group.id),
        )
        .map((node) => ({
          id: node.id,
          width: NODE_WIDTH,
          height: NODE_HEIGHT,
        })),
    })),
    edges: projection.edges
      .filter(
        (edge) =>
          visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target),
      )
      .map((edge) => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
  };

  let result: ElkNode;
  try {
    const Elk = resolveElkConstructor();
    result = await new Elk({
      workerFactory: () =>
        new ElkAlgorithmWorker({ name: "pronto-telescope-elk" }),
    }).layout(graph);
  } catch (error) {
    return computeGridFallbackLayout(
      projection,
      collapsedGroupIds,
      `ELK layout unavailable; using deterministic fallback: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  const nodes: Array<Node<TelescopeNodeData>> = [];
  let groupX = 0;
  let groupY = 0;
  let groupRowHeight = 0;
  const maxGroupRowWidth = 1_480;
  for (const groupResult of result.children ?? []) {
    const group = projection.groups.find(
      (candidate) => candidate.id === groupResult.id,
    );
    if (!group) continue;
    const groupWidth = groupResult.width ?? 260;
    const groupHeight = groupResult.height ?? 180;
    if (groupX > 0 && groupX + groupWidth > maxGroupRowWidth) {
      groupX = 0;
      groupY += groupRowHeight + 84;
      groupRowHeight = 0;
    }
    nodes.push({
      id: group.id,
      type: "telescopeGroup",
      // ELK owns each compound group's internal hierarchy. Disconnected
      // top-level subsystems are packed into stable rows so fit-to-view does
      // not shrink a repository into an unreadable vertical strip.
      position: { x: groupX, y: groupY },
      style: {
        width: groupWidth,
        height: groupHeight,
      },
      selectable: true,
      data: {
        telescopeId: group.id,
        label: group.label,
        kind: group.kind,
        technology: "",
        confidence: group.confidence,
      },
    });
    for (const child of groupResult.children ?? []) {
      const source = projection.nodes.find(
        (candidate) => candidate.id === child.id,
      );
      if (!source) continue;
      nodes.push({
        id: source.id,
        type: "telescopeEntity",
        parentId: group.id,
        extent: "parent",
        position: { x: child.x ?? 24, y: child.y ?? 48 },
        style: { width: NODE_WIDTH, height: NODE_HEIGHT },
        data: {
          telescopeId: source.id,
          label: source.label,
          kind: source.kind,
          technology: source.technology,
          confidence: source.confidence,
        },
      });
    }
    groupX += groupWidth + 84;
    groupRowHeight = Math.max(groupRowHeight, groupHeight);
  }
  const edges: Edge[] = projection.edges
    .filter(
      (edge) =>
        visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target),
    )
    .map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "telescopeFlow",
      data: {
        telescopeId: edge.id,
        label: edge.label,
        confidence: edge.confidence,
        inferred: edge.inferred,
      },
    }));
  return { nodes, edges, engine: "elk" };
}

function computeGridFallbackLayout(
  projection: TelescopeProjection,
  collapsedGroupIds: string[] = [],
  warning = "ELK layout unavailable; using deterministic fallback.",
): TelescopeLayout {
  const collapsedGroups = new Set(collapsedGroupIds);
  const visibleNodeIds = new Set(
    projection.nodes
      .filter((node) => !collapsedGroups.has(node.group_id))
      .map((node) => node.id),
  );
  const nodes: Array<Node<TelescopeNodeData>> = [];
  let cursorX = 0;
  let cursorY = 0;
  let rowHeight = 0;
  const maxRowWidth = 1_480;

  for (const group of projection.groups) {
    const children = projection.nodes.filter(
      (node) => node.group_id === group.id && !collapsedGroups.has(group.id),
    );
    const columns = Math.max(
      1,
      Math.min(4, Math.ceil(Math.sqrt(children.length))),
    );
    const rows = Math.max(1, Math.ceil(children.length / columns));
    const width = collapsedGroups.has(group.id)
      ? 260
      : Math.max(260, columns * (NODE_WIDTH + 24) + 24);
    const height = collapsedGroups.has(group.id)
      ? 180
      : Math.max(180, rows * (NODE_HEIGHT + 24) + 72);
    if (cursorX > 0 && cursorX + width > maxRowWidth) {
      cursorX = 0;
      cursorY += rowHeight + 84;
      rowHeight = 0;
    }
    nodes.push({
      id: group.id,
      type: "telescopeGroup",
      position: { x: cursorX, y: cursorY },
      style: { width, height },
      selectable: true,
      data: {
        telescopeId: group.id,
        label: group.label,
        kind: group.kind,
        technology: "",
        confidence: group.confidence,
      },
    });
    children.forEach((source, index) => {
      nodes.push({
        id: source.id,
        type: "telescopeEntity",
        parentId: group.id,
        extent: "parent",
        position: {
          x: 24 + (index % columns) * (NODE_WIDTH + 24),
          y: 56 + Math.floor(index / columns) * (NODE_HEIGHT + 24),
        },
        style: { width: NODE_WIDTH, height: NODE_HEIGHT },
        data: {
          telescopeId: source.id,
          label: source.label,
          kind: source.kind,
          technology: source.technology,
          confidence: source.confidence,
        },
      });
    });
    cursorX += width + 84;
    rowHeight = Math.max(rowHeight, height);
  }

  const edges: Edge[] = projection.edges
    .filter(
      (edge) =>
        visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target),
    )
    .map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "telescopeFlow",
      data: {
        telescopeId: edge.id,
        label: edge.label,
        confidence: edge.confidence,
        inferred: edge.inferred,
      },
    }));
  return { nodes, edges, engine: "grid-fallback", warning };
}
