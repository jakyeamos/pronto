import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
import {
  useEdgesState,
  useNodesState,
  useReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import * as api from "../api";
import type { EventRecord, RepositorySnapshot } from "../types";
import type { RemediationRun } from "../types/remediation";
import type {
  TelescopeLens,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";
import { layoutTelescope } from "./telescopeLayout";
import {
  buildTelescopeScene,
  type TelescopeSceneLevel,
} from "./telescopeSceneModel";
import {
  emptyPath,
  firstPrimaryFlow,
  metadataForLayoutNode,
  pathForSelection,
  sourceEdgeIdsForLayoutEdge,
  toneForLens,
} from "./telescopeSurfaceUtils";
import type { Selection } from "./telescopeSurfaceTypes";

export function useTelescopeWorkspaceModel({
  repository,
  remediation,
  events,
  initialProjection,
}: {
  repository: RepositorySnapshot;
  remediation: RemediationRun;
  events: EventRecord[];
  initialProjection?: TelescopeProjection;
}): ReturnType<typeof useTelescopeWorkspaceModelInternal> {
  return useTelescopeWorkspaceModelInternal({
    repository,
    remediation,
    events,
    initialProjection,
  });
}

function useTelescopeWorkspaceModelInternal({
  repository,
  remediation,
  events,
  initialProjection,
}: {
  repository: RepositorySnapshot;
  remediation: RemediationRun;
  events: EventRecord[];
  initialProjection?: TelescopeProjection;
}) {
  const [projection, setProjection] = useState<TelescopeProjection | null>(
    initialProjection ?? null,
  );
  const [loading, setLoading] = useState(!initialProjection);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeLens, setActiveLens] = useState<TelescopeLens>("architecture");
  const [sceneLevel, setSceneLevel] = useState<TelescopeSceneLevel>("overview");
  const [selection, setSelection] = useState<Selection>(null);
  const [inspectorMode, setInspectorMode] = useState<"what" | "how">("what");
  const [paused, setPaused] = useState(false);
  const [focusAffected, setFocusAffected] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(
    new Set(),
  );
  const [navigatorQuery, setNavigatorQuery] = useState("");
  const [actionQuery, setActionQuery] = useState("");
  const [layoutState, setLayoutState] = useState<
    "idle" | "working" | "ready" | "error"
  >("idle");
  const [layoutEngine, setLayoutEngine] = useState<
    "elk" | "grid-fallback" | null
  >(null);
  const [layoutWarning, setLayoutWarning] = useState<string | null>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [reducedMotion, setReducedMotion] = useState(false);
  const { fitView, setViewport } = useReactFlow();
  const layoutRequest = useRef(0);
  const scene = useMemo(
    () => (projection ? buildTelescopeScene(projection, sceneLevel) : null),
    [projection, sceneLevel],
  );
  const primaryFlow = useMemo(
    () => (projection && scene ? firstPrimaryFlow(projection, scene) : null),
    [projection, scene],
  );

  const load = useCallback(
    async (refresh = false): Promise<void> => {
      if (initialProjection && !refresh) return;
      setLoading(true);
      setRefreshing(refresh);
      setError(null);
      try {
        const next = refresh
          ? await api.refreshRepositoryTelescope(repository.id)
          : await api.getRepositoryTelescope(repository.id);
        setProjection(next);
      } catch (caught) {
        if (
          refresh &&
          caught instanceof Error &&
          caught.message.includes("refresh cancelled")
        ) {
          return;
        }
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not generate this Telescope projection.",
        );
      } finally {
        setLoading(false);
        setRefreshing(false);
      }
    },
    [initialProjection, repository.id],
  );

  useEffect(() => {
    void load(false);
  }, [load]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = (): void => setReducedMotion(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    if (!projection) return;
    setCollapsedGroups(new Set());
  }, [projection]);

  useEffect(() => {
    if (!projection || !scene) return;
    const request = ++layoutRequest.current;
    setLayoutState("working");
    void layoutTelescope(projection, [], scene)
      .then((layout) => {
        if (request !== layoutRequest.current) return;
        setNodes(layout.nodes);
        setEdges(layout.edges);
        setLayoutEngine(layout.engine ?? "elk");
        setLayoutWarning(layout.warning ?? null);
        setLayoutState("ready");
        window.requestAnimationFrame(() =>
          window.requestAnimationFrame(() => void fitView({ padding: 0.14 })),
        );
      })
      .catch((caught: unknown) => {
        if (request !== layoutRequest.current) return;
        setLayoutState("error");
        setError(
          caught instanceof Error ? caught.message : "Telescope layout failed.",
        );
      });
  }, [fitView, projection, scene, setEdges, setNodes]);

  const selectedPath = useMemo(
    () => (projection ? pathForSelection(projection, selection) : emptyPath()),
    [projection, selection],
  );
  const activeFlow =
    projection && selection?.kind === "flow"
      ? (projection.flows.find((flow) => flow.id === selection.id) ?? null)
      : null;
  const activeAction =
    projection && selection?.kind === "action"
      ? (projection.actions.find((action) => action.id === selection.id) ??
        null)
      : null;

  const nodeTone = useCallback(
    (node: TelescopeNode): string =>
      toneForLens(node, activeLens, repository, remediation, events),
    [activeLens, events, remediation, repository],
  );
  const remediationAffectedNodeIds = useMemo(
    () =>
      new Set(
        projection?.nodes
          .filter(
            (node) =>
              toneForLens(
                node,
                "remediation",
                repository,
                remediation,
                events,
              ) !== "neutral",
          )
          .map((node) => node.id) ?? [],
      ),
    [events, projection, remediation, repository],
  );
  const visibleNodes = useMemo(
    () => nodes.filter((node) => !node.hidden),
    [nodes],
  );
  const visibleEdges = useMemo(
    () => edges.filter((edge) => !edge.hidden),
    [edges],
  );

  useEffect(() => {
    if (!projection || !scene) return;
    setNodes((current) =>
      current.map((node) => {
        const metadata = metadataForLayoutNode(node, scene, projection);
        const selected =
          (selection?.kind === "node" && selection.id === node.id) ||
          (selection?.kind === "node" &&
            metadata.sourceNodeIds.includes(selection.id)) ||
          (selection?.kind === "group" &&
            metadata.sourceGroupIds.includes(selection.id));
        const onSelectedPath =
          metadata.sourceNodeIds.some((id) => selectedPath.nodeIds.has(id)) ||
          metadata.sourceGroupIds.some((id) => selectedPath.groupIds.has(id));
        const dimmed = Boolean(selection && !selected && !onSelectedPath);
        const focusedOut =
          activeLens === "remediation" &&
          focusAffected &&
          !metadata.sourceNodeIds.some((id) =>
            remediationAffectedNodeIds.has(id),
          );
        const collapsedOut =
          sceneLevel === "source" &&
          !metadata.isDistrict &&
          metadata.sourceGroupIds.some((id) => collapsedGroups.has(id));
        const actionFocusedOut =
          selection?.kind === "action" &&
          (selectedPath.nodeIds.size > 0 || selectedPath.edgeIds.size > 0) &&
          !onSelectedPath;
        return {
          ...node,
          hidden: focusedOut || collapsedOut || actionFocusedOut,
          data: {
            ...node.data,
            selected,
            dimmed,
            filtered: focusedOut || actionFocusedOut,
            tone: metadata.sourceNode
              ? nodeTone(metadata.sourceNode)
              : "neutral",
          },
        };
      }),
    );
    setEdges((current) =>
      current.map((edge) => {
        const sourceEdgeIds = sourceEdgeIdsForLayoutEdge(
          edge,
          scene,
          projection,
        );
        const sourceEdges = projection.edges.filter((candidate) =>
          sourceEdgeIds.includes(candidate.id),
        );
        const selected =
          selection?.kind === "edge" && sourceEdgeIds.includes(selection.id);
        const actionEdgeSelected =
          selection?.kind === "action" &&
          sourceEdgeIds.some((id) => selectedPath.edgeIds.has(id));
        const belongsToFlow =
          activeFlow?.edge_ids.some((id) => sourceEdgeIds.includes(id)) ??
          false;
        const activeToken =
          belongsToFlow ||
          actionEdgeSelected ||
          (!selection &&
            primaryFlow?.edge_ids.some((id) => sourceEdgeIds.includes(id)));
        const endpointNodeIds = sourceEdges.flatMap((sourceEdge) => [
          sourceEdge.source,
          sourceEdge.target,
        ]);
        const endpointNodes = projection.nodes.filter((node) =>
          endpointNodeIds.includes(node.id),
        );
        const collapsedOut =
          sceneLevel === "source" &&
          endpointNodes.some((node) => collapsedGroups.has(node.group_id));
        const focusedOut =
          activeLens === "remediation" &&
          focusAffected &&
          !endpointNodeIds.some((id) => remediationAffectedNodeIds.has(id));
        const actionFocusedOut =
          selection?.kind === "action" &&
          (selectedPath.nodeIds.size > 0 || selectedPath.edgeIds.size > 0) &&
          !actionEdgeSelected &&
          !endpointNodeIds.some((id) => selectedPath.nodeIds.has(id));
        return {
          ...edge,
          hidden: collapsedOut || focusedOut || actionFocusedOut,
          data: {
            ...edge.data,
            selected: selected || belongsToFlow || actionEdgeSelected,
            dimmed: Boolean(
              selection &&
              !selected &&
              !belongsToFlow &&
              !actionEdgeSelected &&
              !sourceEdgeIds.some((id) => selectedPath.edgeIds.has(id)),
            ),
            inferred: sourceEdges.some((sourceEdge) => sourceEdge.inferred),
            uncertain:
              sourceEdges.some(
                (sourceEdge) =>
                  sourceEdge.inferred || sourceEdge.confidence !== "high",
              ) ||
              Boolean(
                (edge.data as { uncertain?: boolean } | undefined)?.uncertain,
              ),
            activeToken,
            paused,
            reducedMotion,
            railKind: (edge.data as { railKind?: string } | undefined)
              ?.railKind,
            tokenLabel: (edge.data as { tokenLabel?: string } | undefined)
              ?.tokenLabel,
            onSelectToken: (event: SyntheticEvent): void => {
              event.stopPropagation();
              const flow = projection.flows.find((candidate) =>
                candidate.edge_ids.some((id) => sourceEdgeIds.includes(id)),
              );
              setSelection(
                flow
                  ? { kind: "flow", id: flow.id }
                  : sourceEdgeIds[0]
                    ? { kind: "edge", id: sourceEdgeIds[0] }
                    : null,
              );
            },
          },
        };
      }),
    );
  }, [
    activeAction,
    activeFlow,
    activeLens,
    collapsedGroups,
    focusAffected,
    nodeTone,
    paused,
    primaryFlow,
    projection,
    reducedMotion,
    remediationAffectedNodeIds,
    scene,
    sceneLevel,
    selectedPath,
    selection,
    setEdges,
    setNodes,
  ]);

  const selectOrderedNode = useCallback(
    (delta: number): void => {
      if (!projection?.nodes.length) return;
      const currentIndex =
        selection?.kind === "node"
          ? projection.nodes.findIndex((node) => node.id === selection.id)
          : -1;
      const index =
        (currentIndex + delta + projection.nodes.length) %
        projection.nodes.length;
      setSelection({ kind: "node", id: projection.nodes[index].id });
    },
    [projection, selection],
  );

  return {
    projection,
    loading,
    refreshing,
    error,
    activeLens,
    setActiveLens,
    sceneLevel,
    setSceneLevel,
    selection,
    setSelection,
    inspectorMode,
    setInspectorMode,
    paused,
    setPaused,
    focusAffected,
    setFocusAffected,
    collapsedGroups,
    setCollapsedGroups,
    navigatorQuery,
    setNavigatorQuery,
    actionQuery,
    setActionQuery,
    layoutState,
    layoutEngine,
    layoutWarning,
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    reducedMotion,
    fitView,
    setViewport,
    scene,
    primaryFlow,
    activeFlow,
    activeAction,
    selectedPath,
    remediationAffectedNodeIds,
    visibleNodes,
    visibleEdges,
    load,
    nodeTone,
    selectOrderedNode,
  };
}

export type TelescopeWorkspaceModel = ReturnType<
  typeof useTelescopeWorkspaceModelInternal
>;
