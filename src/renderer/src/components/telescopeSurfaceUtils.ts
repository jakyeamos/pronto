import type { Edge, Node } from "@xyflow/react";
import type { EventRecord, RepositorySnapshot } from "../types";
import type { RemediationRun } from "../types/remediation";
import type {
  TelescopeAction,
  TelescopeFlow,
  TelescopeLens,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";
import type {
  LensSummary,
  SelectedItem,
  Selection,
} from "./telescopeSurfaceTypes";
import type { TelescopeSceneModel } from "./telescopeSceneModel";

export function summarizeActionEvidence(
  action: TelescopeAction,
  repository: RepositorySnapshot,
): {
  label: string;
  detail: string;
  tone: "neutral" | "mint" | "amber" | "coral";
} {
  if (!action.behavior_id) {
    return {
      label: "Not behavior-profiled",
      detail: "This spatial action has no behavior-assurance contract link.",
      tone: "amber",
    };
  }
  if (action.behavior_state === "unresolved") {
    return {
      label: "Behavior link unresolved",
      detail: `${action.behavior_id} · not present in the canonical behavior contract`,
      tone: "amber",
    };
  }
  const assurance = repository.quality.behavior_assurance;
  const matchingScenarios =
    assurance.coverage?.scenarios.filter(
      (scenario) =>
        scenario.behavior_id === action.behavior_id &&
        (action.scenario_ids ?? []).includes(scenario.scenario_id),
    ) ?? [];
  const statuses = new Set(
    matchingScenarios.map((scenario) => scenario.status.toLowerCase()),
  );
  const contractStatus = assurance.contract_status.toLowerCase();
  if (["missing", "invalid", "unavailable"].includes(contractStatus)) {
    return {
      label: "Behavior contract unavailable",
      detail: `${action.behavior_id} · ${humanizeStatus(contractStatus)}`,
      tone: "amber",
    };
  }
  if (statuses.has("failed") || statuses.has("blocked")) {
    return {
      label: "Behavior evidence blocked",
      detail: `${action.behavior_id} · ${humanizeStatus(
        statuses.has("blocked") ? "blocked" : "failed",
      )}`,
      tone: "coral",
    };
  }
  if (statuses.has("stale")) {
    return {
      label: "Behavior evidence stale",
      detail: `${action.behavior_id} · stale receipt or workspace binding`,
      tone: "amber",
    };
  }
  if (
    matchingScenarios.length > 0 &&
    matchingScenarios.every((scenario) => scenario.status === "verified")
  ) {
    return {
      label: "Behavior evidence verified",
      detail: `${action.behavior_id} · ${matchingScenarios.length} verified scenario${matchingScenarios.length === 1 ? "" : "s"}`,
      tone: "mint",
    };
  }
  return {
    label: `Behavior evidence ${humanizeStatus(assurance.result_status || "unknown")}`,
    detail: `${action.behavior_id} · ${matchingScenarios.length} linked scenario${matchingScenarios.length === 1 ? "" : "s"}; no green state inferred from the visual mapping alone`,
    tone: "amber",
  };
}

function humanizeStatus(value: string): string {
  return value.replace(/[_-]+/g, " ");
}
export function summarizeLens(
  lens: TelescopeLens,
  repository: RepositorySnapshot,
  remediation: RemediationRun,
  events: EventRecord[],
): LensSummary {
  const plan = remediation.plans.find(
    (candidate) => candidate.repository_id === repository.id,
  );
  if (lens === "changes") {
    return {
      label: repository.workspace.dirty
        ? `${repository.workspace.added + repository.workspace.removed} changed lines`
        : repository.workspace.sync_state,
      detail: `${repository.workspaces.length} workspace${repository.workspaces.length === 1 ? "" : "s"}; ${repository.custody?.lanes.length ?? 0} verified custody lanes. Change movement is projected toward ${repository.target_branch ?? repository.default_branch ?? "an unknown target"}.`,
      tone: repository.workspace.dirty ? "amber" : "mint",
    };
  }
  if (lens === "quality") {
    const failed = repository.quality.gates.filter(
      (gate) => gate.status === "Failed" || gate.status === "Blocked",
    ).length;
    return {
      label: failed
        ? `${failed} blocked or failing gates`
        : repository.quality.ingestion_status,
      detail: `${repository.quality.findings.actionable_total} actionable findings; evidence freshness is ${repository.quality.findings.freshness}. Aggregated evidence remains repository-scoped when no source anchor is present.`,
      tone: failed
        ? "coral"
        : repository.quality.ingestion_status === "Available"
          ? "mint"
          : "amber",
    };
  }
  if (lens === "remediation") {
    const active = plan?.actions.filter(
      (action) => !["verified", "deferred"].includes(action.status),
    );
    return {
      label: plan ? `${active?.length ?? 0} active actions` : "No current plan",
      detail: plan
        ? `${plan.progress.percentage}% complete. Only source-matched actions tint individual entities; otherwise the evidence stays repository-scoped.`
        : "Refresh remediation to attach a current plan without changing the base topology.",
      tone: active?.some((action) => action.status === "blocked")
        ? "coral"
        : plan
          ? "amber"
          : "neutral",
    };
  }
  if (lens === "delivery") {
    const blocked = repository.pull_requests.some(
      (pullRequest) =>
        pullRequest.checks_state === "Failed" ||
        pullRequest.mergeability === "Blocked",
    );
    return {
      label: `${repository.pull_requests.length} PRs · ${repository.releases.length} releases`,
      detail: `Delivery runs from entrypoints and routes through ${repository.release_rule ? repository.release_rule.name : "an unconfigured release rule"}.`,
      tone: blocked ? "coral" : repository.release_rule ? "blue" : "amber",
    };
  }
  if (lens === "activity") {
    const relevant = events.filter(
      (event) => event.repository_id === repository.id,
    );
    return {
      label: `${relevant.length} verified events`,
      detail: `${repository.custody?.lanes.length ?? 0} custody lanes. Pronto does not infer agent or skill activity from prompts, filenames, or catalog presence.`,
      tone: relevant.length ? "blue" : "neutral",
    };
  }
  if (lens === "intent") {
    return {
      label: `Compass ${repository.project_compass.status}`,
      detail: `${repository.project_compass.open_blockers} blockers and ${repository.project_compass.open_drift} drift records. Compass and ICM remain optional overlays, never the source topology.`,
      tone:
        repository.project_compass.status === "Ready"
          ? repository.project_compass.open_blockers > 0 ||
            repository.project_compass.open_drift > 0
            ? "amber"
            : "mint"
          : "neutral",
    };
  }
  return {
    label: "Source-derived topology",
    detail:
      "The base graph is generated from the active worktree. Select an entity, relationship, group, or flow to inspect evidence.",
    tone: "blue",
  };
}

export function toneForLens(
  node: TelescopeNode,
  lens: TelescopeLens,
  repository: RepositorySnapshot,
  remediation: RemediationRun,
  events: EventRecord[],
): string {
  if (lens === "architecture") return "neutral";
  if (lens === "changes") return repository.workspace.dirty ? "amber" : "mint";
  if (lens === "quality") {
    if (
      repository.quality.gates.some(
        (gate) => gate.status === "Failed" || gate.status === "Blocked",
      )
    )
      return "coral";
    return repository.quality.ingestion_status === "Available"
      ? "mint"
      : "amber";
  }
  if (lens === "delivery")
    return matchesNode(node, "route entrypoint release build deploy")
      ? "blue"
      : "neutral";
  if (lens === "remediation") {
    const plan = remediation.plans.find(
      (candidate) => candidate.repository_id === repository.id,
    );
    const action = plan?.actions.find((candidate) =>
      matchesNode(node, `${candidate.title} ${candidate.summary}`),
    );
    return action?.status === "blocked"
      ? "coral"
      : action
        ? "amber"
        : "neutral";
  }
  if (lens === "activity") {
    return events.some(
      (event) =>
        event.repository_id === repository.id &&
        matchesNode(node, event.summary),
    )
      ? "blue"
      : "neutral";
  }
  if (lens === "intent") {
    const compassText = [
      repository.project_compass.identity,
      ...repository.project_compass.open_blocker_items.map(
        (item) => item.summary,
      ),
      ...repository.project_compass.open_drift_items.map(
        (item) => item.summary,
      ),
    ]
      .filter(Boolean)
      .join(" ");
    return matchesNode(node, compassText) ? "amber" : "neutral";
  }
  return "neutral";
}

function matchesNode(node: TelescopeNode, text: string): boolean {
  const haystack = text.toLowerCase();
  const anchors = [
    node.label,
    node.kind,
    ...node.source_anchors.map((anchor) => anchor.path),
  ]
    .flatMap((value) => value.toLowerCase().split(/[^a-z0-9]+/))
    .filter((value) => value.length > 3);
  return anchors.some((value) => haystack.includes(value));
}
export function emptyPath(): {
  nodeIds: Set<string>;
  edgeIds: Set<string>;
  groupIds: Set<string>;
} {
  return { nodeIds: new Set(), edgeIds: new Set(), groupIds: new Set() };
}

export function pathForSelection(
  projection: TelescopeProjection,
  selection: Selection,
): ReturnType<typeof emptyPath> {
  const result = emptyPath();
  if (!selection) return result;
  if (selection.kind === "flow") {
    const flow = projection.flows.find(
      (candidate) => candidate.id === selection.id,
    );
    flow?.node_ids.forEach((id) => result.nodeIds.add(id));
    flow?.edge_ids.forEach((id) => result.edgeIds.add(id));
  } else if (selection.kind === "action") {
    const action = projection.actions.find(
      (candidate) => candidate.id === selection.id,
    );
    action?.node_ids.forEach((id) => result.nodeIds.add(id));
    action?.edge_ids.forEach((id) => result.edgeIds.add(id));
    const flow = action?.flow_id
      ? projection.flows.find((candidate) => candidate.id === action.flow_id)
      : undefined;
    flow?.node_ids.forEach((id) => result.nodeIds.add(id));
    flow?.edge_ids.forEach((id) => result.edgeIds.add(id));
  } else if (selection.kind === "edge") {
    const edge = projection.edges.find(
      (candidate) => candidate.id === selection.id,
    );
    if (edge) {
      result.nodeIds.add(edge.source);
      result.nodeIds.add(edge.target);
      result.edgeIds.add(edge.id);
    }
  } else if (selection.kind === "group") {
    result.groupIds.add(selection.id);
    projection.nodes
      .filter((node) => node.group_id === selection.id)
      .forEach((node) => result.nodeIds.add(node.id));
  } else {
    const queue = [selection.id];
    result.nodeIds.add(selection.id);
    while (queue.length && result.nodeIds.size < 24) {
      const current = queue.shift();
      for (const edge of projection.edges.filter(
        (candidate) => candidate.source === current,
      )) {
        result.edgeIds.add(edge.id);
        if (!result.nodeIds.has(edge.target)) {
          result.nodeIds.add(edge.target);
          queue.push(edge.target);
        }
      }
    }
  }
  for (const nodeId of result.nodeIds) {
    const node = projection.nodes.find((candidate) => candidate.id === nodeId);
    if (node) result.groupIds.add(node.group_id);
  }
  return result;
}

export function resolveSelection(
  projection: TelescopeProjection,
  selection: Selection,
): SelectedItem {
  if (!selection) return null;
  if (selection.kind === "node") {
    const value = projection.nodes.find((node) => node.id === selection.id);
    return value ? { kind: "node", value } : null;
  }
  if (selection.kind === "group") {
    const value = projection.groups.find((group) => group.id === selection.id);
    return value ? { kind: "group", value } : null;
  }
  if (selection.kind === "edge") {
    const value = projection.edges.find((edge) => edge.id === selection.id);
    return value ? { kind: "edge", value } : null;
  }
  if (selection.kind === "action") {
    const value = projection.actions.find(
      (action) => action.id === selection.id,
    );
    return value ? { kind: "action", value } : null;
  }
  const value = projection.flows.find((flow) => flow.id === selection.id);
  return value ? { kind: "flow", value } : null;
}

interface LayoutNodeMetadata {
  sourceNodeIds: string[];
  sourceGroupIds: string[];
  sourceNode?: TelescopeNode;
  isDistrict: boolean;
}

export function metadataForLayoutNode(
  node: Node,
  scene: TelescopeSceneModel,
  projection: TelescopeProjection,
): LayoutNodeMetadata {
  const building = scene.buildings.find(
    (candidate) => candidate.id === node.id,
  );
  if (building) {
    return {
      sourceNodeIds: building.sourceNodeIds,
      sourceGroupIds: [building.sourceGroupId],
      sourceNode: projection.nodes.find((candidate) =>
        building.sourceNodeIds.includes(candidate.id),
      ),
      isDistrict: false,
    };
  }
  const district = scene.districts.find(
    (candidate) => candidate.id === node.id,
  );
  if (district) {
    const sourceNodeIds = projection.nodes
      .filter((candidate) => candidate.group_id === district.sourceGroupId)
      .map((candidate) => candidate.id);
    return {
      sourceNodeIds,
      sourceGroupIds: [district.sourceGroupId],
      isDistrict: true,
    };
  }
  const data = (node.data ?? {}) as { telescopeId?: string };
  const sourceNode =
    projection.nodes.find(
      (candidate) =>
        candidate.id === node.id || candidate.id === data.telescopeId,
    ) ?? undefined;
  const sourceGroup = projection.groups.find(
    (candidate) =>
      candidate.id === node.id || candidate.id === data.telescopeId,
  );
  return {
    sourceNodeIds: sourceNode ? [sourceNode.id] : [],
    sourceGroupIds: sourceNode
      ? [sourceNode.group_id]
      : sourceGroup
        ? [sourceGroup.id]
        : [],
    sourceNode,
    isDistrict: Boolean(sourceGroup),
  };
}

export function sourceEdgeIdsForLayoutEdge(
  edge: Edge,
  scene: TelescopeSceneModel | null,
  projection: TelescopeProjection,
): string[] {
  const rail = scene?.rails.find((candidate) => candidate.id === edge.id);
  if (rail) return rail.sourceEdgeIds;
  const data = (edge.data ?? {}) as { sourceEdgeIds?: unknown };
  if (
    Array.isArray(data.sourceEdgeIds) &&
    data.sourceEdgeIds.every(
      (value): value is string => typeof value === "string",
    )
  ) {
    return data.sourceEdgeIds;
  }
  return projection.edges.some((candidate) => candidate.id === edge.id)
    ? [edge.id]
    : [];
}

export function firstPrimaryFlow(
  projection: TelescopeProjection,
  scene: TelescopeSceneModel,
): TelescopeFlow | null {
  return (
    projection.flows.find((flow) => flow.id === scene.primaryFlowId) ??
    projection.flows.find((flow) => flow.primary) ??
    projection.flows[0] ??
    null
  );
}
