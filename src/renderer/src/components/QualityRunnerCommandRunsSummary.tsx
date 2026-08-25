import type { ReactElement } from "react";
import { FileSearch, TerminalSquare } from "lucide-react";
import type { QrCommandRunLedgerSnapshot } from "../types";
import { StatusPill, formatTime } from "./ConsolePrimitives";

export function QualityRunnerCommandRunsSummary({
  commandRuns,
  onOpenReport,
}: {
  commandRuns?: QrCommandRunLedgerSnapshot;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const status = commandRuns?.status ?? "No evidence";
  const modes = Object.entries(commandRuns?.mode_counts ?? {}).sort(
    ([a], [b]) => a.localeCompare(b),
  );
  return (
    <section
      className="qr-command-runs-summary"
      aria-label="Quality Runner command runs"
    >
      <div className="qr-command-runs-heading">
        <span>
          <TerminalSquare size={13} />
          <strong>Quality Runner command runs</strong>
        </span>
        <StatusPill
          tone={
            status === "Warnings"
              ? "amber"
              : status === "Available"
                ? "mint"
                : "slate"
          }
        >
          {status}
        </StatusPill>
      </div>
      <p>
        {commandRuns?.detail ??
          "No Quality Runner command runs have been imported."}
      </p>
      {commandRuns && (
        <div className="qr-command-runs-meta">
          <span>{commandRuns.total_count} total runs</span>
          <span>{commandRuns.invalid_count} invalid manifests</span>
          {modes.map(([mode, count]) => (
            <span key={mode}>
              {mode}: {count}
            </span>
          ))}
        </div>
      )}
      {(commandRuns?.runs.length ?? 0) > 0 && (
        <details className="qr-command-runs-list">
          <summary>
            Newest {commandRuns?.visible_count} of {commandRuns?.total_count}{" "}
            runs
          </summary>
          <div>
            {commandRuns?.runs.map((run) => (
              <article key={run.run_id}>
                <div>
                  <span>
                    <strong>{run.mode}</strong>
                    <small>{run.run_id}</small>
                  </span>
                  <span>
                    <StatusPill
                      tone={run.status === "Invalid" ? "red" : "mint"}
                    >
                      {run.status}
                    </StatusPill>
                    <StatusPill
                      tone={run.freshness === "Fresh" ? "mint" : "slate"}
                    >
                      {run.freshness}
                    </StatusPill>
                  </span>
                </div>
                <p>
                  {run.artifact_count} artifacts
                  {run.observed_at ? ` · ${formatTime(run.observed_at)}` : ""}
                </p>
                {onOpenReport && (
                  <button
                    className="quality-report-link"
                    type="button"
                    onClick={() => onOpenReport(run.manifest_path)}
                  >
                    <FileSearch size={12} /> Run manifest
                  </button>
                )}
              </article>
            ))}
          </div>
        </details>
      )}
    </section>
  );
}
