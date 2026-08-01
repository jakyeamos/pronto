import type { ReactElement } from "react";
import type {
  AnalyticsMetricSample,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import {
  attentionSegments,
  chartFreshness,
  chartSource,
  deliverySeries,
  findingSeries,
  formatCount,
  healthSeries,
  latestRepositorySeries,
  latestSample,
  qualityPostureSummary,
  qualitySeries,
  releaseSegments,
  workspaceSegments,
} from "./analyticsChartModel";
import {
  AnalyticsChartCard,
  HorizontalBarChart,
  StackedBarChart,
  TrendChart,
} from "./ChartPrimitives";

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
          description="Raw detector findings remain tied to quality evidence freshness; reviewed dispositions and actionable counts appear in Quality detail."
          source={chartSource(analytics, samples)}
          freshness={chartFreshness(analytics, samples)}
        >
          <TrendChart
            samples={samples}
            series={findingSeries()}
            ariaLabel="Quality finding trend"
            summary="Raw detector total and high-severity quality findings over the last 30 days."
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
