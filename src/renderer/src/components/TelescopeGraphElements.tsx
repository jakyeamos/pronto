import { memo } from "react";
import type { ReactElement } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { CircleDot, Database, Route, Workflow } from "lucide-react";
import type { TelescopeArchetype } from "./telescopeSceneModel";
import { dimensionsForArchetype } from "./telescopeSceneLayout";

export { TelescopeFlowEdge } from "./TelescopeFlowEdge";

interface EntityData extends Record<string, unknown> {
  label: string;
  kind: string;
  technology: string;
  confidence: string;
  archetype?: TelescopeArchetype;
  memberCount?: number;
  sourceFileCount?: number;
  narrativeStatus?: string;
  authored?: boolean;
  tone?: string;
  dimmed?: boolean;
  selected?: boolean;
  filtered?: boolean;
}

function kindIcon(kind: string): ReactElement | null {
  if (kind === "route") return <Route size={13} />;
  if (kind === "store") return <Database size={13} />;
  if (kind === "service") return <Workflow size={13} />;
  return null;
}

function archetypeForEntity(entity: EntityData): TelescopeArchetype {
  if (entity.archetype) return entity.archetype;
  if (entity.kind === "route" || entity.kind === "entrypoint") return "tower";
  if (entity.kind === "store") return "slab-stack";
  if (entity.kind === "integration") return "low-slab";
  return "cube";
}

