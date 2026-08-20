// quality-gate: allow static-ui-test: verifies user-visible accessibility, freshness, unavailable evidence, and chart summary contracts.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  AnalyticsMetricSample,
  AnalyticsSnapshot,
  RepositorySnapshot,
} from "../types";
import { AnalyticsSurface } from "./AnalyticsComponents";
import { metricsShareAxis } from "./AnalyticsCharts";
import {
  dailyLatestSamples,
  formatAnalyticsNumber,
  formatAnalyticsTimestamp,
  metricValue,
} from "./AnalyticsChartFormatting";
import { QualityScatterTooltip } from "./AnalyticsScatterTooltip";
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
    maturity_evidence_coverage: 0.75,
    findings_total: 4,
    high_severity_findings: 1,
    detector_findings_total: 4,
    detector_actionable_findings: 2,
    detector_unreviewed_findings: 3,
    maturity_gap_total: 1,
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
    schema_version: "pronto-analytics/v3",
    generated_at: "2026-07-26T11:00:00Z",
    source: "Local refresh snapshots",
    freshness: "Fresh · observed Jul 26, 11:00 AM",
    range_days: 30,
    retention_days: 90,
    history_available_from: samples[0]?.observed_at,
    portfolio_samples: samples,
    repositories: [],
    metric_catalog: [],
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
            maturity_evidence_coverage: undefined,
            findings_total: undefined,
            high_severity_findings: undefined,
            detector_findings_total: undefined,
            detector_actionable_findings: undefined,
            detector_unreviewed_findings: undefined,
            maturity_gap_total: undefined,
            quality_freshness: "Unavailable",
          }),
        ])}
        repositories={[]}
      />,
    );

    expect(empty).toContain("No refresh history in this range");
    expect(unavailable).toContain("Unavailable");
    expect(unavailable).toContain(
      "Repository maturity and maturity evidence coverage are unavailable",
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

  it("normalizes chart observations by UTC day and formats exact timestamps consistently", () => {
    const samples = [
      makeSample({ observed_at: "2026-08-15T01:00:00Z", findings_total: 1 }),
      makeSample({ observed_at: "2026-08-15T23:17:35Z", findings_total: 2 }),
      makeSample({ observed_at: "2026-08-16T02:00:00Z", findings_total: 3 }),
    ];

    expect(
      dailyLatestSamples(samples).map((sample) => sample.findings_total),
    ).toEqual([2, 3]);
    expect(formatAnalyticsTimestamp(samples[1].observed_at)).toContain(
      "Aug 15, 2026",
    );
    expect(formatAnalyticsTimestamp(samples[1].observed_at)).toContain("UTC");
  });

  it("names repositories and rounds quality scatter values", () => {
    const markup = renderToStaticMarkup(
      <QualityScatterTooltip
        active
        payload={[
          {
            payload: {
              name: "Pronto",
              maturity: 1.7202399872854417,
              coverage: 0.3617142857142857,
            },
          },
        ]}
      />,
    );

    expect(formatAnalyticsNumber(1.7202399872854417)).toBe("1.72");
    expect(markup).toContain("Pronto");
    expect(markup).toContain("Maturity 1.72");
    expect(markup).toContain("Maturity evidence 36.17%");
    expect(markup).not.toContain("1.7202399872854417");
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
    expect(markup).toContain(">Detector findings ↓</button>");
    expect(markup).toContain(">Repository</button>");
  });

  it("omits the retired workspace activity composition card", () => {
    const markup = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([makeSample()])}
        repositories={[]}
      />,
    );

    expect(markup).not.toContain("Activity composition");
    expect(markup).not.toContain('aria-label="Workspace activity composition"');
  });

  it("treats serialized null metrics as unavailable instead of crashing", () => {
    const unavailable = renderToStaticMarkup(
      <AnalyticsSurface
        analytics={makeAnalytics([
          makeSample({
            ci_readiness_score: null,
            maturity_score: null,
            maturity_evidence_coverage: null,
            findings_total: null,
            high_severity_findings: null,
            detector_findings_total: null,
            detector_actionable_findings: null,
            detector_unreviewed_findings: null,
            maturity_gap_total: null,
            quality_freshness: "Fresh",
          }),
        ])}
        repositories={[]}
      />,
    );

    expect(unavailable).toContain(
      "Repository maturity and maturity evidence coverage are unavailable",
    );
    expect(unavailable).toContain("High-severity detector findings");
    expect(unavailable).toContain("Unavailable");
  });

  it("keeps maturity evidence coverage separate from fresh-passing CI", () => {
    const sample = makeSample();

    expect(metricValue(sample, "quality.maturity_evidence_coverage")).toBe(
      0.75,
    );
    expect(metricValue(sample, "quality.fresh_passing_ci_score")).toBe(2.5);
    expect(metricValue(sample, "quality.evidence_score")).toBe(2.5);
  });

  it("labels the evidence matrix with maturity coverage and Pronto configuration", () => {
    const analytics = makeAnalytics();
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

    expect(markup).toContain("Maturity evidence");
    expect(markup).toContain("Pronto release rules");
    expect(markup).toContain(
      'aria-label="Repository maturity evidence coverage"',
    );
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
        ariaLabel="Release readiness composition"
        summary="Release readiness state composition at the latest local refresh."
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

    expect(composition).toContain('aria-label="Release readiness composition"');
    expect(composition).toContain("3");
    expect(composition).toContain("chart-segment-label");
    expect(composition).toContain("4 total");
    expect(composition).toContain(
      "Release readiness state composition at the latest local refresh.",
    );
    expect(comparison).toContain(
      'aria-label="Repository attention comparison"',
    );
    expect(comparison).toContain("repository-with-a-very-long-…");
    expect(comparison).toContain("2 conditions · 1 dirty · 1 unsynced");
  });
});
