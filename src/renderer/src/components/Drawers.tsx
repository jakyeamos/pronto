import type { ReactElement } from "react";
import {
  AlertTriangle,
  BellOff,
  Check,
  ChevronRight,
  Clock3,
  GitBranch,
  MonitorDot,
  ShieldCheck,
  TerminalSquare,
  X,
} from "lucide-react";
import type { Condition, ExternalTool, RepositorySnapshot } from "../types";
import {
  ConditionPill,
  formatTime,
  IconButton,
  StatusPill,
} from "./ConsolePrimitives";

export function DetailDrawer({
  repository,
  onClose,
  onOpenWorkspace,
  onPrepareRepository,
  onLifecycleChange,
  onCondition,
}: {
  repository: RepositorySnapshot;
  onClose: () => void;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (condition: Condition) => void;
}): ReactElement {
  return (
    <div className="drawer-layer" role="presentation">
      <button
        className="drawer-scrim"
        aria-label="Close repository detail"
        type="button"
        onClick={onClose}
      />
      <aside className="detail-drawer" aria-label={`${repository.name} detail`}>
        <div className="drawer-header">
          <div>
            <p className="eyebrow">Repository detail</p>
            <h2>{repository.name}</h2>
          </div>
          <IconButton label="Close repository detail" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        <p className="drawer-path">{repository.path}</p>
        <div className="detail-summary-grid">
          <div>
            <span>Current branch</span>
            <strong>{repository.branch}</strong>
          </div>
          <div>
            <span>Default branch</span>
            <strong>{repository.default_branch ?? "Unknown"}</strong>
          </div>
          <div>
            <span>Lifecycle</span>
            <select
              className="drawer-select"
              aria-label={`Lifecycle for ${repository.name}`}
              value={repository.lifecycle}
              onChange={(event) => void onLifecycleChange(event.target.value)}
            >
              <option>Unconfirmed</option>
              <option>Active</option>
              <option>Maintenance</option>
              <option>Paused</option>
              <option>Archived</option>
            </select>
            {repository.lifecycle === "Unconfirmed" && (
              <small>{repository.lifecycle_candidate} candidate</small>
            )}
          </div>
          <div>
            <span>Remote freshness</span>
            <strong>
              {repository.last_fetch_at
                ? formatTime(repository.last_fetch_at)
                : "Not fetched by Pronto"}
            </strong>
          </div>
        </div>
        <div className="drawer-section">
          <div className="drawer-section-title">
            <h3>Workspaces</h3>
            <span>{repository.workspaces.length}</span>
          </div>
          <div className="workspace-list">
            {repository.workspaces.map((workspace) => (
              <div className="workspace-card" key={workspace.id}>
                <div className="workspace-card-top">
                  <span className="workspace-name">
                    <MonitorDot size={13} />
                    {workspace.is_primary
                      ? "Primary checkout"
                      : "Linked worktree"}
                  </span>
                  <StatusPill tone={workspace.dirty ? "coral" : "mint"}>
                    {workspace.dirty ? "Dirty" : "Clean"}
                  </StatusPill>
                </div>
                <strong>{workspace.branch}</strong>
                <span>{workspace.path}</span>
                <small>
                  {workspace.sync_state} · {workspace.remote_freshness}
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
                </div>
              </div>
            ))}
          </div>
        </div>
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
        <div className="drawer-footer">
          <StatusPill tone="slate" icon={<GitBranch size={11} />}>
            {repository.provider_state}
          </StatusPill>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onPrepareRepository(repository.workspace.id)}
          >
            <TerminalSquare size={15} />
            Review PR / release evidence
          </button>
        </div>
      </aside>
    </div>
  );
}

export function EvidenceDrawer({
  repository,
  condition,
  onClose,
  onExpected,
}: {
  repository: RepositorySnapshot;
  condition: Condition;
  onClose: () => void;
  onExpected: () => void;
}): ReactElement {
  const isExpected = condition.status === "Expected";
  return (
    <div className="drawer-layer" role="presentation">
      <button
        className="drawer-scrim"
        aria-label="Close evidence"
        type="button"
        onClick={onClose}
      />
      <aside
        className="evidence-drawer"
        aria-label={`${condition.title} evidence`}
      >
        <div className="drawer-header">
          <div>
            <p className="eyebrow">Why this is here</p>
            <h2>{condition.title}</h2>
          </div>
          <IconButton label="Close evidence" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        <div className="evidence-hero">
          <ConditionPill condition={condition} />
          <p>{condition.summary}</p>
          <span>Repository · {repository.name}</span>
        </div>
        <div className="evidence-block">
          <h3>Rule</h3>
          <p>{condition.rule}</p>
        </div>
        <div className="evidence-block">
          <h3>Evidence</h3>
          <div className="evidence-list">
            {condition.evidence.map((item) => (
              <div className="evidence-row" key={`${item.label}-${item.value}`}>
                <span>{item.label}</span>
                <strong>{item.value || "Not available"}</strong>
                <small>
                  {item.source} · {formatTime(item.observed_at)}
                </small>
              </div>
            ))}
          </div>
        </div>
        <div className="evidence-block">
          <h3>Missing or bounded</h3>
          {condition.missing.length === 0 ? (
            <p className="evidence-positive">
              <Check size={15} />
              No missing facts recorded for this classification.
            </p>
          ) : (
            <ul className="evidence-missing">
              {condition.missing.map((item) => (
                <li key={item}>
                  <AlertTriangle size={14} />
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
        {condition.freshness && (
          <div className="freshness-note">
            <Clock3 size={15} />
            <span>
              <strong>Freshness</strong>
              {condition.freshness}
            </span>
          </div>
        )}
        <div className="drawer-footer">
          <button
            className="button button-secondary"
            type="button"
            onClick={onExpected}
          >
            <BellOff size={15} />
            {isExpected
              ? "Return to active queue"
              : "Mark current state expected"}
          </button>
          <span className="drawer-footnote">
            Expected state is attached to this exact evidence fingerprint.
          </span>
        </div>
      </aside>
    </div>
  );
}
