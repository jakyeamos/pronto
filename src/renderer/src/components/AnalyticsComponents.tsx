import { useEffect, useMemo, useState, type ReactElement } from "react";
import { SlidersHorizontal } from "lucide-react";
import * as api from "../api";
import type {
  AnalyticsSnapshot,
  GroupConfig,
  ProductConfig,
  RepositorySnapshot,
} from "../types";
import { AnalyticsBuilderWorkspace } from "./AnalyticsBuilderWorkspace";
import { AnalyticsCuratedWorkspace } from "./AnalyticsCuratedWorkspace";

export function AnalyticsSurface({
  analytics: initialAnalytics,
  repositories: _repositories,
  groups = [],
  products = [],
}: {
  analytics: AnalyticsSnapshot;
  repositories: RepositorySnapshot[];
  groups?: GroupConfig[];
  products?: ProductConfig[];
}): ReactElement {
  const [analytics, setAnalytics] = useState(initialAnalytics);
  const [mode, setMode] = useState<"curated" | "builder">("curated");
  const [repositoryFilter, setRepositoryFilter] = useState("all");
  const [freshness, setFreshness] = useState("all");
  const [collectionFilter, setCollectionFilter] = useState("all");
  const [loading, setLoading] = useState(false);
  const ranges = useMemo(
    () =>
      Array.from(new Set([7, 30, 90, analytics.retention_days]))
        .filter((value) => value <= analytics.retention_days)
        .sort((a, b) => a - b),
    [analytics.retention_days],
  );
  useEffect(() => setAnalytics(initialAnalytics), [initialAnalytics]);
  const collectionRepositoryIds =
    collectionFilter === "all"
      ? null
      : collectionFilter.startsWith("group:")
        ? groups.find((item) => `group:${item.id}` === collectionFilter)
            ?.repository_ids
        : products.find((item) => `product:${item.id}` === collectionFilter)
            ?.repository_ids;
  const effectiveRepositoryFilter =
    repositoryFilter !== "all"
      ? repositoryFilter
      : collectionRepositoryIds?.length === 1
        ? collectionRepositoryIds[0]
        : "all";
  const scopedAnalytics = collectionRepositoryIds
    ? {
        ...analytics,
        repositories: analytics.repositories.filter((series) =>
          collectionRepositoryIds.includes(series.repository_id),
        ),
      }
    : analytics;
  const setRange = async (days: number): Promise<void> => {
    setLoading(true);
    try {
      setAnalytics(await api.getAnalytics(days));
    } finally {
      setLoading(false);
    }
  };
  return (
    <section
      className="analytics-surface analytics-workspace"
      aria-label="Analytics workspace"
    >
      <div className="analytics-workspace-header">
        <div>
          <p className="eyebrow">Evidence workspace</p>
          <h2>Analytics that preserve meaning.</h2>
          <p>
            Curated local-refresh history with explicit scale, freshness, and
            coverage boundaries.
          </p>
        </div>
        <div
          className="analytics-mode-switch"
          role="group"
          aria-label="Analytics mode"
        >
          <button
            type="button"
            className={mode === "curated" ? "active" : ""}
            aria-pressed={mode === "curated"}
            onClick={() => setMode("curated")}
          >
            Curated
          </button>
          <button
            type="button"
            className={mode === "builder" ? "active" : ""}
            aria-pressed={mode === "builder"}
            onClick={() => setMode("builder")}
          >
            <SlidersHorizontal size={14} />
            Composer
          </button>
        </div>
      </div>
      <div className="analytics-toolbar">
        <div
          className="analytics-range-control"
          role="group"
          aria-label="Analytics range"
        >
          {ranges.map((days) => (
            <button
              type="button"
              key={days}
              className={analytics.range_days === days ? "active" : ""}
              disabled={loading}
              onClick={() => void setRange(days)}
            >
              {days === analytics.retention_days && ![7, 30, 90].includes(days)
                ? "Retention"
                : `${days}d`}
            </button>
          ))}
        </div>
        <label>
          Collection
          <select
            value={collectionFilter}
            onChange={(event) => {
              setCollectionFilter(event.target.value);
              setRepositoryFilter("all");
            }}
          >
            <option value="all">All groups and products</option>
            {groups.map((item) => (
              <option key={`group:${item.id}`} value={`group:${item.id}`}>
                Group · {item.name}
              </option>
            ))}
            {products.map((item) => (
              <option key={`product:${item.id}`} value={`product:${item.id}`}>
                Product · {item.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Repository
          <select
            value={repositoryFilter}
            onChange={(event) => setRepositoryFilter(event.target.value)}
          >
            <option value="all">All repositories</option>
            {scopedAnalytics.repositories.map((series) => (
              <option key={series.repository_id} value={series.repository_id}>
                {series.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Evidence
          <select
            value={freshness}
            onChange={(event) => setFreshness(event.target.value)}
          >
            <option value="all">All states</option>
            <option value="fresh">Fresh</option>
            <option value="stale">Stale</option>
            <option value="conflicted">Conflicted</option>
            <option value="unavailable">Unavailable</option>
          </select>
        </label>
        <div className="analytics-provenance">
          <strong>{analytics.source}</strong>
          <span>{analytics.freshness}</span>
        </div>
      </div>
      {freshness !== "all" &&
      !analytics.portfolio_samples.some(
        (sample) =>
          (sample.quality_freshness ?? "unavailable").toLowerCase() ===
          freshness,
      ) ? (
        <div className="analytics-state">
          No observations match the selected evidence state.
        </div>
      ) : mode === "curated" ? (
        <AnalyticsCuratedWorkspace
          analytics={scopedAnalytics}
          repositoryFilter={effectiveRepositoryFilter}
          onDrillDown={setRepositoryFilter}
        />
      ) : (
        <AnalyticsBuilderWorkspace analytics={scopedAnalytics} />
      )}
    </section>
  );
}
