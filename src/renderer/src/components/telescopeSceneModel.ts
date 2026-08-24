import type {
  TelescopeEdge,
  TelescopeFlow,
  TelescopeProjection,
} from "../types/telescope";
import {
  MAX_SOURCE_DETAIL_BUILDINGS,
  archetypeForNode,
  buildDistricts,
  collectBuckets,
  lowerConfidence,
  stableId,
  toBuilding,
} from "./telescopeSceneBuckets";

export type TelescopeSceneLevel = "overview" | "subsystems" | "source";
export type TelescopeArchetype =
  | "fin-row"
  | "tower"
  | "slab-stack"
  | "cube"
  | "low-slab";
export type TelescopeRailKind = "data" | "control" | "event" | "import";

export { archetypeForNode, MAX_SOURCE_DETAIL_BUILDINGS };

export interface TelescopeSceneDistrict {
  id: string;
  sourceGroupId: string;
  label: string;
  kind: string;
  summary: string;
  buildingIds: string[];
  sourceNodeCount: number;
  measuredLines: number;
  sourceFileCount: number;
  confidence: string;
  narrativeStatus: string;
  authored: boolean;
}

export interface TelescopeSceneBuilding {
  id: string;
  sourceGroupId: string;
  label: string;
  kind: string;
  technology: string;
  summary: string;
  implementationSummary: string;
  archetype: TelescopeArchetype;
  sourceNodeIds: string[];
  sourcePaths: string[];
  memberCount: number;
  measuredLines: number;
  sourceFileCount: number;
  confidence: string;
  narrativeStatus: string;
  authored: boolean;
  cityRole: string;
  actorLabels: string[];
  payloadLabels: string[];
}

export interface TelescopeSceneRail {
  id: string;
  sourceBuildingId: string;
  targetBuildingId: string;
  label: string;
  railKind: TelescopeRailKind;
  confidence: string;
  inferred: boolean;
  sourceEdgeIds: string[];
  tokenLabel: string;
  uncertain: boolean;
}

export interface TelescopeSceneModel {
  level: TelescopeSceneLevel;
  districts: TelescopeSceneDistrict[];
  buildings: TelescopeSceneBuilding[];
  rails: TelescopeSceneRail[];
  primaryFlowId: string | null;
  primaryRailIds: string[];
  primaryBuildingIds: string[];
  clusteredSourceNodeCount: number;
  sourceDetailBuildingLimit: number;
  scopedSourceNodeCount: number;
  hiddenSourceNodeCount: number;
}

export interface TelescopeSceneScope {
  selectedGroupId?: string | null;
  selectedNodeIds?: string[];
}

export function buildTelescopeScene(
  projection: TelescopeProjection,
  level: TelescopeSceneLevel = "overview",
  scope: TelescopeSceneScope = {},
): TelescopeSceneModel {
  const primaryFlowId =
    projection.narrative?.primary_flow_id ??
    projection.flows.find((flow) => flow.primary)?.id ??
    projection.flows[0]?.id ??
    null;
  const primaryFlow = primaryFlowId
    ? projection.flows.find((flow) => flow.id === primaryFlowId)
    : undefined;
  const scopedNodeIds = sourceNodeScope(projection, level, scope, primaryFlow);
  const buckets = collectBuckets(
    projection,
    level,
    new Set(primaryFlow?.node_ids ?? []),
    scopedNodeIds,
  );
  const buildings = buckets.map((bucket) => toBuilding(bucket, projection));
  const buildingBySourceNode = new Map<string, string>();
  for (const building of buildings) {
    for (const sourceNodeId of building.sourceNodeIds) {
      buildingBySourceNode.set(sourceNodeId, building.id);
    }
  }
  const rails = buildRails(projection.edges, buildingBySourceNode);
  const districts = buildDistricts(
    projection.groups,
    buildings,
    projection.nodes,
  );
  const primaryRailIds = primaryFlow
    ? rails
        .filter((rail) =>
          rail.sourceEdgeIds.some((edgeId) =>
            primaryFlow.edge_ids.includes(edgeId),
          ),
        )
        .map((rail) => rail.id)
    : [];
  const primaryBuildingIds = primaryFlow
    ? primaryFlow.node_ids
        .map((nodeId) => buildingBySourceNode.get(nodeId))
        .filter((id): id is string => Boolean(id))
        .filter((id, index, values) => values.indexOf(id) === index)
    : [];

  return {
    level,
    districts,
    buildings,
    rails,
    primaryFlowId,
    primaryRailIds,
    primaryBuildingIds,
    clusteredSourceNodeCount:
      level === "source"
        ? buckets
            .filter((bucket) => bucket.key.startsWith("source-cluster|"))
            .reduce((total, bucket) => total + bucket.nodes.length, 0)
        : 0,
    sourceDetailBuildingLimit: MAX_SOURCE_DETAIL_BUILDINGS,
    scopedSourceNodeCount: scopedNodeIds.size,
    hiddenSourceNodeCount: Math.max(
      0,
      projection.nodes.length - scopedNodeIds.size,
    ),
  };
}

