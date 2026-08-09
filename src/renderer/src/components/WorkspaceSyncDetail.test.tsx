// quality-gate: allow static-ui-test: verifies unsynced evidence expiry, comparison reason, and the read-only scoped refresh contract remain visible together.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WorkspaceSummary } from "../types";
import { WorkspaceSyncDetailView } from "./WorkspaceSyncDetailView";

const unsyncedWorkspace: WorkspaceSummary = {
  id: "workspace-1",
  path: "/Users/example/projects/pronto",
  is_primary: true,
  branch: "feature/sync-detail",
  dirty: false,
  added: 0,
  removed: 0,
  line_totals_partial: false,
  sync_state: "Behind by 2",
  remote_freshness: "Not fetched by Pronto",
  ahead: 0,
  behind: 2,
  upstream: "origin/main",
  integration_state: "Behind",
  target_branch: "main",
  target_confidence: "High",
  role: "Agent task",
  role_confidence: "High",
  activity: { state: "Unknown", confidence: "Low", signals: [] },
  sync_detail: {
    reason:
      "Workspace branch 'feature/sync-detail' is behind by 2 commits relative to 'origin/main'.",
    evidence_observed_at: "2026-07-30T12:00:00Z",
    evidence_expires_at: "2026-08-01T12:00:00Z",
    evidence_window_minutes: 2880,
    next_safe_action:
      "Run the repository-scoped local refresh command below, then reopen this detail to compare the newly observed evidence.",
    scoped_refresh_command:
      "pronto refresh '/Users/example/projects/pronto' --json",
    authorization:
      "Read-only local Git scan; it persists Pronto evidence only and does not pull, push, merge, rebase, or edit repository files.",
  },
};

describe("WorkspaceSyncDetailView", () => {
  it("shows expiry, the unsynced reason, and the safe scoped refresh contract", () => {
    const markup = renderToStaticMarkup(
      <WorkspaceSyncDetailView
        workspace={unsyncedWorkspace}
        onClose={() => undefined}
      />,
    );

    expect(markup).toContain("Evidence expires");
    expect(markup).toContain("2026");
    expect(markup).toContain("UTC");
    expect(markup).toContain("Why this workspace is unsynced");
    expect(markup).toContain("behind by 2 commits");
    expect(markup).toContain("Next safe scoped refresh");
    expect(markup).toContain(
      "pronto refresh &#x27;/Users/example/projects/pronto&#x27; --json",
    );
    expect(markup).toContain("does not pull, push, merge, rebase");
  });

  it("labels failed Git status as unavailable instead of inferring sync state", () => {
    const markup = renderToStaticMarkup(
      <WorkspaceSyncDetailView
        workspace={{
          ...unsyncedWorkspace,
          branch: "Unknown",
          status_available: false,
          status_error: "Git status failed: fatal: not a git repository",
          sync_state: "Git status unavailable",
          sync_detail: {
            ...unsyncedWorkspace.sync_detail!,
            reason: "Git status failed: fatal: not a git repository",
          },
        }}
        onClose={() => undefined}
      />,
    );

    expect(markup).toContain("Git status unavailable");
    expect(markup).toContain("Why Git status is unavailable");
    expect(markup).not.toContain("Why this workspace is unsynced");
  });
});
