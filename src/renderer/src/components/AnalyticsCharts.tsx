import type { ReactElement } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type {
  AnalyticsMetricSample,
  AnalyticsRepositorySeries,
  MetricDefinition,
} from "../types";
import {
  dailyLatestSamples,
  formatAnalyticsDate,
  formatAnalyticsNumber,
  formatAnalyticsTimestamp,
  metricValue,
} from "./AnalyticsChartFormatting";
import {
  QualityScatterTooltip,
  type QualityScatterTooltipPayload,
} from "./AnalyticsScatterTooltip";

const COLORS = [
  "var(--blue)",
  "var(--mint)",
  "var(--amber)",
  "var(--coral)",
  "var(--violet)",
];

export function metricsShareAxis(metrics: MetricDefinition[]): boolean {
  const [first] = metrics;
  return (
    !first ||
    metrics.every(
      (metric) =>
        metric.unit === first.unit &&
        metric.denominator === first.denominator &&
        metric.aggregation === first.aggregation &&
        metric.time_semantics === first.time_semantics &&
        metric.window_days === first.window_days,
    )
  );
}

export function GovernedTrend({
  samples,
  metrics,
  ariaLabel,
}: {
  samples: AnalyticsMetricSample[];
  metrics: MetricDefinition[];
  ariaLabel: string;
}): ReactElement {
  if (!metricsShareAxis(metrics))
    return (
      <div className="analytics-state analytics-state-conflicted" role="alert">
        These metrics cannot share an axis. Choose metrics with the same unit,
        denominator, aggregation, and time window.
      </div>
    );
  if (samples.length === 0)
    return (
      <div className="analytics-state">No refresh history in this range.</div>
    );
  if (
    metrics.length === 0 ||
    metrics.every((metric) =>
      samples.every((sample) => metricValue(sample, metric.id) === null),
    )
  )
    return (
      <div className="analytics-state">
        This metric was unavailable in the recorded history.
      </div>
    );
  const data = dailyLatestSamples(samples).map((sample) => ({
    observedAt: sample.observed_at,
    timestamp: Date.parse(sample.observed_at),
    ...Object.fromEntries(
      metrics.map((metric) => [metric.id, metricValue(sample, metric.id)]),
    ),
  }));
  return (
    <div className="analytics-chart-frame" role="img" aria-label={ariaLabel}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={data}
          accessibilityLayer
          margin={{ top: 12, right: 18, bottom: 8, left: 0 }}
        >
          <CartesianGrid stroke="var(--chart-grid)" vertical={false} />
          <XAxis
            dataKey="timestamp"
            type="number"
            scale="time"
            domain={["dataMin", "dataMax"]}
            tickFormatter={(value) => formatAnalyticsDate(Number(value))}
            tick={{ fill: "var(--muted)", fontSize: 12 }}
            tickLine={false}
            axisLine={false}
          />
          <YAxis
            tick={{ fill: "var(--muted)", fontSize: 12 }}
            tickLine={false}
            axisLine={false}
            width={42}
          />
          <Tooltip
            contentStyle={{
              background: "var(--panel)",
              border: "1px solid var(--line)",
              borderRadius: 12,
            }}
            labelFormatter={(_, payload) =>
              formatAnalyticsTimestamp(
                payload?.[0]?.payload?.observedAt ?? Date.now(),
              )
            }
            formatter={(value, name) => [
              formatAnalyticsNumber(Number(value)),
              name,
            ]}
          />
          <Legend />
          {metrics.map((metric, index) => (
            <Line
              key={metric.id}
              type="monotone"
              dataKey={metric.id}
              name={metric.label}
              stroke={COLORS[index % COLORS.length]}
              strokeWidth={3}
              dot={{ r: 3, strokeWidth: 0 }}
              activeDot={{ r: 6 }}
              connectNulls={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

export function GovernedBars({
  samples,
  metrics,
  ariaLabel,
  stacked = false,
}: {
  samples: AnalyticsMetricSample[];
  metrics: MetricDefinition[];
  ariaLabel: string;
  stacked?: boolean;
}): ReactElement {
  if (!metricsShareAxis(metrics))
    return (
      <div className="analytics-state analytics-state-conflicted" role="alert">
        These metrics cannot share an axis.
      </div>
    );
  if (!samples.length)
    return (
      <div className="analytics-state">No refresh history in this range.</div>
    );
  if (
    metrics.length === 0 ||
    metrics.every((metric) =>
      samples.every((sample) => metricValue(sample, metric.id) === null),
    )
  )
    return (
      <div className="analytics-state">
        This metric was unavailable in the recorded history.
      </div>
    );
  const data = dailyLatestSamples(samples)
    .slice(-12)
    .map((sample) => ({
      observedAt: sample.observed_at,
      timestamp: Date.parse(sample.observed_at),
      ...Object.fromEntries(
        metrics.map((item) => [item.id, metricValue(sample, item.id)]),
      ),
    }));
  return (
    <div className="analytics-chart-frame" role="img" aria-label={ariaLabel}>
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} accessibilityLayer>
          <CartesianGrid stroke="var(--chart-grid)" vertical={false} />
          <XAxis
            dataKey="timestamp"
            type="number"
            scale="time"
            domain={["dataMin", "dataMax"]}
            tickFormatter={(value) => formatAnalyticsDate(Number(value))}
            tick={{ fill: "var(--muted)", fontSize: 12 }}
          />
          <YAxis tick={{ fill: "var(--muted)", fontSize: 12 }} />
          <Tooltip
            contentStyle={{
              background: "var(--panel)",
              border: "1px solid var(--line)",
              borderRadius: 12,
            }}
            labelFormatter={(_, payload) =>
              formatAnalyticsTimestamp(
                payload?.[0]?.payload?.observedAt ?? Date.now(),
              )
            }
            formatter={(value, name) => [
              formatAnalyticsNumber(Number(value)),
              name,
            ]}
          />
          <Legend />
          {metrics.map((item, index) => (
            <Bar
              key={item.id}
              dataKey={item.id}
              name={item.label}
              fill={COLORS[index % COLORS.length]}
              stackId={stacked ? "governed" : undefined}
              radius={[5, 5, 0, 0]}
            />
          ))}
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function DivergenceChart({
  repositories,
}: {
  repositories: AnalyticsRepositorySeries[];
}): ReactElement {
  const data = repositories
    .map((series) => {
      const sample = series.samples.at(-1);
      return {
        name: series.name,
        ahead: sample ? metricValue(sample, "git.ahead_commits") : null,
        behind: sample
          ? -(metricValue(sample, "git.behind_commits") ?? 0)
          : null,
      };
    })
    .filter((item) => item.ahead != null || item.behind != null)
    .sort(
      (a, b) =>
        Math.abs((b.ahead ?? 0) + (b.behind ?? 0)) -
        Math.abs((a.ahead ?? 0) + (a.behind ?? 0)),
    )
    .slice(0, 10);
  if (!data.length)
    return (
      <div className="analytics-state">Branch divergence is unavailable.</div>
    );
  return (
    <div
      className="analytics-chart-frame analytics-chart-frame-tall"
      role="img"
      aria-label="Ahead and behind commits by repository"
    >
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={data}
          layout="vertical"
          stackOffset="sign"
          accessibilityLayer
          margin={{ left: 18, right: 18 }}
        >
          <CartesianGrid stroke="var(--chart-grid)" horizontal={false} />
          <XAxis type="number" tick={{ fill: "var(--muted)", fontSize: 12 }} />
          <YAxis
            type="category"
            dataKey="name"
            width={118}
            tick={{ fill: "var(--text)", fontSize: 12 }}
            tickLine={false}
            axisLine={false}
          />
          <Tooltip
            contentStyle={{
              background: "var(--panel)",
              border: "1px solid var(--line)",
              borderRadius: 12,
            }}
            formatter={(value, name) => [Math.abs(Number(value)), name]}
          />
          <ReferenceLine x={0} stroke="var(--muted)" />
          <Bar
            dataKey="behind"
            name="Behind"
            fill="var(--coral)"
            radius={[5, 0, 0, 5]}
          />
          <Bar
            dataKey="ahead"
            name="Ahead"
            fill="var(--blue)"
            radius={[0, 5, 5, 0]}
          />
          <Legend />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function QualityCoverageScatter({
  repositories,
}: {
  repositories: AnalyticsRepositorySeries[];
}): ReactElement {
  const data = repositories
    .map((series) => {
      const sample = series.samples.at(-1);
      return {
        name: series.name,
        maturity: sample ? metricValue(sample, "quality.maturity_score") : null,
        evidence: sample ? metricValue(sample, "quality.evidence_score") : null,
      };
    })
    .filter(
      (item): item is { name: string; maturity: number; evidence: number } =>
        item.maturity != null && item.evidence != null,
    );
  if (!data.length)
    return (
      <div className="analytics-state">
        Quality and evidence coverage are unavailable for this cohort.
      </div>
    );
  const xMedian = [...data].sort((a, b) => a.evidence - b.evidence)[
    Math.floor(data.length / 2)
  ].evidence;
  const yMedian = [...data].sort((a, b) => a.maturity - b.maturity)[
    Math.floor(data.length / 2)
  ].maturity;
  return (
    <div
      className="analytics-chart-frame analytics-chart-frame-tall"
      role="img"
      aria-label="Quality maturity versus evidence coverage"
    >
      <ResponsiveContainer width="100%" height="100%">
        <ScatterChart
          accessibilityLayer
          margin={{ top: 18, right: 18, bottom: 12, left: 0 }}
        >
          <CartesianGrid stroke="var(--chart-grid)" />
          <XAxis
            type="number"
            dataKey="evidence"
            name="Evidence coverage"
            domain={[0, 4]}
            tickFormatter={(value) => formatAnalyticsNumber(Number(value))}
            tick={{ fill: "var(--muted)" }}
            label={{
              value: "Evidence coverage",
              position: "insideBottom",
              offset: -8,
              fill: "var(--muted)",
            }}
          />
          <YAxis
            type="number"
            dataKey="maturity"
            name="Maturity"
            domain={[0, 4]}
            tickFormatter={(value) => formatAnalyticsNumber(Number(value))}
            tick={{ fill: "var(--muted)" }}
          />
          <ReferenceLine
            x={xMedian}
            stroke="var(--amber)"
            strokeDasharray="5 5"
          />
          <ReferenceLine
            y={yMedian}
            stroke="var(--amber)"
            strokeDasharray="5 5"
          />
          <Tooltip
            cursor={{ strokeDasharray: "3 3" }}
            content={({ active, payload }) => (
              <QualityScatterTooltip
                active={active}
                payload={
                  payload as
                    ReadonlyArray<QualityScatterTooltipPayload> | undefined
                }
              />
            )}
          />
          <Scatter name="Repositories" data={data} fill="var(--mint)">
            {data.map((entry) => (
              <Cell
                key={entry.name}
                fill={
                  entry.evidence >= xMedian && entry.maturity >= yMedian
                    ? "var(--mint)"
                    : "var(--amber)"
                }
              />
            ))}
          </Scatter>
        </ScatterChart>
      </ResponsiveContainer>
    </div>
  );
}

export function EvidenceHeatmap({
  repositories,
}: {
  repositories: AnalyticsRepositorySeries[];
}): ReactElement {
  const gates = ["Quality", "Findings", "Git sync", "Release"];
  return (
    <div
      className="analytics-heatmap"
      role="table"
      aria-label="Repository evidence coverage"
    >
      <div
        className="analytics-heatmap-row analytics-heatmap-header"
        role="row"
      >
        <span role="columnheader">Repository</span>
        {gates.map((gate) => (
          <span role="columnheader" key={gate}>
            {gate}
          </span>
        ))}
      </div>
      {repositories.slice(0, 12).map((series) => {
        const sample = series.samples.at(-1);
        const states = [
          sample?.ci_readiness_score != null,
          sample?.findings_total != null,
          sample != null,
          (sample?.release_rule_repository_count ?? 0) > 0,
        ];
        return (
          <div
            className="analytics-heatmap-row"
            role="row"
            key={series.repository_id}
          >
            <strong role="rowheader">{series.name}</strong>
            {states.map((available, index) => (
              <span
                role="cell"
                className={
                  available
                    ? "heatmap-cell heatmap-cell-ready"
                    : "heatmap-cell heatmap-cell-missing"
                }
                key={gates[index]}
                title={`${gates[index]}: ${available ? "available" : "unavailable"}`}
              >
                {available ? "Ready" : "Missing"}
              </span>
            ))}
          </div>
        );
      })}
    </div>
  );
}
