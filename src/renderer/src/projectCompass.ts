import type { ProjectCompassTargetSummary } from "./types";

function pluralize(
  count: number,
  singular: string,
  plural = `${singular}s`,
): string {
  return count === 1 ? singular : plural;
}

export function projectCompassCoverageIsIncomplete(
  target: ProjectCompassTargetSummary,
): boolean {
  return (
    target.total_pillar_count === 0 ||
    target.covered_pillar_count < target.total_pillar_count
  );
}

export function projectCompassCoverageLabel(
  target: ProjectCompassTargetSummary,
): string {
  if (target.total_pillar_count === 0) return "Coverage unavailable";

  const outcomeLabel = `${target.scored_outcome_count} scoped ${pluralize(target.scored_outcome_count, "outcome")}`;
  const pillarLabel = `${target.covered_pillar_count}/${target.total_pillar_count} ${pluralize(target.total_pillar_count, "pillar")} covered`;
  return projectCompassCoverageIsIncomplete(target)
    ? `Coverage incomplete · ${outcomeLabel} · ${pillarLabel}`
    : `${outcomeLabel} · ${pillarLabel}`;
}

export function projectCompassProgressLabel(
  target: ProjectCompassTargetSummary,
  targetName = "MVP",
): string {
  const progressLabel =
    target.progress_percent === null
      ? `${targetName} unknown`
      : `${targetName} ${target.progress_percent}%`;
  if (target.total_pillar_count === 0) {
    return `${progressLabel} · coverage unavailable`;
  }

  return `${progressLabel} · ${target.covered_pillar_count}/${target.total_pillar_count} ${pluralize(target.total_pillar_count, "pillar")}`;
}
