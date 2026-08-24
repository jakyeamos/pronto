import type { AnalyticsMetricSample } from "../types";

export function metricValue(
  sample: AnalyticsMetricSample,
  id: string,
): number | null {
  const governed = sample.metrics?.[id];
  if (governed !== undefined) return governed;
  const legacy: Record<string, number | null | undefined> = {
    "git.commits.trailing_30_days": sample.commits_last_30_days,
    "git.ahead_commits": sample.ahead_commit_count,
    "git.behind_commits": sample.behind_commit_count,
    "conditions.active": sample.active_condition_count,
    "workspaces.dirty": sample.dirty_workspace_count,
    "workspaces.unsynced": sample.unsynced_workspace_count,
    "workspaces.activity.active": sample.active_workspace_count,
    "workspaces.activity.interrupted": sample.interrupted_workspace_count,
    "workspaces.activity.idle": sample.idle_workspace_count,
    "workspaces.activity.unknown": sample.unknown_workspace_count,
    "quality.maturity_score": sample.maturity_score,
    "quality.maturity_evidence_coverage": sample.maturity_evidence_coverage,
    "quality.fresh_passing_ci_score": sample.ci_readiness_score,
    "quality.evidence_score": sample.ci_readiness_score,
    "findings.total": sample.findings_total,
    "findings.high_severity": sample.high_severity_findings,
    "findings.detector_total": sample.detector_findings_total,
    "findings.detector_actionable": sample.detector_actionable_findings,
    "findings.detector_unreviewed": sample.detector_unreviewed_findings,
    "maturity.gaps": sample.maturity_gap_total,
    "release.ready_repositories": sample.release_ready_repository_count,
    "release.configured_repositories": sample.release_rule_repository_count,
    "remediation.actions.open": sample.remediation_open_action_count,
    "remediation.actions.in_progress":
      sample.remediation_in_progress_action_count,
    "remediation.actions.blocked": sample.remediation_blocked_action_count,
    "remediation.actions.deferred": sample.remediation_deferred_action_count,
    "remediation.actions.verified": sample.remediation_verified_action_count,
    "remediation.progress_percent": sample.remediation_progress_percent,
  };
  return legacy[id] ?? null;
}

export function formatAnalyticsNumber(value: number): string {
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 2,
  }).format(value);
}

export function formatAnalyticsDate(value: string | number): string {
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(new Date(value));
}

export function formatAnalyticsTimestamp(value: string | number): string {
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: "UTC",
    timeZoneName: "short",
  }).format(new Date(value));
}

export function dailyLatestSamples(
  samples: AnalyticsMetricSample[],
): AnalyticsMetricSample[] {
  const latestByDay = new Map<string, AnalyticsMetricSample>();
  for (const sample of samples) {
    const timestamp = Date.parse(sample.observed_at);
    if (!Number.isFinite(timestamp)) continue;
    const day = new Date(timestamp).toISOString().slice(0, 10);
    const current = latestByDay.get(day);
    if (!current || Date.parse(current.observed_at) < timestamp) {
      latestByDay.set(day, sample);
    }
  }
  return [...latestByDay.values()].sort(
    (left, right) =>
      Date.parse(left.observed_at) - Date.parse(right.observed_at),
  );
}
