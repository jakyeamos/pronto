import type { Edge, Node } from "@xyflow/react";
import type { SyntheticEvent } from "react";
import type { TelescopeLayout } from "./telescopeLayout";
import type {
  TelescopeArchetype,
  TelescopeSceneBuilding,
  TelescopeSceneModel,
} from "./telescopeSceneModel";

export interface TelescopeSceneNodeData extends Record<string, unknown> {
  telescopeId: string;
  label: string;
  kind: string;
  technology: string;
  confidence: string;
  archetype?: TelescopeArchetype;
  memberCount?: number;
  measuredLines?: number;
  sourceFileCount?: number;
  sourceNodeIds?: string[];
  sourcePaths?: string[];
  narrativeStatus?: string;
  authored?: boolean;
  cityRole?: string;
  actorLabels?: string[];
  payloadLabels?: string[];
  district?: boolean;
  tone?: string;
  dimmed?: boolean;
  selected?: boolean;
  filtered?: boolean;
}

export interface TelescopeSceneEdgeData extends Record<string, unknown> {
  telescopeId: string;
  label: string;
  confidence: string;
  railKind: string;
  sourceEdgeIds: string[];
  tokenLabel: string;
  uncertain: boolean;
  selected?: boolean;
  dimmed?: boolean;
  activeToken?: boolean;
  paused?: boolean;
  reducedMotion?: boolean;
  onSelectToken?: (event: SyntheticEvent<SVGCircleElement>) => void;
}

interface SceneRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const DISTRICT_PADDING = 46;
const BUILDING_GAP = 34;
const DISTRICT_GAP = 86;
const MAX_DISTRICT_WIDTH = 820;

export function computeTelescopeSceneLayout(
  scene: TelescopeSceneModel,
): TelescopeLayout {
  const nodes: Array<Node<TelescopeSceneNodeData>> = [];
  let cursorX = 0;
  let cursorY = 0;
  let rowHeight = 0;

  for (const district of scene.districts) {
    const buildings = district.buildingIds
      .map((id) => scene.buildings.find((building) => building.id === id))
      .filter((building): building is TelescopeSceneBuilding =>
        Boolean(building),
      );
    const placements = placeBuildings(buildings);
    const width = Math.max(
      360,
      Math.min(
        MAX_DISTRICT_WIDTH,
        Math.max(
          ...placements.map((placement) => placement.x + placement.width),
          0,
        ) + DISTRICT_PADDING,
      ),
    );
    const height = Math.max(
      260,
      Math.max(
        ...placements.map((placement) => placement.y + placement.height),
        0,
      ) + DISTRICT_PADDING,
    );
    if (
      cursorX > 0 &&
      cursorX + width > MAX_DISTRICT_WIDTH * 2 + DISTRICT_GAP
    ) {
      cursorX = 0;
      cursorY += rowHeight + DISTRICT_GAP;
      rowHeight = 0;
    }
    const rect = { x: cursorX, y: cursorY, width, height };
    nodes.push({
      id: district.id,
      type: "telescopeGroup",
      position: { x: rect.x, y: rect.y },
      style: { width: rect.width, height: rect.height },
      selectable: true,
      data: {
        telescopeId: district.sourceGroupId,
        label: district.label,
        kind: district.kind,
        technology: "",
        confidence: district.confidence,
        memberCount: district.sourceNodeCount,
        measuredLines: district.measuredLines,
        sourceFileCount: district.sourceFileCount,
        narrativeStatus: district.narrativeStatus,
        authored: district.authored,
        district: true,
      },
    });
    placements.forEach((placement) => {
      const building = placement.building;
      nodes.push({
        id: building.id,
        type: "telescopeEntity",
        parentId: district.id,
        extent: "parent",
        position: {
          x: placement.x,
          y: placement.y,
        },
        style: {
          width: placement.width,
          height: placement.height,
        },
        data: {
          telescopeId: building.id,
          label: building.label,
          kind: building.kind,
          technology: building.technology,
          confidence: building.confidence,
          archetype: building.archetype,
          memberCount: building.memberCount,
          measuredLines: building.measuredLines,
          sourceFileCount: building.sourceFileCount,
          sourceNodeIds: building.sourceNodeIds,
          sourcePaths: building.sourcePaths,
          narrativeStatus: building.narrativeStatus,
          authored: building.authored,
          cityRole: building.cityRole,
          actorLabels: building.actorLabels,
          payloadLabels: building.payloadLabels,
        },
      });
    });
    cursorX += width + DISTRICT_GAP;
    rowHeight = Math.max(rowHeight, height);
  }

  const edges: Array<Edge<TelescopeSceneEdgeData>> = scene.rails.map(
    (rail, index) => ({
      id: rail.id,
      source: rail.sourceBuildingId,
      target: rail.targetBuildingId,
      type: "telescopeFlow",
      data: {
        telescopeId: rail.id,
        label: rail.label,
        confidence: rail.confidence,
        railKind: rail.railKind,
        sourceEdgeIds: rail.sourceEdgeIds,
        tokenLabel: rail.tokenLabel,
        uncertain: rail.uncertain,
        railIndex: index,
      },
    }),
  );
  return {
    nodes,
    edges,
    engine: "grid-fallback",
    warning:
      "City scene uses deterministic dimetric packing; source topology remains workspace-bound.",
  };
}

