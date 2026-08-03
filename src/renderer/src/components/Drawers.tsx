import { Fragment, useState } from "react";
import type { ReactElement } from "react";
import {
  ArrowLeft,
  ChevronRight,
  GitBranch,
  MonitorDot,
  ShieldCheck,
  TerminalSquare,
} from "lucide-react";
import type {
  AnalyticsSnapshot,
  Condition,
  ExternalTool,
  QualityGate,
  RepositorySnapshot,
} from "../types";
import { ConditionPill, formatTime, StatusPill } from "./ConsolePrimitives";
import {
  QualityFindingsSummary,
  QualityGateCell,
  QualityGateStatusPill,
  QualityMaturitySummary,
  qualityGateDisplayLabel,
} from "./QualityComponents";
import { ProjectCompassDetail } from "./ProjectCompassDetail";
import { RepositoryAnalyticsPanel } from "./RepositoryAnalyticsPanel";
import { WorkspaceSyncDetailView } from "./WorkspaceSyncDetailView";

export function RepositoryDetailSurface({
  repository,
  analytics,
  onBack,
  onOpenWorkspace,
  onPrepareRepository,
  onTargetBranchChange,
  onLifecycleChange,
  onCondition,
  onOpenReport,
}: {
  repository: RepositorySnapshot;
  analytics: AnalyticsSnapshot;
  onBack: () => void;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onTargetBranchChange: (targetBranch: string) => Promise<void>;
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (condition: Condition) => void;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(
    null,
  );
  const selectedTargetBranch =
    repository.target_branch ?? repository.default_branch ?? "";
  const targetBranches = Array.from(
    new Set(
      [
        selectedTargetBranch,
        ...repository.branches.map((branch) => branch.name),
      ].filter((branch): branch is string => branch.length > 0),
    ),
  );
  const detailQualityGates: QualityGate[] = [
    ...repository.quality.gates,
    ...repository.quality.ci_readiness.applicable_gate_ids
      .filter(
        (gateId) =>
          !repository.quality.gates.some((gate) => gate.id === gateId),
      )
      .map((gateId) => ({
        id: gateId,
        label: qualityGateDisplayLabel(gateId),
        status: "Not configured" as const,
        freshness: "Unknown" as const,
        evidence: [],
      })),
  ];
  return (
    <section
      className="repository-detail-surface"
      aria-label={`${repository.name} detail`}
    >
      <div className="repository-detail-toolbar">
        <button
          className="button button-quiet repository-back-button"
          type="button"
          onClick={onBack}
        >
          <ArrowLeft size={15} />
          Back to Portfolio
        </button>
        <StatusPill tone="slate" icon={<GitBranch size={11} />}>
          {repository.provider_state}
        </StatusPill>
      </div>
      <div className="repository-detail-header">
        <div>
          <p className="eyebrow">Repository detail</p>
          <h1>{repository.name}</h1>
          <p className="repository-detail-path">{repository.path}</p>
          <p className="repository-detail-remote">
            {repository.remote_url ?? "No remote identity connected"}
          </p>
        </div>
        <StatusPill tone={repository.lifecycle === "Active" ? "mint" : "slate"}>
          {repository.lifecycle}
        </StatusPill>
      </div>
      <div className="detail-summary-grid">
        <div>
          <span>Current branch</span>
          <strong>{repository.branch}</strong>
        </div>
        <div>
          <span>Target branch</span>
          <select
            className="drawer-select"
            aria-label={`Target branch for ${repository.name}`}
            value={selectedTargetBranch}
            disabled={targetBranches.length === 0}
            onChange={(event) => void onTargetBranchChange(event.target.value)}
          >
            {targetBranches.length === 0 ? (
              <option value="">Unknown</option>
            ) : (
              targetBranches.map((branch) => (
                <option value={branch} key={branch}>
                  {branch}
                </option>
              ))
            )}
          </select>
          <small>
            {repository.target_branch_configured
              ? `Pronto override · Git default: ${repository.default_branch ?? "Unknown"}`
              : `Following Git default: ${repository.default_branch ?? "Unknown"}`}
          </small>
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
      <ProjectCompassDetail repository={repository} />
      <div className="drawer-section quality-detail-section">
        <div className="drawer-section-title">
          <div>
            <h3>Quality gates</h3>
            <small>
              {repository.quality.ingestion_status} · evidence is read-only and
              source-specific.
            </small>
          </div>
          <StatusPill
            tone={
              repository.quality.ingestion_status === "Available"
                ? "mint"
                : "slate"
            }
          >
            {repository.quality.ingestion_status}
          </StatusPill>
        </div>
        <div className="quality-detail-overview">
          <QualityMaturitySummary
            maturity={repository.quality.maturity}
            readiness={repository.quality.ci_readiness}
            onOpenReport={onOpenReport}
          />
          <QualityFindingsSummary
            findings={repository.quality.findings}
            onOpenReport={onOpenReport}
          />
        </div>
        <div className="quality-detail-gates">
          {detailQualityGates.map((gate) => (
            <QualityGateCell
              gate={gate}
              configured={repository.quality.ci_readiness.configured_gate_ids.includes(
                gate.id,
              )}
              key={gate.id}
              onOpenReport={onOpenReport}
            />
          ))}
        </div>
        {repository.quality.ingestion_message && (
          <p className="quality-inline-empty">
            <ShieldCheck size={14} /> {repository.quality.ingestion_message}
          </p>
        )}
      </div>
      <RepositoryAnalyticsPanel repository={repository} analytics={analytics} />
      <div className="drawer-section repository-release-rule">
        <div className="drawer-section-title">
          <div>
            <h3>Release rule trace</h3>
            <small>
              Required gates are evaluated against their selected evidence
              source.
            </small>
          </div>
          <StatusPill tone={repository.release_rule ? "mint" : "slate"}>
            {repository.release_rule ? "Configured" : "Not configured"}
          </StatusPill>
        </div>
        {repository.release_rule ? (
          <>
            <div className="repository-release-rule-meta">
              <strong>{repository.release_rule.name}</strong>
              <span>
                {repository.release_rule.operator} evaluation ·{" "}
                {repository.release_rule.required_quality_gates.length} quality
                gate
                {repository.release_rule.required_quality_gates.length === 1
                  ? ""
                  : "s"}
              </span>
            </div>
            {repository.release_rule.required_quality_gates.length === 0 ? (
              <p className="quality-inline-empty">
                No quality gates are required by this rule.
              </p>
            ) : (
              <div className="repository-release-rule-list">
                {repository.release_rule.required_quality_gates.map(
                  (requirement) => {
                    const gate = repository.quality.gates.find(
                      (candidate) => candidate.id === requirement.gate_id,
                    );
                    const configuredWithoutEvidence =
                      gate?.status === "Not configured" &&
                      repository.quality.ci_readiness.configured_gate_ids.includes(
                        requirement.gate_id,
                      );
                    return (
                      <div
                        className="repository-release-rule-row"
                        key={requirement.gate_id + "-" + requirement.source}
                      >
                        <span>
                          <strong>{gate?.label ?? requirement.gate_id}</strong>
                          <small>{requirement.source} evidence</small>
                        </span>
                        {configuredWithoutEvidence ? (
                          <StatusPill tone="slate">
                            Awaiting evidence
                          </StatusPill>
                        ) : gate ? (
                          <QualityGateStatusPill
                            status={gate.status}
                            freshness={gate.freshness}
                          />
                        ) : (
                          <StatusPill tone="slate">Not configured</StatusPill>
                        )}
                      </div>
                    );
                  },
                )}
              </div>
            )}
            <small className="repository-release-rule-note">
              Open Review PR / release evidence for the complete eligibility
              trace.
            </small>
          </>
        ) : (
          <p className="quality-inline-empty">
            Add a release rule during preparation to evaluate quality gates
            before release.
          </p>
        )}
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">
          <h3>Workspaces</h3>
          <span>{repository.workspaces.length}</span>
        </div>
        <div className="workspace-list">
          {repository.workspaces.map((workspace) => {
            const showingSyncDetail = selectedWorkspaceId === workspace.id;
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
    </section>
  );
}
