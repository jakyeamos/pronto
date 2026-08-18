import type { ReactElement } from "react";
import type { QualityMaturity, QualityReadiness } from "../types";
import type { TargetEvidenceState } from "../branchEvidence";
import { StatusPill } from "./ConsolePrimitives";
import { QualityMaturitySummary } from "./QualityComponents";

function readableLabel(value: string): string {
  if (value === "not_applicable") return "Not applicable";
  return value.replaceAll(/[._:-]+/g, " ");
}

function statusTone(status: string): string {
  if (status === "maintained" || status === "validated") return "mint";
  if (status === "failed" || status === "blocked") return "red";
  if (status === "stale" || status === "discoverable") return "amber";
  return "slate";
}

function remediation(status: string, score?: number): string {
  if (status === "not_applicable")
    return "No derived-storage surface was detected.";
  if (status === "stale")
    return "Rerun the complete QR fleet audit to refresh this evidence.";
  if (["unknown", "missing", "blocked", "failed", "absent"].includes(status)) {
    return "Restore complete traversal or feed evidence before treating cache lifecycle as verified.";
  }
  if (score === 0)
    return "Separate durable state from clearable storage and document invalidation.";
  if (score === 1)
    return "Add deterministic bounds or remove avoidable repository-local duplication.";
  if (score === 2)
    return "Add pruning, sharing evidence, and complete lifecycle bounds.";
  if (score === 3)
    return "Capture two bounded snapshots plus cold/warm equivalence evidence.";
  return "Bounds and equivalence evidence are current; continue automated enforcement.";
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB"];
  const exponent = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    units.length - 1,
  );
  return `${(value / 1024 ** exponent).toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`;
}

export function CacheDesignSummary({
  maturity,
}: {
  maturity: QualityMaturity;
}): ReactElement {
  const assessment = maturity.cache_design;
  if (!assessment) return <></>;
  const status = maturity.freshness === "Stale" ? "stale" : assessment.status;
  return (
    <section
      className="cache-design-summary"
      aria-label="Cache design maturity"
    >
      <div>
        <strong>Cache design</strong>
        <StatusPill tone={statusTone(status)}>
          {readableLabel(status)}
        </StatusPill>
      </div>
      <span>
        {assessment.status === "not_applicable"
          ? "N/A"
          : assessment.score === undefined
            ? "Not scored"
            : `${assessment.score}/4`}{" "}
        · {formatBytes(assessment.totals.allocated_bytes)} allocated ·{" "}
        {assessment.totals.file_count} files
      </span>
      <small>{remediation(status, assessment.score)}</small>
      {assessment.risk_flags.length > 0 && (
        <small>
          Risks: {assessment.risk_flags.map(readableLabel).join(", ")}
        </small>
      )}
    </section>
  );
}

export function QualityMaturityWithCacheSummary({
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
  return (
    <>
      <QualityMaturitySummary
        maturity={maturity}
        readiness={readiness}
        compact={compact}
        onOpenReport={onOpenReport}
        targetBranch={targetBranch}
        targetCommit={targetCommit}
        targetReadinessState={targetReadinessState}
      />
      <CacheDesignSummary maturity={maturity} />
    </>
  );
}
