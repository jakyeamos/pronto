import type { ReactElement } from "react";
import type {
  AnalyticsRepositorySeries,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import {
  chartFreshness,
  chartSource,
  deliverySeries,
  healthSeries,
  latestSample,
  qualityPostureSummary,
  qualitySeries,
  releaseSegments,
} from "./analyticsChartModel";
import {
  AnalyticsChartCard,
  StackedBarChart,
  TrendChart,
} from "./ChartPrimitives";

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
