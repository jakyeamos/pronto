import { Search } from "lucide-react";
import type { ReactElement } from "react";
import type {
  TelescopeAction,
  TelescopeActionCoverage,
} from "../types/telescope";

export function TelescopeActionPalette({
  actions,
  coverage,
  query,
  selectedActionId,
  onQuery,
  onSelect,
  onClear,
}: {
  actions: TelescopeAction[];
  coverage: TelescopeActionCoverage;
  query: string;
  selectedActionId: string | null;
  onQuery: (query: string) => void;
  onSelect: (actionId: string) => void;
  onClear: () => void;
}): ReactElement {
  const normalized = query.trim().toLowerCase();
  const visibleActions = actions
    .filter((action) => {
      if (!normalized) return true;
      return [
        action.label,
        action.verb,
        action.category,
        action.what_it_does,
        action.how_its_built,
        action.behavior_id ?? "",
        ...action.source_anchors.map((anchor) => anchor.path),
      ]
        .join(" ")
        .toLowerCase()
        .includes(normalized);
    })
    .slice(0, 10);
  const behaviorBacked = coverage.behavior_backed ?? 0;
  const unprofiled = coverage.unprofiled ?? 0;

  return (
    <div className="telescope-actions" aria-label="Telescope action catalog">
      <div className="telescope-actions-heading">
        <div>
          <span className="eyebrow">Action catalog</span>
          <strong>What do you want to understand?</strong>
        </div>
        <span className="telescope-action-coverage">
          {coverage.total} actions · {behaviorBacked} behavior-backed ·{" "}
          {unprofiled} exploratory
        </span>
      </div>
      <div className="telescope-action-search">
        <Search size={14} />
        <input
          id="telescope-action-search-input"
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder="Search actions, behavior IDs, or source paths"
          aria-label="Find a Telescope action"
        />
        {query && (
          <button
            type="button"
            aria-label="Clear action search"
            onClick={() => onQuery("")}
          >
            ×
          </button>
        )}
      </div>
      <div className="telescope-action-results" aria-label="Available actions">
        {visibleActions.length ? (
          visibleActions.map((action) => (
            <button
              className={selectedActionId === action.id ? "active" : ""}
              type="button"
              key={action.id}
              aria-pressed={selectedActionId === action.id}
              onClick={() => onSelect(action.id)}
            >
              <span>{action.verb}</span>
              <strong>{action.label}</strong>
              {action.behavior_id ? (
                <small>behavior-backed</small>
              ) : (
                <small>explore</small>
              )}
            </button>
          ))
        ) : (
          <span className="telescope-action-empty">
            No catalog action matches “{query}”.
          </span>
        )}
        {selectedActionId && (
          <button
            className="telescope-action-reset"
            type="button"
            onClick={onClear}
          >
            Show full city
          </button>
        )}
      </div>
      <small className="telescope-action-note">
        {coverage.inventory_status === "reviewed"
          ? "Reviewed mappings explain the city; behavior evidence still comes from Quality Runner."
          : "Catalog meaning is partial or inferred. Only linked behavior contracts can provide behavioral proof."}
      </small>
    </div>
  );
}
