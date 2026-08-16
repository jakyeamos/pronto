import type { ReactElement } from "react";
import { FileSearch, ShieldAlert } from "lucide-react";
import type { QualityMaturity, QualityReadiness } from "../types";
import {
  hasTargetScope,
  targetEvidenceState,
  targetEvidenceMessage,
  type TargetEvidenceState,
} from "../branchEvidence";
import { StatusPill } from "./ConsolePrimitives";
import {
  agentUsabilityTone,
  maturityDimensionLabel,
  qualityFreshnessLabel,
  qualityRepositoryOutcome,
  readableQualityEvidenceText,
} from "./QualityProjection";
import {
  qualityGateDisplayLabel,
  readinessOpenGateIds,
} from "./QualityComponents";
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
  const repositoryMaturity = maturity.repository_maturity;
  const dimensions = Object.entries(maturity.dimension_scores ?? {}).sort(
    ([leftDimension, leftScore], [rightDimension, rightScore]) =>
      leftScore - rightScore || leftDimension.localeCompare(rightDimension),
  );
  const targetMode = hasTargetScope(targetBranch, targetCommit);
  const maturityState = targetMode
    ? targetEvidenceState({
        targetBranch,
        targetCommit,
        scannedBranch: maturity.scanned_branch,
        scannedCommit: maturity.scanned_commit,
      })
    : "verified";
  const qualityOutcome = maturity.quality_outcome
    ? qualityRepositoryOutcome(maturity.quality_outcome)
    : undefined;
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
              {maturity.audit_id ?? "No audit run"} ·{" "}
              {qualityFreshnessLabel(maturity.freshness)}
            </span>
            {(maturity.gaps ?? []).length > 0 && (
              <ul
                className="quality-maturity-gaps"
                aria-label="Raw maturity gaps"
              >
                {(maturity.gaps ?? []).slice(0, compact ? 2 : 4).map((gap) => (
                  <li key={`${gap.dimension}-${gap.status}`}>
                    <strong>{maturityDimensionLabel(gap.dimension)}</strong>
                    <span>{readableQualityEvidenceText(gap.message)}</span>
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
            {repositoryMaturity
              ? `${repositoryMaturity.evidence.assessed_pillar_count}/${repositoryMaturity.evidence.applicable_pillar_count} applicable pillars · `
              : maturity.scored_dimension_count
                ? `${maturity.scored_dimension_count} legacy dimensions · `
                : ""}
            {maturity.audit_id ?? "No audit run"} ·{" "}
            {qualityFreshnessLabel(maturity.freshness)}
          </small>
          {repositoryMaturity && (
            <div className="repository-maturity-meta">
              <StatusPill tone={agentUsabilityTone(repositoryMaturity.status)}>
                {repositoryMaturity.status}
              </StatusPill>
              <small>
                {Math.round(
                  repositoryMaturity.evidence.evidence_coverage * 100,
                )}
                % evidence ·{" "}
                {Math.round(
                  repositoryMaturity.evidence.fresh_evidence_coverage * 100,
                )}
                % fresh
              </small>
            </div>
          )}
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
        repositoryMaturity && (
          <div
            className="repository-maturity-pillars"
            aria-label="Repository maturity pillars"
          >
            {repositoryMaturity.pillars.map((pillar) => (
              <div key={pillar.id} data-status={pillar.status}>
                <span>{pillar.label}</span>
                <strong>
                  {pillar.applicability === "not_applicable"
                    ? "N/A"
                    : pillar.score === undefined
                      ? "Unknown"
                      : `${pillar.score}/4`}
                </strong>
                {!compact && pillar.missing_capabilities.length > 0 && (
                  <small>
                    Missing: {pillar.missing_capabilities.join(", ")}
                  </small>
                )}
              </div>
            ))}
            {repositoryMaturity.critical_cap.applied && (
              <div className="repository-maturity-cap">
                <ShieldAlert size={12} />
                <span>
                  Score capped at{" "}
                  {repositoryMaturity.critical_cap.maximum_score}/4 by{" "}
                  {repositoryMaturity.critical_cap.reasons.join(", ")}
                </span>
              </div>
            )}
          </div>
        )}
      {(!targetMode || maturityState !== "unavailable") &&
        (maturity.gaps ?? []).length > 0 && (
          <ul className="quality-maturity-gaps" aria-label="Maturity gaps">
            {(maturity.gaps ?? []).slice(0, compact ? 2 : 4).map((gap) => (
              <li key={`${gap.dimension}-${gap.status}`}>
                <strong>{maturityDimensionLabel(gap.dimension)}</strong>
                <span>
                  {gap.score === undefined ? "not scored" : `${gap.score}/4`} ·{" "}
                  {readableQualityEvidenceText(gap.message)}
                </span>
              </li>
            ))}
          </ul>
        )}
      {qualityOutcome?.disposition && (
        <div
          className="quality-maturity-outcome"
          aria-label="Quality outcome disposition"
        >
          <strong>{qualityOutcome.label}</strong>
          <small>{qualityOutcome.disposition}</small>
          {qualityOutcome.next_step ? (
            <small>Next: {qualityOutcome.next_step}</small>
          ) : null}
        </div>
      )}
      {(!targetMode || maturityState !== "unavailable") &&
        dimensions.length > 0 && (
          <details className="quality-maturity-dimensions">
            <summary>{dimensions.length} raw diagnostic dimensions</summary>
            <div
              aria-label="Maturity dimension scores"
              className="quality-maturity-dimension-list"
            >
              {dimensions.map(([dimension, score]) => (
                <span key={dimension} title={dimension}>
                  <b>{maturityDimensionLabel(dimension)}</b>
                  <span>{score}/4</span>
                </span>
              ))}
            </div>
          </details>
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
            <strong>
              {maturity.agent_usability.growth_health.score === undefined
                ? "—"
                : `${maturity.agent_usability.growth_health.score}/4`}
            </strong>
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
