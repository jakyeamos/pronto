import type { ReactElement } from "react";
import { ExternalLink, FileSearch, ShieldAlert } from "lucide-react";
import type {
  QualityEvidence,
  QualityFreshness,
  QualityGate,
  QualityGateStatus,
  QualityMaturity,
  QualityPortfolioSnapshot,
  QualityReadiness,
  RepositorySnapshot,
} from "../types";
import {
  branchEvidenceStatus,
  combineTargetEvidenceStates,
  hasTargetScope,
  projectEvidenceForTarget,
  targetEvidenceState,
  targetEvidenceMessage,
  targetScopeForRepository,
  type TargetEvidenceState,
} from "../branchEvidence";
import { formatTime, StatusPill } from "./ConsolePrimitives";

export { QualityFindingsSummary } from "./QualityFindingsSummary";

function qualityStatusTone(status: QualityGateStatus): string {
  if (status === "Passed") return "mint";
  if (status === "Failed") return "coral";
  if (status === "Blocked") return "red";
  return "slate";
}

function qualityFreshnessTone(freshness: QualityFreshness): string {
  if (freshness === "Fresh") return "mint";
  if (freshness === "Stale" || freshness === "Conflicted") return "amber";
  return "slate";
}

function agentUsabilityTone(status: string): string {
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

export function QualityGateStatusPill({
  status,
  freshness,
}: {
  status: QualityGateStatus;
  freshness?: QualityFreshness;
}): ReactElement {
  return (
    <span className="quality-status-stack">
      <StatusPill tone={qualityStatusTone(status)}>{status}</StatusPill>
      {freshness && freshness !== "Unknown" && (
        <StatusPill tone={qualityFreshnessTone(freshness)}>
          {freshness}
        </StatusPill>
      )}
    </span>
  );
}

export function QualityTraceStatusPill({
  value,
}: {
  value: string;
}): ReactElement {
  const status = (
    ["Passed", "Failed", "Blocked", "Not configured"] as const
  ).find(
    (candidate) => value === candidate || value.startsWith(`${candidate} ·`),
  );
  if (!status) {
    return (
      <StatusPill tone={value === "Unknown" ? "slate" : "amber"}>
        {value}
      </StatusPill>
    );
  }
  const freshness = value.includes(" · ")
    ? value.split(" · ").slice(1).join(" · ")
    : undefined;
  return (
    <QualityGateStatusPill
      status={status}
      freshness={
        freshness === "Fresh" ||
        freshness === "Stale" ||
        freshness === "Unknown" ||
        freshness === "Conflicted"
          ? freshness
          : undefined
      }
    />
  );
}

function EvidenceAction({
  evidence,
  onOpenReport,
}: {
  evidence: QualityEvidence;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement | null {
  if (evidence.report_path && onOpenReport) {
    return (
      <button
        className="quality-report-link"
        type="button"
        onClick={() => onOpenReport(evidence.report_path as string)}
      >
        <FileSearch size={12} />
        Detailed report
      </button>
    );
  }
  if (evidence.report_url) {
    return (
      <a
        className="quality-report-link"
        href={evidence.report_url}
        target="_blank"
        rel="noreferrer"
      >
        <ExternalLink size={12} />
        Open source
      </a>
    );
  }
  return null;
}

export function QualityEvidenceList({
  evidence,
  onOpenReport,
}: {
  evidence: QualityEvidence[];
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  return (
    <div className="quality-evidence-list">
      {evidence.map((item, index) => (
        <div
          className="quality-evidence-row"
          key={`${item.id}-${item.source}-${item.observed_at ?? "unknown"}-${index}`}
        >
          <div className="quality-evidence-heading">
            <strong>
              {item.source} · {item.source_label}
            </strong>
            <QualityGateStatusPill
              status={item.status}
              freshness={item.freshness}
            />
          </div>
          <span>{item.detail || "No additional result detail"}</span>
          <small>
            {item.command ? `${item.command} · ` : ""}
            {item.scanned_commit
              ? `commit ${item.scanned_commit.slice(0, 8)}`
              : item.scanned_branch
                ? `branch ${item.scanned_branch}`
                : "Reference unavailable"}
            {" · "}
            {formatTime(item.observed_at)}
          </small>
          <EvidenceAction evidence={item} onOpenReport={onOpenReport} />
        </div>
      ))}
    </div>
  );
}

export function QualityGateCell({
  gate,
  configured = false,
  compact = false,
  showLabel = true,
  onOpenReport,
  targetBranch,
  targetCommit,
}: {
  gate: QualityGate;
  configured?: boolean;
  compact?: boolean;
  showLabel?: boolean;
  onOpenReport?: (reportPath: string) => void;
  targetBranch?: string;
  targetCommit?: string;
}): ReactElement {
  const targetMode = hasTargetScope(targetBranch, targetCommit);
  const projection = projectQualityGateForTarget(
    gate,
    targetBranch,
    targetCommit,
  );
  const displayedGate = projection.gate;
  const evidenceForDisclosure =
    targetMode && projection.state === "unavailable"
      ? gate.evidence
      : displayedGate.evidence;
  const targetEvidenceUnavailable =
    targetMode &&
    projection.state === "unavailable" &&
    gate.evidence.length > 0;
  const targetEvidenceStale = targetMode && projection.state === "stale";
  const targetEvidenceUnscoped = targetMode && projection.state === "unscoped";
  const configuredWithoutEvidence =
    configured && displayedGate.status === "Not configured";
  return (
    <div
      className={`quality-gate-cell${compact ? " quality-gate-cell-compact" : ""}`}
    >
      {showLabel && (
        <strong className="quality-gate-label">{gate.label}</strong>
      )}
      {targetEvidenceUnavailable ? (
        <StatusPill tone="amber">Target evidence unavailable</StatusPill>
      ) : configuredWithoutEvidence ? (
        <StatusPill tone="slate">Configured</StatusPill>
      ) : (
        <>
          <QualityGateStatusPill
            status={displayedGate.status}
            freshness={displayedGate.freshness}
          />
          {targetEvidenceStale && (
            <StatusPill tone="amber">Stale branch evidence</StatusPill>
          )}
          {targetEvidenceUnscoped && (
            <StatusPill tone="amber">Unscoped evidence</StatusPill>
          )}
        </>
      )}
      <span className="quality-gate-evidence-count">
        {targetEvidenceUnavailable
          ? "Raw evidence is not a target result"
          : targetEvidenceStale
            ? "Selected branch evidence is from an older head"
            : targetEvidenceUnscoped
              ? "Evidence scope is not recorded"
              : displayedGate.evidence.length === 0
                ? "No evidence"
                : `${displayedGate.evidence.length} source${displayedGate.evidence.length === 1 ? "" : "s"}`}
      </span>
      {evidenceForDisclosure.length > 0 && (
        <details className="quality-evidence-disclosure">
          <summary>
            {targetEvidenceUnavailable
              ? "Raw scanned evidence"
              : targetEvidenceStale
                ? "Stale branch evidence"
                : targetEvidenceUnscoped
                  ? "Unscoped evidence"
                  : "Expand evidence"}
          </summary>
          {targetEvidenceStale && (
            <small className="quality-inline-empty">
              The selected branch matches, but this scan predates the selected
              target head.
            </small>
          )}
          {targetEvidenceUnscoped && (
            <small className="quality-inline-empty">
              This evidence has no branch/head provenance and is not a target
              result.
            </small>
          )}
          {targetEvidenceUnavailable && (
            <small className="quality-inline-empty">
              Branch/head provenance does not match the selected target.
            </small>
          )}
          <QualityEvidenceList
            evidence={evidenceForDisclosure}
            onOpenReport={onOpenReport}
          />
        </details>
      )}
    </div>
  );
}

const QUALITY_GATE_LABELS: Record<string, string> = {
  build: "Build",
  runtime_smoke: "Smoke",
  tests: "Tests",
  lint: "Lint",
  formatter: "Formatter",
  typecheck: "Typecheck",
  dead_code: "Dead-code",
  secrets_scan: "Secrets scan",
  dependency_audit: "Dependency audit",
};

function readinessOpenGateIds(readiness: QualityReadiness): string[] {
  return Array.from(
    new Set([
      ...readiness.missing_gate_ids,
      ...readiness.stale_gate_ids,
      ...readiness.failed_gate_ids,
      ...readiness.blocked_gate_ids,
    ]),
  );
}

export function qualityGateDisplayLabel(gateId: string): string {
  return (
    QUALITY_GATE_LABELS[gateId] ??
    gateId
      .replace(/^custom:/, "")
      .split("_")
      .filter(Boolean)
      .map((part) => `${part[0]?.toUpperCase() ?? ""}${part.slice(1)}`)
      .join(" ")
  );
}

export function qualityConfigurationSummary(
  quality: QualityPortfolioSnapshot,
): {
  configured: number;
  ideal: number;
  fullRepositories: number;
  repositories: number;
  unscoredRepositories: number;
} {
  return {
    configured: quality.ci_configuration_configured_gate_count ?? 0,
    ideal: quality.ci_configuration_ideal_gate_count ?? 0,
    fullRepositories: quality.ci_configuration_full_repository_count ?? 0,
    repositories: quality.ci_configuration_repository_count ?? 0,
    unscoredRepositories:
      quality.ci_configuration_unscored_repository_count ?? 0,
  };
}

export function qualityEvidenceSummary(quality: QualityPortfolioSnapshot): {
  freshPassing: number;
  ideal: number;
} {
  return {
    freshPassing: quality.ci_evidence_fresh_passing_gate_count ?? 0,
    ideal: quality.ci_evidence_ideal_gate_count ?? 0,
  };
}

function QualityReadinessSummary({
  readiness,
  compact = false,
  targetEvidenceState: readinessEvidenceState = "verified",
}: {
  readiness: QualityReadiness;
  compact?: boolean;
  targetEvidenceState?: TargetEvidenceState;
}): ReactElement {
  const openGateIds = readinessOpenGateIds(readiness);
  const configuredGateIds = readiness.configured_gate_ids ?? [];
  const unconfiguredGateIds = readiness.unconfigured_gate_ids ?? [];
  const applicableGateCount = readiness.applicable_gate_ids.length;
  const evidenceGateCount = readiness.covered_gate_ids.length;
  const freshPassingGateCount = readiness.fresh_passing_gate_ids ?? [];
  return (
    <div
      className={`quality-readiness${compact ? " quality-readiness-compact" : ""}`}
    >
      <div className="quality-readiness-heading">
        <span>CI configuration</span>
        <strong>
          {readiness.configuration_score == null
            ? "—"
            : `${configuredGateIds.length}/${applicableGateCount}`}
        </strong>
      </div>
      {readinessEvidenceState === "unavailable" ? (
        <>
          <StatusPill tone="amber">Target readiness unavailable</StatusPill>
          <small>
            Branch/head-matched evidence is unavailable for the selected target.
          </small>
          <details className="quality-readiness-disclosure">
            <summary>Raw readiness evidence</summary>
            <span>
              Raw configuration: {configuredGateIds.length}/
              {applicableGateCount} ideal gates · raw evidence:{" "}
              {evidenceGateCount}/{applicableGateCount}
            </span>
          </details>
        </>
      ) : (
        <>
          {readinessEvidenceState === "stale" && (
            <>
              <StatusPill tone="amber">Stale branch evidence</StatusPill>
              <small>
                The selected branch matches, but the scan predates the selected
                target head.
              </small>
            </>
          )}
          {readinessEvidenceState === "unscoped" && (
            <>
              <StatusPill tone="amber">Unscoped evidence</StatusPill>
              <small>
                Readiness evidence has no branch/head provenance and is not a
                target result.
              </small>
            </>
          )}
          {readiness.configuration_score == null ? (
            <small>No matched recommendation profile</small>
          ) : (
            <>
              <small>
                {configuredGateIds.length}/{applicableGateCount} ideal gates
                configured
              </small>
              <small>
                Fresh passing evidence: {freshPassingGateCount.length}/
                {applicableGateCount}
              </small>
              <small>
                Imported evidence: {evidenceGateCount}/{applicableGateCount}
              </small>
              {unconfiguredGateIds.length > 0 ? (
                <details className="quality-readiness-disclosure">
                  <summary>
                    {unconfiguredGateIds.length} gate configuration update
                    {unconfiguredGateIds.length === 1 ? "" : "s"} needed
                  </summary>
                  <span>
                    {unconfiguredGateIds
                      .map(qualityGateDisplayLabel)
                      .join(", ")}
                  </span>
                </details>
              ) : openGateIds.length > 0 ? (
                <details className="quality-readiness-disclosure">
                  <summary>
                    {openGateIds.length} gate evidence update
                    {openGateIds.length === 1 ? "" : "s"} needed
                  </summary>
                  <span>
                    {openGateIds.map(qualityGateDisplayLabel).join(", ")}
                  </span>
                </details>
              ) : null}
            </>
          )}
        </>
      )}
    </div>
  );
}

export function QualityMaturitySummary({
  maturity,
  readiness,
  compact = false,
  onOpenReport,
  targetBranch,
  targetCommit,
  targetReadinessState = "verified",
}: {
  maturity: QualityMaturity;
  readiness: QualityReadiness;
  compact?: boolean;
  onOpenReport?: (reportPath: string) => void;
  targetBranch?: string;
  targetCommit?: string;
  targetReadinessState?: TargetEvidenceState;
}): ReactElement {
  const targetMode = hasTargetScope(targetBranch, targetCommit);
  const maturityState = targetMode
    ? targetEvidenceState({
        targetBranch,
        targetCommit,
        scannedBranch: maturity.scanned_branch,
        scannedCommit: maturity.scanned_commit,
      })
    : "verified";
  return (
    <div
      className={`quality-maturity${compact ? " quality-maturity-compact" : ""}`}
    >
      {targetMode && maturityState === "unavailable" ? (
        <>
          <strong>—</strong>
          <span>Target maturity unavailable</span>
          <small>
            {targetEvidenceMessage({
              targetBranch,
              targetCommit,
              scannedBranch: maturity.scanned_branch,
              scannedCommit: maturity.scanned_commit,
            })}
          </small>
          <details className="quality-maturity-raw-details">
            <summary>Raw maturity evidence</summary>
            <span>
              Raw score: {maturity.score_display ?? "Not scored"} ·{" "}
              {maturity.audit_id ?? "No audit run"} · {maturity.freshness}
            </span>
            {(maturity.gaps ?? []).length > 0 && (
              <ul
                className="quality-maturity-gaps"
                aria-label="Raw maturity gaps"
              >
                {(maturity.gaps ?? []).slice(0, compact ? 2 : 4).map((gap) => (
                  <li key={`${gap.dimension}-${gap.status}`}>
                    <strong>{gap.dimension.replaceAll("_", " ")}</strong>
                    <span>{gap.message}</span>
                  </li>
                ))}
              </ul>
            )}
          </details>
        </>
      ) : (
        <>
          <strong>{maturity.score_display ?? "Not scored"}</strong>
          <span>
            {maturityState === "stale"
              ? "Stale branch evidence"
              : maturityState === "unscoped"
                ? "Unscoped maturity evidence"
                : maturity.score_display
                  ? "/ 4 maturity"
                  : "Audit unavailable"}
          </span>
          <small>
            {maturity.scored_dimension_count
              ? `${maturity.scored_dimension_count} dimensions · `
              : ""}
            {maturity.audit_id ?? "No audit run"} · {maturity.freshness}
          </small>
          {targetMode && maturityState !== "verified" && (
            <small>
              {targetEvidenceMessage({
                targetBranch,
                targetCommit,
                scannedBranch: maturity.scanned_branch,
                scannedCommit: maturity.scanned_commit,
              })}
            </small>
          )}
        </>
      )}
      {(!targetMode || maturityState !== "unavailable") &&
        (maturity.gaps ?? []).length > 0 && (
          <ul className="quality-maturity-gaps" aria-label="Maturity gaps">
            {(maturity.gaps ?? []).slice(0, compact ? 2 : 4).map((gap) => (
              <li key={`${gap.dimension}-${gap.status}`}>
                <strong>{gap.dimension.replaceAll("_", " ")}</strong>
                <span>
                  {gap.score === undefined ? "unknown" : `${gap.score}/4`} ·{" "}
                  {gap.message}
                </span>
              </li>
            ))}
          </ul>
        )}
      {maturity.agent_usability && (
        <details className="agent-usability-summary">
          <summary>
            <span>Agent usability</span>
            <strong>
              {maturity.agent_usability.covered_lane_count}/
              {maturity.agent_usability.applicable_lane_count} lanes
            </strong>
            <StatusPill
              tone={agentUsabilityTone(maturity.agent_usability.status)}
            >
              {maturity.agent_usability.status.replaceAll("_", " ")}
            </StatusPill>
          </summary>
          <ul
            className="agent-usability-lanes"
            aria-label="Agent usability lanes"
          >
            {maturity.agent_usability.lanes.map((lane) => (
              <li key={lane.id}>
                <span>{lane.label}</span>
                <strong>
                  {lane.score === undefined ? "—" : `${lane.score}/4`}
                </strong>
                <small>{lane.message}</small>
              </li>
            ))}
          </ul>
          <div className="agent-usability-growth">
            <span>Growth health</span>
            <StatusPill
              tone={agentUsabilityTone(
                maturity.agent_usability.growth_health.status,
              )}
            >
              {maturity.agent_usability.growth_health.status.replaceAll(
                "_",
                " ",
              )}
            </StatusPill>
            <small>
              {
                maturity.agent_usability.growth_health
                  .routed_agent_document_count
              }
              /{maturity.agent_usability.growth_health.agent_document_count}{" "}
              agent docs routed ·{" "}
              {maturity.agent_usability.growth_health.skill_count} skills in{" "}
              {maturity.agent_usability.growth_health.family_count} families ·{" "}
              {maturity.agent_usability.growth_health.skill_covered_tool_count}/
              {maturity.agent_usability.growth_health.tool_count} tools
              skill-covered
            </small>
            <small>{maturity.agent_usability.growth_health.message}</small>
          </div>
        </details>
      )}
      <QualityReadinessSummary
        readiness={readiness}
        compact={compact}
        targetEvidenceState={targetMode ? targetReadinessState : "verified"}
      />
      {maturity.report_path && onOpenReport && (
        <button
          className="quality-report-link"
          type="button"
          onClick={() => onOpenReport(maturity.report_path as string)}
        >
          <FileSearch size={12} />
          {targetMode && maturityState === "unavailable"
            ? "Raw audit finding"
            : "Audit finding"}
        </button>
      )}
    </div>
  );
}

export interface QualityAttentionItem {
  kind: "gate" | "findings";
  label: string;
  detail: string;
  staleOnly: boolean;
  targetVerified: boolean;
  targetEvidenceState: TargetEvidenceState;
  gate?: QualityGate;
}

export function qualityAttentionItems(
  repository: RepositorySnapshot,
): QualityAttentionItem[] {
  const requiredGateIds = new Set(
    (repository.release_rule?.required_quality_gates ?? []).map(
      (requirement) => requirement.gate_id,
    ),
  );
  const configuredGateIds = new Set(
    repository.quality.ci_readiness.configured_gate_ids,
  );
  const target = targetScopeForRepository(repository);
  const targetMode = hasTargetScope(target.branch, target.commit);
  const items: QualityAttentionItem[] = [];
  for (const gate of repository.quality.gates) {
    const projection = projectQualityGateForTarget(
      gate,
      target.branch,
      target.commit,
    );
    const displayedGate = projection.gate;
    const required = requiredGateIds.has(gate.id);
    const configuredWithoutEvidence =
      configuredGateIds.has(gate.id) && gate.status === "Not configured";
    const rawNeedsAttention =
      gate.status === "Failed" ||
      gate.status === "Blocked" ||
      gate.freshness === "Stale" ||
      gate.freshness === "Conflicted" ||
      (required && gate.status === "Not configured");
    const targetEvidenceUnavailable =
      targetMode &&
      projection.state === "unavailable" &&
      gate.evidence.length > 0;
    const targetEvidenceNeedsReview =
      targetMode &&
      (projection.state === "stale" || projection.state === "unscoped");
    const needsAttention =
      rawNeedsAttention ||
      targetEvidenceUnavailable ||
      targetEvidenceNeedsReview;
    if (needsAttention) {
      const staleOnly =
        !targetEvidenceUnavailable &&
        (gate.freshness === "Stale" || projection.state === "stale") &&
        gate.status !== "Failed" &&
        gate.status !== "Blocked" &&
        !(required && gate.status === "Not configured");
      items.push({
        kind: "gate",
        label: `${gate.label}${required ? " · release required" : ""}`,
        detail: targetEvidenceUnavailable
          ? "Target evidence unavailable · raw evidence is not a target result"
          : projection.state === "stale"
            ? "Stale branch evidence · scan predates the selected target head"
            : projection.state === "unscoped"
              ? "Unscoped evidence · branch/head provenance is not recorded"
              : configuredWithoutEvidence
                ? "Configured · no evidence"
                : `${gate.status} · ${gate.freshness}`,
        staleOnly,
        targetVerified: projection.verified,
        targetEvidenceState: projection.state,
        gate: displayedGate,
      });
    }
  }
  const findingsTargetVerified =
    !targetMode ||
    branchEvidenceStatus({
      targetBranch: target.branch,
      targetCommit: target.commit,
      scannedBranch: repository.quality.findings.scanned_branch,
      scannedCommit: repository.quality.findings.scanned_commit,
    }) === "verified";
  const findingsEvidenceState = targetMode
    ? targetEvidenceState({
        targetBranch: target.branch,
        targetCommit: target.commit,
        scannedBranch: repository.quality.findings.scanned_branch,
        scannedCommit: repository.quality.findings.scanned_commit,
      })
    : "verified";
  if (repository.quality.findings.high_severity_total > 0) {
    const findingsLabel = findingsTargetVerified
      ? "High-severity QR findings"
      : findingsEvidenceState === "stale"
        ? "High-severity QR findings · stale branch evidence"
        : findingsEvidenceState === "unscoped"
          ? "High-severity QR findings · unscoped evidence"
          : "High-severity QR findings · target unverified";
    const findingsDetail = findingsTargetVerified
      ? `${repository.quality.findings.high_severity_total} critical or high finding${repository.quality.findings.high_severity_total === 1 ? "" : "s"}`
      : findingsEvidenceState === "stale"
        ? "Stale branch evidence · scan predates the selected target head"
        : findingsEvidenceState === "unscoped"
          ? "Unscoped evidence · branch/head provenance is not recorded"
          : "Target evidence unavailable · raw findings are not a target result";
    items.push({
      kind: "findings",
      label: findingsLabel,
      detail: findingsDetail,
      staleOnly: false,
      targetVerified: findingsTargetVerified,
      targetEvidenceState: findingsEvidenceState,
    });
  }
  return items;
}

export function QualityAttentionList({
  repository,
  onOpenRepository,
  onOpenReport,
}: {
  repository: RepositorySnapshot;
  onOpenRepository: () => void;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  return (
    <div className="quality-attention-list">
      {qualityAttentionItems(repository).map((item) => (
        <div
          className="quality-attention-item"
          key={`${item.kind}-${item.label}`}
        >
          <button
            className="quality-attention-main"
            type="button"
            onClick={onOpenRepository}
          >
            <ShieldAlert size={14} />
            <span>
              <strong>{item.label}</strong>
              <small>{item.detail}</small>
            </span>
          </button>
          {item.gate && item.targetVerified && (
            <QualityGateStatusPill
              status={item.gate.status}
              freshness={item.gate.freshness}
            />
          )}
          {item.gate && !item.targetVerified && (
            <StatusPill tone="amber">
              {item.targetEvidenceState === "stale"
                ? "Stale branch evidence"
                : item.targetEvidenceState === "unscoped"
                  ? "Unscoped evidence"
                  : "Target unverified"}
            </StatusPill>
          )}
          {item.gate?.evidence[0] &&
            (item.targetVerified ||
              item.targetEvidenceState === "stale" ||
              item.targetEvidenceState === "unscoped") && (
              <EvidenceAction
                evidence={item.gate.evidence[0]}
                onOpenReport={onOpenReport}
              />
            )}
          {item.kind === "findings" &&
            repository.quality.findings.report_path && (
              <button
                className="quality-report-link"
                type="button"
                onClick={() =>
                  onOpenReport?.(
                    repository.quality.findings.report_path as string,
                  )
                }
              >
                <FileSearch size={12} />
                Report
              </button>
            )}
        </div>
      ))}
    </div>
  );
}
