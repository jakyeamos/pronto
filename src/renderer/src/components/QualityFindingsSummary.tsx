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

function readableQualityEvidenceText(value: string): string {
  return value.replace(/\bunknown\b/gi, "not confirmed");
}

function recordSummary(record: Record<string, string> | undefined): string {
  return Object.entries(record ?? {})
    .map(([key, value]) => key + " " + value)
    .join(" · ");
}

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
  const categoryKeys = new Set([
    ...Object.keys(findings.category_counts ?? {}),
    ...Object.keys(findings.actionable_category_counts ?? {}),
  ]);
  const categories = Array.from(categoryKeys)
    .map((category) => ({
      category,
      detected: findings.category_counts?.[category] ?? 0,
      actionable:
        findings.actionable_category_counts?.[category] ??
        findings.category_counts?.[category] ??
        0,
    }))
    .filter(({ detected, actionable }) => detected > 0 || actionable > 0)
    .sort(
      (left, right) =>
        right.actionable - left.actionable ||
        right.detected - left.detected ||
        left.category.localeCompare(right.category),
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
  const refreshRequired = findings.refresh_required === true;
  const targetEvidenceUsable = targetMatchesEvidence && !refreshRequired;
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
  const findingsCount = (
    findings.detector_findings_total ?? findings.total
  ).toLocaleString();
  const actionableCount = (
    findings.detector_actionable_total ?? findings.actionable_total
  ).toLocaleString();
  const unreviewedCount = (
    findings.detector_unreviewed_total ?? findings.unreviewed_total
  ).toLocaleString();
  const findingsBreakdown = (
    <div className="quality-findings-breakdown">
      <span className="quality-findings-scope-note">
        {refreshRequired
          ? "The prior detector count is retained as raw evidence, but the pinned detector receipt requires a refresh."
          : targetMatchesEvidence
            ? "Breakdown matches the selected target."
            : evidenceState === "stale"
              ? "Breakdown is from the selected branch at an older head; it is not the current target result."
              : evidenceState === "unscoped"
                ? "Breakdown is from evidence without branch/head provenance; it is not a target result."
                : "Breakdown below is from the scanned evidence and is not a target result."}
      </span>
      <div className="quality-severity-list">
        <span>
          <b>{actionableCount}</b> actionable detector findings
        </span>
        <span>
          <b>{unreviewedCount}</b> unreviewed detector findings
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
      {categories.length > 0 && (
        <details className="quality-finding-categories">
          <summary>
            {categories.length.toLocaleString()} finding{" "}
            {categories.length === 1 ? "category" : "categories"}
          </summary>
          <div
            aria-label="Detector finding categories"
            className="quality-finding-category-list"
          >
            {categories.map(({ category, detected, actionable }) => (
              <span key={category} title={category}>
                <b>{actionable.toLocaleString()}</b> actionable ·{" "}
                {detected.toLocaleString()} detected ·{" "}
                {category.replace(/[:_-]+/g, " ")}
              </span>
            ))}
          </div>
        </details>
      )}
    </div>
  );
  return (
    <div className="quality-findings-summary">
      <div
        className={`quality-findings-total${targetEvidenceUsable ? "" : " quality-findings-total-unverified"}`}
      >
        <strong
          aria-label={
            targetEvidenceUsable
              ? undefined
              : refreshRequired
                ? "Detector findings refresh required"
                : branchEvidenceVisible
                  ? undefined
                  : "Target detector findings unavailable"
          }
        >
          {targetEvidenceUsable || (!refreshRequired && branchEvidenceVisible)
            ? findingsCount
            : "—"}
        </strong>
        <span>
          {refreshRequired
            ? "Detector findings refresh required"
            : targetMatchesEvidence
              ? "Detector findings verified for target"
              : evidenceState === "stale"
                ? "Detector findings from stale branch evidence"
                : evidenceState === "unscoped"
                  ? "Detector findings from unscoped evidence"
                  : "Target detector findings unavailable"}
        </span>
      </div>
      {branchEvidenceVisible && !refreshRequired ? (
        findingsBreakdown
      ) : (
        <details className="quality-findings-raw-details">
          <summary className="quality-findings-raw">
            <span>
              {refreshRequired
                ? "Refresh-required detector evidence"
                : "Raw scanned evidence"}
            </span>
            <strong>{findingsCount}</strong>
            <span>Detector findings retained from scanned evidence</span>
          </summary>
          {findingsBreakdown}
        </details>
      )}
      <div
        className={`quality-findings-provenance quality-findings-provenance-${refreshRequired ? "warning" : provenanceTone}`}
        aria-label="Detector findings provenance"
        role="status"
      >
        <span>
          <b>Target:</b> {normalizedTargetBranch ?? "Target not specified"}
          {shortTargetCommit ? ` @ ${shortTargetCommit}` : ""}
        </span>
        <span>
          <b>Evidence:</b> {evidenceDescriptor || "branch/commit unavailable"}
        </span>
        <span>{provenanceMessage}</span>
      </div>
      {refreshRequired && (
        <div className="quality-findings-refresh-warning" role="alert">
          <b>Refresh required.</b>{" "}
          {readableQualityEvidenceText(
            findings.refresh_required_reason ??
              "The detector configuration or execution evidence is not current.",
          )}
        </div>
      )}
      <div className="quality-findings-detector-meta">
        <span>
          <b>Enabled:</b> {findings.enabled_detector_count ?? 0} detector
          {(findings.enabled_detector_count ?? 0) === 1 ? "" : "s"} ·{" "}
          {findings.enabled_rule_count ?? 0} rule
          {(findings.enabled_rule_count ?? 0) === 1 ? "" : "s"}
        </span>
        {recordSummary(findings.producer_versions) && (
          <span>
            <b>Producer:</b> {recordSummary(findings.producer_versions)}
          </span>
        )}
        {findings.qr_version && (
          <span>
            <b>QR:</b> {findings.qr_version}
          </span>
        )}
        {findings.target_sha && (
          <span>
            <b>Target SHA:</b> {findings.target_sha.slice(0, 12)}
          </span>
        )}
        {findings.refresh_time && (
          <span>
            <b>Detector refresh:</b> {formatTime(findings.refresh_time)}
          </span>
        )}
        {typeof findings.delta_total === "number" && (
          <span>
            <b>Delta since prior comparable scan:</b>{" "}
            {findings.delta_total > 0 ? "+" : ""}
            {findings.delta_total}
          </span>
        )}
        {(Object.keys(findings.ruleset_fingerprints ?? {}).length > 0 ||
          Object.keys(findings.configuration_fingerprints ?? {}).length > 0 ||
          Object.keys(findings.producer_source_shas ?? {}).length > 0) && (
          <details>
            <summary>Detector fingerprints</summary>
            <span>
              Ruleset:{" "}
              {recordSummary(findings.ruleset_fingerprints) || "not reported"}
            </span>
            <span>
              Configuration:{" "}
              {recordSummary(findings.configuration_fingerprints) ||
                "not reported"}
            </span>
            <span>
              Producer source:{" "}
              {recordSummary(findings.producer_source_shas) || "not reported"}
            </span>
          </details>
        )}
      </div>
      <div className="quality-findings-meta">
        <span>{readableQualityEvidenceText(findings.freshness)}</span>
        <span
          title={
            findings.disposition_message
              ? readableQualityEvidenceText(findings.disposition_message)
              : undefined
          }
        >
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
