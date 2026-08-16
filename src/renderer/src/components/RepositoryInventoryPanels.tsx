import { Fragment, useState, type ReactElement } from "react";
import { ChevronRight, MonitorDot, ShieldCheck } from "lucide-react";
import type { Condition, ExternalTool, RepositorySnapshot } from "../types";
import { ConditionPill, StatusPill } from "./ConsolePrimitives";
import { WorkspaceSyncDetailView } from "./WorkspaceSyncDetailView";

export function RepositoryInventoryPanels({
  repository,
  onOpenWorkspace,
  onPrepareRepository,
  onCondition,
}: {
  repository: RepositorySnapshot;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onCondition: (condition: Condition) => void;
}): ReactElement {
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(
    null,
  );
  return (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">
          <h3>Workspaces</h3>
          <span>{repository.workspaces.length}</span>
        </div>
        <div className="workspace-list">
          {repository.workspaces.map((workspace) => {
            const showingSyncDetail = selectedWorkspaceId === workspace.id;
            const gitStatusUnavailable = workspace.status_available === false;
            return (
              <Fragment key={workspace.id}>
                <div className="workspace-card">
                  <div className="workspace-card-top">
                    <span className="workspace-name">
                      <MonitorDot size={13} />
                      {workspace.is_primary
                        ? "Primary checkout"
                        : "Linked worktree"}
                    </span>
                    <StatusPill
                      tone={
                        gitStatusUnavailable
                          ? "amber"
                          : workspace.dirty
                            ? "coral"
                            : "mint"
                      }
                    >
                      {gitStatusUnavailable
                        ? "Git status unavailable"
                        : workspace.dirty
                          ? "Dirty"
                          : "Clean"}
                    </StatusPill>
                  </div>
                  <strong>
                    {gitStatusUnavailable
                      ? "Git status unavailable"
                      : workspace.branch}
                  </strong>
                  <span>{workspace.path}</span>
                  <small>
                    {gitStatusUnavailable
                      ? workspace.status_error || "Git status unavailable"
                      : `${workspace.sync_state} · ${workspace.remote_freshness}`}
                  </small>
                  <div className="workspace-activity">
                    <StatusPill
                      tone={
                        workspace.activity.state === "Active"
                          ? "blue"
                          : workspace.activity.state.startsWith("Interrupted")
                            ? "coral"
                            : "slate"
                      }
                    >
                      {workspace.activity.state}
                    </StatusPill>
                    <span>{workspace.activity.confidence} confidence</span>
                  </div>
                  <small>
                    {workspace.activity.signals
                      .map((signal) => signal.summary)
                      .join(" · ")}
                  </small>
                  {workspace.activity.manifest && (
                    <small>
                      {workspace.activity.manifest.title ||
                        workspace.activity.manifest.task_id ||
                        "Structured agent task metadata"}
                    </small>
                  )}
                  <div className="workspace-actions">
                    {(
                      [
                        ["file_browser", "Finder"],
                        ["terminal", "Terminal"],
                        ["editor", "Editor"],
                        ["git_client", "Git client"],
                      ] as Array<[ExternalTool, string]>
                    ).map(([tool, label]) => (
                      <button
                        className="button button-quiet"
                        type="button"
                        key={tool}
                        onClick={() => void onOpenWorkspace(workspace.id, tool)}
                      >
                        {label}
                      </button>
                    ))}
                    <button
                      className="button button-quiet"
                      type="button"
                      onClick={() => void onPrepareRepository(workspace.id)}
                    >
                      Review preparation
                    </button>
                    {workspace.sync_state !== "Synced" && (
                      <button
                        className="button button-secondary"
                        type="button"
                        aria-expanded={showingSyncDetail}
                        onClick={() =>
                          setSelectedWorkspaceId(
                            showingSyncDetail ? null : workspace.id,
                          )
                        }
                      >
                        {showingSyncDetail
                          ? "Hide sync detail"
                          : "View sync detail"}
                      </button>
                    )}
                  </div>
                </div>
                {showingSyncDetail && (
                  <WorkspaceSyncDetailView
                    workspace={workspace}
                    onClose={() => setSelectedWorkspaceId(null)}
                  />
                )}
              </Fragment>
            );
          })}
        </div>
      </div>
      {repository.custody && (
        <div className="drawer-section">
          <div className="drawer-section-title">
            <h3>Custody</h3>
            <span>{repository.custody.lanes.length}</span>
          </div>
          <StatusPill
            tone={
              repository.custody.status === "attention_required"
                ? "amber"
                : "mint"
            }
          >
            {repository.custody.status.replaceAll("_", " ")}
          </StatusPill>
          <small>
            Read-only projection · {repository.custody.source} · no mutation
            authority
          </small>
          <small>{repository.custody.next_safe_step}</small>
          {repository.custody.unleased_worktrees.length > 0 && (
            <div className="workspace-card">
              <strong>Unleased worktrees</strong>
              <span>
                {repository.custody.unleased_worktrees.length} worktree(s)
                require a lease review.
              </span>
            </div>
          )}
          {repository.custody.lanes.length > 0 ? (
            <div className="branch-table">
              {repository.custody.lanes.map((lane) => (
                <div className="branch-row" key={lane.task_id}>
                  <div>
                    <strong>{lane.task_id}</strong>
                    <span>
                      {lane.state} · {lane.disposition}
                    </span>
                    {lane.branch && <span>{lane.branch}</span>}
                    <small>{lane.next_action}</small>
                  </div>
                  <StatusPill
                    tone={
                      lane.state === "active" || lane.state === "closed"
                        ? "mint"
                        : lane.state === "unknown" || lane.state === "contested"
                          ? "coral"
                          : "amber"
                    }
                  >
                    {lane.disposition}
                  </StatusPill>
                </div>
              ))}
            </div>
          ) : (
            <div className="drawer-empty">No custody lanes recorded.</div>
          )}
        </div>
      )}
      {repository.submodules.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">
            <h3>Submodules</h3>
            <span>{repository.submodules.length}</span>
          </div>
          <div className="branch-table">
            {repository.submodules.map((submodule) => (
              <div className="branch-row" key={submodule.path}>
                <div>
                  <strong>{submodule.path}</strong>
                  <span>{submodule.commit ?? "Commit unavailable"}</span>
                </div>
                <StatusPill
                  tone={submodule.status === "Checked out" ? "mint" : "amber"}
                >
                  {submodule.status}
                </StatusPill>
              </div>
            ))}
          </div>
        </div>
      )}
      <div className="drawer-section">
        <div className="drawer-section-title">
          <h3>Conditions</h3>
          <span>{repository.conditions.length}</span>
        </div>
        {repository.conditions.length === 0 ? (
          <div className="drawer-empty">
            <ShieldCheck size={17} />
            No current conditions.
          </div>
        ) : (
          <div className="drawer-conditions">
            {repository.conditions.map((condition) => (
              <button
                className="drawer-condition"
                type="button"
                key={condition.id}
                onClick={() => onCondition(condition)}
              >
                <ConditionPill condition={condition} />
                <span>{condition.summary}</span>
                <ChevronRight size={15} />
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">
          <h3>Branches</h3>
          <span>{repository.branches.length}</span>
        </div>
        <div className="branch-table">
          {repository.branches.map((branch) => (
            <div className="branch-row" key={branch.name}>
              <div>
                <strong>{branch.name}</strong>
                <span>
                  {branch.role} · {branch.role_confidence} confidence
                </span>
              </div>
              <StatusPill
                tone={
                  branch.integration_state === "Integration eligible"
                    ? "mint"
                    : "slate"
                }
              >
                {branch.integration_state}
              </StatusPill>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
