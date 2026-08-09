import type { ReactElement } from "react";
import { FileSearch } from "lucide-react";
import type { QualityFindings } from "../types";
import {
  branchEvidenceStatus,
  hasTargetScope,
  targetEvidenceState,
  targetEvidenceDescriptor,
  targetEvidenceMessage,
} from "../branchEvidence";
import { formatTime } from "./ConsolePrimitives";

export function QualityFindingsSummary({
  findings,
  onOpenReport,
  targetBranch,
  targetCommit,
}: {
  findings: QualityFindings;
  onOpenReport?: (reportPath: string) => void;
  targetBranch?: string;
  targetCommit?: string;
}): ReactElement {
  const severity = Object.entries(findings.severity_counts);
  const dispositions = Object.entries(findings.disposition_counts).sort(
    ([left], [right]) => left.localeCompare(right),
  );
  const normalizedTargetBranch = targetBranch?.trim() || undefined;
  const normalizedTargetCommit = targetCommit?.trim() || undefined;
  const scannedBranch = findings.scanned_branch?.trim() || undefined;
  const scannedCommit = findings.scanned_commit?.trim() || undefined;
  const shortTargetCommit = normalizedTargetCommit?.slice(0, 8);
  const targetMode = hasTargetScope(
    normalizedTargetBranch,
    normalizedTargetCommit,
  );
  const evidenceStatus = branchEvidenceStatus({
    targetBranch: normalizedTargetBranch,
    targetCommit: normalizedTargetCommit,
    scannedBranch,
    scannedCommit,
  });
  const evidenceState = targetMode
    ? targetEvidenceState({
        targetBranch: normalizedTargetBranch,
        targetCommit: normalizedTargetCommit,
        scannedBranch,
        scannedCommit,
      })
    : "verified";
  const targetMatchesEvidence = evidenceStatus === "verified" || !targetMode;
  const branchEvidenceVisible =
    targetMatchesEvidence ||
    evidenceState === "stale" ||
    evidenceState === "unscoped";
  const provenanceTone = targetMatchesEvidence
    ? "match"
    : evidenceState === "stale"
      ? "warning"
      : "unknown";
  const provenanceMessage = targetEvidenceMessage({
    targetBranch: normalizedTargetBranch,
    targetCommit: normalizedTargetCommit,
    scannedBranch,
    scannedCommit,
  });
  const evidenceDescriptor = targetEvidenceDescriptor({
    scannedBranch,
    scannedCommit,
  });
  const findingsCount = findings.total.toLocaleString();
  const findingsBreakdown = (
    <div className="quality-findings-breakdown">
      <span className="quality-findings-scope-note">
        {targetMatchesEvidence
          ? "Breakdown matches the selected target."
          : evidenceState === "stale"
            ? "Breakdown is from the selected branch at an older head; it is not the current target result."
            : evidenceState === "unscoped"
              ? "Breakdown is from evidence without branch/head provenance; it is not a target result."
              : "Breakdown below is from the scanned evidence and is not a target result."}
      </span>
      <div className="quality-severity-list">
        <span>
          <b>{findings.actionable_total.toLocaleString()}</b> actionable
        </span>
        <span>
          <b>{findings.unreviewed_total.toLocaleString()}</b> awaiting review
        </span>
      </div>
      {severity.length > 0 ? (
        <div className="quality-severity-list">
          {severity.map(([label, count]) => (
            <span key={label}>
              <b>{count.toLocaleString()}</b> {label}
            </span>
          ))}
        </div>
      ) : (
        <span className="quality-muted">No severity breakdown</span>
      )}
      {dispositions.length > 0 && (
        <div className="quality-severity-list">
          {dispositions.map(([status, count]) => (
            <span key={status}>
              <b>{count.toLocaleString()}</b> {status.replaceAll("_", " ")}
            </span>
          ))}
        </div>
      )}
    </div>
  );
  return (
    <div className="quality-findings-summary">
      <div
        className={`quality-findings-total${targetMatchesEvidence ? "" : " quality-findings-total-unverified"}`}
      >
        <strong
          aria-label={
            branchEvidenceVisible ? undefined : "Target QR findings unavailable"
          }
        >
          {branchEvidenceVisible ? findingsCount : "—"}
        </strong>
        <span>
          {targetMatchesEvidence
            ? "QR findings verified for target"
            : evidenceState === "stale"
              ? "QR findings from stale branch evidence"
              : evidenceState === "unscoped"
                ? "QR findings from unscoped evidence"
                : "Target QR findings unavailable"}
        </span>
      </div>
      {branchEvidenceVisible ? (
        findingsBreakdown
      ) : (
        <details className="quality-findings-raw-details">
          <summary className="quality-findings-raw">
            <span>Raw scanned evidence</span>
            <strong>{findingsCount}</strong>
            <span>QR findings detected in scanned evidence</span>
          </summary>
          {findingsBreakdown}
        </details>
      )}
      <div
        className={`quality-findings-provenance quality-findings-provenance-${provenanceTone}`}
        aria-label="QR findings provenance"
        role="status"
      >
        <span>
          <b>Target:</b> {normalizedTargetBranch ?? "Unknown"}
          {shortTargetCommit ? ` @ ${shortTargetCommit}` : ""}
        </span>
        <span>
          <b>Evidence:</b> {evidenceDescriptor || "branch/commit unavailable"}
        </span>
        <span>{provenanceMessage}</span>
      </div>
      <div className="quality-findings-meta">
        <span>{findings.freshness}</span>
        <span title={findings.disposition_message}>
          Review ledger: {findings.disposition_status}
        </span>
        {findings.stale_disposition_total > 0 && (
          <span>
            {findings.stale_disposition_total} inactive review{" "}
            {findings.stale_disposition_total === 1 ? "decision" : "decisions"}
          </span>
        )}
        <span>{formatTime(findings.observed_at)}</span>
        {findings.report_path && onOpenReport && (
          <button
            className="quality-report-link"
            type="button"
            onClick={() => onOpenReport(findings.report_path as string)}
          >
            <FileSearch size={12} />
            Detailed report
          </button>
        )}
      </div>
    </div>
  );
}
