import { useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, ReactElement } from "react";
import {
  Check,
  ChevronRight,
  EyeOff,
  GitBranch,
  Minus,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  SlidersHorizontal,
  Sparkles,
  Workflow as WorkflowIcon,
  X,
} from "lucide-react";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import type {
  Connection,
  ConnectionInput,
  ConnectionNode,
  ConnectionNodeInput,
  ConnectionsSnapshot,
  RepositorySnapshot,
  Workflow,
  WorkflowInput,
  WorkflowStepInput,
} from "../types";

const nodeKindOrder = [
  "repository",
  "workspace",
  "package",
  "module",
  "service",
  "tool",
  "environment",
  "data-store",
  "person",
  "team",
];

const relationshipColors: Record<string, string> = {
  dependency: "#6f9dff",
  handoff: "#bd8cff",
  trigger: "#f2bd69",
  deployment: "#73d3ad",
  runtime: "#58c6d8",
  tool: "#f28f9b",
  "code-dependency": "#8b9dbb",
};

const relationshipTypes = [
  "dependency",
  "handoff",
  "trigger",
  "deployment",
  "runtime",
  "tool",
  "code-dependency",
];

const nodeKinds = [
  "repository",
  "workspace",
  "package",
  "module",
  "service",
  "tool",
  "environment",
  "data-store",
  "person",
  "team",
];

type Selection =
  | { kind: "node"; id: string }
  | { kind: "connection"; id: string }
  | { kind: "workflow"; id: string }
  | null;

interface WorkflowStepDraft {
  nodeId: string;
  actionLabel: string;
  command: string;
  connectionId: string;
}

interface ConnectionsSurfaceProps {
  connections: ConnectionsSnapshot;
  repositories: RepositorySnapshot[];
  globalQuery?: string;
  selectedRepositoryId?: string | null;
  isRefreshing: boolean;
  onRefresh: (repositoryId?: string) => Promise<void>;
  onSaveNode: (input: ConnectionNodeInput) => Promise<void>;
  onDeleteNode: (nodeId: string) => Promise<void>;
  onSaveConnection: (input: ConnectionInput) => Promise<void>;
  onDeleteConnection: (connectionId: string) => Promise<void>;
  onSaveWorkflow: (input: WorkflowInput) => Promise<void>;
  onDeleteWorkflow: (workflowId: string) => Promise<void>;
  onReview: (
    recordType: "node" | "connection" | "workflow",
    recordId: string,
    reviewState: "Suggested" | "Confirmed" | "Overridden" | "Hidden",
    label?: string,
  ) => Promise<void>;
  onToggleAdapter: (adapterId: string, enabled: boolean) => Promise<void>;
  onOpenRepository: (repository: RepositorySnapshot) => void;
}

function displayNode(node: ConnectionNode): string {
  return node.label_override?.trim() || node.label;
}

function displayConnection(connection: Connection): string {
  return connection.label_override?.trim() || connection.label;
}

function displayWorkflow(workflow: Workflow): string {
  return workflow.name_override?.trim() || workflow.name;
}

function confidenceTone(confidence: string): string {
  const normalized = confidence.toLowerCase();
  if (normalized.includes("high")) return "mint";
  if (normalized.includes("medium")) return "amber";
  if (normalized.includes("low")) return "coral";
  return "slate";
}

function statusTone(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized.includes("confirmed") || normalized.includes("active"))
    return "mint";
  if (normalized.includes("stale") || normalized.includes("suggested"))
    return "amber";
  if (normalized.includes("hidden")) return "slate";
  if (normalized.includes("unsupported") || normalized.includes("failed"))
    return "coral";
  return "blue";
}

function originLabel(origin: string): string {
  if (origin === "Manual" || origin === "Discovered") return origin;
  return `Discovered · ${origin}`;
}

