import {
  useState,
  type Dispatch,
  type ReactElement,
  type SetStateAction,
} from "react";
import {
  ArrowDown,
  ArrowUp,
  Columns2,
  Grip,
  Plus,
  Save,
  X,
} from "lucide-react";
import * as api from "../api";
import type {
  AnalyticsChartType,
  AnalyticsSnapshot,
  AnalyticsView,
  AnalyticsWidgetConfig,
  MetricDefinition,
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

function metric(
  catalog: MetricDefinition[],
  id: string,
): MetricDefinition | undefined {
  return catalog.find((item) => item.id === id);
}

function widgetMetrics(
  widget: AnalyticsWidgetConfig,
  catalog: MetricDefinition[],
): MetricDefinition[] {
  return widget.metric_ids
    .map((id) => metric(catalog, id))
    .filter((item): item is MetricDefinition => Boolean(item));
}

export function AnalyticsBuilderWorkspace({
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
      return;
    }
    setWidgets([]);
    setName("My analytics view");
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
          widgets.map((widget, index) => (
            <BuilderWidget
              key={widget.id}
              widget={widget}
              index={index}
              total={widgets.length}
              analytics={analytics}
              catalog={catalog}
              move={move}
              setWidgets={setWidgets}
            />
          ))
        )}
      </div>
    </div>
  );
}

function BuilderWidget({
  widget,
  index,
  total,
  analytics,
  catalog,
  move,
  setWidgets,
}: {
  widget: AnalyticsWidgetConfig;
  index: number;
  total: number;
  analytics: AnalyticsSnapshot;
  catalog: MetricDefinition[];
  move: (index: number, delta: number) => void;
  setWidgets: Dispatch<SetStateAction<AnalyticsWidgetConfig[]>>;
}): ReactElement {
  const definitions = widgetMetrics(widget, catalog);
  const visual =
    widget.chart_type === "bar" || widget.chart_type === "stacked-bar" ? (
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
    ) : widget.chart_type === "heatmap" || widget.chart_type === "table" ? (
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
            disabled={index === total - 1}
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
}
