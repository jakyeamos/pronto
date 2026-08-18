import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  useReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Activity,
  Boxes,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  GitBranch,
  LoaderCircle,
  LocateFixed,
  Pause,
  Play,
  RefreshCw,
  Rocket,
  Search,
  ShieldCheck,
  Target,
  Wrench,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
  type SyntheticEvent,
} from "react";
import * as api from "../api";
import type { EventRecord, ExternalTool, RepositorySnapshot } from "../types";
import type { RemediationRun } from "../types/remediation";
import type {
  TelescopeEdge,
  TelescopeFlow,
  TelescopeGroup,
  TelescopeLens,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import {
  TelescopeEntityNode,
  TelescopeFlowEdge,
  TelescopeGroupNode,
} from "./TelescopeGraphElements";
import { layoutTelescope } from "./telescopeLayout";

const nodeTypes = {
  telescopeEntity: TelescopeEntityNode,
  telescopeGroup: TelescopeGroupNode,
};
const edgeTypes = { telescopeFlow: TelescopeFlowEdge };

const lensOptions: Array<{
  id: TelescopeLens;
  label: string;
  icon: typeof Boxes;
}> = [
  { id: "architecture", label: "Architecture", icon: Boxes },
  { id: "changes", label: "Changes", icon: GitBranch },
  { id: "quality", label: "Quality", icon: ShieldCheck },
  { id: "remediation", label: "Remediation", icon: Wrench },
  { id: "delivery", label: "Delivery", icon: Rocket },
  { id: "activity", label: "Activity", icon: Activity },
  { id: "intent", label: "Intent", icon: Target },
];

type Selection =
  | { kind: "node"; id: string }
  | { kind: "group"; id: string }
  | { kind: "edge"; id: string }
  | { kind: "flow"; id: string }
  | null;

const INITIAL_EXPANDED_GROUP_NODE_LIMIT = 12;

export function TelescopeSurface(props: {
  repository: RepositorySnapshot;
  remediation: RemediationRun;
  events: EventRecord[];
  initialProjection?: TelescopeProjection;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
}): ReactElement {
  return (
    <ReactFlowProvider>
      <TelescopeWorkspace {...props} />
    </ReactFlowProvider>
  );
}

function TelescopeWorkspace({
  repository,
  remediation,
  events,
  initialProjection,
  onOpenWorkspace,
}: {
  repository: RepositorySnapshot;
  remediation: RemediationRun;
  events: EventRecord[];
  initialProjection?: TelescopeProjection;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
}): ReactElement {
  const [projection, setProjection] = useState<TelescopeProjection | null>(
    initialProjection ?? null,
  );
  const [loading, setLoading] = useState(!initialProjection);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeLens, setActiveLens] = useState<TelescopeLens>("architecture");
  const [selection, setSelection] = useState<Selection>(null);
  const [inspectorMode, setInspectorMode] = useState<"what" | "how">("what");
  const [paused, setPaused] = useState(false);
  const [focusAffected, setFocusAffected] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(
    new Set(),
  );
  const [navigatorQuery, setNavigatorQuery] = useState("");
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
    setCollapsedGroups(
      new Set(
        projection.groups
          .filter(
            (group) =>
              projection.nodes.filter((node) => node.group_id === group.id)
                .length > INITIAL_EXPANDED_GROUP_NODE_LIMIT,
          )
          .map((group) => group.id),
      ),
    );
  }, [projection]);

  useEffect(() => {
    if (!projection) return;
    const request = ++layoutRequest.current;
    setLayoutState("working");
    void layoutTelescope(projection, [...collapsedGroups])
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
  }, [collapsedGroups, fitView, projection, setEdges, setNodes]);

  const selectedPath = useMemo(
    () => (projection ? pathForSelection(projection, selection) : emptyPath()),
    [projection, selection],
  );
  const activeFlow =
    projection && selection?.kind === "flow"
      ? projection.flows.find((flow) => flow.id === selection.id)
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
    if (!projection) return;
    setNodes((current) =>
      current.map((node) => {
        const source = projection.nodes.find(
          (candidate) => candidate.id === node.id,
        );
        const isGroup = projection.groups.some((group) => group.id === node.id);
        const selected = selection?.id === node.id;
        const dimmed = Boolean(
          selection &&
          !selected &&
          !selectedPath.nodeIds.has(node.id) &&
          !selectedPath.groupIds.has(node.id),
        );
        const focusedOut =
          activeLens === "remediation" &&
          focusAffected &&
          (source
            ? !remediationAffectedNodeIds.has(source.id)
            : isGroup &&
              !projection.nodes.some(
                (candidate) =>
                  candidate.group_id === node.id &&
                  remediationAffectedNodeIds.has(candidate.id),
              ));
        return {
          ...node,
          hidden:
            focusedOut ||
            (source ? collapsedGroups.has(source.group_id) : false),
          data: {
            ...node.data,
            selected,
            dimmed,
            filtered: focusedOut,
            tone: source ? nodeTone(source) : isGroup ? "neutral" : undefined,
          },
        };
      }),
    );
    setEdges((current) =>
      current.map((edge) => {
        const source = projection.edges.find(
          (candidate) => candidate.id === edge.id,
        );
        const selected = selection?.id === edge.id;
        const belongsToFlow = activeFlow?.edge_ids.includes(edge.id) ?? false;
        const activeToken =
          belongsToFlow ||
          (!selection && projection.flows[0]?.edge_ids.includes(edge.id));
        return {
          ...edge,
          hidden: [edge.source, edge.target].some((nodeId) => {
            const node = projection.nodes.find(
              (candidate) => candidate.id === nodeId,
            );
            return node
              ? collapsedGroups.has(node.group_id) ||
                  (activeLens === "remediation" &&
                    focusAffected &&
                    !remediationAffectedNodeIds.has(node.id))
              : false;
          }),
          data: {
            ...edge.data,
            selected: selected || belongsToFlow,
            dimmed: Boolean(
              selection &&
              !selected &&
              !belongsToFlow &&
              !selectedPath.edgeIds.has(edge.id),
            ),
            inferred: source?.inferred,
            activeToken,
            paused,
            reducedMotion,
            onSelectToken: (event: SyntheticEvent): void => {
              event.stopPropagation();
              const flow = projection.flows.find((candidate) =>
                candidate.edge_ids.includes(edge.id),
              );
              setSelection(
                flow
                  ? { kind: "flow", id: flow.id }
                  : { kind: "edge", id: edge.id },
              );
            },
          },
        };
      }),
    );
  }, [
    activeFlow,
    activeLens,
    collapsedGroups,
    focusAffected,
    nodeTone,
    paused,
    projection,
    reducedMotion,
    remediationAffectedNodeIds,
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

  if (loading && !projection) {
    return (
      <div className="telescope-loading" role="status">
        <LoaderCircle className="spin" size={18} />
        <strong>Building the workspace map…</strong>
        <span>Reading topology, symbols, relationships, and evidence.</span>
      </div>
    );
  }
  if (!projection) {
    return (
      <div className="telescope-loading telescope-error" role="alert">
        <CircleAlert size={18} />
        <strong>Telescope is unavailable</strong>
        <span>{error ?? "No projection was returned."}</span>
        <button
          className="button button-secondary"
          onClick={() => void load(true)}
        >
          Try again
        </button>
      </div>
    );
  }

  const selectedItem = resolveSelection(projection, selection);
  const lensSummary = summarizeLens(
    activeLens,
    repository,
    remediation,
    events,
  );

  return (
    <section
      className="telescope-workspace"
      aria-label={`${repository.name} Telescope`}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "ArrowRight" || event.key === "ArrowDown") {
          event.preventDefault();
          selectOrderedNode(1);
        } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
          event.preventDefault();
          selectOrderedNode(-1);
        } else if (event.key === "0") {
          event.preventDefault();
          void fitView({ padding: 0.14 });
        }
      }}
    >
      <header className="telescope-strip">
        <div>
          <strong>{projection.repository_name}</strong>
          <span>{projection.binding.branch}</span>
          <code>
            {projection.binding.commit?.slice(0, 8) ?? "unknown commit"}
          </code>
          {projection.binding.dirty && <em>dirty</em>}
        </div>
        <div>
          <span>
            {projection.nodes.length} entities · {projection.edges.length}{" "}
            relationships
          </span>
          <span>
            {projection.coverage.supported_source_files}/
            {projection.coverage.examined_source_files} adapted
          </span>
          <span>Generated {formatTime(projection.binding.generated_at)}</span>
          <button
            className="button button-quiet telescope-refresh"
            type="button"
            onClick={() => {
              if (refreshing) {
                void api.cancelRepositoryTelescopeRefresh(repository.id);
              } else {
                void load(true);
              }
            }}
          >
            {refreshing ? (
              <CircleAlert size={13} />
            ) : (
              <RefreshCw className={loading ? "spin" : ""} size={13} />
            )}
            {refreshing ? "Cancel" : "Refresh"}
          </button>
        </div>
      </header>
      <div className="telescope-lenses" aria-label="Telescope lenses">
        {lensOptions.map((lens) => {
          const Icon = lens.icon;
          return (
            <button
              className={activeLens === lens.id ? "active" : ""}
              type="button"
              aria-pressed={activeLens === lens.id}
              key={lens.id}
              onClick={() => setActiveLens(lens.id)}
            >
              <Icon size={13} />
              {lens.label}
            </button>
          );
        })}
        {activeLens === "remediation" && (
          <button
            type="button"
            className={focusAffected ? "active" : ""}
            aria-pressed={focusAffected}
            disabled={remediationAffectedNodeIds.size === 0}
            title={
              remediationAffectedNodeIds.size === 0
                ? "No remediation actions have source-matched architecture yet."
                : undefined
            }
            onClick={() => setFocusAffected((current) => !current)}
          >
            Show affected only
          </button>
        )}
        {activeLens === "remediation" && focusAffected && (
          <span className="telescope-focus-count" role="status">
            {remediationAffectedNodeIds.size} affected{" "}
            {remediationAffectedNodeIds.size === 1 ? "entity" : "entities"}{" "}
            shown
          </span>
        )}
        <span className={`telescope-lens-state tone-${lensSummary.tone}`}>
          {layoutState === "working"
            ? "Laying out architecture…"
            : layoutState === "error"
              ? "Layout unavailable"
              : `${visibleNodes.length} map objects · ${layoutEngine === "grid-fallback" ? "fallback layout" : lensSummary.label}`}
        </span>
      </div>
      <div className="telescope-frame">
        <TelescopeNavigator
          projection={projection}
          query={navigatorQuery}
          collapsedGroups={collapsedGroups}
          selection={selection}
          onQuery={setNavigatorQuery}
          onToggleGroup={(groupId) => {
            setCollapsedGroups((current) => {
              const next = new Set(current);
              if (next.has(groupId)) next.delete(groupId);
              else next.add(groupId);
              return next;
            });
          }}
          onSelect={(nextSelection) => {
            if (nextSelection?.kind === "node") {
              const node = projection.nodes.find(
                (candidate) => candidate.id === nextSelection.id,
              );
              if (node && collapsedGroups.has(node.group_id)) {
                setCollapsedGroups((current) => {
                  const next = new Set(current);
                  next.delete(node.group_id);
                  return next;
                });
              }
            }
            setSelection(nextSelection);
          }}
        />
        <div className="telescope-canvas" aria-label="Architecture canvas">
          <ReactFlow
            nodes={visibleNodes}
            edges={visibleEdges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeClick={(_, node) =>
              setSelection({
                kind: projection.groups.some((group) => group.id === node.id)
                  ? "group"
                  : "node",
                id: node.id,
              })
            }
            onEdgeClick={(_, edge) =>
              setSelection({ kind: "edge", id: edge.id })
            }
            onPaneClick={() => setSelection(null)}
            fitView
            minZoom={0.04}
            maxZoom={1.8}
            nodesFocusable
            edgesFocusable
            proOptions={{ hideAttribution: true }}
          >
            <Background gap={28} size={1} color="rgba(89, 111, 139, .15)" />
            <MiniMap
              pannable
              zoomable
              className="telescope-minimap"
              nodeColor={(node) =>
                node.type === "telescopeGroup" ? "#dce7f4" : "#8ab6ff"
              }
              maskColor="rgba(246, 248, 251, .78)"
            />
            <Controls showInteractive={false} />
          </ReactFlow>
          <div className="telescope-canvas-actions">
            <button
              type="button"
              aria-label={
                paused ? "Resume flow animation" : "Pause flow animation"
              }
              onClick={() => setPaused((current) => !current)}
            >
              {paused ? <Play size={13} /> : <Pause size={13} />}
              {paused ? "Resume" : "Pause"}
            </button>
            <button
              type="button"
              onClick={() => void fitView({ padding: 0.14 })}
            >
              <LocateFixed size={13} /> Fit
            </button>
            <button
              type="button"
              onClick={() => void setViewport({ x: 0, y: 0, zoom: 1 })}
            >
              Reset
            </button>
          </div>
          {projection.flows.length > 0 && (
            <div
              className="telescope-flow-picker"
              aria-label="Architecture flows"
            >
              {projection.flows.map((flow) => (
                <button
                  className={selection?.id === flow.id ? "active" : ""}
                  type="button"
                  key={flow.id}
                  onClick={() => setSelection({ kind: "flow", id: flow.id })}
                >
                  <span /> {flow.label}
                </button>
              ))}
            </div>
          )}
        </div>
        <TelescopeInspector
          item={selectedItem}
          mode={inspectorMode}
          lens={activeLens}
          lensSummary={lensSummary}
          repository={repository}
          onMode={setInspectorMode}
          onOpenWorkspace={() =>
            onOpenWorkspace(repository.workspace.id, "editor")
          }
        />
      </div>
      {(error ||
        layoutWarning ||
        projection.warnings.length > 0 ||
        reducedMotion) && (
        <footer className="telescope-warnings">
          {error && <span>{error}</span>}
          {layoutWarning && <span>{layoutWarning}</span>}
          {projection.warnings.map((warning) => (
            <span key={warning.code}>{warning.message}</span>
          ))}
          {reducedMotion && (
            <span>Flow motion is reduced by system preference.</span>
          )}
        </footer>
      )}
    </section>
  );
}