function titleCase(value: string): string {
  return value
    .split(/[-_ ]/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function EvidenceList({
  evidence,
}: {
  evidence: ConnectionNode["evidence"];
}): ReactElement {
  if (evidence.length === 0) {
    return <p className="connections-muted">No source evidence recorded.</p>;
  }
  return (
    <div className="connections-evidence-list">
      {evidence.map((item, index) => (
        <div
          className="connections-evidence-row"
          key={`${item.adapter}-${index}`}
        >
          <div className="connections-evidence-heading">
            <strong>{item.adapter}</strong>
            <span>{item.freshness}</span>
          </div>
          {item.source_path && <code>{item.source_path}</code>}
          <p>{item.detail}</p>
          <small>{formatTime(item.observed_at)}</small>
          {item.command && (
            <div className="connections-command-block">
              <span>Displayed only · never executed</span>
              <code>{item.command}</code>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function FilterSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
}): ReactElement {
  return (
    <label className="connections-filter-field">
      <span>{label}</span>
      <select
        className="drawer-select"
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {options.map((option) => (
          <option value={option.value} key={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

export function ConnectionsSurface({
  connections,
  repositories,
  globalQuery = "",
  selectedRepositoryId = null,
  isRefreshing,
  onRefresh,
  onSaveNode,
  onDeleteNode,
  onSaveConnection,
  onDeleteConnection,
  onSaveWorkflow,
  onDeleteWorkflow,
  onReview,
  onToggleAdapter,
  onOpenRepository,
}: ConnectionsSurfaceProps): ReactElement {
  const [search, setSearch] = useState("");
  const [repositoryFilter, setRepositoryFilter] = useState(
    selectedRepositoryId ?? "all",
  );
  const [nodeKindFilter, setNodeKindFilter] = useState("all");
  const [relationshipFilter, setRelationshipFilter] = useState("all");
  const [workflowFilter, setWorkflowFilter] = useState("all");
  const [originFilter, setOriginFilter] = useState("all");
  const [confidenceFilter, setConfidenceFilter] = useState("all");
  const [showStale, setShowStale] = useState(false);
  const [selection, setSelection] = useState<Selection>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [expandedForm, setExpandedForm] = useState<
    "node" | "connection" | "workflow" | null
  >(null);
  const [nodeKind, setNodeKind] = useState("service");
  const [nodeLabel, setNodeLabel] = useState("");
  const [nodeIdentity, setNodeIdentity] = useState("");
  const [nodeRepositoryId, setNodeRepositoryId] = useState("");
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionTarget, setConnectionTarget] = useState("");
  const [connectionType, setConnectionType] = useState("dependency");
  const [connectionLabel, setConnectionLabel] = useState("");
  const [workflowName, setWorkflowName] = useState("");
  const [workflowScope, setWorkflowScope] = useState("Cross-repository");
  const [workflowSteps, setWorkflowSteps] = useState<WorkflowStepDraft[]>([]);
  const [workflowStepNode, setWorkflowStepNode] = useState("");
  const [workflowStepAction, setWorkflowStepAction] = useState("");
  const [workflowStepCommand, setWorkflowStepCommand] = useState("");
  const [workflowStepConnection, setWorkflowStepConnection] = useState("");
  const [renameValue, setRenameValue] = useState("");
  const dragRef = useRef<{
    x: number;
    y: number;
    panX: number;
    panY: number;
  } | null>(null);

  useEffect(() => {
    setRepositoryFilter(selectedRepositoryId ?? "all");
  }, [selectedRepositoryId]);

  const effectiveSearch = `${globalQuery} ${search}`.trim().toLowerCase();
  const nodeById = useMemo(
    () => new Map(connections.nodes.map((node) => [node.id, node])),
    [connections.nodes],
  );
  const workflowById = useMemo(
    () =>
      new Map(connections.workflows.map((workflow) => [workflow.id, workflow])),
    [connections.workflows],
  );
  const selectedWorkflow =
    selection?.kind === "workflow"
      ? workflowById.get(selection.id)
      : workflowFilter !== "all"
        ? workflowById.get(workflowFilter)
        : undefined;

  const visibleGraph = useMemo(() => {
    const activeNodes = connections.nodes.filter((node) => {
      if (node.status === "Hidden") return false;
      if (!showStale && node.status === "Stale") return false;
      if (nodeKindFilter !== "all" && node.kind !== nodeKindFilter)
        return false;
      if (originFilter !== "all" && node.origin !== originFilter) return false;
      if (
        confidenceFilter !== "all" &&
        node.confidence.toLowerCase() !== confidenceFilter.toLowerCase()
      )
        return false;
      if (!effectiveSearch) return true;
      return [node.label, node.identity, node.kind]
        .join(" ")
        .toLowerCase()
        .includes(effectiveSearch);
    });

    const relationshipMatches = connections.connections.filter((connection) => {
      if (
        connection.status === "Hidden" ||
        connection.review_state === "Hidden"
      )
        return false;
      if (!showStale && connection.status === "Stale") return false;
      if (
        relationshipFilter !== "all" &&
        connection.relationship_type !== relationshipFilter
      )
        return false;
      if (originFilter !== "all" && connection.origin !== originFilter)
        return false;
      if (
        confidenceFilter !== "all" &&
        connection.confidence.toLowerCase() !== confidenceFilter.toLowerCase()
      )
        return false;
      if (!effectiveSearch) return true;
      const source = nodeById.get(connection.source_node_id);
      const target = nodeById.get(connection.target_node_id);
      return [
        connection.relationship_type,
        connection.label,
        connection.fingerprint,
        source ? displayNode(source) : "",
        target ? displayNode(target) : "",
      ]
        .join(" ")
        .toLowerCase()
        .includes(effectiveSearch);
    });

    const workflowMatches = connections.workflows.filter((workflow) => {
      if (workflow.status === "Hidden" || workflow.review_state === "Hidden")
        return false;
      if (!showStale && workflow.status === "Stale") return false;
      if (workflowFilter !== "all" && workflow.id !== workflowFilter)
        return false;
      if (originFilter !== "all" && workflow.origin !== originFilter)
        return false;
      if (
        effectiveSearch &&
        !displayWorkflow(workflow).toLowerCase().includes(effectiveSearch)
      )
        return false;
      return true;
    });

    const scopedRepositoryIds = new Set(
      activeNodes
        .filter((node) => node.repository_id === repositoryFilter)
        .map((node) => node.id),
    );
    const neighborhoodIds = new Set(scopedRepositoryIds);
    if (repositoryFilter !== "all") {
      relationshipMatches.forEach((connection) => {
        if (
          scopedRepositoryIds.has(connection.source_node_id) ||
          scopedRepositoryIds.has(connection.target_node_id)
        ) {
          neighborhoodIds.add(connection.source_node_id);
          neighborhoodIds.add(connection.target_node_id);
        }
      });
    }
    if (selectedWorkflow) {
      selectedWorkflow.steps.forEach((step) =>
        neighborhoodIds.add(step.node_id),
      );
      selectedWorkflow.steps.forEach((step) => {
        if (step.connection_id) {
          const connection = connections.connections.find(
            (candidate) => candidate.id === step.connection_id,
          );
          if (connection) {
            neighborhoodIds.add(connection.source_node_id);
            neighborhoodIds.add(connection.target_node_id);
          }
        }
      });
    }

    const nodes = activeNodes.filter(
      (node) => repositoryFilter === "all" || neighborhoodIds.has(node.id),
    );
    const nodeIds = new Set(nodes.map((node) => node.id));
    const edges = relationshipMatches.filter(
      (connection) =>
        nodeIds.has(connection.source_node_id) &&
        nodeIds.has(connection.target_node_id),
    );
    const workflows = workflowMatches.filter((workflow) => {
      if (repositoryFilter === "all") return true;
      return workflow.participating_repositories.includes(repositoryFilter);
    });
    return { nodes, edges, workflows };
  }, [
    confidenceFilter,
    connections,
    effectiveSearch,
    nodeById,
    nodeKindFilter,
    originFilter,
    relationshipFilter,
    repositoryFilter,
    selectedWorkflow,
    showStale,
    workflowFilter,
  ]);

  const layout = useMemo(() => {
    const grouped = new Map<string, ConnectionNode[]>();
    visibleGraph.nodes.forEach((node) => {
      const group = grouped.get(node.kind) ?? [];
      group.push(node);
      grouped.set(node.kind, group);
    });
    const orderedKinds = [...grouped.keys()].sort((left, right) => {
      const leftIndex = nodeKindOrder.indexOf(left);
      const rightIndex = nodeKindOrder.indexOf(right);
      return (
        (leftIndex < 0 ? 999 : leftIndex) - (rightIndex < 0 ? 999 : rightIndex)
      );
    });
    const positions = new Map<string, { x: number; y: number }>();
    orderedKinds.forEach((kind, column) => {
      const nodes = grouped.get(kind) ?? [];
      nodes
        .sort((left, right) =>
          displayNode(left).localeCompare(displayNode(right)),
        )
        .forEach((node, row) => {
          positions.set(node.id, {
            x: 44 + column * 186,
            y: 52 + row * 92,
          });
        });
    });
    const maxRows = Math.max(
      1,
      ...orderedKinds.map((kind) => grouped.get(kind)?.length ?? 0),
    );
    return {
      positions,
      width: Math.max(980, orderedKinds.length * 186 + 60),
      height: Math.max(500, maxRows * 92 + 82),
      orderedKinds,
    };
  }, [visibleGraph.nodes]);

  const selectedNode =
    selection?.kind === "node" ? nodeById.get(selection.id) : undefined;
  const selectedConnection =
    selection?.kind === "connection"
      ? connections.connections.find(
          (connection) => connection.id === selection.id,
        )
      : undefined;
  const selectedWorkflowForInspector =
    selection?.kind === "workflow" ? workflowById.get(selection.id) : undefined;
  const selectedRecordLabel = selectedNode
    ? displayNode(selectedNode)
    : selectedConnection
      ? displayConnection(selectedConnection)
      : selectedWorkflowForInspector
        ? displayWorkflow(selectedWorkflowForInspector)
        : "";

  useEffect(() => {
    if (selectedNode) setRenameValue(displayNode(selectedNode));
    else if (selectedConnection)
      setRenameValue(displayConnection(selectedConnection));
    else if (selectedWorkflowForInspector)
      setRenameValue(displayWorkflow(selectedWorkflowForInspector));
    else setRenameValue("");
  }, [selectedConnection, selectedNode, selectedWorkflowForInspector]);

  const repositoryOptions = [
    { value: "all", label: "All repositories" },
    ...repositories.map((repository) => ({
      value: repository.id,
      label: repository.name,
    })),
  ];
  const origins = Array.from(
    new Set([
      ...connections.nodes.map((node) => node.origin),
      ...connections.connections.map((connection) => connection.origin),
      ...connections.workflows.map((workflow) => workflow.origin),
    ]),
  ).sort();

  const clearFilters = (): void => {
    setSearch("");
    setRepositoryFilter(selectedRepositoryId ?? "all");
    setNodeKindFilter("all");
    setRelationshipFilter("all");
    setWorkflowFilter("all");
    setOriginFilter("all");
    setConfidenceFilter("all");
    setShowStale(false);
  };

  const handleMapPointerDown = (
    event: ReactPointerEvent<SVGSVGElement>,
  ): void => {
    if (
      event.target instanceof Element &&
      event.target.closest("[data-map-item]")
    )
      return;
    dragRef.current = {
      x: event.clientX,
      y: event.clientY,
      panX: pan.x,
      panY: pan.y,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    setIsDragging(true);
  };

  const handleMapPointerMove = (
    event: ReactPointerEvent<SVGSVGElement>,
  ): void => {
    if (!dragRef.current) return;
    setPan({
      x: dragRef.current.panX + event.clientX - dragRef.current.x,
      y: dragRef.current.panY + event.clientY - dragRef.current.y,
    });
  };

  const handleMapPointerUp = (): void => {
    dragRef.current = null;
    setIsDragging(false);
  };

  const handleSaveNode = async (): Promise<void> => {
    if (!nodeLabel.trim()) return;
    await onSaveNode({
      kind: nodeKind,
      label: nodeLabel.trim(),
      identity: nodeIdentity.trim() || undefined,
      repository_id: nodeRepositoryId || undefined,
    });
    setNodeLabel("");
    setNodeIdentity("");
    setExpandedForm(null);
  };

  const handleSaveConnection = async (): Promise<void> => {
    if (
      !connectionSource ||
      !connectionTarget ||
      connectionSource === connectionTarget
    )
      return;
    await onSaveConnection({
      source_node_id: connectionSource,
      target_node_id: connectionTarget,
      relationship_type: connectionType,
      label: connectionLabel.trim() || undefined,
    });
    setConnectionLabel("");
    setExpandedForm(null);
  };

  const handleAddWorkflowStep = (): void => {
    if (!workflowStepNode || !workflowStepAction.trim()) return;
    setWorkflowSteps((steps) => [
      ...steps,
      {
        nodeId: workflowStepNode,
        actionLabel: workflowStepAction.trim(),
        command: workflowStepCommand.trim(),
        connectionId: workflowStepConnection,
      },
    ]);
    setWorkflowStepAction("");
    setWorkflowStepCommand("");
    setWorkflowStepConnection("");
  };

  const handleSaveWorkflow = async (): Promise<void> => {
    if (!workflowName.trim() || workflowSteps.length === 0) return;
    const steps: WorkflowStepInput[] = workflowSteps.map((step) => ({
      node_id: step.nodeId,
      action_label: step.actionLabel,
      command: step.command || undefined,
      connection_id: step.connectionId || undefined,
    }));
    await onSaveWorkflow({
      name: workflowName.trim(),
      scope: workflowScope,
      repository_ids: Array.from(
        new Set(
          workflowSteps
            .map((step) => nodeById.get(step.nodeId)?.repository_id)
            .filter((repositoryId): repositoryId is string =>
              Boolean(repositoryId),
            ),
        ),
      ),
      steps,
    });
    setWorkflowName("");
    setWorkflowSteps([]);
    setExpandedForm(null);
  };

  const selectRecord = (nextSelection: Selection): void => {
    setSelection(nextSelection);
    if (nextSelection?.kind === "workflow") setWorkflowFilter(nextSelection.id);
  };

  const handleRename = async (): Promise<void> => {
    if (
      !selection ||
      !renameValue.trim() ||
      renameValue.trim() === selectedRecordLabel
    )
      return;
    if (selection.kind === "node")
      await onReview("node", selection.id, "Overridden", renameValue.trim());
    if (selection.kind === "connection")
      await onReview(
        "connection",
        selection.id,
        "Overridden",
        renameValue.trim(),
      );
    if (selection.kind === "workflow")
      await onReview(
        "workflow",
        selection.id,
        "Overridden",
        renameValue.trim(),
      );
  };

  const graphHasRecords = visibleGraph.nodes.length > 0;
  const visibleConnectionCount = visibleGraph.edges.length;
  const hiddenCount = connections.nodes.filter(
    (node) => node.status === "Hidden",
  ).length;
  const staleCount =
    connections.nodes.filter((node) => node.status === "Stale").length +
    connections.connections.filter(
      (connection) => connection.status === "Stale",
    ).length +
    connections.workflows.filter((workflow) => workflow.status === "Stale")
      .length;

  return (
    <section
      className="connections-surface"
      aria-label="Connections and workflows"
    >
      <div className="connections-summary-row">
        <div>
          <p className="eyebrow">Portfolio process map</p>
          <h2>Relationships with receipts.</h2>
          <p className="connections-lede">
            Discoveries stay local, reviewable, and separate from executable
            actions. Multiple edges and workflows can describe the same
            repository boundary.
          </p>
        </div>
        <div className="connections-summary-metrics">
          <div>
            <strong>{connections.nodes.length}</strong>
            <span>nodes</span>
          </div>
          <div>
            <strong>{connections.connections.length}</strong>
            <span>edges</span>
          </div>
          <div>
            <strong>{connections.workflows.length}</strong>
            <span>workflows</span>
          </div>
        </div>
      </div>

      <div className="connections-toolbar">
        <label className="connections-search">
          <Search size={15} />
          <input
            aria-label="Search connections"
            placeholder="Search nodes, edges, workflows"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch("")}
              aria-label="Clear connection search"
            >
              <X size={14} />
            </button>
          )}
        </label>
        <button
          className="button button-secondary"
          type="button"
          onClick={() =>
            void onRefresh(
              repositoryFilter === "all" ? undefined : repositoryFilter,
            )
          }
          disabled={isRefreshing}
        >
          <RefreshCw size={14} className={isRefreshing ? "spin" : undefined} />
          {isRefreshing ? "Refreshing" : "Refresh evidence"}
        </button>
        <button
          className="button button-quiet"
          type="button"
          onClick={clearFilters}
          title="Reset map filters"
        >
          <RotateCcw size={14} />
          Reset
        </button>
      </div>

      <div className="connections-filter-grid">
        <FilterSelect
          label="Repository"
          value={repositoryFilter}
          onChange={setRepositoryFilter}
          options={repositoryOptions}
        />
        <FilterSelect
          label="Node type"
          value={nodeKindFilter}
          onChange={setNodeKindFilter}
          options={[
            { value: "all", label: "All node types" },
            ...nodeKinds.map((kind) => ({
              value: kind,
              label: titleCase(kind),
            })),
          ]}
        />
        <FilterSelect
          label="Relationship"
          value={relationshipFilter}
          onChange={setRelationshipFilter}
          options={[
            { value: "all", label: "All relationships" },
            ...relationshipTypes.map((type) => ({
              value: type,
              label: titleCase(type),
            })),
          ]}
        />
        <FilterSelect
          label="Workflow"
          value={workflowFilter}
          onChange={setWorkflowFilter}
          options={[
            { value: "all", label: "All workflows" },
            ...connections.workflows.map((workflow) => ({
              value: workflow.id,
              label: displayWorkflow(workflow),
            })),
          ]}
        />
        <FilterSelect
          label="Origin"
          value={originFilter}
          onChange={setOriginFilter}
          options={[
            { value: "all", label: "All origins" },
            ...origins.map((origin) => ({
              value: origin,
              label: originLabel(origin),
            })),
          ]}
        />
        <FilterSelect
          label="Confidence"
          value={confidenceFilter}
          onChange={setConfidenceFilter}
          options={[
            { value: "all", label: "All confidence" },
            { value: "High", label: "High" },
            { value: "Medium", label: "Medium" },
            { value: "Low", label: "Low" },
            { value: "Unknown", label: "Unknown" },
          ]}
        />
        <label className="connections-stale-toggle">
          <input
            type="checkbox"
            checked={showStale}
            onChange={(event) => setShowStale(event.target.checked)}
          />
          <span>Show stale ({staleCount})</span>
        </label>
      </div>

      <div className="connections-layout">
        <div className="connections-map-column">
          <div className="connections-map-card">
            <div className="connections-map-header">
              <div>
                <strong>
                  {repositoryFilter === "all"
                    ? "Global map"
                    : "Repository neighborhood"}
                </strong>
                <span>
                  {visibleGraph.nodes.length} visible nodes ·{" "}
                  {visibleConnectionCount} visible relationships
                </span>
              </div>
              <div className="connections-map-controls">
                <IconButtonLike
                  label="Zoom out"
                  onClick={() =>
                    setZoom((value) => Math.max(0.55, value - 0.1))
                  }
                >
                  <Minus size={14} />
                </IconButtonLike>
                <span className="connections-zoom-label">
                  {Math.round(zoom * 100)}%
                </span>
                <IconButtonLike
                  label="Zoom in"
                  onClick={() => setZoom((value) => Math.min(1.8, value + 0.1))}
                >
                  <Plus size={14} />
                </IconButtonLike>
                <IconButtonLike
                  label="Reset map view"
                  onClick={() => {
                    setZoom(1);
                    setPan({ x: 0, y: 0 });
                  }}
                >
                  <RotateCcw size={14} />
                </IconButtonLike>
              </div>
            </div>
            {graphHasRecords ? (
              <div
                className={`connections-map-viewport${isDragging ? " is-dragging" : ""}`}
              >
                <svg
                  className="connections-map"
                  role="img"
                  aria-label="Evidence-backed connections map"
                  viewBox={`0 0 ${layout.width} ${layout.height}`}
                  onPointerDown={handleMapPointerDown}
                  onPointerMove={handleMapPointerMove}
                  onPointerUp={handleMapPointerUp}
                  onPointerCancel={handleMapPointerUp}
                  onWheel={(event) => {
                    event.preventDefault();
                    setZoom((value) =>
                      Math.max(
                        0.55,
                        Math.min(
                          1.8,
                          value + (event.deltaY > 0 ? -0.05 : 0.05),
                        ),
                      ),
                    );
                  }}
                >
                  <defs>
                    <marker
                      id="connections-arrow"
                      viewBox="0 0 10 10"
                      refX="9"
                      refY="5"
                      markerWidth="6"
                      markerHeight="6"
                      orient="auto-start-reverse"
                    >
                      <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
                    </marker>
                  </defs>
                  <g transform={`translate(${pan.x} ${pan.y}) scale(${zoom})`}>
                    {visibleGraph.edges.map((connection, index) => {
                      const source = layout.positions.get(
                        connection.source_node_id,
                      );
                      const target = layout.positions.get(
                        connection.target_node_id,
                      );
                      if (!source || !target) return null;
                      const offset = ((index % 3) - 1) * 7;
                      const x1 = source.x + 142;
                      const y1 = source.y + 25;
                      const x2 = target.x;
                      const y2 = target.y + 25;
                      const color =
                        relationshipColors[connection.relationship_type] ??
                        "#8190a9";
                      return (
                        <g
                          key={connection.id}
                          data-map-item="edge"
                          className={`connections-edge ${selection?.kind === "connection" && selection.id === connection.id ? "is-selected" : ""}`}
                          onClick={(event) => {
                            event.stopPropagation();
                            selectRecord({
                              kind: "connection",
                              id: connection.id,
                            });
                          }}
                        >
                          <path
                            d={`M ${x1} ${y1} C ${x1 + 36} ${y1 + offset}, ${x2 - 36} ${y2 - offset}, ${x2} ${y2}`}
                            stroke={color}
                            markerEnd="url(#connections-arrow)"
                          />
                          <text
                            x={(x1 + x2) / 2}
                            y={(y1 + y2) / 2 - 7}
                            fill={color}
                          >
                            {titleCase(connection.relationship_type)}
                          </text>
                        </g>
                      );
                    })}
                    {visibleGraph.nodes.map((node) => {
                      const position = layout.positions.get(node.id);
                      if (!position) return null;
                      const isSelected =
                        selection?.kind === "node" && selection.id === node.id;
                      return (
                        <g
                          key={node.id}
                          data-map-item="node"
                          role="button"
                          tabIndex={0}
                          aria-label={`${displayNode(node)} ${node.kind}`}
                          className={`connections-node ${isSelected ? "is-selected" : ""}`}
                          transform={`translate(${position.x} ${position.y})`}
                          onClick={(event) => {
                            event.stopPropagation();
                            selectRecord({ kind: "node", id: node.id });
                          }}
                          onKeyDown={(event) => {
                            if (event.key === "Enter" || event.key === " ")
                              selectRecord({ kind: "node", id: node.id });
                          }}
                        >
                          <rect width="142" height="50" rx="10" />
                          <rect
                            className="connections-node-accent"
                            width="4"
                            height="50"
                            rx="2"
                            fill={relationshipColors[node.kind] ?? "#8b9dbb"}
                          />
                          <text className="connections-node-kind" x="14" y="16">
                            {titleCase(node.kind)}
                          </text>
                          <text
                            className="connections-node-label"
                            x="14"
                            y="34"
                          >
                            {displayNode(node).slice(0, 21)}
                            {displayNode(node).length > 21 ? "…" : ""}
                          </text>
                          <title>{`${displayNode(node)} · ${originLabel(node.origin)} · ${node.confidence} confidence`}</title>
                        </g>
                      );
                    })}
                  </g>
                </svg>
              </div>
            ) : (
              <div className="connections-empty-map">
                <div className="connections-empty-icon">
                  <GitBranch size={22} />
                </div>
                <div>
                  <p className="eyebrow">No visible evidence</p>
                  <h3>
                    {connections.nodes.length === 0
                      ? "Refresh to discover relationships."
                      : "No records match these filters."}
                  </h3>
                  <p>
                    {connections.nodes.length === 0
                      ? "Static discovery reads local repository configuration and keeps every result traceable."
                      : "Stale and hidden records stay out of the default map. Reset filters or show stale evidence to inspect them."}
                  </p>
                </div>
                <button
                  className="button button-secondary"
                  type="button"
                  onClick={
                    connections.nodes.length === 0
                      ? () => void onRefresh()
                      : clearFilters
                  }
                >
                  {connections.nodes.length === 0
                    ? "Refresh evidence"
                    : "Reset filters"}
                </button>
              </div>
            )}
            <div className="connections-legend">
              <span>
                <SlidersHorizontal size={13} /> Relationships
              </span>
              {relationshipTypes.slice(0, 6).map((type) => (
                <button
                  key={type}
                  type="button"
                  onClick={() => setRelationshipFilter(type)}
                >
                  <i style={{ backgroundColor: relationshipColors[type] }} />
                  {titleCase(type)}
                </button>
              ))}
              <span className="connections-legend-hint">
                Drag to pan · scroll to zoom
              </span>
            </div>
          </div>

          <div className="connections-lower-grid">
            <div className="connections-panel">
              <div className="connections-panel-heading">
                <div>
                  <p className="eyebrow">Workflow-first</p>
                  <h3>Reusable flows</h3>
                </div>
                <WorkflowIcon size={18} />
              </div>
              {visibleGraph.workflows.length === 0 ? (
                <p className="connections-muted">
                  No workflows in this scope yet.
                </p>
              ) : (
                <div className="connections-workflow-list">
                  {visibleGraph.workflows.map((workflow) => (
                    <button
                      className={`connections-workflow-row ${selection?.kind === "workflow" && selection.id === workflow.id ? "is-selected" : ""}`}
                      type="button"
                      key={workflow.id}
                      onClick={() =>
                        selectRecord({ kind: "workflow", id: workflow.id })
                      }
                    >
                      <span>
                        <strong>{displayWorkflow(workflow)}</strong>
                        <small>
                          {workflow.steps.length} ordered steps ·{" "}
                          {workflow.participating_repositories.length} repos
                          {workflow.steps.length > 0 &&
                            ` · ${workflow.steps.map((step) => step.action_label).join(" → ")}`}
                        </small>
                      </span>
                      <ChevronRight size={15} />
                    </button>
                  ))}
                </div>
              )}
            </div>
            <div className="connections-panel">
              <div className="connections-panel-heading">
                <div>
                  <p className="eyebrow">Adapter posture</p>
                  <h3>Evidence sources</h3>
                </div>
                <Sparkles size={18} />
              </div>
              <div className="connections-adapter-list">
                {connections.adapters.map((adapter) => (
                  <label className="connections-adapter-row" key={adapter.id}>
                    <input
                      type="checkbox"
                      checked={adapter.enabled}
                      onChange={(event) =>
                        void onToggleAdapter(adapter.id, event.target.checked)
                      }
                    />
                    <span>
                      <strong>{titleCase(adapter.id)}</strong>
                      <small>
                        {adapter.permission_state} · {adapter.freshness}
                      </small>
                      {adapter.failure_message && (
                        <em>{adapter.failure_message}</em>
                      )}
                    </span>
                  </label>
                ))}
              </div>
            </div>
          </div>
        </div>

        <aside className="connections-inspector-column">
          <div className="connections-inspector">
            <div className="connections-panel-heading">
              <div>
                <p className="eyebrow">Evidence inspector</p>
                <h3>{selectedRecordLabel || "Select a map record"}</h3>
              </div>
              {selection && (
                <button
                  className="button button-quiet"
                  type="button"
                  onClick={() => setSelection(null)}
                >
                  Clear
                </button>
              )}
            </div>
            {selectedNode ? (
              <NodeInspector
                node={selectedNode}
                repositories={repositories}
                onOpenRepository={onOpenRepository}
                onReview={onReview}
                onDelete={onDeleteNode}
                renameValue={renameValue}
                setRenameValue={setRenameValue}
                onRename={handleRename}
              />
            ) : selectedConnection ? (
              <ConnectionInspector
                connection={selectedConnection}
                source={nodeById.get(selectedConnection.source_node_id)}
                target={nodeById.get(selectedConnection.target_node_id)}
                onReview={onReview}
                onDelete={onDeleteConnection}
                renameValue={renameValue}
                setRenameValue={setRenameValue}
                onRename={handleRename}
              />
            ) : selectedWorkflowForInspector ? (
              <WorkflowInspector
                workflow={selectedWorkflowForInspector}
                nodeById={nodeById}
                onReview={onReview}
                onDelete={onDeleteWorkflow}
                renameValue={renameValue}
                setRenameValue={setRenameValue}
                onRename={handleRename}
              />
            ) : (
              <div className="connections-inspector-empty">
                <EyeOff size={18} />
                <p>
                  Every edge and step keeps its source, freshness, and
                  confidence here.
                </p>
                <span>
                  Choose a node, relationship, or workflow on the map.
                </span>
              </div>
            )}
          </div>

          <div className="connections-panel connections-manual-panel">
            <div className="connections-panel-heading">
              <div>
                <p className="eyebrow">Local-only editing</p>
                <h3>Add a record</h3>
              </div>
              <StatusPill tone="slate">Never executes</StatusPill>
            </div>
            <div className="connections-form-tabs">
              {(["node", "connection", "workflow"] as const).map((form) => (
                <button
                  className={expandedForm === form ? "is-active" : ""}
                  type="button"
                  key={form}
                  onClick={() =>
                    setExpandedForm(expandedForm === form ? null : form)
                  }
                >
                  {form === "node"
                    ? "Node"
                    : form === "connection"
                      ? "Edge"
                      : "Workflow"}
                </button>
              ))}
            </div>
            {expandedForm === "node" && (
              <div className="connections-form">
                <input
                  className="connection-input"
                  placeholder="Label"
                  value={nodeLabel}
                  onChange={(event) => setNodeLabel(event.target.value)}
                />
                <div className="connections-form-row">
                  <select
                    className="drawer-select"
                    value={nodeKind}
                    onChange={(event) => setNodeKind(event.target.value)}
                    aria-label="Manual node type"
                  >
                    {nodeKinds.map((kind) => (
                      <option value={kind} key={kind}>
                        {titleCase(kind)}
                      </option>
                    ))}
                  </select>
                  <select
                    className="drawer-select"
                    value={nodeRepositoryId}
                    onChange={(event) =>
                      setNodeRepositoryId(event.target.value)
                    }
                    aria-label="Manual node repository"
                  >
                    <option value="">Portfolio-wide</option>
                    {repositories.map((repository) => (
                      <option value={repository.id} key={repository.id}>
                        {repository.name}
                      </option>
                    ))}
                  </select>
                </div>
                <input
                  className="connection-input"
                  placeholder="Stable identity (optional)"
                  value={nodeIdentity}
                  onChange={(event) => setNodeIdentity(event.target.value)}
                />
                <button
                  className="button button-primary"
                  type="button"
                  disabled={!nodeLabel.trim()}
                  onClick={() => void handleSaveNode()}
                >
                  <Plus size={14} /> Add node
                </button>
              </div>
            )}
            {expandedForm === "connection" && (
              <div className="connections-form">
                <div className="connections-form-row">
                  <select
                    className="drawer-select"
                    value={connectionSource}
                    onChange={(event) =>
                      setConnectionSource(event.target.value)
                    }
                    aria-label="Connection source"
                  >
                    <option value="">Source node</option>
                    {connections.nodes
                      .filter((node) => node.status !== "Hidden")
                      .map((node) => (
                        <option value={node.id} key={node.id}>
                          {displayNode(node)}
                        </option>
                      ))}
                  </select>
                  <select
                    className="drawer-select"
                    value={connectionTarget}
                    onChange={(event) =>
                      setConnectionTarget(event.target.value)
                    }
                    aria-label="Connection target"
                  >
                    <option value="">Target node</option>
                    {connections.nodes
                      .filter((node) => node.status !== "Hidden")
                      .map((node) => (
                        <option value={node.id} key={node.id}>
                          {displayNode(node)}
                        </option>
                      ))}
                  </select>
                </div>
                <div className="connections-form-row">
                  <select
                    className="drawer-select"
                    value={connectionType}
                    onChange={(event) => setConnectionType(event.target.value)}
                    aria-label="Relationship type"
                  >
                    {relationshipTypes.map((type) => (
                      <option value={type} key={type}>
                        {titleCase(type)}
                      </option>
                    ))}
                  </select>
                  <input
                    className="connection-input"
                    placeholder="Label (optional)"
                    value={connectionLabel}
                    onChange={(event) => setConnectionLabel(event.target.value)}
                  />
                </div>
                <button
                  className="button button-primary"
                  type="button"
                  disabled={
                    !connectionSource ||
                    !connectionTarget ||
                    connectionSource === connectionTarget
                  }
                  onClick={() => void handleSaveConnection()}
                >
                  <Plus size={14} /> Add edge
                </button>
              </div>
            )}
            {expandedForm === "workflow" && (
              <div className="connections-form">
                <div className="connections-form-row">
                  <input
                    className="connection-input"
                    placeholder="Workflow name"
                    value={workflowName}
                    onChange={(event) => setWorkflowName(event.target.value)}
                  />
                  <select
                    className="drawer-select"
                    value={workflowScope}
                    onChange={(event) => setWorkflowScope(event.target.value)}
                    aria-label="Workflow scope"
                  >
                    <option>Cross-repository</option>
                    <option>Local</option>
                  </select>
                </div>
                <div className="connections-workflow-draft">
                  {workflowSteps.map((step, index) => (
                    <div
                      className="connections-draft-step"
                      key={`${step.nodeId}-${index}`}
                    >
                      <span>{index + 1}</span>
                      <strong>{step.actionLabel}</strong>
                      <small>
                        {displayNode(
                          nodeById.get(step.nodeId) ??
                            ({ label: step.nodeId } as ConnectionNode),
                        )}
                      </small>
                      <button
                        type="button"
                        aria-label={`Remove workflow step ${index + 1}`}
                        onClick={() =>
                          setWorkflowSteps((steps) =>
                            steps.filter((_, stepIndex) => stepIndex !== index),
                          )
                        }
                      >
                        <X size={13} />
                      </button>
                    </div>
                  ))}
                </div>
                <select
                  className="drawer-select"
                  value={workflowStepNode}
                  onChange={(event) => setWorkflowStepNode(event.target.value)}
                  aria-label="Workflow step node"
                >
                  <option value="">Step node</option>
                  {connections.nodes
                    .filter((node) => node.status !== "Hidden")
                    .map((node) => (
                      <option value={node.id} key={node.id}>
                        {displayNode(node)}
                      </option>
                    ))}
                </select>
                <input
                  className="connection-input"
                  placeholder="Action label"
                  value={workflowStepAction}
                  onChange={(event) =>
                    setWorkflowStepAction(event.target.value)
                  }
                />
                <input
                  className="connection-input"
                  placeholder="Exact command (optional, redacted on save)"
                  value={workflowStepCommand}
                  onChange={(event) =>
                    setWorkflowStepCommand(event.target.value)
                  }
                />
                <select
                  className="drawer-select"
                  value={workflowStepConnection}
                  onChange={(event) =>
                    setWorkflowStepConnection(event.target.value)
                  }
                  aria-label="Reusable workflow connection"
                >
                  <option value="">Reusable edge (optional)</option>
                  {connections.connections.map((connection) => (
                    <option value={connection.id} key={connection.id}>
                      {displayConnection(connection)}
                    </option>
                  ))}
                </select>
                <div className="connections-form-actions">
                  <button
                    className="button button-secondary"
                    type="button"
                    disabled={!workflowStepNode || !workflowStepAction.trim()}
                    onClick={handleAddWorkflowStep}
                  >
                    <Plus size={14} /> Add step
                  </button>
                  <button
                    className="button button-primary"
                    type="button"
                    disabled={
                      !workflowName.trim() || workflowSteps.length === 0
                    }
                    onClick={() => void handleSaveWorkflow()}
                  >
                    <Check size={14} /> Save workflow
                  </button>
                </div>
              </div>
            )}
          </div>
          {hiddenCount > 0 && (
            <p className="connections-hidden-note">
              <EyeOff size={14} /> {hiddenCount} hidden record
              {hiddenCount === 1 ? "" : "s"} remain available through review.
            </p>
          )}
        </aside>
      </div>
    </section>
  );
}

function IconButtonLike({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: ReactElement;
}): ReactElement {
  return (
    <button
      className="connections-icon-button"
      type="button"
      aria-label={label}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function ReviewControls({
  recordType,
  recordId,
  reviewState,
  canDelete,
  onReview,
  onDelete,
  renameValue,
  setRenameValue,
  onRename,
}: {
  recordType: "node" | "connection" | "workflow";
  recordId: string;
  reviewState: string;
  canDelete: boolean;
  onReview: ConnectionsSurfaceProps["onReview"];
  onDelete: () => Promise<void>;
  renameValue: string;
  setRenameValue: (value: string) => void;
  onRename: () => Promise<void>;
}): ReactElement {
  return (
    <div className="connections-review-controls">
      <div className="connections-review-actions">
        <button
          className="button button-secondary"
          type="button"
          onClick={() => void onReview(recordType, recordId, "Confirmed")}
        >
          <Check size={14} /> Confirm
        </button>
        <button
          className="button button-quiet"
          type="button"
          onClick={() => void onReview(recordType, recordId, "Hidden")}
        >
          <EyeOff size={14} /> Hide
        </button>
        {reviewState === "Hidden" && (
          <button
            className="button button-quiet"
            type="button"
            onClick={() => void onReview(recordType, recordId, "Suggested")}
          >
            <RotateCcw size={14} /> Restore
          </button>
        )}
      </div>
      <label className="connections-rename-field">
        <span>Local label override</span>
        <div>
          <input
            className="connection-input"
            value={renameValue}
            onChange={(event) => setRenameValue(event.target.value)}
          />
          <button
            className="button button-secondary"
            type="button"
            disabled={!renameValue.trim()}
            onClick={() => void onRename()}
          >
            Rename
          </button>
        </div>
      </label>
      {canDelete ? (
        <button
          className="connections-delete-button"
          type="button"
          onClick={() => void onDelete()}
        >
          Delete manual record
        </button>
      ) : (
        <p className="connections-muted">
          Discovered records can be reviewed or hidden; their source evidence
          remains intact.
        </p>
      )}
    </div>
  );
}

function NodeInspector({
  node,
  repositories,
  onOpenRepository,
  onReview,
  onDelete,
  renameValue,
  setRenameValue,
  onRename,
}: {
  node: ConnectionNode;
  repositories: RepositorySnapshot[];
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onReview: ConnectionsSurfaceProps["onReview"];
  onDelete: (id: string) => Promise<void>;
  renameValue: string;
  setRenameValue: (value: string) => void;
  onRename: () => Promise<void>;
}): ReactElement {
  const repository = node.repository_id
    ? repositories.find((candidate) => candidate.id === node.repository_id)
    : undefined;
  return (
    <div className="connections-inspector-body">
      <div className="connections-inspector-badges">
        <StatusPill tone={statusTone(node.status)}>{node.status}</StatusPill>
        <StatusPill tone={confidenceTone(node.confidence)}>
          {node.confidence} confidence
        </StatusPill>
        <StatusPill tone="slate">{originLabel(node.origin)}</StatusPill>
      </div>
      <dl className="connections-detail-list">
        <div>
          <dt>Kind</dt>
          <dd>{titleCase(node.kind)}</dd>
        </div>
        <div>
          <dt>Stable identity</dt>
          <dd>
            <code>{node.identity}</code>
          </dd>
        </div>
        <div>
          <dt>Last seen</dt>
          <dd>{formatTime(node.last_seen_at)}</dd>
        </div>
      </dl>
      {repository && (
        <button
          className="button button-secondary connections-open-repository"
          type="button"
          onClick={() => onOpenRepository(repository)}
        >
          <GitBranch size={14} /> Open {repository.name}
        </button>
      )}
      <div className="connections-inspector-section">
        <h4>Evidence</h4>
        <EvidenceList evidence={node.evidence} />
      </div>
      <ReviewControls
        recordType="node"
        recordId={node.id}
        reviewState={node.status}
        canDelete={node.origin === "Manual"}
        onReview={onReview}
        onDelete={() => onDelete(node.id)}
        renameValue={renameValue}
        setRenameValue={setRenameValue}
        onRename={onRename}
      />
    </div>
  );
}

function ConnectionInspector({
  connection,
  source,
  target,
  onReview,
  onDelete,
  renameValue,
  setRenameValue,
  onRename,
}: {
  connection: Connection;
  source?: ConnectionNode;
  target?: ConnectionNode;
  onReview: ConnectionsSurfaceProps["onReview"];
  onDelete: (id: string) => Promise<void>;
  renameValue: string;
  setRenameValue: (value: string) => void;
  onRename: () => Promise<void>;
}): ReactElement {
  return (
    <div className="connections-inspector-body">
      <div className="connections-inspector-badges">
        <StatusPill tone="blue">
          {titleCase(connection.relationship_type)}
        </StatusPill>
        <StatusPill tone={statusTone(connection.status)}>
          {connection.status}
        </StatusPill>
        <StatusPill tone={confidenceTone(connection.confidence)}>
          {connection.confidence} confidence
        </StatusPill>
      </div>
      <div className="connections-route-card">
        <span>{source ? displayNode(source) : connection.source_node_id}</span>
        <ChevronRight size={15} />
        <span>{target ? displayNode(target) : connection.target_node_id}</span>
      </div>
      <dl className="connections-detail-list">
        <div>
          <dt>Relationship</dt>
          <dd>{displayConnection(connection)}</dd>
        </div>
        <div>
          <dt>Fingerprint</dt>
          <dd>
            <code>{connection.fingerprint}</code>
          </dd>
        </div>
        <div>
          <dt>Last seen</dt>
          <dd>{formatTime(connection.last_seen_at)}</dd>
        </div>
      </dl>
      <div className="connections-inspector-section">
        <h4>Evidence</h4>
        <EvidenceList evidence={connection.evidence} />
      </div>
      <ReviewControls
        recordType="connection"
        recordId={connection.id}
        reviewState={connection.review_state}
        canDelete={connection.origin === "Manual"}
        onReview={onReview}
        onDelete={() => onDelete(connection.id)}
        renameValue={renameValue}
        setRenameValue={setRenameValue}
        onRename={onRename}
      />
    </div>
  );
}

export function WorkflowInspector({
  workflow,
  nodeById,
  onReview,
  onDelete,
  renameValue,
  setRenameValue,
  onRename,
}: {
  workflow: Workflow;
  nodeById: Map<string, ConnectionNode>;
  onReview: ConnectionsSurfaceProps["onReview"];
  onDelete: (id: string) => Promise<void>;
  renameValue: string;
  setRenameValue: (value: string) => void;
  onRename: () => Promise<void>;
}): ReactElement {
  return (
    <div className="connections-inspector-body">
      <div className="connections-inspector-badges">
        <StatusPill tone="violet">{workflow.scope}</StatusPill>
        <StatusPill tone={statusTone(workflow.status)}>
          {workflow.status}
        </StatusPill>
        <StatusPill tone="slate">{originLabel(workflow.origin)}</StatusPill>
      </div>
      <p className="connections-workflow-summary">
        {workflow.steps.length} ordered step
        {workflow.steps.length === 1 ? "" : "s"} across{" "}
        {workflow.participating_repositories.length} repositories.
      </p>
      <ol className="connections-inspector-steps">
        {workflow.steps.map((step) => (
          <li key={step.id}>
            <span>{step.order + 1}</span>
            <div>
              <strong>{step.action_label}</strong>
              <small>
                {nodeById.get(step.node_id)
                  ? displayNode(nodeById.get(step.node_id) as ConnectionNode)
                  : step.node_id}
              </small>
              {step.command && <code>{step.command}</code>}
            </div>
          </li>
        ))}
      </ol>
      <div className="connections-inspector-section">
        <h4>Evidence</h4>
        <EvidenceList evidence={workflow.evidence} />
      </div>
      <ReviewControls
        recordType="workflow"
        recordId={workflow.id}
        reviewState={workflow.review_state}
        canDelete={workflow.origin === "Manual"}
        onReview={onReview}
        onDelete={() => onDelete(workflow.id)}
        renameValue={renameValue}
        setRenameValue={setRenameValue}
        onRename={onRename}
      />
    </div>
  );
}
