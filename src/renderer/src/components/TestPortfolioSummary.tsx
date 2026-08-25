import type { ReactElement } from "react";
import { FileSearch, ListChecks } from "lucide-react";
import type { TestPortfolioRunSnapshot, TestPortfolioSnapshot } from "../types";
import { StatusPill, formatTime } from "./ConsolePrimitives";

function statusTone(status: string): string {
  if (status === "Proof passed" || status === "Audited") return "mint";
  if (status === "Blocked" || status === "Invalid") return "red";
  if (status === "Review required") return "amber";
  return "slate";
}

function RunSummary({
  label,
  run,
  onOpenReport,
}: {
  label: string;
  run?: TestPortfolioRunSnapshot;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  if (!run) {
    return (
      <article className="test-portfolio-run test-portfolio-run-empty">
        <strong>{label}</strong>
        <span>No imported run</span>
      </article>
    );
  }
  return (
    <article className="test-portfolio-run">
      <div>
        <strong>{label}</strong>
        <span>
          <StatusPill tone={statusTone(run.status)}>{run.status}</StatusPill>
          <StatusPill tone={run.freshness === "Fresh" ? "mint" : "slate"}>
            {run.freshness}
          </StatusPill>
        </span>
      </div>
      <small>
        {run.run_id}
        {run.observed_at ? ` · ${formatTime(run.observed_at)}` : ""}
      </small>
      <p>{run.detail}</p>
      <div className="test-portfolio-run-counts">
        {run.operation === "portfolio-audit" ? (
          <>
            <span>{run.removal_candidate_count} candidates</span>
            <span>{run.insufficient_evidence_count} insufficient</span>
          </>
        ) : (
          <>
            <span>{run.removed_test_count} tests assessed</span>
            <span>{run.blocker_ids.length} blockers</span>
          </>
        )}
      </div>
      {onOpenReport && (
        <button
          className="quality-report-link"
          type="button"
          onClick={() => onOpenReport(run.report_path)}
        >
          <FileSearch size={12} /> Source artifact
        </button>
      )}
    </article>
  );
}

export function TestPortfolioSummary({
  testPortfolio,
  onOpenReport,
}: {
  testPortfolio?: TestPortfolioSnapshot;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const status = testPortfolio?.status ?? "No evidence";
  return (
    <section
      className="test-portfolio-summary"
      aria-label="Test portfolio evidence"
    >
      <div className="test-portfolio-heading">
        <span>
          <ListChecks size={13} />
          <strong>Test portfolio evidence</strong>
        </span>
        <StatusPill tone={statusTone(status)}>{status}</StatusPill>
      </div>
      <p>
        {testPortfolio?.detail ??
          "No Quality Runner test-portfolio command run has been imported."}
      </p>
      <small className="test-portfolio-authority">
        Advisory only · never authorizes source mutation or test deletion
      </small>
      <div className="test-portfolio-runs">
        <RunSummary
          label="Portfolio audit"
          run={testPortfolio?.audit}
          onOpenReport={onOpenReport}
        />
        <RunSummary
          label="Removal proof"
          run={testPortfolio?.removal_proof}
          onOpenReport={onOpenReport}
        />
      </div>
    </section>
  );
}
