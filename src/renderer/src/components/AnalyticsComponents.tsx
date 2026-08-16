import { useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import {
  ArrowDown,
  ArrowUp,
  Columns2,
  Grip,
  Plus,
  Save,
  SlidersHorizontal,
  X,
} from "lucide-react";
import * as api from "../api";
import type {
  AnalyticsChartType,
  AnalyticsSnapshot,
  AnalyticsView,
  AnalyticsWidgetConfig,
  GroupConfig,
  MetricDefinition,
  ProductConfig,
  RepositorySnapshot,
} from "../types";
import {
  DivergenceChart,
  EvidenceHeatmap,
  GovernedBars,
  GovernedTrend,
  QualityCoverageScatter,
  metricsShareAxis,
} from "./AnalyticsCharts";

const BUILDER_TYPES: AnalyticsChartType[] = [
  "line",
  "bar",
  "diverging-bar",
  "scatter",
  "stacked-bar",
  "heatmap",
  "table",
];

function latest<T>(items: T[]): T | undefined {
  return items.at(-1);
}
function count(value: number | null | undefined): string {
  return value == null ? "Unavailable" : new Intl.NumberFormat().format(value);
}
function metric(
  catalog: MetricDefinition[],
  id: string,
): MetricDefinition | undefined {
  return catalog.find((item) => item.id === id);
}

function ChartCard({
  eyebrow,
  title,
  description,
  children,
  wide = false,
}: {
  eyebrow: string;
  title: string;
  description: string;
  children: ReactElement;
  wide?: boolean;
}): ReactElement {
  return (
    <article
      className={
        wide
          ? "analytics-workspace-card analytics-workspace-card-wide"
          : "analytics-workspace-card"
      }
    >
      <header>
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </header>
      {children}
    </article>
  );
}

function RepositoryTable({
  analytics,
  onDrillDown,
}: {
  analytics: AnalyticsSnapshot;
  onDrillDown: (id: string) => void;
}): ReactElement {
  type SortKey =
    "repository" | "maturity" | "evidence" | "findings" | "dirty" | "sync";
  const [sortKey, setSortKey] = useState<SortKey>("findings");
  const [sortDirection, setSortDirection] = useState<
    "ascending" | "descending"
  >("descending");
  const valueFor = (
    row: ReturnType<typeof latestRepositoryRow>,
    key: SortKey,
  ): number | string => {
    if (key === "repository") return row.series.name.toLocaleLowerCase();
    if (key === "maturity") return row.sample?.maturity_score ?? -1;
    if (key === "evidence") return row.sample?.ci_readiness_score ?? -1;
    if (key === "findings")
      return (
        row.sample?.detector_findings_total ?? row.sample?.findings_total ?? -1
      );
    if (key === "dirty") return row.sample?.dirty_workspace_count ?? -1;
    return row.sample?.unsynced_workspace_count ?? -1;
  };
  const rows = analytics.repositories.map(latestRepositoryRow).sort((a, b) => {
    const left = valueFor(a, sortKey);
    const right = valueFor(b, sortKey);
    const comparison =
      typeof left === "string" && typeof right === "string"
        ? left.localeCompare(right)
        : Number(left) - Number(right);
    return sortDirection === "ascending" ? comparison : -comparison;
  });
  const sortBy = (nextKey: SortKey): void => {
    if (nextKey === sortKey) {
      setSortDirection((current) =>
        current === "ascending" ? "descending" : "ascending",
      );
      return;
    }
    setSortKey(nextKey);
    setSortDirection(nextKey === "repository" ? "ascending" : "descending");
  };
  const header = (label: string, key: SortKey): ReactElement => (
    <th aria-sort={sortKey === key ? sortDirection : "none"}>
      <button
        type="button"
        className="analytics-table-sort"
        onClick={() => sortBy(key)}
      >
        {label}
        {sortKey === key ? (sortDirection === "ascending" ? " ↑" : " ↓") : ""}
      </button>
    </th>
  );
  return (
    <div className="analytics-table-wrap">
      <table className="analytics-table">
        <thead>
          <tr>
            {header("Repository", "repository")}
            {header("Maturity", "maturity")}
            {header("Evidence", "evidence")}
            {header("Detector findings", "findings")}
            {header("Dirty", "dirty")}
            {header("Sync", "sync")}
          </tr>
        </thead>
        <tbody>
          {rows.map(({ series, sample }) => (
            <tr key={series.repository_id}>
              <th>
                <button
                  type="button"
                  className="analytics-table-link"
                  onClick={() => onDrillDown(series.repository_id)}
                >
                  {series.name}
                </button>
              </th>
              <td>{count(sample?.maturity_score)}</td>
              <td>{count(sample?.ci_readiness_score)}</td>
              <td>
                {count(
                  sample?.detector_findings_total ?? sample?.findings_total,
                )}
              </td>
              <td>{count(sample?.dirty_workspace_count)}</td>
              <td>{count(sample?.unsynced_workspace_count)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function latestRepositoryRow(
  series: AnalyticsSnapshot["repositories"][number],
) {
  return { series, sample: latest(series.samples) };
}

function FindingRail({
  analytics,
}: {
  analytics: AnalyticsSnapshot;
}): ReactElement {
  const findings = analytics.findings ?? [];
  return (
    <aside
      className="analytics-findings"
      aria-label="Deterministic analytics findings"
    >
      <div className="analytics-findings-heading">
        <div>
          <p className="eyebrow">Observed findings</p>
          <h3>What changed or needs evidence</h3>
        </div>
        <span>{findings.length}</span>
      </div>
      {findings.length ? (
        <ol>
          {findings.map((finding) => (
            <li
              key={finding.id}
              className={`analytics-finding analytics-finding-${finding.severity}`}
            >
              <span>{finding.kind.replace("-", " ")}</span>
              <strong>{finding.title}</strong>
              <p>{finding.detail}</p>
            </li>
          ))}
        </ol>
      ) : (
        <div className="analytics-state">
          No deterministic finding was generated for this range.
        </div>
      )}
    </aside>
  );
}

function CuratedWorkspace({
  analytics,
  repositoryFilter,
  onDrillDown,
}: {
  analytics: AnalyticsSnapshot;
  repositoryFilter: string;
  onDrillDown: (id: string) => void;
}): ReactElement {
  const samples = analytics.portfolio_samples;
  const latestSample = latest(samples);
  const catalog = analytics.metric_catalog ?? [];
  const repositories =
    repositoryFilter === "all"
      ? analytics.repositories
      : analytics.repositories.filter(
          (series) => series.repository_id === repositoryFilter,
        );
  const pick = (...ids: string[]) =>
    ids
      .map((id) => metric(catalog, id))
      .filter((item): item is MetricDefinition => Boolean(item));
  return (
    <>
      <FindingRail analytics={analytics} />
      <div className="analytics-kpi-grid">
        <div>
          <span>Refresh observations</span>
          <strong>{samples.length}</strong>
          <small>{analytics.range_days}-day query</small>
        </div>
        <div>
          <span>Local commits</span>
          <strong>{count(latestSample?.commits_last_30_days)}</strong>
          <small>Trailing 30-day window</small>
        </div>
        <div>
          <span>High-severity detector findings</span>
          <strong>{count(latestSample?.high_severity_findings)}</strong>
          <small>
            {latestSample?.quality_freshness ?? "Evidence unavailable"}
          </small>
        </div>
        <div>
          <span>Release ready</span>
          <strong>
            {latestSample
              ? `${latestSample.release_ready_repository_count}/${latestSample.release_rule_repository_count}`
              : "Unavailable"}
          </strong>
          <small>Configured rules only</small>
        </div>
      </div>
      <div className="analytics-workspace-grid">
        <ChartCard
          eyebrow="Delivery activity"
          title="Commit activity"
          description="Commit volume uses its own trailing-window scale. Branch divergence is never mixed onto this axis."
          wide
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("git.commits.trailing_30_days")}
            ariaLabel="Commit activity over time"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Branch state"
          title="Ahead and behind"
          description="Diverging bars compare point-in-time branch counts around a shared zero baseline."
          wide
        >
          <DivergenceChart repositories={repositories} />
        </ChartCard>
        <ChartCard
          eyebrow="Cohort posture"
          title="Quality × evidence coverage"
          description="Median lines create cohort-relative quadrants. Missing scores are excluded and remain visible in findings."
          wide
        >
          <QualityCoverageScatter repositories={repositories} />
        </ChartCard>
        <ChartCard
          eyebrow="Fleet friction"
          title="Workspace friction"
          description="Dirty and unsynced workspace counts share the same scope, denominator, aggregation, and time semantics."
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("workspaces.dirty", "workspaces.unsynced")}
            ariaLabel="Workspace friction over time"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Workspace activity"
          title="Activity composition"
          description="Active, interrupted, idle, and unknown workspaces are shown as a point-in-time composition over recent refreshes."
        >
          <GovernedBars
            samples={samples}
            metrics={pick(
              "workspaces.activity.active",
              "workspaces.activity.interrupted",
              "workspaces.activity.idle",
              "workspaces.activity.unknown",
            )}
            ariaLabel="Workspace activity composition"
            stacked
          />
        </ChartCard>
        <ChartCard
          eyebrow="Operational conditions"
          title="Active conditions"
          description="Open operational conditions remain on their own governed scale."
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("conditions.active")}
            ariaLabel="Active operational conditions over time"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Release readiness"
          title="Configured versus ready"
          description="Ready repositories are compared only with repositories that have configured local release rules."
        >
          <GovernedTrend
            samples={samples}
            metrics={pick(
              "release.configured_repositories",
              "release.ready_repositories",
            )}
            ariaLabel="Configured and release-ready repositories over time"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Remediation backlog"
          title="Action composition"
          description="Open, in-progress, blocked, deferred, and verified actions remain evidence-bound status counts."
        >
          <GovernedBars
            samples={samples}
            metrics={pick(
              "remediation.actions.open",
              "remediation.actions.in_progress",
              "remediation.actions.blocked",
              "remediation.actions.deferred",
              "remediation.actions.verified",
            )}
            ariaLabel="Remediation backlog composition"
            stacked
          />
        </ChartCard>
        <ChartCard
          eyebrow="Remediation progress"
          title="Verified eligible weight"
          description="Progress is verified action weight divided by non-deferred remediation weight; unavailable history is not rendered as zero."
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("remediation.progress_percent")}
            ariaLabel="Remediation progress over time"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Finding trend"
          title="Detector finding severity over time"
          description="Raw detector findings and high-severity detector findings share source, denominator, aggregation, and time semantics; maturity gaps remain a separate metric."
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("findings.detector_total", "findings.high_severity")}
            ariaLabel="Finding severity trend"
          />
        </ChartCard>
        <ChartCard
          eyebrow="Evidence matrix"
          title="Repository × gate coverage"
          description="Unavailable evidence is a first-class state, never rendered as zero."
        >
          <EvidenceHeatmap repositories={repositories} />
        </ChartCard>
        <ChartCard
          eyebrow="Repository comparison"
          title="Sortable evidence table"
          description="Use a repository name to drill the workspace down to one series."
          wide
        >
          <RepositoryTable
            analytics={{ ...analytics, repositories }}
            onDrillDown={onDrillDown}
          />
        </ChartCard>
      </div>
    </>
  );
}

function widgetCompatibleMetrics(
  widget: AnalyticsWidgetConfig,
  catalog: MetricDefinition[],
): MetricDefinition[] {
  return widget.metric_ids
    .map((id) => metric(catalog, id))
    .filter((item): item is MetricDefinition => Boolean(item));
}

function BuilderWorkspace({
  analytics,
}: {
  analytics: AnalyticsSnapshot;
}): ReactElement {
  const [widgets, setWidgets] = useState<AnalyticsWidgetConfig[]>([]);
  const [selectedMetric, setSelectedMetric] = useState(
    analytics.metric_catalog?.[0]?.id ?? "",
  );
  const [selectedType, setSelectedType] = useState<AnalyticsChartType>("line");
  const [name, setName] = useState("My analytics view");
  const [status, setStatus] = useState("");
  const [savedViews, setSavedViews] = useState(analytics.views ?? []);
  const [selectedViewId, setSelectedViewId] = useState(
    analytics.default_view_id ?? "curated",
  );
  const catalog = analytics.metric_catalog ?? [];
  const chosen = metric(catalog, selectedMetric);
  const allowed = chosen?.allowed_visualizations ?? [];
  const addWidget = (): void => {
    if (!chosen || !allowed.includes(selectedType)) return;
    setWidgets((current) => [
      ...current,
      {
        id: `widget-${Date.now()}`,
        title: chosen.label,
        metric_ids: [chosen.id],
        chart_type: selectedType,
        grouping: chosen.scope,
        width: 1,
        height: 1,
        order: current.length,
      },
    ]);
  };
  const move = (index: number, delta: number): void =>
    setWidgets((current) => {
      const next = [...current];
      const target = index + delta;
      if (target < 0 || target >= next.length) return current;
      [next[index], next[target]] = [next[target], next[index]];
      return next.map((widget, order) => ({ ...widget, order }));
    });
  const save = async (): Promise<void> => {
    const now = new Date().toISOString();
    const existing = savedViews.find(
      (view) => view.id === selectedViewId && !view.builtin,
    );
    const view: AnalyticsView = {
      schema_version: "pronto-analytics-view/v1",
      id:
        existing?.id ??
        `view-${
          name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, "-")
            .replace(/^-|-$/g, "") || Date.now()
        }`,
      name,
      builtin: false,
      is_default: existing?.is_default ?? false,
      filters: {
        range_days: analytics.range_days,
        repository_ids: [],
        group_ids: [],
        product_ids: [],
        freshness: "all",
      },
      widgets,
      created_at: existing?.created_at ?? now,
      updated_at: now,
    };
    try {
      const next = await api.saveAnalyticsView(view);
      setSavedViews(next);
      setSelectedViewId(view.id);
      setStatus("Saved locally");
    } catch (caught) {
      setStatus(
        caught instanceof Error ? caught.message : "Could not save this view",
      );
    }
  };
  const restore = (id: string): void => {
    setSelectedViewId(id);
    const view = savedViews.find((item) => item.id === id);
    if (view && !view.builtin) {
      setName(view.name);
      setWidgets(
        view.widgets.filter((widget) =>
          widget.metric_ids.every((metricId) =>
            catalog.some((item) => item.id === metricId),
          ),
        ),
      );
    } else {
      setWidgets([]);
      setName("My analytics view");
    }
  };
  const remove = async (): Promise<void> => {
    if (selectedViewId === "curated") return;
    try {
      const next = await api.deleteAnalyticsView(selectedViewId);
      setSavedViews(next);
      restore("curated");
      setStatus("View deleted");
    } catch (caught) {
      setStatus(
        caught instanceof Error ? caught.message : "Could not delete this view",
      );
    }
  };
  const makeDefault = async (): Promise<void> => {
    try {
      const next = await api.setDefaultAnalyticsView(selectedViewId);
      setSavedViews(next);
      setStatus("Default view updated");
    } catch (caught) {
      setStatus(
        caught instanceof Error
          ? caught.message
          : "Could not update the default view",
      );
    }
  };
  return (
    <div className="analytics-builder-layout">
      <aside className="analytics-builder-panel">
        <p className="eyebrow">Chart composer</p>
        <h3>Build from governed metrics</h3>
        <p>
          Formulas and dual axes are excluded. Chart choices follow each metric
          contract.
        </p>
        <label>
          Saved view
          <select
            value={selectedViewId}
            onChange={(event) => restore(event.target.value)}
          >
            {savedViews.map((view) => (
              <option key={view.id} value={view.id}>
                {view.name}
                {view.is_default ? " · default" : ""}
              </option>
            ))}
          </select>
        </label>
        <div className="analytics-builder-view-actions">
          <button
            type="button"
            className="button button-quiet"
            onClick={() => void makeDefault()}
          >
            Set default
          </button>
          <button
            type="button"
            className="button button-quiet"
            onClick={() => void remove()}
            disabled={selectedViewId === "curated"}
          >
            Delete
          </button>
        </div>
        <hr />
        <label>
          Metric
          <select
            value={selectedMetric}
            onChange={(event) => {
              const next = metric(catalog, event.target.value);
              setSelectedMetric(event.target.value);
              setSelectedType(next?.allowed_visualizations[0] ?? "table");
            }}
          >
            {catalog.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          Chart type
          <select
            value={selectedType}
            onChange={(event) =>
              setSelectedType(event.target.value as AnalyticsChartType)
            }
          >
            {BUILDER_TYPES.map((type) => (
              <option
                key={type}
                value={type}
                disabled={!allowed.includes(type)}
              >
                {type}
              </option>
            ))}
          </select>
        </label>
        <button
          className="button button-primary"
          type="button"
          onClick={addWidget}
          disabled={!chosen || !allowed.includes(selectedType)}
        >
          <Plus size={15} />
          Add widget
        </button>
        <hr />
        <label>
          View name
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <button
          className="button button-quiet"
          type="button"
          onClick={() => void save()}
          disabled={!widgets.length}
        >
          <Save size={15} />
          Save local view
        </button>
        {status && (
          <p className="analytics-builder-status" role="status">
            {status}
          </p>
        )}
      </aside>
      <div className="analytics-builder-canvas">
        {widgets.length === 0 ? (
          <div className="analytics-builder-empty">
            <Grip size={24} />
            <h3>Add your first widget</h3>
            <p>
              Long-tail metrics stay here instead of overcrowding the curated
              view.
            </p>
          </div>
        ) : (
          widgets.map((widget, index) => {
            const definitions = widgetCompatibleMetrics(widget, catalog);
            const visual =
              widget.chart_type === "bar" ||
              widget.chart_type === "stacked-bar" ? (
                <GovernedBars
                  samples={analytics.portfolio_samples}
                  metrics={definitions}
                  ariaLabel={widget.title}
                  stacked={widget.chart_type === "stacked-bar"}
                />
              ) : widget.chart_type === "diverging-bar" ? (
                <DivergenceChart repositories={analytics.repositories} />
              ) : widget.chart_type === "scatter" ? (
                <QualityCoverageScatter repositories={analytics.repositories} />
              ) : widget.chart_type === "heatmap" ||
                widget.chart_type === "table" ? (
                <EvidenceHeatmap repositories={analytics.repositories} />
              ) : (
                <GovernedTrend
                  samples={analytics.portfolio_samples}
                  metrics={definitions}
                  ariaLabel={widget.title}
                />
              );
            return (
              <article
                className={`analytics-builder-widget analytics-builder-widget-w${widget.width}`}
                key={widget.id}
              >
                <header>
                  <div>
                    <span>{widget.chart_type}</span>
                    <h3>{widget.title}</h3>
                  </div>
                  <div
                    className="analytics-widget-controls"
                    aria-label={`Controls for ${widget.title}`}
                  >
                    <button
                      type="button"
                      onClick={() => move(index, -1)}
                      disabled={index === 0}
                      aria-label={`Move ${widget.title} earlier`}
                    >
                      <ArrowUp size={14} />
                    </button>
                    <button
                      type="button"
                      onClick={() => move(index, 1)}
                      disabled={index === widgets.length - 1}
                      aria-label={`Move ${widget.title} later`}
                    >
                      <ArrowDown size={14} />
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        setWidgets((current) =>
                          current.map((item) =>
                            item.id === widget.id
                              ? { ...item, width: item.width === 1 ? 2 : 1 }
                              : item,
                          ),
                        )
                      }
                      aria-label={`Resize ${widget.title}`}
                    >
                      <Columns2 size={14} />
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        setWidgets((current) =>
                          current.filter((item) => item.id !== widget.id),
                        )
                      }
                      aria-label={`Remove ${widget.title}`}
                    >
                      <X size={14} />
                    </button>
                  </div>
                </header>
                {!metricsShareAxis(definitions) ? (
                  <div className="analytics-state analytics-state-conflicted">
                    Incompatible axis contract.
                  </div>
                ) : (
                  visual
                )}
              </article>
            );
          })
        )}
      </div>
    </div>
  );
}

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
        <CuratedWorkspace
          analytics={scopedAnalytics}
          repositoryFilter={effectiveRepositoryFilter}
          onDrillDown={setRepositoryFilter}
        />
      ) : (
        <BuilderWorkspace analytics={scopedAnalytics} />
      )}
    </section>
  );
}
