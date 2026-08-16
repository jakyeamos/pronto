import type { ReactElement, ReactNode } from "react";
import type { AnalyticsMetricSample } from "../types";

export interface TrendSeries {
  label: string;
  color: string;
  getValue: (sample: AnalyticsMetricSample) => number | null | undefined;
  formatValue?: (value: number) => string;
}

export interface StackedBarSegment {
  label: string;
  color: string;
  value: number;
}

export interface HorizontalBarItem {
  label: string;
  color: string;
  value: number | null | undefined;
  detail?: string;
}

interface ChartEmptyStateProps {
  label: string;
  detail: string;
}

function ChartEmptyState({
  label,
  detail,
}: ChartEmptyStateProps): ReactElement {
  return (
    <div className="chart-empty" role="status" aria-label={label}>
      <strong>{label}</strong>
      <span>{detail}</span>
    </div>
  );
}

function formatCount(value: number): string {
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 }).format(
    value,
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Unknown";
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
  }).format(date);
}

function formatDefaultValue(value: number): string {
  return Number.isInteger(value) ? formatCount(value) : value.toFixed(1);
}

function chartPath(
  points: Array<{ x: number; y: number } | undefined>,
): string {
  let path = "";
  let shouldMove = true;
  points.forEach((point) => {
    if (!point) {
      shouldMove = true;
      return;
    }
    path += `${shouldMove ? "M" : " L"}${point.x.toFixed(2)},${point.y.toFixed(2)}`;
    shouldMove = false;
  });
  return path;
}

const svgLineColors: Record<string, string> = {
  "#68d6b1": "#68d6b1",
  "#77a8ff": "#77a8ff",
  "#b9a5ff": "#b9a5ff",
  "#f08e8e": "#f08e8e",
  "#f2bc71": "#f2bc71",
  "var(--amber)": "#f2bc71",
  "var(--blue)": "#77a8ff",
  "var(--coral)": "#f08e8e",
  "var(--mint)": "#68d6b1",
  "var(--violet)": "#b9a5ff",
};
const svgLineClasses: Record<string, string> = {
  "#68d6b1": "chart-line-mint",
  "#77a8ff": "chart-line-blue",
  "#b9a5ff": "chart-line-violet",
  "#f08e8e": "chart-line-coral",
  "#f2bc71": "chart-line-amber",
};

function svgLineColor(value: string): string {
  return svgLineColors[value] ?? value;
}

function svgLineClass(value: string): string {
  return svgLineClasses[svgLineColor(value)] ?? "chart-line-custom";
}

export function AnalyticsChartCard({
  eyebrow,
  title,
  description,
  source,
  freshness,
  summary,
  compact = false,
  children,
}: {
  eyebrow: string;
  title: string;
  description: string;
  source: string;
  freshness: string;
  summary?: string;
  compact?: boolean;
  children: ReactNode;
}): ReactElement {
  return (
    <section
      className={`analytics-chart-card ${compact ? "analytics-chart-card-compact" : ""}`}
    >
      <div className="analytics-chart-card-heading">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </div>
      {children}
      {summary && <p className="chart-summary">{summary}</p>}
      <div className="analytics-chart-card-footer">
        <span>{source}</span>
        <span>{freshness}</span>
      </div>
    </section>
  );
}

