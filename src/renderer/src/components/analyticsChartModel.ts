import type { AnalyticsMetricSample, AnalyticsSnapshot } from "../types";
import type { StackedBarSegment, TrendSeries } from "./ChartPrimitives";

const BLUE = "var(--blue)";
const MINT = "var(--mint)";
const AMBER = "var(--amber)";
const CORAL = "var(--coral)";
const VIOLET = "var(--violet)";

export function latestSample(
  samples: AnalyticsMetricSample[],
): AnalyticsMetricSample | undefined {
  return samples[samples.length - 1];
}

function formatScore(value: number | null | undefined): string {
  return value == null ? "Unavailable" : `${value.toFixed(1)}/4`;
}

function formatCount(value: number | null | undefined): string {
  return value == null ? "Unavailable" : `${value}`;
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

export function chartSource(
  analytics: AnalyticsSnapshot,
  samples: AnalyticsMetricSample[],
): string {
  if (samples.length === 0) return analytics.source;
  const suffix = samples.length === 1 ? "" : "s";
  return `${analytics.source} · ${samples.length} observation${suffix}`;
}

export function chartFreshness(
  analytics: AnalyticsSnapshot,
  samples: AnalyticsMetricSample[],
): string {
  return samples.length > 0
    ? formatObservedAt(latestSample(samples)?.observed_at)
    : analytics.freshness;
}

export function healthSeries(): TrendSeries[] {
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

export function deliverySeries(): TrendSeries[] {
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

export function qualitySeries(): TrendSeries[] {
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

export function releaseSegments(
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

export function qualityPostureSummary(
  sample: AnalyticsMetricSample | undefined,
): string {
  if (!sample) return "No refresh sample is available for quality posture.";
  return `Maturity ${formatScore(sample.maturity_score)} · Fresh passing evidence score ${formatScore(sample.ci_readiness_score)} · ${formatCount(sample.detector_findings_total ?? sample.findings_total)} detector findings · Quality evidence ${sample.quality_freshness ?? "Unavailable"}`;
}
