import type { ReactElement } from "react";
import { ExternalLink, FileSearch } from "lucide-react";
import type {
  QualityEvidence,
  QualityFreshness,
  QualityGate,
  QualityGateStatus,
  QualityPortfolioSnapshot,
  QualityReadiness,
} from "../types";
import { hasTargetScope } from "../branchEvidence";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import {
  qualityFreshnessLabel,
  qualityFreshnessTone,
  qualityStatusTone,
  projectQualityGateForTarget,
} from "./QualityProjection";
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
      {freshness && (
        <StatusPill tone={qualityFreshnessTone(freshness)}>
          {qualityFreshnessLabel(freshness)}
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
        {value === "Unknown" ? "Evidence not confirmed" : value}
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

export function EvidenceAction({
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
            {item.verification_level
              ? `${item.verification_level.replaceAll("_", " ")} · `
              : ""}
            {item.target_kind ? `target ${item.target_kind} · ` : ""}
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

export function readinessOpenGateIds(readiness: QualityReadiness): string[] {
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

export { QualityFindingsSummary } from "./QualityFindingsSummary";
export { WebReadinessSummary } from "./WebReadinessSummary";
export {
  macControlFreshnessLabel,
  projectQualityGateForTarget,
  projectQualityReadinessForTarget,
  QualityOutcomeSummary,
} from "./QualityProjection";
export { QualityMaturitySummary } from "./QualityMaturityComponents";
export {
  QualityAttentionList,
  qualityAttentionItems,
} from "./QualityAttentionComponents";
