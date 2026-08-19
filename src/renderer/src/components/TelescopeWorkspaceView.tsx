import { Background, Controls, MiniMap, ReactFlow } from "@xyflow/react";
import {
  Activity,
  Boxes,
  CircleAlert,
  GitBranch,
  LoaderCircle,
  LocateFixed,
  Pause,
  Play,
  RefreshCw,
  Rocket,
  ShieldCheck,
  Target,
  Wrench,
} from "lucide-react";
import { useEffect, useState, type ReactElement } from "react";
import * as api from "../api";
import { formatTime } from "./ConsolePrimitives";
import {
  TelescopeEntityNode,
  TelescopeGroupNode,
} from "./TelescopeGraphElements";
import { TelescopeFlowEdge } from "./TelescopeFlowEdge";
import { TelescopeActionPalette } from "./TelescopeActionPalette";
import { TelescopeInspector } from "./TelescopeInspector";
import { TelescopeNavigator } from "./TelescopeNavigator";
import {
  resolveSelection,
  sourceEdgeIdsForLayoutEdge,
  summarizeLens,
} from "./telescopeSurfaceUtils";
import type { TelescopeWorkspaceProps } from "./telescopeSurfaceTypes";
import type { TelescopeWorkspaceModel } from "./useTelescopeWorkspaceModel";
import type {
  TelescopeAction,
  TelescopeLens,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";
import type { TelescopeSceneLevel } from "./telescopeSceneModel";

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

export function TelescopeWorkspaceView({
  repository,
  remediation,
  events,
  onOpenWorkspace,
  onPrepareRepository,
  model,
}: TelescopeWorkspaceProps & { model: TelescopeWorkspaceModel }): ReactElement {
  const {
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
    onNodesChange,
    onEdgesChange,
    reducedMotion,
    fitView,
    setViewport,
    scene,
    primaryFlow,
    activeAction,
    remediationAffectedNodeIds,
    visibleNodes,
    visibleEdges,
    load,
    selectOrderedNode,
  } = model;
  const [tourIndex, setTourIndex] = useState(0);

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
  const selectedSourceNode =
    selection?.kind === "node"
      ? (projection.nodes.find((node) => node.id === selection.id) ?? null)
      : null;
  const selectedDistrict =
    selection?.kind === "group"
      ? (projection.groups.find((group) => group.id === selection.id) ?? null)
      : selectedSourceNode
        ? (projection.groups.find(
            (group) => group.id === selectedSourceNode.group_id,
          ) ?? null)
        : null;
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
            {scene?.buildings.length ?? 0} buildings ·{" "}
            {scene?.rails.length ?? 0} rails
          </span>
          <span>
            {projection.coverage.supported_source_files}/
            {projection.coverage.examined_source_files} adapted
          </span>
          <span>Narrative {projection.narrative?.status ?? "missing"}</span>
          <span>Map {projection.map_readiness?.state ?? "measured"}</span>
          {scene?.clusteredSourceNodeCount ? (
            <span>
              Source detail groups {scene.clusteredSourceNodeCount} entities for
              a responsive canvas
            </span>
          ) : null}
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
      <TelescopeActionPalette
        actions={projection.actions}
        coverage={projection.action_coverage}
        query={actionQuery}
        selectedActionId={activeAction?.id ?? null}
        onQuery={setActionQuery}
        onSelect={(actionId) => {
          setSelection({ kind: "action", id: actionId });
          setSceneLevel("subsystems");
          setTourIndex(0);
          setCollapsedGroups(new Set());
        }}
        onClear={() => {
          setSelection(null);
          setSceneLevel("overview");
        }}
      />
      {projection.map_readiness &&
        projection.map_readiness.state !== "reviewed" && (
          <TelescopeMapWorkshop
            projection={projection}
            onPrepare={() =>
              onPrepareRepository?.(repository.workspace.id) ??
              Promise.resolve()
            }
          />
        )}
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
        <div className="telescope-scene-levels" aria-label="Map detail level">
          {(
            [
              ["overview", "Overview"],
              ["subsystems", "Subsystems"],
              ["source", "Source detail"],
            ] as Array<[TelescopeSceneLevel, string]>
          ).map(([level, label]) => (
            <button
              className={sceneLevel === level ? "active" : ""}
              type="button"
              aria-pressed={sceneLevel === level}
              key={level}
              onClick={() => {
                if (level === "source" && !selectedSourceNode) {
                  const fallbackNodeId =
                    primaryFlow?.node_ids[0] ?? projection.nodes[0]?.id;
                  if (fallbackNodeId) {
                    setSelection({ kind: "node", id: fallbackNodeId });
                  }
                }
                setSceneLevel(level);
              }}
            >
              {label}
            </button>
          ))}
        </div>
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
      <nav className="telescope-breadcrumbs" aria-label="Map location">
        <button
          type="button"
          onClick={() => {
            setSelection(null);
            setSceneLevel("overview");
          }}
        >
          City
        </button>
        {sceneLevel !== "overview" && selectedDistrict && (
          <>
            <span>›</span>
            <button
              type="button"
              onClick={() => {
                setSelection({ kind: "group", id: selectedDistrict.id });
                setSceneLevel("subsystems");
              }}
            >
              {selectedDistrict.label}
            </button>
          </>
        )}
        {sceneLevel === "source" && selectedSourceNode && (
          <>
            <span>›</span>
            <strong>{selectedSourceNode.label}</strong>
          </>
        )}
      </nav>
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
        <div
          className={`telescope-canvas ${sceneLevel === "source" ? "is-source-detail" : ""}`}
          aria-label="Architecture canvas"
        >
          <ReactFlow
            nodes={visibleNodes}
            edges={visibleEdges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeClick={(_, node) => {
              const district = scene?.districts.find(
                (candidate) => candidate.id === node.id,
              );
              if (district) {
                setSelection({ kind: "group", id: district.sourceGroupId });
                return;
              }
              const building = scene?.buildings.find(
                (candidate) => candidate.id === node.id,
              );
              if (building?.sourceNodeIds[0]) {
                setSelection({
                  kind: "node",
                  id: building.sourceNodeIds[0],
                });
                return;
              }
              setSelection({
                kind: projection.groups.some((group) => group.id === node.id)
                  ? "group"
                  : "node",
                id: node.id,
              });
            }}
            onEdgeClick={(_, edge) => {
              const sourceEdgeIds = sourceEdgeIdsForLayoutEdge(
                edge,
                scene,
                projection,
              );
              const flow = projection.flows.find((candidate) =>
                candidate.edge_ids.some((id) => sourceEdgeIds.includes(id)),
              );
              setSelection(
                flow
                  ? { kind: "flow", id: flow.id }
                  : sourceEdgeIds[0]
                    ? { kind: "edge", id: sourceEdgeIds[0] }
                    : { kind: "edge", id: edge.id },
              );
            }}
            onPaneClick={() => setSelection(null)}
            fitView
            nodesDraggable={false}
            nodesConnectable={false}
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
          {sceneLevel === "source" && (
            <TelescopeSourceDetail
              node={selectedSourceNode}
              projection={projection}
              onOpenWorkspace={() =>
                onOpenWorkspace(repository.workspace.id, "editor")
              }
            />
          )}
          {activeAction && sceneLevel !== "source" && (
            <TelescopeActionTour
              action={activeAction}
              projection={projection}
              index={tourIndex}
              onIndex={setTourIndex}
              onExit={() => {
                setSelection(null);
                setSceneLevel("overview");
              }}
            />
          )}
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
                  className={
                    selection?.id === flow.id ||
                    (!selection && primaryFlow?.id === flow.id)
                      ? "active"
                      : ""
                  }
                  type="button"
                  aria-label={`Flow: ${flow.label}`}
                  key={flow.id}
                  onClick={() => setSelection({ kind: "flow", id: flow.id })}
                >
                  <span />{" "}
                  {flow.primary || primaryFlow?.id === flow.id ? "★ " : ""}
                  {flow.label}
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
          {sceneLevel !== "overview" && scene?.hiddenSourceNodeCount ? (
            <span>
              {scene.hiddenSourceNodeCount} unrelated source entities are hidden
              from this semantic scope and remain available through navigation.
            </span>
          ) : null}
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

function TelescopeActionTour({
  action,
  projection,
  index,
  onIndex,
  onExit,
}: {
  action: TelescopeAction;
  projection: TelescopeProjection;
  index: number;
  onIndex: (index: number) => void;
  onExit: () => void;
}): ReactElement {
  const stops = action.node_ids
    .map((nodeId) => projection.nodes.find((node) => node.id === nodeId))
    .filter((node): node is TelescopeNode => Boolean(node));
  const safeIndex = Math.min(index, Math.max(0, stops.length - 1));
  const stop = stops[safeIndex];
  const step = action.explanation?.steps[safeIndex];
  return (
    <section
      className="telescope-action-tour"
      aria-label={`${action.label} guided city story`}
    >
      <div>
        <span className="eyebrow">Guided city story</span>
        <strong>{action.label}</strong>
        <small>
          {stops.length
            ? `Stop ${safeIndex + 1} of ${stops.length}`
            : "Action overview"}
        </small>
      </div>
      <h3>{stop?.label ?? "Mapped neighborhood"}</h3>
      <p>{step ?? stop?.semantic_summary ?? action.what_it_does}</p>
      {stop?.source_anchors[0] && (
        <code>
          {stop.source_anchors[0].path}
          {stop.source_anchors[0].line
            ? `:${stop.source_anchors[0].line}`
            : ""}
        </code>
      )}
      <div>
        <button
          type="button"
          disabled={safeIndex === 0}
          onClick={() => onIndex(safeIndex - 1)}
        >
          Previous
        </button>
        <button
          type="button"
          disabled={safeIndex >= stops.length - 1}
          onClick={() => onIndex(safeIndex + 1)}
        >
          Next stop
        </button>
        <button type="button" onClick={onExit}>
          Leave tour
        </button>
      </div>
    </section>
  );
}

function TelescopeSourceDetail({
  node,
  projection,
  onOpenWorkspace,
}: {
  node: TelescopeNode | null;
  projection: TelescopeProjection;
  onOpenWorkspace: () => Promise<void>;
}): ReactElement {
  const [visibleEvidence, setVisibleEvidence] = useState(10);
  useEffect(() => {
    setVisibleEvidence(10);
  }, [node?.id]);

  if (!node) {
    return (
      <aside
        className="telescope-source-detail"
        aria-label="Building source detail"
      >
        <span className="eyebrow">Enter a building</span>
        <h2>Select a building to inspect its local implementation</h2>
        <p>
          Source detail intentionally keeps the view building-local. Choose a
          building to inspect its files, symbols, and immediate handoffs.
        </p>
      </aside>
    );
  }

  const relationships = projection.edges.filter(
    (edge) => edge.source === node.id || edge.target === node.id,
  );
  const evidence = [...node.source_anchors].sort(
    (left, right) =>
      left.path.localeCompare(right.path) ||
      (left.line ?? 0) - (right.line ?? 0),
  );
  return (
    <aside
      className="telescope-source-detail"
      aria-label={`${node.label} source detail`}
    >
      <div className="telescope-source-heading">
        <div>
          <span className="eyebrow">Inside this building</span>
          <h2>{node.label}</h2>
          <p>{node.implementation_summary}</p>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => void onOpenWorkspace()}
        >
          Open source
        </button>
      </div>
      <div className="telescope-source-sections">
        <section>
          <h3>Behavioral steps</h3>
          {node.explanation?.steps?.length ? (
            <ol>
              {node.explanation.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          ) : (
            <p className="telescope-source-empty">
              No reviewed behavioral steps yet.
            </p>
          )}
        </section>
        <section>
          <h3>Immediate handoffs</h3>
          <ul>
            {relationships.map((edge) => {
              const peerId = edge.source === node.id ? edge.target : edge.source;
              const peer = projection.nodes.find(
                (candidate) => candidate.id === peerId,
              );
              return (
                <li key={edge.id}>
                  <strong>
                    {edge.source === node.id ? "Sends to" : "Receives from"} {" "}
                    {peer?.label ?? peerId}
                  </strong>
                  <span>
                    {edge.label} · {edge.confidence} confidence
                  </span>
                </li>
              );
            })}
            {relationships.length === 0 && (
              <li>No source-backed handoffs were extracted.</li>
            )}
          </ul>
        </section>
        <section>
          <h3>Symbols and data</h3>
          <div className="telescope-source-chips">
            {[...node.symbols, ...node.data_shapes].map((item) => (
              <code key={item}>{item}</code>
            ))}
            {node.symbols.length + node.data_shapes.length === 0 && (
              <span>No named symbols or data shapes extracted.</span>
            )}
          </div>
        </section>
        <section>
          <h3>Source evidence</h3>
          <div className="telescope-source-evidence-list">
            {evidence.slice(0, visibleEvidence).map((anchor) => (
              <div
                key={`${anchor.path}:${anchor.line ?? 0}:${anchor.symbol ?? ""}`}
              >
                <code>{anchor.path}</code>
                <span>
                  {anchor.line ? `line ${anchor.line}` : "file"}
                  {anchor.symbol ? ` · ${anchor.symbol}` : ""}
                </span>
              </div>
            ))}
          </div>
          {visibleEvidence < evidence.length && (
            <button
              className="button button-quiet"
              type="button"
              onClick={() => setVisibleEvidence((count) => count + 10)}
            >
              Show 10 more
            </button>
          )}
        </section>
      </div>
    </aside>
  );
}

function TelescopeMapWorkshop({
  projection,
  onPrepare,
}: {
  projection: TelescopeProjection;
  onPrepare: () => Promise<void>;
}): ReactElement {
  const [expanded, setExpanded] = useState(false);
  const [preparing, setPreparing] = useState(false);
  const task = [...(projection.knowledge_tasks ?? [])].sort(
    (left, right) =>
      left.dependency_order - right.dependency_order ||
      left.id.localeCompare(right.id),
  )[0];
  return (
    <section className="telescope-map-workshop" aria-label="Map Workshop">
      <div className="telescope-workshop-intro">
        <span className="eyebrow">Map workshop</span>
        <strong>
          {projection.map_readiness?.state === "reviewable"
            ? "This city is ready for human review"
            : "Pronto needs one consequential answer before this city can publish"}
        </strong>
        <p>{projection.map_readiness?.reason}</p>
      </div>
      {task ? (
        <div className="telescope-workshop-question">
          <div>
            <small>Next question · unlocks {task.unlocks.join(", ")}</small>
            <h3>{task.question}</h3>
            <p>{task.summary}</p>
          </div>
          {task.candidate_answers.length > 0 && (
            <div className="telescope-workshop-candidates">
              {task.candidate_answers.slice(0, 3).map((candidate) => (
                <span key={candidate}>{candidate}</span>
              ))}
            </div>
          )}
          <div className="telescope-workshop-actions">
            <button
              className="button button-primary"
              type="button"
              disabled={preparing}
              onClick={() => {
                setPreparing(true);
                void onPrepare().finally(() => setPreparing(false));
              }}
            >
              {preparing ? "Preparing…" : "Answer next question"}
            </button>
            <button
              className="button button-quiet"
              type="button"
              onClick={() => setExpanded((current) => !current)}
            >
              {expanded ? "Hide evidence" : "Why Pronto is asking"}
            </button>
          </div>
          {expanded && (
            <div className="telescope-workshop-evidence">
              <p>
                This is a guarded draft task. It can prepare manifest evidence,
                but it cannot mark the map reviewed.
              </p>
              <ul>
                {task.evidence.slice(0, 6).map((anchor) => (
                  <li key={`${anchor.path}:${anchor.line ?? 0}`}>
                    <code>{anchor.path}</code>
                    {anchor.line ? `:${anchor.line}` : ""}
                  </li>
                ))}
                {task.evidence.length === 0 && (
                  <li>No source anchor can answer this question yet.</li>
                )}
              </ul>
            </div>
          )}
        </div>
      ) : (
        <div className="telescope-workshop-question">
          <h3>Review the important claims against the measured city</h3>
          <p>
            Explicit review is still required for the current source
            fingerprint.
          </p>
        </div>
      )}
    </section>
  );
}
