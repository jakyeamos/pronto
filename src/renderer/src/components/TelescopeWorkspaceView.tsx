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
import type { ReactElement } from "react";
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
import type { TelescopeLens } from "../types/telescope";
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
            {scene?.buildings.length ?? 0} buildings ·{" "}
            {scene?.rails.length ?? 0} rails
          </span>
          <span>
            {projection.coverage.supported_source_files}/
            {projection.coverage.examined_source_files} adapted
          </span>
          <span>Narrative {projection.narrative?.status ?? "missing"}</span>
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
          setCollapsedGroups(new Set());
        }}
        onClear={() => {
          setSelection(null);
          setSceneLevel("overview");
        }}
      />
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
              onClick={() => setSceneLevel(level)}
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
        Boolean(scene?.clusteredSourceNodeCount) ||
        reducedMotion) && (
        <footer className="telescope-warnings">
          {error && <span>{error}</span>}
          {layoutWarning && <span>{layoutWarning}</span>}
          {scene?.clusteredSourceNodeCount ? (
            <span>
              Source detail is capped at {scene.sourceDetailBuildingLimit}{" "}
              buildings; the navigator still exposes every underlying source
              entity.
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
