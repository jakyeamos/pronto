// quality-gate: allow static-ui-test: verifies user-visible accessibility, freshness, unavailable evidence, and chart summary contracts.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  AnalyticsMetricSample,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import {
  AnalyticsSurface,
  AnalyticsDashboardBand,
} from "./AnalyticsComponents";
import {
  HorizontalBarChart,
  StackedBarChart,
  TrendChart,
  type TrendSeries,
} from "./ChartPrimitives";
import { RepositoryAnalyticsPanel } from "./RepositoryAnalyticsPanel";

function makeSample(
  overrides: Partial<AnalyticsMetricSample> = {},
): AnalyticsMetricSample {
  return {
    observed_at: "2026-07-26T11:00:00Z",
    repository_count: 3,
    workspace_count: 4,
    branch_count: 5,
    active_condition_count: 2,
    dirty_workspace_count: 1,
    unsynced_workspace_count: 1,
    active_workspace_count: 1,
    interrupted_workspace_count: 1,
    idle_workspace_count: 1,
    unknown_workspace_count: 1,
    ahead_commit_count: 2,
    behind_commit_count: 1,
    commits_last_30_days: 12,
    ci_readiness_score: 2.5,
    maturity_score: 3,
    findings_total: 4,
    high_severity_findings: 1,
    ci_readiness_scored_repository_count: 2,
    maturity_scored_repository_count: 2,
    findings_repository_count: 2,
    release_rule_repository_count: 2,
    release_ready_repository_count: 1,
    quality_freshness: "Fresh",
    ...overrides,
  };
}

function makeAnalytics(
  samples: AnalyticsMetricSample[] = [],
): AnalyticsSnapshot {
  return {
    schema_version: "pronto-analytics/v1",
    generated_at: "2026-07-26T11:00:00Z",
    source: "Local refresh snapshots",
    freshness: "Fresh · observed Jul 26, 11:00 AM",
    range_days: 30,
    retention_days: 90,
    history_available_from: samples[0]?.observed_at,
    portfolio_samples: samples,
    repositories: [],
  };
}

const conditionSeries: TrendSeries[] = [
  {
    label: "Active conditions",
    color: "var(--amber)",
    getValue: (sample) => sample.active_condition_count,
  },
];

describe("analytics charts", () => {
  it("renders an accessible trend with visible values and insufficient-history guidance", () => {
    const markup = renderToStaticMarkup(
      <TrendChart
        samples={[makeSample()]}
        series={conditionSeries}
        ariaLabel="Fleet health trend"
        summary="Active conditions over the last 30 days."
      />,
    );

    expect(markup).toContain('<svg class="chart-svg"');
    expect(markup).toContain('aria-label="Fleet health trend"');
    expect(markup).toContain("Active conditions");
    expect(markup).toContain(">2<");
    expect(markup).toContain(
      "One observation only. Refresh again to build a trend.",
    );
    expect(markup).toContain("Active conditions over the last 30 days.");
  });

  it("renders explicit empty and unavailable evidence states", () => {
    const empty = renderToStaticMarkup(
      <AnalyticsDashboardBand analytics={makeAnalytics()} />,
    );
    const unavailable = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([
          makeSample({
            ci_readiness_score: undefined,
            maturity_score: undefined,
            findings_total: undefined,
            high_severity_findings: undefined,
            quality_freshness: "Unavailable",
          }),
        ])}
        repositories={[]}
      />,
    );

    expect(empty).toContain("No refresh history yet");
    expect(empty).toContain(
      "Run a local refresh to record the first observation.",
    );
    expect(unavailable).toContain("Evidence unavailable");
    expect(unavailable).toContain("Unavailable");
    expect(unavailable).toContain("Fresh passing evidence score Unavailable");
  });

  it("treats serialized null metrics as unavailable instead of crashing", () => {
    const unavailable = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([
          makeSample({
            ci_readiness_score: null,
            maturity_score: null,
            findings_total: null,
            high_severity_findings: null,
            quality_freshness: "Fresh",
          }),
        ])}
        repositories={[]}
      />,
    );

    expect(unavailable).toContain("Evidence unavailable");
    expect(unavailable).toContain("Maturity Unavailable");
    expect(unavailable).toContain("Fresh passing evidence score Unavailable");
    expect(unavailable).toContain("Unavailable detected findings");
  });

  it("renders repository analytics from the matching repository series", () => {
    const repository = {
      id: "repository-1",
      name: "Pronto",
    } as RepositorySnapshot;
    const analytics = makeAnalytics();
    analytics.repositories = [
      {
        repository_id: repository.id,
        name: repository.name,
        samples: [makeSample()],
      },
    ];

    const markup = renderToStaticMarkup(
      <RepositoryAnalyticsPanel
        repository={repository}
        analytics={analytics}
      />,
    );

    expect(markup).toContain("Pronto health trend");
    expect(markup).toContain("Pronto delivery trend");
    expect(markup).toContain("Pronto quality trajectory");
    expect(markup).toContain("Pronto release readiness context");
    expect(markup).toContain("1 observation");
  });

  it("keeps composition and comparison charts legible with accessible summaries", () => {
    const composition = renderToStaticMarkup(
      <StackedBarChart
        segments={[
          { label: "Active", color: "var(--mint)", value: 3 },
          { label: "Interrupted", color: "var(--coral)", value: 1 },
        ]}
        ariaLabel="Workspace activity composition"
        summary="Workspace activity state composition at the latest local refresh."
      />,
    );
    const comparison = renderToStaticMarkup(
      <HorizontalBarChart
        items={[
          {
            label: "repository-with-a-very-long-name-that-needs-truncation",
            color: "var(--amber)",
            value: 4,
            detail: "2 conditions · 1 dirty · 1 unsynced",
          },
        ]}
        ariaLabel="Repository attention comparison"
        summary="Repositories compared by local attention signals."
      />,
    );

    expect(composition).toContain(
      'aria-label="Workspace activity composition"',
    );
    expect(composition).toContain("3");
    expect(composition).toContain(
      "Workspace activity state composition at the latest local refresh.",
    );
    expect(comparison).toContain(
      'aria-label="Repository attention comparison"',
    );
    expect(comparison).toContain("repository-with-a-very…");
    expect(comparison).toContain("2 conditions · 1 dirty · 1 unsynced");
  });
});
