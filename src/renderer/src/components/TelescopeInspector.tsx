import { Boxes } from "lucide-react";
import type { ReactElement } from "react";
import type { RepositorySnapshot } from "../types";
import type { TelescopeLens } from "../types/telescope";
import { StatusPill } from "./ConsolePrimitives";
import { summarizeActionEvidence } from "./telescopeSurfaceUtils";
import type { LensSummary, SelectedItem } from "./telescopeSurfaceTypes";

export function TelescopeInspector({
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
        <InspectorItem item={item} mode={mode} repository={repository} />
      ) : (
        <div className="telescope-inspector-empty">
          <Boxes size={20} />
          <strong>Select anything on the map</strong>
          <p>
            Inspect an entity, subsystem, relationship, moving flow token, or
            catalog action.
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
  repository,
}: {
  item: SelectedItem;
  mode: "what" | "how";
  repository: RepositorySnapshot;
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
  if (item.kind === "action") {
    const action = item.value;
    const evidence = summarizeActionEvidence(action, repository);
    return (
      <div className="telescope-inspector-content">
        <p className="eyebrow">
          {action.verb} · {action.category}
        </p>
        <h2>{action.label}</h2>
        <StatusPill tone={evidence.tone}>{evidence.label}</StatusPill>
        <p>{mode === "what" ? action.what_it_does : action.how_its_built}</p>
        <div className="telescope-action-evidence">
          <span>Behavior evidence</span>
          <strong>{evidence.detail}</strong>
        </div>
        {mode === "how" && (
          <>
            <InspectorList
              title="Behavior contract"
              values={
                action.behavior_id
                  ? [action.behavior_id]
                  : ["Not linked · unprofiled"]
              }
            />
            <InspectorList
              title="Scenarios"
              values={action.scenario_ids ?? []}
            />
            <InspectorList
              title="Source anchors"
              values={action.source_anchors.map(
                (anchor) =>
                  `${anchor.path}${anchor.line ? `:${anchor.line}` : ""}`,
              )}
            />
            <small>
              Projection status: {action.status} ·{" "}
              {action.behavior_state ?? "unknown"} ·{" "}
              {action.behavior_verification ?? "not-profiled"}.
            </small>
          </>
        )}
        <small>
          {action.read_only ? "Read-only focus" : "Guarded handoff available"} ·{" "}
          {action.guarded
            ? "actions hand off through Pronto guards"
            : "no guarded handoff"}
          ; no mutation is performed.
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
