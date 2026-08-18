import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import {
  ArrowLeft,
  GitBranch,
  LoaderCircle,
  RefreshCw,
  ShieldCheck,
  TerminalSquare,
} from "lucide-react";
import type {
  AnalyticsSnapshot,
  Condition,
  EventRecord,
  ExternalTool,
  QualityGate,
  RepositorySnapshot,
} from "../types";
import type { RemediationRun } from "../types/remediation";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import {
  QualityFindingsSummary,
  QualityGateCell,
  QualityGateStatusPill,
  WebReadinessSummary,
  projectQualityGateForTarget,
  projectQualityReadinessForTarget,
  qualityGateDisplayLabel,
} from "./QualityComponents";
import { QualityMaturityWithCacheSummary } from "./CacheDesignSummary";
import { targetScopeForRepository } from "../branchEvidence";
import { ProjectCompassDetail } from "./ProjectCompassDetail";
import { InstalledRuntimeParityDetail } from "./InstalledRuntimeParityDetail";
import { RepositoryAnalyticsPanel } from "./RepositoryAnalyticsPanel";
import { RepositoryInventoryPanels } from "./RepositoryInventoryPanels";
import { TelescopeSurface } from "./TelescopeSurface";

const EMPTY_REMEDIATION_RUN: RemediationRun = {
  schema_version: "unavailable",
  id: "unavailable",
  generated_at: "",
  status: "Unavailable",
  eligible_repository_ids: [],
  eligible_repository_paths: [],
  refresh_steps: [],
  excluded_repositories: [],
  closures: [],
  plans: [],
};