function TelescopeNavigator({
  projection,
  query,
  collapsedGroups,
  selection,
  onQuery,
  onToggleGroup,
  onSelect,
}: {
  projection: TelescopeProjection;
  query: string;
  collapsedGroups: Set<string>;
  selection: Selection;
  onQuery: (query: string) => void;
  onToggleGroup: (groupId: string) => void;
  onSelect: (selection: Selection) => void;
}): ReactElement {
  const normalized = query.trim().toLowerCase();
  return (
    <aside className="telescope-navigator" aria-label="Architecture navigator">
      <label>
        <Search size={13} />
        <input
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder="Find an entity"
          aria-label="Find a Telescope entity"
        />
      </label>
      <div>
        {projection.groups.map((group) => {
          const groupNodes = projection.nodes.filter(
            (node) =>
              node.group_id === group.id &&
              (!normalized ||
                `${node.label} ${node.kind} ${node.technology}`
                  .toLowerCase()
                  .includes(normalized)),
          );
          if (normalized && groupNodes.length === 0) return null;
          const collapsed = collapsedGroups.has(group.id);
          return (
            <section key={group.id}>
              <button
                className={selection?.id === group.id ? "active" : ""}
                type="button"
                onClick={() => {
                  onSelect({ kind: "group", id: group.id });
                  onToggleGroup(group.id);
                }}
              >
                {collapsed ? (
                  <ChevronRight size={12} />
                ) : (
                  <ChevronDown size={12} />
                )}
                <strong>{group.label}</strong>
                <span>{groupNodes.length}</span>
              </button>
              {!collapsed && (
                <ul>
                  {groupNodes.map((node) => (
                    <li key={node.id}>
                      <button
                        className={selection?.id === node.id ? "active" : ""}
                        type="button"
                        onClick={() => onSelect({ kind: "node", id: node.id })}
                      >
                        <i className={`kind-${node.kind}`} />
                        <span>
                          {node.label}
                          <small>{node.kind}</small>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </aside>
  );
}

type SelectedItem =
  | { kind: "node"; value: TelescopeNode }
  | { kind: "group"; value: TelescopeGroup }
  | { kind: "edge"; value: TelescopeEdge }
  | { kind: "flow"; value: TelescopeFlow }
  | null;

function TelescopeInspector({
  item,
  mode,
  lens,
  lensSummary,
  repository,
  onMode,
  onOpenWorkspace,
}: {
  item: SelectedItem;
  mode: "what" | "how";
  lens: TelescopeLens;
  lensSummary: LensSummary;
  repository: RepositorySnapshot;
  onMode: (mode: "what" | "how") => void;
  onOpenWorkspace: () => Promise<void>;
}): ReactElement {
  return (
    <aside className="telescope-inspector" aria-label="Telescope inspector">
      <div className="telescope-inspector-tabs">
        <button
          className={mode === "what" ? "active" : ""}
          type="button"
          onClick={() => onMode("what")}
        >
          What it does
        </button>
        <button
          className={mode === "how" ? "active" : ""}
          type="button"
          onClick={() => onMode("how")}
        >
          How it’s built
        </button>
      </div>
      {item ? (
        <InspectorItem item={item} mode={mode} />
      ) : (
        <div className="telescope-inspector-empty">
          <Boxes size={20} />
          <strong>Select anything on the map</strong>
          <p>
            Inspect an entity, subsystem, relationship, or moving flow token.
          </p>
        </div>
      )}
      <div className="telescope-lens-inspector">
        <span>{lens} lens</span>
        <strong>{lensSummary.label}</strong>
        <p>{lensSummary.detail}</p>
        {lens === "intent" && (
          <small>
            Compass: {repository.project_compass.status} · ICM adapter:
            unavailable until a verified projection is registered.
          </small>
        )}
      </div>
      <button
        className="button button-secondary telescope-source-handoff"
        type="button"
        onClick={() => void onOpenWorkspace()}
      >
        Open source workspace
      </button>
    </aside>
  );
}

function InspectorItem({
  item,
  mode,
}: {
  item: SelectedItem;
  mode: "what" | "how";
}): ReactElement | null {
  if (!item) return null;
  if (item.kind === "node") {
    const node = item.value;
    return (
      <div className="telescope-inspector-content">
        <p className="eyebrow">{node.kind}</p>
        <h2>{node.label}</h2>
        <StatusPill tone={node.confidence === "high" ? "mint" : "amber"}>
          {node.confidence} confidence
        </StatusPill>
        <p>
          {mode === "what"
            ? node.semantic_summary
            : node.implementation_summary}
        </p>
        {mode === "how" && (
          <>
            <InspectorList title="Technology" values={[node.technology]} />
            <InspectorList title="Symbols" values={node.symbols} />
            <InspectorList title="Data shapes" values={node.data_shapes} />
            <InspectorList
              title="Evidence"
              values={node.source_anchors.map(
                (anchor) =>
                  `${anchor.path}${anchor.line ? `:${anchor.line}` : ""}`,
              )}
            />
            <small>
              Summary status: {node.summary_status}. Generated descriptions are
              derived, not confirmed source facts.
            </small>
          </>
        )}
      </div>
    );
  }
  if (item.kind === "group") {
    return (
      <div className="telescope-inspector-content">
        <p className="eyebrow">Subsystem</p>
        <h2>{item.value.label}</h2>
        <p>{item.value.summary}</p>
        <small>{item.value.confidence} confidence grouping</small>
      </div>
    );
  }
  if (item.kind === "edge") {
    return (
      <div className="telescope-inspector-content">
        <p className="eyebrow">Relationship</p>
        <h2>{item.value.label}</h2>
        <p>
          Direction: {item.value.direction}. Provenance: {item.value.provenance}
          .
        </p>
        <small>
          {item.value.confidence} confidence
          {item.value.inferred ? " · inferred" : " · resolved"}
        </small>
      </div>
    );
  }
  return (
    <div className="telescope-inspector-content">
      <p className="eyebrow">{item.value.kind} flow</p>
      <h2>{item.value.label}</h2>
      <p>
        {item.value.node_ids.length} entities across{" "}
        {item.value.edge_ids.length} directional handoffs.
      </p>
      <InspectorList
        title="Static data shape"
        values={item.value.data_shape ? [item.value.data_shape] : []}
      />
      <small>
        {item.value.provenance} · no runtime payload values are captured.
      </small>
    </div>
  );
}

function InspectorList({
  title,
  values,
}: {
  title: string;
  values: string[];
}): ReactElement {
  return (
    <div className="telescope-inspector-list">
      <span>{title}</span>
      {values.length ? (
        values.map((value) => <code key={value}>{value}</code>)
      ) : (
        <small>Unavailable</small>
      )}
    </div>
  );
}

interface LensSummary {
  label: string;
  detail: string;
  tone: "neutral" | "blue" | "mint" | "amber" | "coral";
}

function summarizeLens(
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

function toneForLens(
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

function emptyPath(): {
  nodeIds: Set<string>;
  edgeIds: Set<string>;
  groupIds: Set<string>;
} {
  return { nodeIds: new Set(), edgeIds: new Set(), groupIds: new Set() };
}

function pathForSelection(
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

function resolveSelection(
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
  const value = projection.flows.find((flow) => flow.id === selection.id);
  return value ? { kind: "flow", value } : null;
}
