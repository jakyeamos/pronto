import type { ReactElement } from "react";
import { FileSearch } from "lucide-react";
import type { QualityFindings } from "../types";
import { formatTime } from "./ConsolePrimitives";

export function QualityFindingsSummary({
  findings,
  onOpenReport,
}: {
  findings: QualityFindings;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const severity = Object.entries(findings.severity_counts);
  const dispositions = Object.entries(findings.disposition_counts).sort(
    ([left], [right]) => left.localeCompare(right),
  );
  return (
    <div className="quality-findings-summary">
      <div className="quality-findings-total">
        <strong>{findings.total}</strong>
        <span>QR findings detected</span>
      </div>
      <div className="quality-severity-list">
        <span>
          <b>{findings.actionable_total}</b> actionable
        </span>
        <span>
          <b>{findings.unreviewed_total}</b> awaiting review
        </span>
      </div>
      {severity.length > 0 ? (
        <div className="quality-severity-list">
          {severity.map(([label, count]) => (
            <span key={label}>
              <b>{count}</b> {label}
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
              <b>{count}</b> {status.replaceAll("_", " ")}
            </span>
          ))}
        </div>
      )}
      <div className="quality-findings-meta">
        <span>{findings.freshness}</span>
        <span>Review ledger: {findings.disposition_status}</span>
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