export function RepositoryDetailSurface({
  repository,
  backLabel = "Back to Portfolio",
  analytics,
  remediation = EMPTY_REMEDIATION_RUN,
  events = [],
  isRefreshing,
  onBack,
  onOpenWorkspace,
  onPrepareRepository,
  onTargetBranchChange,
  onLifecycleChange,
  onCondition,
  onOpenReport,
}: {
  repository: RepositorySnapshot;
  backLabel?: string;
  analytics: AnalyticsSnapshot;
  remediation?: RemediationRun;
  events?: EventRecord[];
  isRefreshing: boolean;
  onBack: () => void;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onTargetBranchChange: (targetBranch: string) => Promise<void>;
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (condition: Condition) => void;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const [detailView, setDetailView] = useState<"overview" | "telescope">(
    "overview",
  );
  const target = targetScopeForRepository(repository);
  const selectedTargetBranch = target.branch ?? "";
  const selectedTargetCommit = target.commit;
  const [pendingTargetBranch, setPendingTargetBranch] = useState<string | null>(
    null,
  );
  useEffect(() => {
    if (!isRefreshing) setPendingTargetBranch(null);
  }, [isRefreshing]);
  const displayedTargetBranch = pendingTargetBranch ?? selectedTargetBranch;
  const displayedTargetCommit = pendingTargetBranch
    ? repository.branches.find((branch) => branch.name === pendingTargetBranch)
        ?.last_commit
    : selectedTargetCommit;
  const targetEvidenceRefreshing = isRefreshing && pendingTargetBranch !== null;
  const targetReadiness = projectQualityReadinessForTarget(
    repository.quality.ci_readiness,
    repository.quality.gates,
    target.branch,
    target.commit,
  );
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
          {backLabel}
        </button>
        <StatusPill tone="slate" icon={<GitBranch size={11} />}>
          {repository.provider_state}
        </StatusPill>
      </div>
      <div className="repository-detail-view-switch" role="tablist">
        <button
          className={detailView === "overview" ? "active" : ""}
          type="button"
          role="tab"
          aria-selected={detailView === "overview"}
          onClick={() => setDetailView("overview")}
        >
          Overview
        </button>
        <button
          className={detailView === "telescope" ? "active" : ""}
          type="button"
          role="tab"
          aria-selected={detailView === "telescope"}
          onClick={() => setDetailView("telescope")}
        >
          Telescope
        </button>
      </div>
      {detailView === "telescope" ? (
        <TelescopeSurface
          repository={repository}
          remediation={remediation}
          events={events}
          onOpenWorkspace={onOpenWorkspace}
        />
      ) : (
        <>
          <div className="repository-detail-header">
            <div>
              <p className="eyebrow">Repository detail</p>
              <h1>{repository.name}</h1>
              <p className="repository-detail-path">{repository.path}</p>
              <p className="repository-detail-remote">
                {repository.remote_url ?? "No remote identity connected"}
              </p>
            </div>
            <StatusPill
              tone={repository.lifecycle === "Active" ? "mint" : "slate"}
            >
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
                value={displayedTargetBranch}
                disabled={targetBranches.length === 0 || isRefreshing}
                onChange={(event) => {
                  const branch = event.target.value;
                  setPendingTargetBranch(branch);
                  void onTargetBranchChange(branch);
                }}
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
              <button
                className="button button-secondary target-evidence-refresh-button"
                type="button"
                aria-label={`Refresh target evidence for ${repository.name}`}
                disabled={!displayedTargetBranch || isRefreshing}
                onClick={() => {
                  if (!displayedTargetBranch) return;
                  setPendingTargetBranch(displayedTargetBranch);
                  void onTargetBranchChange(displayedTargetBranch);
                }}
              >
                <RefreshCw size={12} />
                Refresh evidence
              </button>
              <small>
                {repository.target_branch_configured
                  ? `Pronto override · Git default: ${repository.default_branch ?? "Unknown"}`
                  : `Following Git default: ${repository.default_branch ?? "Unknown"}`}
              </small>
              <small className="target-branch-note">
                Selecting a branch or refreshing evidence checks existing target
                evidence first, then runs QR quality and fleet audits in a clean
                disposable worktree when the target head changed or matching
                evidence is unavailable; your active workspace is not switched.
              </small>
              <small>
                Evidence target: {displayedTargetBranch || "Unknown"}
                {displayedTargetCommit
                  ? ` @ ${displayedTargetCommit.slice(0, 8)}`
                  : " · head unavailable"}
              </small>
              {targetEvidenceRefreshing && (
                <div
                  className="target-evidence-loading"
                  role="status"
                  aria-live="polite"
                >
                  <LoaderCircle
                    size={15}
                    className="target-evidence-loading-icon spin"
                  />
                  <span>Refreshing evidence for {displayedTargetBranch}…</span>
                  <small>
                    Resolving the target head and checking existing evidence. A
                    QR audit runs only when the target head changed or matching
                    evidence is unavailable. Existing evidence is held until the
                    check completes.
                  </small>
                </div>
              )}
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
          {repository.quality.installed_runtime?.applicability ===
            "applicable" && (
            <InstalledRuntimeParityDetail
              runtime={repository.quality.installed_runtime}
            />
          )}
          <div className="drawer-section quality-detail-section">
            <div className="drawer-section-title">
              <div>
                <h3>Quality gates</h3>
                <small>
                  {repository.quality.ingestion_status} · evidence is read-only
                  and source-specific.
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
            {targetEvidenceRefreshing ? (
              <div className="target-evidence-loading" role="status">
                <LoaderCircle
                  size={15}
                  className="target-evidence-loading-icon spin"
                />
                <span>Ingesting {displayedTargetBranch} evidence…</span>
                <small>
                  Quality gates, maturity, findings, and remediation will update
                  together when the audit finishes.
                </small>
              </div>
            ) : (
              <>
                <div className="quality-detail-overview">
                  <QualityMaturityWithCacheSummary
                    maturity={repository.quality.maturity}
                    readiness={targetReadiness.readiness}
                    targetBranch={target.branch}
                    targetCommit={target.commit}
                    targetReadinessState={targetReadiness.state}
                    onOpenReport={onOpenReport}
                  />
                  <QualityFindingsSummary
                    findings={repository.quality.findings}
                    targetBranch={selectedTargetBranch || undefined}
                    targetCommit={selectedTargetCommit}
                    onOpenReport={onOpenReport}
                  />
                  <WebReadinessSummary
                    webReadiness={repository.quality.web_readiness}
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
                      targetBranch={target.branch}
                      targetCommit={target.commit}
                    />
                  ))}
                </div>
                {repository.quality.ingestion_message && (
                  <p className="quality-inline-empty">
                    <ShieldCheck size={14} />{" "}
                    {repository.quality.ingestion_message}
                  </p>
                )}
              </>
            )}
          </div>
          <RepositoryAnalyticsPanel
            repository={repository}
            analytics={analytics}
          />
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
            {targetEvidenceRefreshing ? (
              <div className="target-evidence-loading" role="status">
                <LoaderCircle
                  size={15}
                  className="target-evidence-loading-icon spin"
                />
                <span>Refreshing release remediation evidence…</span>
                <small>
                  The release trace will use the newly ingested target results.
                </small>
              </div>
            ) : repository.release_rule ? (
              <>
                <div className="repository-release-rule-meta">
                  <strong>{repository.release_rule.name}</strong>
                  <span>
                    {repository.release_rule.operator} evaluation ·{" "}
                    {repository.release_rule.required_quality_gates.length}{" "}
                    quality gate
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
                        const gateProjection = gate
                          ? projectQualityGateForTarget(
                              gate,
                              target.branch,
                              target.commit,
                            )
                          : undefined;
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
                              <strong>
                                {gate?.label ?? requirement.gate_id}
                              </strong>
                              <small>
                                {requirement.source} evidence ·{" "}
                                {requirement.policy ?? "block"}
                                {requirement.minimum_verification_level
                                  ? ` · ${requirement.minimum_verification_level.replaceAll("_", " ")}`
                                  : " · any level"}
                              </small>
                            </span>
                            {gateProjection &&
                            gateProjection.state === "unavailable" &&
                            (gate?.evidence.length ?? 0) > 0 ? (
                              <StatusPill tone="amber">
                                Target evidence unavailable
                              </StatusPill>
                            ) : gateProjection?.state === "stale" ? (
                              <StatusPill tone="amber">
                                Stale branch evidence
                              </StatusPill>
                            ) : gateProjection?.state === "unscoped" ? (
                              <StatusPill tone="amber">
                                Unscoped evidence
                              </StatusPill>
                            ) : configuredWithoutEvidence ? (
                              <StatusPill tone="slate">
                                Awaiting evidence
                              </StatusPill>
                            ) : gateProjection ? (
                              <QualityGateStatusPill
                                status={gateProjection.gate.status}
                                freshness={gateProjection.gate.freshness}
                              />
                            ) : (
                              <StatusPill tone="slate">
                                Not configured
                              </StatusPill>
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
          <RepositoryInventoryPanels
            repository={repository}
            onOpenWorkspace={onOpenWorkspace}
            onPrepareRepository={onPrepareRepository}
            onCondition={onCondition}
          />
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
        </>
      )}
    </section>
  );
}