export const TelescopeEntityNode = memo(function TelescopeEntityNode({
  data,
}: NodeProps): ReactElement {
  const entity = data as EntityData;
  const archetype = archetypeForEntity(entity);
  const dimensions = dimensionsForArchetype(archetype);
  const classes = [
    "telescope-entity",
    "telescope-building-node",
    "telescope-tone-" + (entity.tone ?? "neutral"),
    "telescope-archetype-" + archetype,
    entity.dimmed ? "is-dimmed" : "",
    entity.selected ? "is-selected" : "",
    entity.filtered ? "is-filtered" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const memberLabel =
    entity.memberCount && entity.memberCount > 1
      ? entity.memberCount + " members"
      : "single subsystem";
  return (
    <div
      className={classes}
      style={{ minWidth: dimensions.width, minHeight: dimensions.height }}
      aria-hidden={entity.filtered || undefined}
      aria-label={entity.label}
      title={entity.label}
    >
      <Handle type="target" position={Position.Left} />
      <BuildingSilhouette archetype={archetype} label={entity.label} />
      <div className="telescope-building-copy">
        <strong>{entity.label}</strong>
        <small>
          {memberLabel}
          {entity.sourceFileCount
            ? " · " + entity.sourceFileCount + " files"
            : ""}
        </small>
      </div>
      <div className="telescope-building-meta">
        {kindIcon(entity.kind)}
        <span>{entity.kind}</span>
        {entity.narrativeStatus && entity.narrativeStatus !== "derived" && (
          <em>{entity.narrativeStatus}</em>
        )}
        {entity.confidence !== "high" && (
          <i title={entity.confidence + " confidence"}>
            <CircleDot size={10} />
          </i>
        )}
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
});

export const TelescopeGroupNode = memo(function TelescopeGroupNode({
  data,
}: NodeProps): ReactElement {
  const group = data as EntityData;
  const classes = [
    "telescope-group-node",
    "telescope-district-node",
    group.dimmed ? "is-dimmed" : "",
    group.selected ? "is-selected" : "",
    group.filtered ? "is-filtered" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div
      className={classes}
      aria-hidden={group.filtered || undefined}
      aria-label={group.label}
      title={group.label}
    >
      <div className="telescope-district-marker" aria-hidden="true" />
      <div>
        <strong>{group.label}</strong>
        <small>
          {group.memberCount ?? 0} entities
          {group.sourceFileCount
            ? " · " + group.sourceFileCount + " files"
            : ""}
        </small>
      </div>
      <span className="telescope-district-kind">{group.kind}</span>
    </div>
  );
});

function BuildingSilhouette({
  archetype,
  label,
}: {
  archetype: TelescopeArchetype;
  label: string;
}): ReactElement {
  const width = 240;
  const height = 190;
  const baseY = 174;
  const depth = 38;
  const topY =
    archetype === "tower"
      ? 26
      : archetype === "fin-row"
        ? 66
        : archetype === "low-slab"
          ? 98
          : 82;
  const left = 46;
  const right = 166;
  const sideRight = right + depth;
  const topPoints =
    left +
    "," +
    topY +
    " " +
    right +
    "," +
    topY +
    " " +
    sideRight +
    "," +
    (topY + 18) +
    " " +
    (left + depth) +
    "," +
    (topY + 18);
  const frontPoints =
    left +
    "," +
    (topY + 18) +
    " " +
    right +
    "," +
    (topY + 18) +
    " " +
    right +
    "," +
    baseY +
    " " +
    left +
    "," +
    baseY;
  const sidePoints =
    right +
    "," +
    (topY + 18) +
    " " +
    sideRight +
    "," +
    (topY + 36) +
    " " +
    sideRight +
    "," +
    (baseY - 12) +
    " " +
    right +
    "," +
    baseY;
  return (
    <svg
      className="telescope-building-silhouette"
      viewBox={"0 0 " + width + " " + height}
      role="img"
      aria-label={label + " " + archetype + " building"}
    >
      <title>{label + " " + archetype + " building"}</title>
      <ellipse
        className="building-shadow"
        cx="126"
        cy="174"
        rx={archetype === "tower" ? "60" : "76"}
        ry="10"
      />
      <polygon className="building-top" points={topPoints} />
      <polygon className="building-front" points={frontPoints} />
      <polygon className="building-side" points={sidePoints} />
      <BuildingDetails
        archetype={archetype}
        topY={topY}
        baseY={baseY}
        left={left}
        right={right}
      />
    </svg>
  );
}

function BuildingDetails({
  archetype,
  topY,
  baseY,
  left,
  right,
}: {
  archetype: TelescopeArchetype;
  topY: number;
  baseY: number;
  left: number;
  right: number;
}): ReactElement {
  if (archetype === "fin-row") {
    return (
      <g className="building-fins">
        {[0, 1, 2, 3, 4].map((index) => {
          const x = left + 12 + index * 23;
          return (
            <path
              key={index}
              d={
                "M " +
                x +
                " " +
                (topY + 32) +
                " L " +
                (x + 8) +
                " " +
                (topY + 28) +
                " L " +
                (x + 8) +
                " " +
                (baseY - 10) +
                " L " +
                x +
                " " +
                (baseY - 14) +
                " Z"
              }
            />
          );
        })}
      </g>
    );
  }
  if (archetype === "slab-stack") {
    return (
      <g className="building-slab-lines">
        {[0, 1, 2].map((index) => (
          <path
            key={index}
            d={
              "M " +
              (left + 7) +
              " " +
              (topY + 44 + index * 34) +
              " L " +
              (right - 8) +
              " " +
              (topY + 44 + index * 34)
            }
          />
        ))}
      </g>
    );
  }
  if (archetype === "tower") {
    return (
      <g className="building-tower-lines">
        <path
          d={
            "M " +
            (left + 24) +
            " " +
            (topY + 38) +
            " L " +
            (left + 24) +
            " " +
            (baseY - 12)
          }
        />
        <path
          d={
            "M " +
            (left + 47) +
            " " +
            (topY + 38) +
            " L " +
            (left + 47) +
            " " +
            (baseY - 12)
          }
        />
        <path
          d={
            "M " +
            (left + 70) +
            " " +
            (topY + 38) +
            " L " +
            (left + 70) +
            " " +
            (baseY - 12)
          }
        />
        <path
          className="building-spire"
          d={
            "M " +
            (left + 60) +
            " " +
            topY +
            " L " +
            (left + 60) +
            " " +
            (topY - 16)
          }
        />
      </g>
    );
  }
  if (archetype === "low-slab") {
    return (
      <path
        className="building-low-line"
        d={
          "M " +
          (left + 10) +
          " " +
          (topY + 40) +
          " L " +
          (right - 10) +
          " " +
          (topY + 40)
        }
      />
    );
  }
  return (
    <g className="building-window-grid">
      {[0, 1, 2].map((index) => (
        <path
          key={index}
          d={
            "M " +
            (left + 18 + index * 24) +
            " " +
            (topY + 42) +
            " L " +
            (left + 18 + index * 24) +
            " " +
            (baseY - 12)
          }
        />
      ))}
    </g>
  );
}