export function TrendChart({
  samples,
  series,
  ariaLabel,
  summary,
  yMax,
  compact = false,
}: {
  samples: AnalyticsMetricSample[];
  series: TrendSeries[];
  ariaLabel: string;
  summary: string;
  yMax?: number;
  compact?: boolean;
}): ReactElement {
  if (samples.length === 0) {
    return (
      <ChartEmptyState
        label="No refresh history yet"
        detail="Run a local refresh to record the first observation."
      />
    );
  }
  const width = 640;
  const height = compact ? 208 : 260;
  const padding = { top: 22, right: 24, bottom: 40, left: 52 };
  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;
  const values = series.flatMap((item) =>
    samples
      .map(item.getValue)
      .filter((value): value is number => value != null),
  );
  if (values.length === 0) {
    return (
      <ChartEmptyState
        label="Evidence unavailable"
        detail="No source metric was available in the selected refresh history."
      />
    );
  }
  const maximum = yMax ?? Math.max(...values, 1);
  const minimum = 0;
  const valueRange = Math.max(maximum - minimum, 1);
  const pointFor = (
    index: number,
    value: number,
  ): { x: number; y: number } => ({
    x:
      padding.left +
      (samples.length === 1
        ? plotWidth / 2
        : (index / (samples.length - 1)) * plotWidth),
    y: padding.top + ((maximum - value) / valueRange) * plotHeight,
  });
  const gridValues = [maximum, maximum / 2, minimum];

  return (
    <div className="chart-visual">
      <svg
        className="chart-svg"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={ariaLabel}
      >
        <title>{ariaLabel}</title>
        <desc>{summary}</desc>
        {gridValues.map((value) => {
          const y = padding.top + ((maximum - value) / valueRange) * plotHeight;
          return (
            <g key={value}>
              <line
                className="chart-grid-line"
                x1={padding.left}
                x2={width - padding.right}
                y1={y}
                y2={y}
              />
              <text className="chart-axis-label" x={padding.left - 8} y={y + 3}>
                {formatDefaultValue(value)}
              </text>
            </g>
          );
        })}
        <line
          className="chart-axis-line"
          x1={padding.left}
          x2={width - padding.right}
          y1={height - padding.bottom}
          y2={height - padding.bottom}
        />
        {series.map((item) => {
          const points = samples.map((sample, index) => {
            const value = item.getValue(sample);
            return value == null ? undefined : pointFor(index, value);
          });
          const path = chartPath(points);
          return (
            <g key={item.label}>
              {path && (
                <path
                  className={`chart-line ${svgLineClass(item.color)}`}
                  d={path}
                  fill="none"
                  stroke={svgLineColor(item.color)}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={12}
                  style={{ opacity: 1, strokeWidth: 12 }}
                />
              )}
              {points.map((point, index) =>
                point ? (
                  <circle
                    className="chart-point"
                    cx={point.x}
                    cy={point.y}
                    fill={item.color}
                    r={samples.length === 1 ? 5 : 4}
                    key={`${item.label}-${index}`}
                  />
                ) : null,
              )}
            </g>
          );
        })}
        <text className="chart-axis-label" x={padding.left} y={height - 10}>
          {formatDate(samples[0].observed_at)}
        </text>
        <text
          className="chart-axis-label chart-axis-label-end"
          x={width - padding.right}
          y={height - 10}
        >
          {formatDate(samples[samples.length - 1].observed_at)}
        </text>
      </svg>
      <div className="chart-legend" aria-label="Latest chart values">
        {series.map((item) => {
          const latest = item.getValue(samples[samples.length - 1]);
          return (
            <div
              className="chart-legend-item chart-legend-item-trend"
              key={item.label}
            >
              <span
                className="chart-legend-mark chart-legend-line"
                style={{ background: item.color }}
              />
              <span>{item.label}</span>
              <strong>
                {latest == null
                  ? "Unavailable"
                  : (item.formatValue?.(latest) ?? formatDefaultValue(latest))}
              </strong>
            </div>
          );
        })}
      </div>
      <p className="chart-summary">{summary}</p>
      {samples.length === 1 && (
        <p className="chart-insufficient">
          One observation only. Refresh again to build a trend.
        </p>
      )}
    </div>
  );
}

