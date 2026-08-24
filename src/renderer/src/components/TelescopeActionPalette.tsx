import { Search } from "lucide-react";
import type { ReactElement } from "react";
import type {
  TelescopeAction,
  TelescopeActionCoverage,
} from "../types/telescope";
import { routeTelescopeActions } from "./telescopeActionRouting";

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
  const visibleMatches = routeTelescopeActions(query, actions);
  const behaviorBacked = coverage.behavior_backed ?? 0;
  const unprofiled = coverage.unprofiled ?? 0;
  const hasQuery = query.trim().length > 0;

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
          onKeyDown={(event) => {
            if (event.key === "Enter" && visibleMatches[0]) {
              event.preventDefault();
              onSelect(visibleMatches[0].action.id);
            }
          }}
          placeholder="Ask how a workflow works…"
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
      <div className="telescope-action-route" aria-live="polite">
        {hasQuery
          ? visibleMatches.length > 0
            ? `${visibleMatches.length} related action${visibleMatches.length === 1 ? "" : "s"} · press Enter to focus the top neighborhood`
            : `No related action for “${query}”. Try a noun like search, release, quality, or workspace.`
          : "Ask a question in plain language, then choose a related action to focus its neighborhood."}
      </div>
      <div className="telescope-action-results" aria-label="Available actions">
        {visibleMatches.length ? (
          visibleMatches.map((match) => (
            <button
              className={selectedActionId === match.action.id ? "active" : ""}
              type="button"
              key={match.action.id}
              aria-pressed={selectedActionId === match.action.id}
              onClick={() => onSelect(match.action.id)}
            >
              <span>{match.action.verb}</span>
              <strong>{match.action.label}</strong>
              <small>
                {match.relationship === "direct"
                  ? "direct match"
                  : "related match"}
              </small>
              <small className="telescope-action-match">
                {match.explanation}
              </small>
              {match.action.behavior_id ? (
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
