import type { ReactElement } from "react";
import type { WorkspaceSummary } from "../types";
import { formatExactTime, StatusPill } from "./ConsolePrimitives";

export function WorkspaceSyncDetailView({
  workspace,
  onClose,
}: {
  workspace: WorkspaceSummary;
  onClose: () => void;
}): ReactElement {
  const detail = workspace.sync_detail;
  const gitStatusUnavailable = workspace.status_available === false;
  return (
    <section
      className="workspace-sync-detail"
      aria-label={`${gitStatusUnavailable ? "Git status unavailable" : "Unsynced workspace detail"} for ${workspace.branch}`}
    >
      <div className="workspace-sync-detail-header">
        <div>
          <p className="eyebrow">
            {gitStatusUnavailable
              ? "Git status unavailable"
              : "Unsynced workspace"}
          </p>
          <h4>Sync evidence detail</h4>
          <small>
            {workspace.branch} · {workspace.path}
          </small>
        </div>
        <StatusPill tone="amber">{workspace.sync_state}</StatusPill>
      </div>
      {detail ? (
        <>
          <div className="workspace-sync-detail-grid">
            <div>
              <span>Evidence observed</span>
              <strong>{formatExactTime(detail.evidence_observed_at)}</strong>
            </div>
            <div>
              <span>Evidence expires</span>
              <strong>{formatExactTime(detail.evidence_expires_at)}</strong>
            </div>
            <div>
              <span>Evidence window</span>
              <strong>
                {detail.evidence_window_minutes >= 60
                  ? `${Math.round(detail.evidence_window_minutes / 60)} hours`
                  : `${detail.evidence_window_minutes} minutes`}
              </strong>
            </div>
          </div>
          <div className="workspace-sync-detail-copy">
            <span>
              {gitStatusUnavailable
                ? "Why Git status is unavailable"
                : "Why this workspace is unsynced"}
            </span>
            <p>{detail.reason}</p>
          </div>
          <div className="workspace-sync-detail-copy">
            <span>Next safe scoped refresh</span>
            <p>{detail.next_safe_action}</p>
            <code>{detail.scoped_refresh_command}</code>
            <small>{detail.authorization}</small>
          </div>
        </>
      ) : (
        <p className="quality-inline-empty">
          Sync detail is unavailable in this snapshot. Run the scoped local
          refresh from the CLI, then reopen this repository detail.
        </p>
      )}
      <button
        className="button button-quiet workspace-sync-detail-close"
        type="button"
        onClick={onClose}
      >
        Close sync detail
      </button>
    </section>
  );
}
