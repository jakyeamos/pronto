import type { ReactElement } from "react";
import type { RemediationAction, RemediationActionStatus } from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

const actionStatuses: RemediationActionStatus[] = [
  "open",
  "in_progress",
  "blocked",
  "deferred",
  "verified",
];

export function remediationStatusTone(status: string): string {
  const normalized = status.toLowerCase();
  if (
    normalized === "completed" ||
    normalized === "verified" ||
    normalized === "clear"
  )
    return "mint";
  if (normalized === "in_progress" || normalized === "in progress")
    return "blue";
  if (normalized === "blocked" || normalized === "failed") return "red";
  if (normalized === "deferred" || normalized === "partial") return "amber";
  if (
    normalized === "open" ||
    normalized === "pending" ||
    normalized === "attention"
  )
    return "coral";
  return "slate";
}

function labelForDomain(domain: string): string {
  return domain
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function remediationStatusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

export function formatRemediationTime(value?: string | null): string {
  return value ? formatTime(value) : "Not recorded";
}

function actionEvidenceIsFresh(action: RemediationAction): boolean {
  return action.evidence.some(
    (item) => item.freshness.toLowerCase() === "fresh",
  );
}

export function RemediationActionRow({
  action,
  onUpdateStatus,
}: {
  action: RemediationAction;
  onUpdateStatus: (
    actionId: string,
    status: RemediationActionStatus,
  ) => Promise<void>;
}): ReactElement {
  return (
    <article className="remediation-action-card">
      <div className="remediation-action-heading">
        <div>
          <div className="remediation-action-kicker">
            <StatusPill tone="slate">
              {labelForDomain(action.domain)}
            </StatusPill>
            <span>{action.priority}</span>
            <span>{action.weight} pts</span>
          </div>
          <h3>{action.title}</h3>
        </div>
        <select
          className="remediation-status-select"
          aria-label={`Status for ${action.title}`}
          value={action.status}
          onChange={(event) =>
            void onUpdateStatus(
              action.id,
              event.target.value as RemediationActionStatus,
            )
          }
        >
          {actionStatuses.map((status) => (
            <option value={status} key={status}>
              {remediationStatusLabel(status)}
            </option>
          ))}
        </select>
      </div>
      <div className="remediation-action-status-line">
        <StatusPill tone={remediationStatusTone(action.status)}>
          {remediationStatusLabel(action.status)}
        </StatusPill>
        <StatusPill tone={actionEvidenceIsFresh(action) ? "mint" : "amber"}>
          {actionEvidenceIsFresh(action)
            ? "Fresh evidence"
            : "Evidence needs refresh"}
        </StatusPill>
        <span>Updated {formatRemediationTime(action.updated_at)}</span>
      </div>
      <p>{action.summary}</p>
      <details className="remediation-action-details">
        <summary>Acceptance criteria and evidence</summary>
        <div className="remediation-action-detail-grid">
          <div>
            <strong>Acceptance criteria</strong>
            <ul>
              {action.acceptance_criteria.map((criterion) => (
                <li key={criterion}>{criterion}</li>
              ))}
            </ul>
          </div>
          <div>
            <strong>Evidence</strong>
            {action.evidence.length === 0 ? (
              <span className="remediation-muted">
                No source evidence recorded.
              </span>
            ) : (
              <div className="remediation-evidence-list">
                {action.evidence.map((item, index) => (
                  <div
                    className="remediation-evidence-item"
                    key={`${item.source}-${item.label}-${index}`}
                  >
                    <div>
                      <strong>
                        {item.source} · {item.label}
                      </strong>
                      <StatusPill tone={remediationStatusTone(item.status)}>
                        {item.status}
                      </StatusPill>
                    </div>
                    <span>{item.detail}</span>
                    <small>
                      {item.freshness} ·{" "}
                      {formatRemediationTime(item.observed_at)}
                    </small>
                    {item.report_path && <code>{item.report_path}</code>}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
        {action.notes && (
          <div className="remediation-notes">
            <strong>Notes</strong>
            <span>{action.notes}</span>
          </div>
        )}
      </details>
    </article>
  );
}