function sourceNodeScope(
  projection: TelescopeProjection,
  level: TelescopeSceneLevel,
  scope: TelescopeSceneScope,
  primaryFlow: TelescopeFlow | undefined,
): Set<string> {
  if (level === "overview") {
    return new Set(projection.nodes.map((node) => node.id));
  }

  const selected = new Set(scope.selectedNodeIds ?? []);
  const selectedGroupId =
    scope.selectedGroupId ??
    projection.nodes.find((node) => selected.has(node.id))?.group_id ??
    null;

  if (level === "subsystems") {
    const districtNodes = new Set(
      projection.nodes
        .filter((node) => node.group_id === selectedGroupId)
        .map((node) => node.id),
    );
    if (districtNodes.size === 0) {
      for (const nodeId of primaryFlow?.node_ids ?? []) {
        districtNodes.add(nodeId);
      }
    }
    const districtSeed = new Set(districtNodes);
    for (const edge of projection.edges) {
      if (districtSeed.has(edge.source)) districtNodes.add(edge.target);
      if (districtSeed.has(edge.target)) districtNodes.add(edge.source);
    }
    return districtNodes;
  }

  if (selected.size === 0) {
    const fallback = primaryFlow?.node_ids[0] ?? projection.nodes[0]?.id;
    if (fallback) selected.add(fallback);
  }
  const local = new Set(selected);
  for (const edge of projection.edges) {
    if (selected.has(edge.source)) local.add(edge.target);
    if (selected.has(edge.target)) local.add(edge.source);
  }
  return local;
}

function buildRails(
  edges: TelescopeEdge[],
  buildingBySourceNode: Map<string, string>,
): TelescopeSceneRail[] {
  const grouped = new Map<string, TelescopeSceneRail>();
  for (const edge of edges) {
    const sourceBuildingId = buildingBySourceNode.get(edge.source);
    const targetBuildingId = buildingBySourceNode.get(edge.target);
    if (
      !sourceBuildingId ||
      !targetBuildingId ||
      sourceBuildingId === targetBuildingId
    ) {
      continue;
    }
    const railKind = normalizeRailKind(edge.rail_kind, edge.kind);
    const key = sourceBuildingId + "|" + targetBuildingId + "|" + railKind;
    const current = grouped.get(key);
    if (current) {
      current.sourceEdgeIds.push(edge.id);
      current.inferred ||= edge.inferred;
      current.uncertain ||= edge.inferred || edge.confidence !== "high";
      current.confidence = lowerConfidence(current.confidence, edge.confidence);
      continue;
    }
    grouped.set(key, {
      id: "rail-" + stableId(key),
      sourceBuildingId,
      targetBuildingId,
      label: edge.label,
      railKind,
      confidence: edge.confidence,
      inferred: edge.inferred,
      sourceEdgeIds: [edge.id],
      tokenLabel: tokenLabelForEdge(edge),
      uncertain: edge.inferred || edge.confidence !== "high",
    });
  }
  return [...grouped.values()]
    .map((rail) => ({
      ...rail,
      sourceEdgeIds: [...rail.sourceEdgeIds].sort(),
    }))
    .sort((left, right) => left.id.localeCompare(right.id));
}

function normalizeRailKind(
  railKind: string | undefined,
  relationshipKind: string,
): TelescopeRailKind {
  if (railKind === "data" || railKind === "control" || railKind === "event") {
    return railKind;
  }
  if (relationshipKind === "dynamic") return "event";
  if (relationshipKind === "uses" || relationshipKind === "contains") {
    return "control";
  }
  return "import";
}

function tokenLabelForEdge(edge: TelescopeEdge): string {
  const candidate = edge.label
    .replace(/^(imports|uses|contains|loads at runtime)\s*/i, "")
    .trim();
  if (candidate && candidate.length <= 28) return candidate;
  if (edge.kind === "dynamic") return "event";
  if (edge.kind === "uses") return "control";
  return "payload";
}

export function sourceNodeIdsForSceneBuilding(
  scene: TelescopeSceneModel,
  buildingId: string,
): string[] {
  return (
    scene.buildings.find((building) => building.id === buildingId)
      ?.sourceNodeIds ?? []
  );
}

export function sourceEdgeIdsForSceneRail(
  scene: TelescopeSceneModel,
  railId: string,
): string[] {
  return scene.rails.find((rail) => rail.id === railId)?.sourceEdgeIds ?? [];
}

export function firstFlowForScene(
  projection: TelescopeProjection,
  scene: TelescopeSceneModel,
): TelescopeFlow | null {
  return (
    projection.flows.find((flow) => flow.id === scene.primaryFlowId) ??
    projection.flows[0] ??
    null
  );
}
