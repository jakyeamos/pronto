import type { QualityMaturity, QualityPortfolioSnapshot } from "../types";

type MacControlPortfolioSnapshot = NonNullable<
  QualityPortfolioSnapshot["mac_control_ideal_state"]
>;

export type MacControlScoreLane = "implementation" | "live";

const ACCEPTED_FLEET_AUDIT_STATUSES = new Set(["ready", "ready with blockers"]);

const ASSESSED_MAC_SCORE_STATUSES = new Set([
  "passed",
  "failed",
  "review required",
]);

function hasText(value: string | undefined): boolean {
  return Boolean(value?.trim());
}

function hasFiniteScore(score: number | undefined, display?: string): boolean {
  return Number.isFinite(score) && hasText(display);
}

function normalized(value: string | undefined): string {
  return value?.trim().toLowerCase() ?? "";
}

/**
 * A fleet score is displayable only when it belongs to a current, imported
 * audit with a usable population and a numeric score. Paths and cached score
 * fields alone are deliberately insufficient.
 */
export function hasAcceptedFleetMaturityAudit(
  quality: QualityPortfolioSnapshot,
): boolean {
  return Boolean(
    hasText(quality.latest_audit_id) &&
    hasText(quality.latest_audit_at) &&
    quality.matched_repository_count > 0 &&
    ACCEPTED_FLEET_AUDIT_STATUSES.has(normalized(quality.audit_status)) &&
    hasFiniteScore(quality.maturity_score, quality.maturity_score_display),
  );
}

/**
 * Repository maturity scores need their own fresh audit identity and
 * observation timestamp. This also keeps target-branch callers responsible
 * for checking the selected target state before exposing the value.
 */
export function hasAcceptedRepositoryMaturityEvidence(
  maturity: QualityMaturity,
): boolean {
  return Boolean(
    normalized(maturity.freshness) === "fresh" &&
    hasText(maturity.audit_id) &&
    hasText(maturity.observed_at) &&
    hasFiniteScore(maturity.score, maturity.score_display),
  );
}

/**
 * Mac Control scores are valid only from a fresh, non-blocked lane with the
 * evidence denominator needed to interpret the score. A live score also needs
 * at least one measured task; a 0/N unmeasured report is not a zero score.
 */
export function macControlScoreDisplay(
  macControl: MacControlPortfolioSnapshot,
  lane: MacControlScoreLane,
): string | undefined {
  const score =
    lane === "implementation"
      ? macControl.implementation_score
      : macControl.live_score;
  const display =
    lane === "implementation"
      ? macControl.implementation_score_display
      : macControl.live_score_display;
  const laneStatus =
    lane === "implementation"
      ? macControl.implementation_status
      : macControl.live_status;
  const denominator =
    lane === "implementation"
      ? macControl.implementation_criteria_total
      : macControl.live_task_count;
  const measured =
    lane === "implementation"
      ? macControl.implementation_criteria_passed_count
      : macControl.measured_task_count;

  const hasUnassessedStatus = [macControl.status, laneStatus].some(
    (status) => !ASSESSED_MAC_SCORE_STATUSES.has(normalized(status)),
  );

  if (
    normalized(macControl.freshness) !== "fresh" ||
    hasUnassessedStatus ||
    !hasFiniteScore(score, display) ||
    typeof denominator !== "number" ||
    !Number.isFinite(denominator) ||
    denominator <= 0 ||
    (lane === "live" &&
      (typeof measured !== "number" ||
        !Number.isFinite(measured) ||
        measured <= 0))
  ) {
    return undefined;
  }

  return display;
}
