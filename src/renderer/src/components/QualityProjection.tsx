import type { ReactElement } from "react";
import type {
  QualityEvidence,
  QualityFreshness,
  QualityGate,
  QualityGateStatus,
  QualityPortfolioSnapshot,
  QualityReadiness,
  QualityRepositoryOutcome,
} from "../types";
import {
  combineTargetEvidenceStates,
  hasTargetScope,
  projectEvidenceForTarget,
  type TargetEvidenceState,
} from "../branchEvidence";

const QUALITY_OUTCOME_ORDER = [
  "checks_failing",
  "verification_blocked",
  "review_needed",
  "evidence_unknown",
  "healthy",
] as const;

const EVIDENCE_REVIEW_PRESENTATION = {
  label: "Evidence review required",
  meaning:
    "Required evidence is not current or confirmed. This is an evidence gap, not a failed-test result.",
  next_step:
    "Identify the listed evidence gaps, reconcile the target branch and commit, then rerun the audit.",
} as const;

export function readableQualityEvidenceText(value: string): string {
  return value.replace(/\bunknown\b/gi, "not confirmed");
}

export function qualityFreshnessLabel(freshness: QualityFreshness): string {
  return freshness === "Unknown" ? "Evidence not confirmed" : freshness;
}

export function macControlFreshnessLabel(
  freshness: string | undefined,
): string {
  switch (freshness?.trim().toLowerCase()) {
    case "fresh":
      return "Fresh evidence";
    case "stale":
      return "Stale evidence—rerun the fleet audit";
    case "unknown":
      return "Fleet freshness incomplete—inspect repository blockers";
    case "not configured":
      return "No fleet evidence has been configured";
    default:
      return freshness?.trim() || "No fleet freshness evidence is available";
  }
}

function qualityOutcomeDefinition(
  state: string,
  definition?: {
    label: string;
    meaning: string;
    next_step?: string;
  },
): { label: string; meaning: string; next_step?: string } | undefined {
  const presentation =
    state === "evidence_unknown" ? EVIDENCE_REVIEW_PRESENTATION : definition;
  if (!presentation) return undefined;
  return {
    ...presentation,
    label: readableQualityEvidenceText(presentation.label),
    meaning: readableQualityEvidenceText(presentation.meaning),
    next_step: presentation.next_step
      ? readableQualityEvidenceText(presentation.next_step)
      : undefined,
  };
}

export function qualityRepositoryOutcome(
  outcome: QualityRepositoryOutcome,
): QualityRepositoryOutcome {
  const evidenceReview = outcome.state === "evidence_unknown";
  return {
    ...outcome,
    label: evidenceReview
      ? EVIDENCE_REVIEW_PRESENTATION.label
      : readableQualityEvidenceText(outcome.label),
    disposition: outcome.disposition
      ? readableQualityEvidenceText(outcome.disposition)
      : evidenceReview
        ? EVIDENCE_REVIEW_PRESENTATION.meaning
        : undefined,
    next_step: evidenceReview
      ? EVIDENCE_REVIEW_PRESENTATION.next_step
      : outcome.next_step
        ? readableQualityEvidenceText(outcome.next_step)
        : undefined,
  };
}

export function QualityOutcomeSummary({
  quality,
}: {
  quality: QualityPortfolioSnapshot;
}): ReactElement {
  const outcomes = QUALITY_OUTCOME_ORDER.flatMap((state) => {
    const count = quality.quality_outcome_counts?.[state] ?? 0;
    const definition = qualityOutcomeDefinition(
      state,
      quality.quality_outcome_taxonomy?.[state],
    );
    return count > 0 && definition ? [{ state, count, ...definition }] : [];
  });

  if (outcomes.length === 0) return <></>;
  return (
    <details className="quality-outcome-summary" open>
      <summary>Repository quality outcomes</summary>
      <ul>
        {outcomes.map((outcome) => (
          <li key={outcome.state} title={outcome.meaning}>
            <strong>{outcome.count}</strong>
            <span>{outcome.label}</span>
            <small>{outcome.meaning}</small>
            {outcome.next_step ? (
              <small>Next: {outcome.next_step}</small>
            ) : null}
          </li>
        ))}
      </ul>
    </details>
  );
}

export function qualityStatusTone(status: QualityGateStatus): string {
  if (status === "Passed") return "mint";
  if (status === "Failed") return "coral";
  if (status === "Blocked") return "red";
  return "slate";
}

export function qualityFreshnessTone(freshness: QualityFreshness): string {
  if (freshness === "Fresh") return "mint";
  if (freshness === "Stale" || freshness === "Conflicted") return "amber";
  return "slate";
}

export function maturityDimensionLabel(dimension: string): string {
  if (dimension === "diagnosability.stable_error_codes") {
    return "Stable error codes";
  }
  return dimension.replace(/[._:-]+/g, " ");
}