export function StackedBarChart({
  segments,
  ariaLabel,
  summary,
  compact = false,
}: {
  segments: StackedBarSegment[];
  ariaLabel: string;
  summary: string;
  compact?: boolean;
}): ReactElement {
  const visibleSegments = segments.filter((segment) => segment.value > 0);
  const total = visibleSegments.reduce(
    (sum, segment) => sum + segment.value,
    0,
  );
  if (total === 0) {
    return (
      <ChartEmptyState
        label="No activity evidence yet"
        detail="A successful local refresh will populate this composition."
      />
    );
  }
  const width = 640;
  const height = compact ? 142 : 160;
  const barX = 18;
  const barWidth = width - 36;
  const barY = 42;
  const barHeight = 38;
  let offset = barX;
  return (
    <div className="chart-visual">
      <svg
        className="chart-svg chart-svg-stacked"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={ariaLabel}
      >
        <title>{ariaLabel}</title>
        <desc>{summary}</desc>
        <rect
          className="chart-track"
          x={barX}
          y={barY}
          width={barWidth}
          height={barHeight}
          rx={7}
        />
        {visibleSegments.map((segment) => {
          const segmentWidth = (segment.value / total) * barWidth;
          const segmentX = offset;
          const element = (
            <g key={segment.label}>
              <rect
                x={segmentX}
                y={barY}
                width={segmentWidth}
                height={barHeight}
                fill={segment.color}
                rx={segmentWidth === barWidth ? 7 : 0}
              />
              {segmentWidth >= 46 && (
                <text
                  className="chart-segment-label"
                  x={segmentX + segmentWidth / 2}
                  y={barY + 24}
                >
                  {formatCount(segment.value)}
                </text>
              )}
            </g>
          );
          offset += segmentWidth;
          return element;
        })}
        <text className="chart-total-label" x={barX} y={barY - 12}>
          {formatCount(total)} total
        </text>
      </svg>
      <div
        className="chart-legend chart-legend-stacked"
        aria-label="Chart legend"
      >
        {segments.map((segment) => (
          <div className="chart-legend-item" key={segment.label}>
            <span
              className="chart-legend-mark"
              style={{ background: segment.color }}
            />
            <span>{segment.label}</span>
            <strong>{formatCount(segment.value)}</strong>
            <small>{Math.round((segment.value / total) * 100)}%</small>
          </div>
        ))}
      </div>
      <p className="chart-summary">{summary}</p>
    </div>
  );
}

function shortLabel(value: string): string {
  return value.length > 31 ? `${value.slice(0, 28)}…` : value;
}

export function HorizontalBarChart({
  items,
  ariaLabel,
  summary,
}: {
  items: HorizontalBarItem[];
  ariaLabel: string;
  summary: string;
}): ReactElement {
  const visibleItems = items.filter(
    (item): item is HorizontalBarItem & { value: number } => item.value != null,
  );
  if (visibleItems.length === 0) {
    return (
      <ChartEmptyState
        label="No repository comparison yet"
        detail="Repository comparisons appear after a local refresh records current state."
      />
    );
  }
  const width = 640;
  const rowHeight = 52;
  const height = Math.max(116, visibleItems.length * rowHeight + 24);
  const labelWidth = 220;
  const barWidth = width - labelWidth - 72;
  const maximum = Math.max(...visibleItems.map((item) => item.value ?? 0), 1);
  return (
    <div className="chart-visual chart-visual-comparison">
      <svg
        className="chart-svg"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={ariaLabel}
      >
        <title>{ariaLabel}</title>
        <desc>{summary}</desc>
        {visibleItems.map((item, index) => {
          const value = item.value ?? 0;
          const y = 20 + index * rowHeight;
          return (
            <g key={item.label}>
              <title>{`${item.label}: ${formatDefaultValue(value)}`}</title>
              <text className="chart-comparison-label" x={0} y={y + 13}>
                {shortLabel(item.label)}
              </text>
              <rect
                className="chart-track"
                x={labelWidth}
                y={y + 2}
                width={barWidth}
                height={22}
                rx={5}
              />
              <rect
                x={labelWidth}
                y={y + 2}
                width={(value / maximum) * barWidth}
                height={22}
                rx={5}
                fill={item.color}
              />
              <text className="chart-comparison-value" x={width - 2} y={y + 18}>
                {formatDefaultValue(value)}
              </text>
              {item.detail && (
                <text
                  className="chart-comparison-detail"
                  x={labelWidth}
                  y={y + 42}
                >
                  {item.detail}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <p className="chart-summary">{summary}</p>
    </div>
  );
}
