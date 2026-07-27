import type { ReactElement, ReactNode } from "react";
import type { AnalyticsMetricSample } from "../types";

export interface TrendSeries {
  label: string;
  color: string;
  getValue: (sample: AnalyticsMetricSample) => number | undefined;
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
  value: number | undefined;
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
  const height = compact ? 176 : 220;
  const padding = { top: 15, right: 16, bottom: 32, left: 42 };
  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;
  const values = series.flatMap((item) =>
    samples
      .map(item.getValue)
      .filter((value): value is number => value !== undefined),
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
            return value === undefined ? undefined : pointFor(index, value);
          });
          const path = chartPath(points);
          return (
            <g key={item.label}>
              {path && (
                <path
                  className="chart-line"
                  d={path}
                  stroke={item.color}
                  vectorEffect="non-scaling-stroke"
                />
              )}
              {points.map((point, index) =>
                point ? (
                  <circle
                    className="chart-point"
                    cx={point.x}
                    cy={point.y}
                    fill={item.color}
                    r={samples.length === 1 ? 4 : 3}
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
      <div className="chart-legend" aria-label="Chart legend">
        {series.map((item) => {
          const latest = item.getValue(samples[samples.length - 1]);
          return (
            <div className="chart-legend-item" key={item.label}>
              <span
                className="chart-legend-mark"
                style={{ background: item.color }}
              />
              <span>{item.label}</span>
              <strong>
                {latest === undefined
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
  const height = compact ? 132 : 150;
  const barX = 18;
  const barWidth = width - 36;
  const barY = 35;
  const barHeight = 34;
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
          const element = (
            <rect
              key={segment.label}
              x={offset}
              y={barY}
              width={segmentWidth}
              height={barHeight}
              fill={segment.color}
              rx={segmentWidth === barWidth ? 7 : 0}
            />
          );
          offset += segmentWidth;
          return element;
        })}
        <text className="chart-total-label" x={barX} y={barY - 12}>
          {formatCount(total)} total observations
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
  return value.length > 25 ? `${value.slice(0, 22)}…` : value;
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
  const visibleItems = items.filter((item) => item.value !== undefined);
  if (visibleItems.length === 0) {
    return (
      <ChartEmptyState
        label="No repository comparison yet"
        detail="Repository comparisons appear after a local refresh records current state."
      />
    );
  }
  const width = 640;
  const rowHeight = 42;
  const height = Math.max(104, visibleItems.length * rowHeight + 22);
  const labelWidth = 190;
  const barWidth = width - labelWidth - 64;
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
          const y = 19 + index * rowHeight;
          return (
            <g key={item.label}>
              <title>{`${item.label}: ${formatDefaultValue(value)}`}</title>
              <text className="chart-comparison-label" x={0} y={y + 13}>
                {shortLabel(item.label)}
              </text>
              <rect
                className="chart-track"
                x={labelWidth}
                y={y + 3}
                width={barWidth}
                height={17}
                rx={5}
              />
              <rect
                x={labelWidth}
                y={y + 3}
                width={(value / maximum) * barWidth}
                height={17}
                rx={5}
                fill={item.color}
              />
              <text className="chart-comparison-value" x={width - 2} y={y + 16}>
                {formatDefaultValue(value)}
              </text>
              {item.detail && (
                <text
                  className="chart-comparison-detail"
                  x={labelWidth}
                  y={y + 35}
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
