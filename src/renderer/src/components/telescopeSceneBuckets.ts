import type {
  TelescopeGroup,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";
import type {
  TelescopeArchetype,
  TelescopeSceneBuilding,
  TelescopeSceneDistrict,
  TelescopeSceneLevel,
} from "./telescopeSceneModel";
interface BuildingBucket {
  key: string;
  group: TelescopeGroup;
  nodes: TelescopeNode[];
  authored: boolean;
  label?: string;
  summary?: string;
  archetype?: TelescopeArchetype;
}

export const MAX_OVERVIEW_BUILDINGS = 24;
const MAX_BUILDINGS_PER_DISTRICT = 6;
export const MAX_SOURCE_DETAIL_BUILDINGS = 96;

export function archetypeForNode(
  node: Pick<TelescopeNode, "kind"> & {
    visual_archetype?: string;
  },
): TelescopeArchetype {
  if (node.visual_archetype && isArchetype(node.visual_archetype)) {
    return node.visual_archetype;
  }
  if (node.kind === "route" || node.kind === "entrypoint") return "tower";
  if (node.kind === "store") return "slab-stack";
  if (node.kind === "interface") return "fin-row";
  if (node.kind === "integration") return "low-slab";
  return "cube";
}
export function collectBuckets(
  projection: TelescopeProjection,
  level: TelescopeSceneLevel,
  primaryNodeIds: Set<string>,
): BuildingBucket[] {
  const groups = new Map(projection.groups.map((group) => [group.id, group]));
  const sourceBuckets = new Map<string, BuildingBucket>();
  const groupNodes = new Map<string, TelescopeNode[]>();
  for (const node of projection.nodes) {
    const nodes = groupNodes.get(node.group_id) ?? [];
    nodes.push(node);
    groupNodes.set(node.group_id, nodes);
  }

  for (const [groupId, nodes] of groupNodes) {
    const group = groups.get(groupId);
    if (!group) continue;
    for (const node of nodes) {
      const authoredId = node.visual_building_id;
      const bucketKey =
        level === "source"
          ? "source|" + node.id
          : authoredId
            ? "authored|" + authoredId
            : "kind|" + groupId + "|" + kindBucket(node.kind);
      const bucket = sourceBuckets.get(bucketKey);
      if (bucket) {
        bucket.nodes.push(node);
      } else {
        sourceBuckets.set(bucketKey, {
          key: bucketKey,
          group,
          nodes: [node],
          authored: Boolean(authoredId),
        });
      }
    }
  }

  let buckets = [...sourceBuckets.values()];
  if (level === "overview" && buckets.length > MAX_OVERVIEW_BUILDINGS) {
    buckets = compactOverviewBuckets(buckets);
  }
  if (level === "source" && buckets.length > MAX_SOURCE_DETAIL_BUILDINGS) {
    buckets = compactSourceBuckets(buckets, primaryNodeIds);
  }
  return buckets.sort((left, right) => left.key.localeCompare(right.key));
}

function compactSourceBuckets(
  buckets: BuildingBucket[],
  primaryNodeIds: Set<string>,
): BuildingBucket[] {
  const groupIds = [
    ...new Set(buckets.map((bucket) => bucket.group.id)),
  ].sort();
  const individualBudget = Math.max(
    0,
    MAX_SOURCE_DETAIL_BUILDINGS - groupIds.length,
  );
  const ordered = [...buckets].sort(
    (left, right) =>
      Number(right.nodes.some((node) => primaryNodeIds.has(node.id))) -
        Number(left.nodes.some((node) => primaryNodeIds.has(node.id))) ||
      Number(right.authored) - Number(left.authored) ||
      measuredLinesForBucket(right) - measuredLinesForBucket(left) ||
      left.key.localeCompare(right.key),
  );
  const retained = ordered.slice(0, individualBudget);
  const retainedKeys = new Set(retained.map((bucket) => bucket.key));
  const remainderByGroup = new Map<string, BuildingBucket[]>();
  for (const bucket of ordered) {
    if (retainedKeys.has(bucket.key)) continue;
    const remainder = remainderByGroup.get(bucket.group.id) ?? [];
    remainder.push(bucket);
    remainderByGroup.set(bucket.group.id, remainder);
  }
  const clusters = [...remainderByGroup.entries()].map(
    ([groupId, groupBuckets]) => {
      const group = groupBuckets[0].group;
      const nodes = groupBuckets.flatMap((bucket) => bucket.nodes);
      return {
        key: "source-cluster|" + groupId,
        group,
        nodes,
        authored: false,
        label: "More " + group.label + " source modules",
        summary:
          nodes.length +
          " source entities grouped to keep detail view responsive. Select an entity in the navigator to inspect its evidence.",
        archetype: "slab-stack" as TelescopeArchetype,
      } satisfies BuildingBucket;
    },
  );
  return [...retained, ...clusters];
}

function measuredLinesForBucket(bucket: BuildingBucket): number {
  return bucket.nodes.reduce(
    (total, node) => total + (node.measured_lines ?? 0),
    0,
  );
}

function compactOverviewBuckets(buckets: BuildingBucket[]): BuildingBucket[] {
  const byGroup = new Map<string, BuildingBucket[]>();
  for (const bucket of buckets) {
    const groupBuckets = byGroup.get(bucket.group.id) ?? [];
    groupBuckets.push(bucket);
    byGroup.set(bucket.group.id, groupBuckets);
  }
  const retained: BuildingBucket[] = [];
  for (const groupBuckets of byGroup.values()) {
    const ordered = [...groupBuckets].sort(
      (left, right) =>
        Number(right.authored) - Number(left.authored) ||
        right.nodes.length - left.nodes.length ||
        left.key.localeCompare(right.key),
    );
    retained.push(...ordered.slice(0, MAX_BUILDINGS_PER_DISTRICT));
    const remainder = ordered.slice(MAX_BUILDINGS_PER_DISTRICT);
    if (remainder.length) {
      retained.push({
        key: "other|" + remainder[0].group.id,
        group: remainder[0].group,
        nodes: remainder.flatMap((bucket) => bucket.nodes),
        authored: false,
      });
    }
  }
  if (retained.length <= MAX_OVERVIEW_BUILDINGS) return retained;
  const ordered = [...retained].sort(
    (left, right) =>
      Number(right.authored) - Number(left.authored) ||
      right.nodes.length - left.nodes.length ||
      left.key.localeCompare(right.key),
  );
  const keep = ordered.slice(0, MAX_OVERVIEW_BUILDINGS);
  const overflow = ordered.slice(MAX_OVERVIEW_BUILDINGS);
  if (!overflow.length) return keep;
  const byGroupId = new Map<string, BuildingBucket>();
  for (const bucket of keep) {
    const existing = byGroupId.get(bucket.group.id);
    if (existing) existing.nodes.push(...bucket.nodes);
    else byGroupId.set(bucket.group.id, bucket);
  }
  for (const bucket of overflow) {
    const target = byGroupId.get(bucket.group.id);
    if (target) {
      target.nodes.push(...bucket.nodes);
      continue;
    }
    const fallback = keep[keep.length - 1];
    if (fallback) fallback.nodes.push(...bucket.nodes);
  }
  return keep;
}

export function toBuilding(bucket: BuildingBucket): TelescopeSceneBuilding {
  const representative = [...bucket.nodes].sort(
    (left, right) =>
      (right.measured_lines ?? 0) - (left.measured_lines ?? 0) ||
      left.id.localeCompare(right.id),
  )[0];
  const sourcePaths = bucket.nodes
    .flatMap((node) =>
      node.source_paths?.length
        ? node.source_paths
        : node.source_anchors.map((anchor) => anchor.path),
    )
    .filter((path, index, values) => values.indexOf(path) === index)
    .sort();
  const archetype =
    bucket.archetype ??
    bucket.nodes
      .map((node) => archetypeForNode(node))
      .sort((left, right) => archetypeRank(right) - archetypeRank(left))[0];
  const narrativeStatus = bucket.nodes.some(
    (node) => node.narrative_status === "stale",
  )
    ? "stale"
    : bucket.nodes.some((node) => node.narrative_status === "reviewed")
      ? "reviewed"
      : bucket.nodes.some((node) => node.narrative_status === "draft")
        ? "draft"
        : "derived";
  return {
    id: "building-" + stableId(bucket.key),
    sourceGroupId: bucket.group.id,
    label:
      bucket.label ??
      (bucket.key.startsWith("kind|") && bucket.nodes.length > 1
        ? kindLabel(bucket.nodes[0].kind)
        : representative.label),
    kind:
      bucket.nodes.length > 1
        ? kindBucket(representative.kind)
        : representative.kind,
    technology: representative.technology,
    summary:
      bucket.summary ??
      (bucket.nodes.length > 1
        ? bucket.nodes.length +
          " related " +
          kindLabel(representative.kind).toLowerCase() +
          " modules grouped for overview."
        : representative.semantic_summary),
    implementationSummary: representative.implementation_summary,
    archetype,
    sourceNodeIds: bucket.nodes.map((node) => node.id).sort(),
    sourcePaths,
    memberCount: bucket.nodes.length,
    measuredLines: bucket.nodes.reduce(
      (total, node) => total + (node.measured_lines ?? 0),
      0,
    ),
    sourceFileCount: bucket.nodes.reduce(
      (total, node) => total + (node.source_file_count ?? 1),
      0,
    ),
    confidence: confidenceForNodes(bucket.nodes),
    narrativeStatus,
    authored: bucket.authored,
  };
}

export function buildDistricts(
  groups: TelescopeGroup[],
  buildings: TelescopeSceneBuilding[],
  nodes: TelescopeNode[],
): TelescopeSceneDistrict[] {
  return groups
    .map((group) => {
      const groupBuildings = buildings.filter(
        (building) => building.sourceGroupId === group.id,
      );
      return {
        id: "district-" + group.id,
        sourceGroupId: group.id,
        label: group.label,
        kind: group.kind,
        summary: group.summary,
        buildingIds: groupBuildings.map((building) => building.id),
        sourceNodeCount: nodes.filter((node) => node.group_id === group.id)
          .length,
        measuredLines: group.measured_lines ?? 0,
        sourceFileCount:
          group.source_file_count ??
          nodes
            .filter((node) => node.group_id === group.id)
            .reduce((total, node) => total + (node.source_file_count ?? 1), 0),
        confidence: group.confidence,
        narrativeStatus: group.narrative_status ?? "derived",
        authored: group.visual_override_provenance === "authored-manifest",
      };
    })
    .filter((district) => district.buildingIds.length > 0);
}
function kindBucket(kind: string): string {
  if (kind === "route" || kind === "entrypoint") return "route";
  if (kind === "store") return "store";
  if (kind === "service") return "service";
  if (kind === "interface") return "interface";
  if (kind === "integration") return "integration";
  return "module";
}

function kindLabel(kind: string): string {
  switch (kindBucket(kind)) {
    case "route":
      return "Routes";
    case "store":
      return "Data stores";
    case "service":
      return "Services";
    case "interface":
      return "Interfaces";
    case "integration":
      return "Integrations";
    default:
      return "Modules";
  }
}

function confidenceForNodes(nodes: TelescopeNode[]): string {
  if (nodes.every((node) => node.confidence === "high")) return "high";
  if (nodes.some((node) => node.confidence === "partial")) return "partial";
  return "medium";
}

export function lowerConfidence(left: string, right: string): string {
  const rank = { high: 0, medium: 1, partial: 2, low: 3 } as Record<
    string,
    number
  >;
  return (rank[right] ?? 1) > (rank[left] ?? 1) ? right : left;
}

function archetypeRank(archetype: TelescopeArchetype): number {
  return {
    tower: 5,
    "slab-stack": 4,
    "fin-row": 3,
    cube: 2,
    "low-slab": 1,
  }[archetype];
}

function isArchetype(value: string): value is TelescopeArchetype {
  return ["fin-row", "tower", "slab-stack", "cube", "low-slab"].includes(value);
}

export function stableId(value: string): string {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
