import type { ReactElement } from "react";
import type {
  AnalyticsMetricSample,
  AnalyticsRepositorySeries,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import {
  AnalyticsChartCard,
  HorizontalBarChart,
  StackedBarChart,
  TrendChart,
  type HorizontalBarItem,
  type StackedBarSegment,
  type TrendSeries,
} from "./ChartPrimitives";

const BLUE = "var(--blue)";
const MINT = "var(--mint)";
const AMBER = "var(--amber)";
const CORAL = "var(--coral)";
const VIOLET = "var(--violet)";

function latestSample(
  samples: AnalyticsMetricSample[],
): AnalyticsMetricSample | undefined {
  return samples[samples.length - 1];
}

function formatScore(value: number | undefined): string {
  return value === undefined ? "Unavailable" : `${value.toFixed(1)}/4`;
}

function formatCount(value: number | undefined): string {
  return value === undefined ? "Unavailable" : `${value}`;
}

function formatObservedAt(value: string | undefined): string {
  if (!value) return "No observation yet";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Observation time unavailable";
  return `Observed ${new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date)}`;
}

function chartSource(
  analytics: AnalyticsSnapshot,
  samples: AnalyticsMetricSample[],
): string {
  return samples.length > 0
    ? `${analytics.source} · ${samples.length} observation${samples.length === 1 ? "" : "s"}`
    : analytics.source;
}

function chartFreshness(
  analytics: AnalyticsSnapshot,
  samples: AnalyticsMetricSample[],
): string {
  return samples.length > 0
    ? formatObservedAt(latestSample(samples)?.observed_at)
    : analytics.freshness;
}

function healthSeries(): TrendSeries[] {
  return [
    {
      label: "Active conditions",
      color: AMBER,
      getValue: (sample) => sample.active_condition_count,
    },
    {
      label: "Dirty workspaces",
      color: CORAL,
      getValue: (sample) => sample.dirty_workspace_count,
    },
    {
      label: "Unsynced workspaces",
      color: BLUE,
      getValue: (sample) => sample.unsynced_workspace_count,
    },
  ];
}

function deliverySeries(): TrendSeries[] {
  return [
    {
      label: "Commits · 30 days",
      color: MINT,
      getValue: (sample) => sample.commits_last_30_days,
    },
    {
      label: "Ahead commits",
      color: BLUE,
      getValue: (sample) => sample.ahead_commit_count,
    },
    {
      label: "Behind commits",
      color: AMBER,
      getValue: (sample) => sample.behind_commit_count,
    },
  ];
}

function qualitySeries(): TrendSeries[] {
  return [
    {
      label: "Maturity",
      color: VIOLET,
      getValue: (sample) => sample.maturity_score,
      formatValue: formatScore,
    },
    {
      label: "Fresh passing evidence score",
      color: BLUE,
      getValue: (sample) => sample.ci_readiness_score,
      formatValue: formatScore,
    },
  ];
}

function findingSeries(): TrendSeries[] {
  return [
    {
      label: "All findings",
      color: AMBER,
      getValue: (sample) => sample.findings_total,
    },
    {
      label: "High severity",
      color: CORAL,
      getValue: (sample) => sample.high_severity_findings,
    },
  ];
}

function workspaceSegments(
  sample: AnalyticsMetricSample | undefined,
): StackedBarSegment[] {
  return [
    {
      label: "Active",
      color: MINT,
      value: sample?.active_workspace_count ?? 0,
    },
    {
      label: "Interrupted",
      color: CORAL,
      value: sample?.interrupted_workspace_count ?? 0,
    },
    { label: "Idle", color: BLUE, value: sample?.idle_workspace_count ?? 0 },
    {
      label: "Unknown",
      color: AMBER,
      value: sample?.unknown_workspace_count ?? 0,
    },
  ];
}

function attentionSegments(
  sample: AnalyticsMetricSample | undefined,
): StackedBarSegment[] {
  return [
    {
      label: "Active conditions",
      color: AMBER,
      value: sample?.active_condition_count ?? 0,
    },
    {
      label: "Dirty workspaces",
      color: CORAL,
      value: sample?.dirty_workspace_count ?? 0,
    },
    {
      label: "Unsynced workspaces",
      color: BLUE,
      value: sample?.unsynced_workspace_count ?? 0,
    },
  ];
}

function releaseSegments(
  sample: AnalyticsMetricSample | undefined,
): StackedBarSegment[] {
  const configured = sample?.release_rule_repository_count ?? 0;
  const ready = Math.min(
    sample?.release_ready_repository_count ?? 0,
    configured,
  );
  return [
    { label: "Threshold met", color: MINT, value: ready },
    { label: "Configured, pending", color: AMBER, value: configured - ready },
    {
      label: "No local rule",
      color: "var(--faint)",
      value: Math.max((sample?.repository_count ?? 0) - configured, 0),
    },
  ];
}

function latestRepositorySeries(
  analytics: AnalyticsSnapshot,
  repositories: RepositorySnapshot[],
): HorizontalBarItem[] {
  return repositories
    .map((repository) => {
      const series = analytics.repositories.find(
        (candidate) => candidate.repository_id === repository.id,
      );
      const sample = latestSample(series?.samples ?? []);
      if (!sample) {
        return {
          label: repository.name,
          color: "var(--faint)",
          value: undefined,
        };
      }
      const attention =
        sample.active_condition_count +
        sample.dirty_workspace_count +
        sample.unsynced_workspace_count;
      return {
        label: repository.name,
        color: attention > 0 ? AMBER : MINT,
        value: attention,
        detail: `${sample.active_condition_count} conditions · ${sample.dirty_workspace_count} dirty · ${sample.unsynced_workspace_count} unsynced`,
      };
    })
    .sort((left, right) => (right.value ?? -1) - (left.value ?? -1))
    .slice(0, 8);
}

function qualityPostureSummary(
  sample: AnalyticsMetricSample | undefined,
): string {
  if (!sample) return "No refresh sample is available for quality posture.";
  return `Maturity ${formatScore(sample.maturity_score)} · Fresh passing evidence score ${formatScore(sample.ci_readiness_score)} · ${formatCount(sample.findings_total)} findings · Quality evidence ${sample.quality_freshness ?? "Unavailable"}`;
}

function AnalyticsChartSet({
  analytics,
  samples,
  compact = false,
}: {
  analytics: AnalyticsSnapshot;
  samples: AnalyticsMetricSample[];
  compact?: boolean;
}): ReactElement {
  const latest = latestSample(samples);
  return (
    <>
      <AnalyticsChartCard
        eyebrow="Health trend"
        title="Fleet health"
        description="Conditions and local workspace friction observed at refresh time."
        source={chartSource(analytics, samples)}
        freshness={chartFreshness(analytics, samples)}
        compact={compact}
      >
        <TrendChart
          samples={samples}
          series={healthSeries()}
          ariaLabel="Fleet health trend"
          summary="Active conditions, dirty workspaces, and unsynced workspaces over the last 30 days."
          compact={compact}
        />
      </AnalyticsChartCard>
      <AnalyticsChartCard
        eyebrow="Delivery activity"
        title="Change flow"
        description="Local commits and branch divergence, with missing Git evidence kept visible."
        source={chartSource(analytics, samples)}
        freshness={chartFreshness(analytics, samples)}
        compact={compact}
      >
        <TrendChart
          samples={samples}
          series={deliverySeries()}
          ariaLabel="Delivery activity trend"
          summary="Local commits observed in the trailing 30 days alongside ahead and behind branch counts."
          compact={compact}
        />
      </AnalyticsChartCard>
      <AnalyticsChartCard
        eyebrow="Quality posture"
        title="Maturity and fresh passing evidence"
        description="External maturity and fresh passing evidence each use an explicit four-point score; unavailable evidence is not zero."
        source={chartSource(analytics, samples)}
        freshness={chartFreshness(analytics, samples)}
        summary={qualityPostureSummary(latest)}
        compact={compact}
      >
        <TrendChart
          samples={samples}
          series={qualitySeries()}
          ariaLabel="Quality posture trend"
          summary="Fleet maturity and fresh passing evidence scores over the last 30 days."
          yMax={4}
          compact={compact}
        />
      </AnalyticsChartCard>
    </>
  );
}

export function AnalyticsDashboardBand({
  analytics,
}: {
  analytics: AnalyticsSnapshot;
}): ReactElement {
  return (
    <section
      className="analytics-dashboard-band"
      aria-label="Portfolio analytics"
    >
      <div className="analytics-section-heading">
        <div>
          <p className="eyebrow">Last 30 days · read-only</p>
          <h2>Portfolio signals</h2>
          <p>Compact trends from successful local refresh snapshots.</p>
        </div>
        <span className="analytics-range-note">{analytics.freshness}</span>
      </div>
      <div className="analytics-dashboard-grid">
        <AnalyticsChartSet
          analytics={analytics}
          samples={analytics.portfolio_samples}
          compact
        />
      </div>
    </section>
  );
}

export function AnalyticsSurface({
  analytics,
  repositories,
}: {
  analytics: AnalyticsSnapshot;
  repositories: RepositorySnapshot[];
}): ReactElement {
  const samples = analytics.portfolio_samples;
  const latest = latestSample(samples);
  const repositoryComparison = latestRepositorySeries(analytics, repositories);
  return (
    <section className="analytics-surface" aria-label="Analytics">
      <div className="analytics-surface-header">
        <div>
          <p className="eyebrow">Fixed range · Last 30 days</p>
          <h2>Evidence, over time.</h2>
          <p>
            These charts are read-only aggregates captured after successful
            local refreshes. Remote-provider history is intentionally excluded.
          </p>
        </div>
        <div className="analytics-source-block">
          <strong>{analytics.source}</strong>
          <span>{analytics.freshness}</span>
          <span>Retention · {analytics.retention_days} days</span>
        </div>
      </div>
      <div className="analytics-summary-grid">
        <div>
          <span>Refresh observations</span>
          <strong>{samples.length}</strong>
          <small>
            {analytics.history_available_from
              ? "History available"
              : "Awaiting first refresh"}
          </small>
        </div>
        <div>
          <span>Repositories covered</span>
          <strong>{latest?.repository_count ?? repositories.length}</strong>
          <small>Local repository snapshots</small>
        </div>
        <div>
          <span>Latest local commits</span>
          <strong>{formatCount(latest?.commits_last_30_days)}</strong>
          <small>Trailing 30-day window</small>
        </div>
        <div>
          <span>Release thresholds met</span>
          <strong>
            {latest
              ? `${latest.release_ready_repository_count}/${latest.release_rule_repository_count}`
              : "Unavailable"}
          </strong>
          <small>Configured local rules only</small>
        </div>
      </div>
      <div className="analytics-chart-grid">
        <AnalyticsChartSet analytics={analytics} samples={samples} />
        <AnalyticsChartCard
          eyebrow="Workspace activity"
          title="Activity states"
          description="Current refresh composition across active, interrupted, idle, and unknown workspaces."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <StackedBarChart
            segments={workspaceSegments(latest)}
            ariaLabel="Workspace activity composition"
            summary="Workspace activity state composition at the latest local refresh."
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Quality and security"
          title="Finding trend"
          description="Findings remain tied to quality evidence freshness and may be unavailable between audits."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <TrendChart
            samples={samples}
            series={findingSeries()}
            ariaLabel="Quality finding trend"
            summary="Total and high-severity quality findings over the last 30 days."
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Attention composition"
          title="Why work is waiting"
          description="The latest local sample separates active conditions from workspace and sync friction."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <StackedBarChart
            segments={attentionSegments(latest)}
            ariaLabel="Attention composition"
            summary="Latest attention composition across active conditions, dirty workspaces, and unsynced workspaces."
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Repository comparison"
          title="Attention load by repository"
          description="A compact comparison of current local attention signals; repository names are labels only."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <HorizontalBarChart
            items={repositoryComparison}
            ariaLabel="Repository attention comparison"
            summary="Repositories compared by the sum of active conditions, dirty workspaces, and unsynced workspaces."
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Release readiness"
          title="Configured thresholds"
          description="Local release rules are shown as context; Pronto does not publish or mutate releases here."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <StackedBarChart
            segments={releaseSegments(latest)}
            ariaLabel="Release readiness context"
            summary="Latest composition of configured release thresholds, thresholds met, and repositories without a local rule."
          />
        </AnalyticsChartCard>
      </div>
    </section>
  );
}

export function RepositoryAnalyticsPanel({
  repository,
  analytics,
}: {
  repository: RepositorySnapshot;
  analytics: AnalyticsSnapshot;
}): ReactElement {
  const series: AnalyticsRepositorySeries = analytics.repositories.find(
    (candidate) => candidate.repository_id === repository.id,
  ) ?? { repository_id: repository.id, name: repository.name, samples: [] };
  const samples = series.samples;
  const latest = latestSample(samples);
  return (
    <div className="drawer-section repository-analytics-section">
      <div className="drawer-section-title">
        <div>
          <h3>Analytics</h3>
          <small>Last 30 days · refresh snapshots only · read-only</small>
        </div>
        <span>
          {samples.length} observation{samples.length === 1 ? "" : "s"}
        </span>
      </div>
      <div className="repository-analytics-grid">
        <AnalyticsChartCard
          eyebrow="Health"
          title="Attention trend"
          description="Conditions, dirty workspaces, and unsynced workspaces."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
          compact
        >
          <TrendChart
            samples={samples}
            series={healthSeries()}
            ariaLabel={`${repository.name} health trend`}
            summary={`${repository.name} health signals over the last 30 days.`}
            compact
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Delivery"
          title="Change flow"
          description="Local commits and branch divergence."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
          compact
        >
          <TrendChart
            samples={samples}
            series={deliverySeries()}
            ariaLabel={`${repository.name} delivery trend`}
            summary={`${repository.name} local delivery signals over the last 30 days.`}
            compact
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Quality trajectory"
          title="Maturity and fresh passing evidence"
          description="External maturity and fresh passing evidence scores are shown only when quality evidence provides them."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
          summary={qualityPostureSummary(latest)}
          compact
        >
          <TrendChart
            samples={samples}
            series={qualitySeries()}
            ariaLabel={`${repository.name} quality trajectory`}
            summary={`${repository.name} maturity and fresh passing evidence scores over the last 30 days.`}
            yMax={4}
            compact
          />
        </AnalyticsChartCard>
        <AnalyticsChartCard
          eyebrow="Release context"
          title="Threshold readiness"
          description="Configured local release rules remain an evidence boundary, not an action."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
          compact
        >
          <StackedBarChart
            segments={releaseSegments(latest)}
            ariaLabel={`${repository.name} release readiness context`}
            summary={`${repository.name} release threshold configuration at the latest local refresh.`}
            compact
          />
        </AnalyticsChartCard>
      </div>
    </div>
  );
}
