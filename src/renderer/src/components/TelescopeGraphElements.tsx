import { memo } from "react";
import type { ReactElement, SyntheticEvent } from "react";
import {
  BaseEdge,
  getSmoothStepPath,
  Handle,
  Position,
  type EdgeProps,
  type NodeProps,
} from "@xyflow/react";
import { Boxes, CircleDot, Database, Route, Workflow } from "lucide-react";

interface EntityData extends Record<string, unknown> {
  label: string;
  kind: string;
  technology: string;
  confidence: string;
  tone?: string;
  dimmed?: boolean;
  selected?: boolean;
  filtered?: boolean;
}

function kindIcon(kind: string): ReactElement {
  if (kind === "route") return <Route size={13} />;
  if (kind === "store") return <Database size={13} />;
  if (kind === "service") return <Workflow size={13} />;
  return <Boxes size={13} />;
}

export const TelescopeEntityNode = memo(function TelescopeEntityNode({
  data,
}: NodeProps): ReactElement {
  const entity = data as EntityData;
  return (
    <div
      className={`telescope-entity telescope-tone-${entity.tone ?? "neutral"} ${entity.dimmed ? "is-dimmed" : ""} ${entity.selected ? "is-selected" : ""} ${entity.filtered ? "is-filtered" : ""}`}
      aria-hidden={entity.filtered || undefined}
    >
      <Handle type="target" position={Position.Left} />
      <div className="telescope-entity-icon">{kindIcon(entity.kind)}</div>
      <span>
        <strong>{entity.label}</strong>
        <small>
          {entity.kind} · {entity.technology}
        </small>
      </span>
      {entity.confidence !== "high" && (
        <i title={`${entity.confidence} confidence`}>
          <CircleDot size={10} />
        </i>
      )}
      <Handle type="source" position={Position.Right} />
    </div>
  );
});

export const TelescopeGroupNode = memo(function TelescopeGroupNode({
  data,
}: NodeProps): ReactElement {
  const group = data as EntityData;
  return (
    <div
      className={`telescope-group-node ${group.dimmed ? "is-dimmed" : ""} ${group.selected ? "is-selected" : ""} ${group.filtered ? "is-filtered" : ""}`}
      aria-hidden={group.filtered || undefined}
    >
      <strong>{group.label}</strong>
      <small>{group.kind}</small>
    </div>
  );
});

interface FlowEdgeData extends Record<string, unknown> {
  selected?: boolean;
  dimmed?: boolean;
  activeToken?: boolean;
  paused?: boolean;
  reducedMotion?: boolean;
  inferred?: boolean;
  onSelectToken?: (event: SyntheticEvent<SVGCircleElement>) => void;
}

export const TelescopeFlowEdge = memo(function TelescopeFlowEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
  data,
}: EdgeProps): ReactElement {
  const edge = (data ?? {}) as FlowEdgeData;
  const [path] = getSmoothStepPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    borderRadius: 18,
  });
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        className={`telescope-edge ${edge.selected ? "is-selected" : ""} ${edge.dimmed ? "is-dimmed" : ""} ${edge.inferred ? "is-inferred" : ""}`}
      />
      {edge.activeToken && (
        <circle
          className={`telescope-flow-token ${edge.paused || edge.reducedMotion ? "is-paused" : ""}`}
          r="5"
          cx={edge.paused || edge.reducedMotion ? sourceX : undefined}
          cy={edge.paused || edge.reducedMotion ? sourceY : undefined}
          role="button"
          aria-label={
            edge.reducedMotion
              ? "Inspect static flow token"
              : "Inspect moving flow token"
          }
          tabIndex={0}
          onClick={edge.onSelectToken}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              edge.onSelectToken?.(event);
            }
          }}
        >
          {!edge.paused && !edge.reducedMotion && (
            <animateMotion dur="2.8s" repeatCount="indefinite" path={path} />
          )}
        </circle>
      )}
    </>
  );
});