export function dimensionsForArchetype(
  archetype: TelescopeArchetype | undefined,
): { width: number; height: number } {
  switch (archetype) {
    case "tower":
      return { width: 154, height: 166 };
    case "fin-row":
      return { width: 206, height: 126 };
    case "slab-stack":
      return { width: 188, height: 142 };
    case "low-slab":
      return { width: 174, height: 106 };
    default:
      return { width: 164, height: 122 };
  }
}

export function routeRailWaypoints(
  source: { x: number; y: number },
  target: { x: number; y: number },
  lane = 0,
): Array<{ x: number; y: number }> {
  const direction = target.x >= source.x ? 1 : -1;
  const separation = Math.max(52, Math.abs(target.x - source.x) * 0.32);
  const laneOffset = (lane % 3) * 16;
  const sourceBend = source.x + direction * separation + laneOffset;
  const targetBend = target.x - direction * separation - laneOffset;
  const bend = (sourceBend + targetBend) / 2;
  return [
    source,
    { x: sourceBend, y: source.y },
    { x: bend, y: source.y + (target.y - source.y) / 2 },
    { x: targetBend, y: target.y },
    target,
  ];
}

function placeBuildings(
  buildings: TelescopeSceneBuilding[],
): Array<SceneRect & { building: TelescopeSceneBuilding }> {
  const placements: Array<SceneRect & { building: TelescopeSceneBuilding }> =
    [];
  buildings.forEach((building, index) => {
    const dimensions = dimensionsForArchetype(building.archetype);
    const column = index % 3;
    const row = Math.floor(index / 3);
    const desired = {
      x: DISTRICT_PADDING + column * 238 - row * 112,
      y: DISTRICT_PADDING + (column + row) * 76,
      width: dimensions.width,
      height: dimensions.height,
      building,
    };
    desired.x -= Math.min(0, desired.x);
    desired.y -= Math.min(0, desired.y);
    while (
      placements.some((placement) =>
        rectanglesOverlap(desired, placement, BUILDING_GAP),
      )
    ) {
      desired.x += 28;
    }
    placements.push(desired);
  });
  const minX = Math.min(...placements.map((placement) => placement.x), 0);
  const minY = Math.min(...placements.map((placement) => placement.y), 0);
  return placements.map((placement) => ({
    ...placement,
    x: placement.x - minX,
    y: placement.y - minY,
  }));
}

function rectanglesOverlap(
  left: SceneRect,
  right: SceneRect,
  gap: number,
): boolean {
  return !(
    left.x + left.width + gap <= right.x ||
    right.x + right.width + gap <= left.x ||
    left.y + left.height + gap <= right.y ||
    right.y + right.height + gap <= left.y
  );
}
