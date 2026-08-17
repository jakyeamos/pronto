// quality-gate: allow static-ui-test: verifies user-visible accessibility, freshness, unavailable evidence, and chart summary contracts.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  AnalyticsMetricSample,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import { AnalyticsSurface } from "./AnalyticsComponents";
import { metricsShareAxis, metricValue } from "./AnalyticsCharts";
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
    schema_version: "pronto-analytics/v2",
    generated_at: "2026-07-26T11:00:00Z",
    source: "Local refresh snapshots",
    freshness: "Fresh · observed Jul 26, 11:00 AM",
    range_days: 30,
    retention_days: 90,
    history_available_from: samples[0]?.observed_at,
    portfolio_samples: samples,
    repositories: [],
    metric_catalog: [
      {
        id: "findings.repositories.unavailable",
        label: "Findings unavailable",
        description: "Repositories without detector-level findings evidence.",
        unit: "repositories",
        denominator: "portfolio",
        scope: "portfolio",
        time_semantics: "point-in-time",
        window_days: null,
        aggregation: "sum",
        polarity: "lower-is-better",
        source: "quality-runner",
        freshness: "local-refresh",
        allowed_visualizations: ["stacked-bar"],
      },
      {
        id: "findings.repositories.available",
        label: "Findings evidence available",
        description:
          "Repositories with detector-level findings evidence, including evidenced zero counts.",
        unit: "repositories",
        denominator: "portfolio",
        scope: "portfolio",
        time_semantics: "point-in-time",
        window_days: null,
        aggregation: "sum",
        polarity: "higher-is-better",
        source: "quality-runner",
        freshness: "local-refresh",
        allowed_visualizations: ["stacked-bar"],
      },
    ],
    findings: [],
    views: [],
    default_view_id: "curated",
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
    expect(markup).toContain('aria-label="Latest chart values"');
    expect(markup).toContain("chart-legend-line");
    expect(markup).toContain('fill="none"');
    expect(markup).toContain("chart-line-amber");
    expect(markup).toContain('stroke="#f2bc71"');
    expect(markup).toContain(
      "One observation only. Refresh again to build a trend.",
    );
    expect(markup).toContain("Active conditions over the last 30 days.");
  });

  it("renders explicit empty and unavailable evidence states", () => {
    const empty = renderToStaticMarkup(
      <AnalyticsSurface analytics={makeAnalytics()} repositories={[]} />,
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

    expect(empty).toContain("No refresh history in this range");
    expect(unavailable).toContain("Unavailable");
    expect(unavailable).toContain(
      "Quality and evidence coverage are unavailable",
    );
  });

  it("rejects metrics with mismatched windows or denominators on one axis", () => {
    expect(
      metricsShareAxis([
        {
          id: "commits",
          label: "Commits",
          description: "",
          unit: "commits",
          denominator: "portfolio",
          scope: "portfolio",
          time_semantics: "trailing-window",
          window_days: 30,
          aggregation: "sum",
          polarity: "neutral",
          source: "git",
          freshness: "local-refresh",
          allowed_visualizations: ["line"],
        },
        {
          id: "ahead",
          label: "Ahead",
          description: "",
          unit: "commits",
          denominator: "workspaces",
          scope: "repository",
          time_semantics: "point-in-time",
          aggregation: "sum",
          polarity: "lower-is-better",
          source: "git",
          freshness: "local-refresh",
          allowed_visualizations: ["bar"],
        },
      ]),
    ).toBe(false);
  });

  it("exposes keyboard-operable repository comparison sorting", () => {
    const analytics = makeAnalytics([makeSample()]);
    analytics.repositories = [
      {
        repository_id: "repository-1",
        name: "Pronto",
        samples: [makeSample()],
      },
    ];

    const markup = renderToStaticMarkup(
      <AnalyticsSurface analytics={analytics} repositories={[]} />,
    );

    expect(markup).toContain("Sortable evidence table");
    expect(markup).toContain('aria-sort="descending"');
    expect(markup).toContain(">Findings ↓</button>");
    expect(markup).toContain(">Repository</button>");
  });

  it("replaces unknown workspace activity with findings evidence coverage", () => {
    const sample = makeSample();
    const markup = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([sample])}
        repositories={[]}
      />,
    );

    expect(metricValue(sample, "findings.repositories.available")).toBe(2);
    expect(metricValue(sample, "findings.repositories.unavailable")).toBe(1);
    expect(markup).toContain("Findings evidence coverage");
    expect(markup).toContain("detector-level findings evidence");
    expect(markup).toContain("a findings count is unavailable");
    expect(markup).not.toContain("Workspace activity composition");
  });

  it("hides the findings rail when there are no deterministic findings", () => {
    const empty = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([makeSample()])}
        repositories={[]}
      />,
    );
    const withFinding = makeAnalytics([makeSample()]);
    withFinding.findings = [
      {
        id: "freshness-gap",
        kind: "coverage-gap",
        severity: "attention",
        title: "Fresh evidence is incomplete",
        detail: "Some applicable evidence has not been refreshed.",
        metric_ids: ["quality.evidence_score"],
      },
    ];
    const populated = renderToStaticMarkup(
      <AnalyticsSurface analytics={withFinding} repositories={[]} />,
    );

    expect(empty).not.toContain("Observed findings");
    expect(empty).not.toContain("No deterministic finding was generated");
    expect(populated).toContain("Observed findings");
    expect(populated).toContain("Fresh evidence is incomplete");
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

    expect(unavailable).toContain(
      "Quality and evidence coverage are unavailable",
    );
    expect(unavailable).toContain("High-severity findings");
    expect(unavailable).toContain("Unavailable");
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
    expect(composition).toContain("chart-segment-label");
    expect(composition).toContain("4 total");
    expect(composition).toContain(
      "Workspace activity state composition at the latest local refresh.",
    );
    expect(comparison).toContain(
      'aria-label="Repository attention comparison"',
    );
    expect(comparison).toContain("repository-with-a-very-long-…");
    expect(comparison).toContain("2 conditions · 1 dirty · 1 unsynced");
  });
});
