import type { ReactElement } from "react";
import { ExternalLink, FileSearch, Globe2 } from "lucide-react";
import type { WebReadinessSnapshot } from "../types";
import { StatusPill, formatTime } from "./ConsolePrimitives";

function statusTone(status: string): string {
  if (status === "Ready") return "mint";
  if (status === "Warnings") return "amber";
  if (status === "Blocked") return "red";
  return "slate";
}

function levelLabel(level: string): string {
  return level.replaceAll("_", " ");
}

export function WebReadinessSummary({
  webReadiness,
  onOpenReport,
}: {
  webReadiness?: WebReadinessSnapshot;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const status = webReadiness?.status ?? "Unknown";
  const checks = webReadiness?.checks ?? [];
  const target = webReadiness?.target;
  return (
    <section className="web-readiness-summary" aria-label="Web readiness">
      <div className="web-readiness-heading">
        <span>
          <Globe2 size={13} />
          <strong>Web readiness</strong>
        </span>
        <div>
          <StatusPill tone={statusTone(status)}>{status}</StatusPill>
          {webReadiness && (
            <StatusPill
              tone={webReadiness.freshness === "Fresh" ? "mint" : "slate"}
            >
              {webReadiness.freshness}
            </StatusPill>
          )}
        </div>
      </div>
      <p>
        {webReadiness?.applicability_reason ??
          "No Quality Runner web-readiness report has been imported."}
      </p>
      {webReadiness && (
        <div className="web-readiness-meta">
          <span>
            {webReadiness.applicability.replaceAll("_", " ")} · target{" "}
            {target?.kind || "unknown"}
          </span>
          <span>
            {webReadiness.passed_count} passed · {webReadiness.failed_count}{" "}
            failed · {webReadiness.blocked_count} blocked ·{" "}
            {webReadiness.unknown_count} unknown · {webReadiness.warning_count}{" "}
            warnings
          </span>
          {webReadiness.observed_at && (
            <span>Observed {formatTime(webReadiness.observed_at)}</span>
          )}
          {(target?.provider || target?.deployment_id) && (
            <span>
              {[target.provider, target.deployment_id]
                .filter(Boolean)
                .join(" · ")}
            </span>
          )}
        </div>
      )}
      {(target?.url || (webReadiness?.report_path && onOpenReport)) && (
        <div className="web-readiness-actions">
          {target?.url && (
            <a
              className="quality-report-link"
              href={target.url}
              target="_blank"
              rel="noreferrer"
            >
              <ExternalLink size={12} /> Deployment target
            </a>
          )}
          {webReadiness?.report_path && onOpenReport && (
            <button
              className="quality-report-link"
              type="button"
              onClick={() => onOpenReport(webReadiness.report_path as string)}
            >
              <FileSearch size={12} /> Source report
            </button>
          )}
        </div>
      )}
      {checks.length > 0 && (
        <details className="web-readiness-checks">
          <summary>{checks.length} checks by evidence level and route</summary>
          <div>
            {checks.map((check, index) => (
              <article key={`${check.id}-${check.verification_level}-${index}`}>
                <span>
                  <strong>{check.label}</strong>
                  <StatusPill
                    tone={statusTone(
                      check.status === "passed"
                        ? "Ready"
                        : check.status === "failed"
                          ? "Blocked"
                          : "Unknown",
                    )}
                  >
                    {check.status}
                  </StatusPill>
                </span>
                <small>
                  {check.policy} · {levelLabel(check.verification_level)}
                  {check.routes.length > 0
                    ? ` · ${check.routes.join(", ")}`
                    : ""}
                </small>
                <p>{check.detail}</p>
              </article>
            ))}
          </div>
        </details>
      )}
    </section>
  );
}