export function agentUsabilityTone(status: string): string {
  if (status === "healthy" || status === "behavior_verified") return "mint";
  if (status === "blocked") return "red";
  if (status === "attention" || status === "stale") return "amber";
  return "slate";
}

export interface TargetQualityGateProjection {
  gate: QualityGate;
  verified: boolean;
  state: TargetEvidenceState;
}

function aggregateGateStatus(evidence: QualityEvidence[]): QualityGateStatus {
  if (evidence.some((item) => item.status === "Failed")) return "Failed";
  if (evidence.some((item) => item.status === "Blocked")) return "Blocked";
  if (evidence.some((item) => item.status === "Passed")) return "Passed";
  return "Not configured";
}

function aggregateGateFreshness(evidence: QualityEvidence[]): QualityFreshness {
  const hasPassingEvidence = evidence.some((item) => item.status === "Passed");
  const hasAttentionEvidence = evidence.some(
    (item) => item.status === "Failed" || item.status === "Blocked",
  );
  if (hasPassingEvidence && hasAttentionEvidence) return "Conflicted";
  if (evidence.some((item) => item.freshness === "Stale")) return "Stale";
  if (evidence.some((item) => item.freshness === "Fresh")) return "Fresh";
  return "Unknown";
}

export function projectQualityGateForTarget(
  gate: QualityGate,
  targetBranch?: string | null,
  targetCommit?: string | null,
): TargetQualityGateProjection {
  if (!hasTargetScope(targetBranch, targetCommit)) {
    return { gate, verified: true, state: "unscoped" };
  }
  const targetEvidence = projectEvidenceForTarget(
    gate.evidence,
    targetBranch,
    targetCommit,
  );
  if (targetEvidence.evidence.length === 0) {
    return {
      gate:
        gate.evidence.length === 0
          ? {
              ...gate,
              status: "Not configured",
              freshness: "Unknown",
              evidence: [],
            }
          : { ...gate, evidence: [] },
      verified: false,
      state: targetEvidence.state,
    };
  }
  return {
    gate: {
      ...gate,
      status: aggregateGateStatus(targetEvidence.evidence),
      freshness: aggregateGateFreshness(targetEvidence.evidence),
      evidence: targetEvidence.evidence,
    },
    verified: targetEvidence.state === "verified",
    state: targetEvidence.state,
  };
}

export interface TargetQualityReadinessProjection {
  readiness: QualityReadiness;
  verified: boolean;
  state: TargetEvidenceState;
}

export function projectQualityReadinessForTarget(
  readiness: QualityReadiness,
  gates: QualityGate[],
  targetBranch?: string | null,
  targetCommit?: string | null,
): TargetQualityReadinessProjection {
  if (!hasTargetScope(targetBranch, targetCommit)) {
    return { readiness, verified: true, state: "unscoped" };
  }

  const gateById = new Map(gates.map((gate) => [gate.id, gate]));
  const projected = readiness.applicable_gate_ids.map((gateId) => {
    const gate = gateById.get(gateId) ?? {
      id: gateId,
      label: gateId,
      status: "Not configured" as const,
      freshness: "Unknown" as const,
      evidence: [],
    };
    return projectQualityGateForTarget(gate, targetBranch, targetCommit);
  });
  const evidenceStates = projected.flatMap(({ state }, index) => {
    const rawGate = gates.find(
      (gate) => gate.id === readiness.applicable_gate_ids[index],
    );
    return rawGate?.evidence.length ? [state] : [];
  });
  const state = combineTargetEvidenceStates(evidenceStates);
  const verified = state === "verified";
  const coveredGateIds = projected
    .filter(({ gate }) => gate.evidence.length > 0)
    .map(({ gate }) => gate.id);
  const freshPassingGateIds = projected
    .filter(
      ({ gate }) =>
        gate.status === "Passed" &&
        gate.freshness === "Fresh" &&
        gate.evidence.length > 0,
    )
    .map(({ gate }) => gate.id);
  const missingGateIds = projected
    .filter(({ gate }) => gate.status === "Not configured")
    .map(({ gate }) => gate.id);
  const staleGateIds = projected
    .filter(({ gate }) => gate.freshness === "Stale")
    .map(({ gate }) => gate.id);
  const failedGateIds = projected
    .filter(({ gate }) => gate.status === "Failed")
    .map(({ gate }) => gate.id);
  const blockedGateIds = projected
    .filter(({ gate }) => gate.status === "Blocked")
    .map(({ gate }) => gate.id);

  return {
    readiness: {
      ...readiness,
      covered_gate_ids: coveredGateIds,
      fresh_passing_gate_ids: freshPassingGateIds,
      missing_gate_ids: missingGateIds,
      stale_gate_ids: staleGateIds,
      failed_gate_ids: failedGateIds,
      blocked_gate_ids: blockedGateIds,
    },
    verified,
    state,
  };
}
