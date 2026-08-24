import { memo } from "react";
import type { ReactElement, SyntheticEvent } from "react";
import { BaseEdge, type EdgeProps } from "@xyflow/react";
import type { TelescopeRailKind } from "./telescopeSceneModel";
import { routeRailWaypoints } from "./telescopeSceneLayout";

interface FlowEdgeData extends Record<string, unknown> {
  label?: string;
  confidence?: string;
  railKind?: TelescopeRailKind | string;
  sourceEdgeIds?: string[];
  tokenLabel?: string;
  uncertain?: boolean;
  selected?: boolean;
  dimmed?: boolean;
  activeToken?: boolean;
  paused?: boolean;
  reducedMotion?: boolean;
  inferred?: boolean;
  railIndex?: number;
  onSelectToken?: (event: SyntheticEvent<SVGCircleElement>) => void;
}

function railKind(value: string | undefined): TelescopeRailKind {
  if (value === "data" || value === "control" || value === "event") {
    return value;
  }
  return "import";
}

function pathForWaypoints(points: Array<{ x: number; y: number }>): string {
  return points
    .map(
      (point, index) => (index === 0 ? "M " : "L ") + point.x + " " + point.y,
    )
    .join(" ");
}

export const TelescopeFlowEdge = memo(function TelescopeFlowEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  markerEnd,
  data,
}: EdgeProps): ReactElement {
  const edge = (data ?? {}) as FlowEdgeData;
  const kind = railKind(edge.railKind);
  const path = pathForWaypoints(
    routeRailWaypoints(
      { x: sourceX, y: sourceY },
      { x: targetX, y: targetY },
      edge.railIndex ?? 0,
    ),
  );
  const classes = [
    "telescope-edge",
    "telescope-rail-kind-" + kind,
    edge.selected ? "is-selected" : "",
    edge.dimmed ? "is-dimmed" : "",
    edge.inferred || edge.uncertain ? "is-uncertain" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const tokenLabel = edge.tokenLabel ?? "flow";
  const tokenAriaLabel = edge.tokenLabel
    ? "Inspect " + tokenLabel + " flow token"
    : edge.reducedMotion
      ? "Inspect static flow token"
      : "Inspect moving flow token";
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        className={classes}
        interactionWidth={24}
      />
      {edge.activeToken && (
        <g
          className={
            "telescope-flow-token-group" +
            (edge.paused || edge.reducedMotion ? " is-paused" : "")
          }
          aria-label={tokenLabel + " flow"}
        >
          <circle
            className="telescope-flow-token-halo"
            r="11"
            cx={sourceX}
            cy={sourceY}
            aria-hidden="true"
          />
          <circle
            className="telescope-flow-token"
            r="5"
            cx={edge.paused || edge.reducedMotion ? sourceX : undefined}
            cy={edge.paused || edge.reducedMotion ? sourceY : undefined}
            role="button"
            aria-label={tokenAriaLabel}
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
          <text
            className="telescope-flow-token-label"
            x={sourceX + 12}
            y={sourceY - 12}
            aria-hidden="true"
          >
            {tokenLabel}
          </text>
        </g>
      )}
    </>
  );
});
