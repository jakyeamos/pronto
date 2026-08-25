import { ExternalLink, Radar, RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import type { ReactElement } from "react";
import { GitHubVisibilityDialog } from "./GitHubVisibilityDialog";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import type {
  GitHubVisibility,
  PortfolioSnapshot,
  ReleaseRadarRow,
  RemoteRepositorySnapshot,
} from "../types";

type RadarFilter =
  | "all"
  | "public"
  | "private"
  | "unknown"
  | "framing"
  | "needs_sync"
  | "github_only";

function toneForState(value: string): string {
  if (
    ["eligible", "in_sync", "fresh", "confirmed", "assessed"].some((item) =>
      value.startsWith(item),
    )
  ) {
    return "mint";
  }
  if (
    [
      "private",
      "internal",
      "suppressed",
      "configured",
      "provider evidence",
    ].some((item) => value.startsWith(item))
  ) {
    return "blue";
  }
  if (
    ["stale", "needs_sync", "missing", "unknown", "blocked"].some((item) =>
      value.startsWith(item),
    )
  ) {
    return "amber";
  }
  return "slate";
}

function matchesFilter(row: ReleaseRadarRow, filter: RadarFilter): boolean {
  if (filter === "all") return true;
  if (filter === "public") return row.visibility === "public";
  if (filter === "private") {
    return row.visibility === "private" || row.visibility === "internal";
  }
  if (filter === "unknown") {
    return (
      row.visibility === "unknown" || row.visibility_freshness === "blocked"
    );
  }
  if (filter === "framing") return row.public_framing === "eligible";
  if (filter === "needs_sync") return row.profile_link_sync === "needs_sync";
  return row.locality === "GitHub only";
}

function labelForFilter(filter: RadarFilter): string {
  return {
    all: "All fleet",
    public: "Public",
    private: "Private/internal",
    unknown: "Unknown/blocked",
    framing: "Framing eligible",
    needs_sync: "Profile drift",
    github_only: "GitHub-only",
  }[filter];
}

function rowSummary(row: ReleaseRadarRow): string {
  if (row.blocker) return row.blocker;
  if (row.public_framing === "eligible") {
    return "Provider visibility and profile metadata are safe for public framing; readiness evidence remains independent.";
  }
  return "No public presentation is promoted from this row.";
}

export function ReleaseRadarSurface({
  snapshot,
  remoteRepositories,
  isRefreshing,
  onRefresh,
  onChangeVisibility,
}: {
  snapshot: PortfolioSnapshot["release_radar"];
  remoteRepositories: RemoteRepositorySnapshot[];
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onChangeVisibility: (
    repositoryId: string,
    targetVisibility: GitHubVisibility,
    confirmation: string,
  ) => Promise<void>;
}): ReactElement {
  const [filter, setFilter] = useState<RadarFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedRepository, setSelectedRepository] =
    useState<RemoteRepositorySnapshot | null>(null);
  const rows = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return snapshot.rows.filter(
      (row) =>
        matchesFilter(row, filter) &&
        (!normalizedQuery ||
          (row.name + " " + (row.full_name ?? ""))
            .toLowerCase()
            .includes(normalizedQuery)),
    );
  }, [filter, query, snapshot.rows]);

  return (
    <div className="release-radar-layout">
      <section className="surface-panel release-radar-hero">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Full-fleet distribution control</p>
            <h2>Release Radar</h2>
            <p>
              One deduplicated view of local and GitHub repositories. Visibility
              governs public framing and profile links; product, demo-material,
              and release evidence stay separate.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh GitHub"}
          </button>
        </div>
        <div className="release-radar-summary">
          <div className="release-radar-summary-icon">
            <Radar size={20} />
          </div>
          <div>
            <strong>
              {snapshot.counts.total} repositories ·{" "}
              {snapshot.counts.public_framing_eligible} framing eligible
            </strong>
            <p>
              GitHub {snapshot.provider_state.toLowerCase()} ·{" "}
              {snapshot.provider_last_refresh_at
                ? "read " + formatTime(snapshot.provider_last_refresh_at)
                : "no provider readback"}{" "}
              · profile catalog {snapshot.profile_state.toLowerCase()}
            </p>
          </div>
        </div>
        <div className="release-radar-counts">
          <span>
            <strong>{snapshot.counts.local_and_remote}</strong> joined
          </span>
          <span>
            <strong>{snapshot.counts.github_only}</strong> GitHub-only
          </span>
          <span>
            <strong>{snapshot.counts.local_only}</strong> local-only
          </span>
          <span>
            <strong>{snapshot.counts.public_fresh}</strong> public/fresh
          </span>
          <span>
            <strong>{snapshot.counts.private_or_internal_fresh}</strong>{" "}
            suppressed
          </span>
          <span>
            <strong>{snapshot.counts.unknown_or_blocked}</strong> blocked
          </span>
        </div>
      </section>
      <section className="surface-panel">
        <div className="release-radar-toolbar">
          <label className="release-radar-search">
            <span className="sr-only">Filter repositories</span>
            <input
              type="search"
              placeholder="Filter by repository or owner"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <div
            className="release-radar-filters"
            aria-label="Release Radar filters"
          >
            {(
              [
                "all",
                "public",
                "private",
                "unknown",
                "framing",
                "needs_sync",
                "github_only",
              ] as RadarFilter[]
            ).map((value) => (
              <button
                className={
                  "button button-compact " +
                  (filter === value ? "button-primary" : "button-secondary")
                }
                type="button"
                key={value}
                onClick={() => setFilter(value)}
                aria-pressed={filter === value}
              >
                {labelForFilter(value)}
              </button>
            ))}
          </div>
        </div>
        {rows.length === 0 ? (
          <div className="surface-empty">
            <Radar size={18} />
            <span>No Release Radar rows match this filter.</span>
          </div>
        ) : (
          <div className="release-radar-table-wrap">
            <table className="release-radar-table">
              <thead>
                <tr>
                  <th>Repository</th>
                  <th>Visibility</th>
                  <th>Public framing</th>
                  <th>Profile README</th>
                  <th>Product</th>
                  <th>Demo materials</th>
                  <th>Release</th>
                  <th>Next action / blocker</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => {
                  const remote = row.remote_repository_id
                    ? remoteRepositories.find(
                        (candidate) =>
                          candidate.id === row.remote_repository_id,
                      )
                    : undefined;
                  return (
                    <tr key={row.id}>
                      <td>
                        <div className="release-radar-repository">
                          <strong>{row.name}</strong>
                          <small>
                            {row.full_name ?? row.locality} ·{" "}
                            {row.visibility_freshness}
                          </small>
                          <StatusPill
                            tone={
                              row.locality === "GitHub only" ? "amber" : "slate"
                            }
                          >
                            {row.locality}
                          </StatusPill>
                        </div>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.visibility)}>
                          {row.visibility}
                        </StatusPill>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.public_framing)}>
                          {row.public_framing}
                        </StatusPill>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.profile_link_sync)}>
                          {row.profile_link_sync}
                        </StatusPill>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.product_readiness)}>
                          {row.product_readiness}
                        </StatusPill>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.demo_materials)}>
                          {row.demo_materials}
                        </StatusPill>
                      </td>
                      <td>
                        <StatusPill tone={toneForState(row.release_readiness)}>
                          {row.release_readiness}
                        </StatusPill>
                      </td>
                      <td>
                        <div className="release-radar-next-action">
                          <strong>{row.next_action}</strong>
                          <small>{rowSummary(row)}</small>
                          <div className="release-radar-row-actions">
                            {row.github_url && (
                              <a
                                className="icon-button"
                                href={row.github_url}
                                target="_blank"
                                rel="noreferrer"
                                aria-label={
                                  "Open " +
                                  (row.full_name ?? row.name) +
                                  " on GitHub"
                                }
                              >
                                <ExternalLink size={14} />
                              </a>
                            )}
                            {remote?.viewer_can_administer === true &&
                              remote.visibility &&
                              row.visibility_freshness === "fresh" && (
                                <button
                                  className="button button-secondary button-compact"
                                  type="button"
                                  onClick={() => setSelectedRepository(remote)}
                                  disabled={isRefreshing}
                                >
                                  Change visibility
                                </button>
                              )}
                          </div>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>
      {selectedRepository && (
        <GitHubVisibilityDialog
          repository={selectedRepository}
          onCancel={() => setSelectedRepository(null)}
          onConfirm={(targetVisibility, confirmation) =>
            onChangeVisibility(
              selectedRepository.id,
              targetVisibility,
              confirmation,
            )
          }
        />
      )}
    </div>
  );
}
