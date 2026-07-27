import { useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Clock3,
  Download,
  ExternalLink,
  FileCheck2,
  GitBranch,
  RefreshCw,
  ShieldAlert,
  SkipForward,
} from "lucide-react";
import type {
  RemediationAction,
  RemediationActionStatus,
  RemediationRun,
  RepositorySnapshot,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

const actionStatuses: RemediationActionStatus[] = [
  "open",
  "in_progress",
  "blocked",
  "deferred",
  "verified",
];

function toneForStatus(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized === "completed" || normalized === "verified") return "mint";
  if (normalized === "in_progress" || normalized === "in progress")
    return "blue";
  if (normalized === "blocked" || normalized === "failed") return "red";
  if (normalized === "deferred" || normalized === "partial") return "amber";
  if (normalized === "open" || normalized === "pending") return "coral";
  return "slate";
}

function labelForDomain(domain: string): string {
  return domain
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function actionStatusLabel(status: string): string {
  return status.replace("_", " ");
}

function formatNullableTime(value?: string | null): string {
  return value ? formatTime(value) : "Not recorded";
}

function actionEvidenceIsFresh(action: RemediationAction): boolean {
  return action.evidence.some(
    (item) => item.freshness.toLowerCase() === "fresh",
  );
}

function ActionRow({
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
              {actionStatusLabel(status)}
            </option>
          ))}
        </select>
      </div>
      <div className="remediation-action-status-line">
        <StatusPill tone={toneForStatus(action.status)}>
          {actionStatusLabel(action.status)}
        </StatusPill>
        <StatusPill tone={actionEvidenceIsFresh(action) ? "mint" : "amber"}>
          {actionEvidenceIsFresh(action)
            ? "Fresh evidence"
            : "Evidence needs refresh"}
        </StatusPill>
        <span>Updated {formatNullableTime(action.updated_at)}</span>
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
                      <StatusPill tone={toneForStatus(item.status)}>
                        {item.status}
                      </StatusPill>
                    </div>
                    <span>{item.detail}</span>
                    <small>
                      {item.freshness} · {formatNullableTime(item.observed_at)}
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

export function RemediationSurface({
  run,
  repositories,
  isRefreshing,
  onRefresh,
  onExport,
  onUpdateStatus,
  onOpenRepository,
}: {
  run: RemediationRun;
  repositories: RepositorySnapshot[];
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onExport: () => Promise<void>;
  onUpdateStatus: (
    actionId: string,
    status: RemediationActionStatus,
  ) => Promise<void>;
  onOpenRepository: (repository: RepositorySnapshot) => void;
}): ReactElement {
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(
    run.plans[0]?.id ?? null,
  );
  const selectedPlan = useMemo(
    () => run.plans.find((plan) => plan.id === selectedPlanId) ?? run.plans[0],
    [run.plans, selectedPlanId],
  );
  const openActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status !== "verified").length,
    0,
  );
  const verifiedActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status === "verified").length,
    0,
  );
  const blockedActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status === "blocked").length,
    0,
  );

  useEffect(() => {
    if (
      selectedPlanId &&
      !run.plans.some((plan) => plan.id === selectedPlanId)
    ) {
      setSelectedPlanId(run.plans[0]?.id ?? null);
    }
  }, [run.plans, selectedPlanId]);

  const selectedRepository = selectedPlan
    ? repositories.find(
        (repository) => repository.id === selectedPlan.repository_id,
      )
    : undefined;

  return (
    <div className="remediation-surface">
      <section className="remediation-overview-grid">
        <div className="remediation-overview-card remediation-overview-card-accent">
          <span>Refresh state</span>
          <strong>{run.status.replace("_", " ")}</strong>
          <small>
            {run.source_refresh_id
              ? `Run ${run.source_refresh_id}`
              : "No full refresh recorded yet"}
          </small>
          <FileCheck2 size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Eligible repositories</span>
          <strong>{run.plans.length}</strong>
          <small>{run.eligible_repository_paths.length} bounded paths</small>
          <GitBranch size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Actions to work</span>
          <strong>{openActionCount}</strong>
          <small>
            {verifiedActionCount} verified · {blockedActionCount} blocked
          </small>
          <CircleAlert size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Excluded in-progress work</span>
          <strong>{run.excluded_repositories.length}</strong>
          <small>Soundscape and Tenure stay out of the plan</small>
          <ShieldAlert size={18} />
        </div>
      </section>

      <section className="surface-panel remediation-control-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Evidence refresh contract</p>
            <h2>Refresh everything before acting</h2>
            <p>
              QR doctor gates the run, then Pronto refreshes local state, QR
              artifacts, replay/report/feed evidence, provider context, and
              per-repository plans.
            </p>
          </div>
          <div className="remediation-control-actions">
            <button
              className="button button-primary"
              type="button"
              disabled={isRefreshing}
              onClick={() => void onRefresh()}
            >
              <RefreshCw
                size={15}
                className={isRefreshing ? "spin" : undefined}
              />
              {isRefreshing ? "Refreshing…" : "Run full refresh"}
            </button>
            <button
              className="button button-secondary"
              type="button"
              disabled={isRefreshing || run.plans.length === 0}
              onClick={() => void onExport()}
            >
              <Download size={15} />
              Export plans
            </button>
          </div>
        </div>
        <div className="remediation-run-meta">
          <span>Generated {formatNullableTime(run.generated_at)}</span>
          <span>{run.eligible_repository_ids.length} eligible IDs</span>
          {run.message && (
            <span className="remediation-run-message">{run.message}</span>
          )}
        </div>
      </section>

      {run.refresh_steps.length > 0 && (
        <section className="surface-panel remediation-refresh-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Run ledger</p>
              <h2>What was refreshed</h2>
            </div>
            <StatusPill tone={toneForStatus(run.status)}>
              {run.status}
            </StatusPill>
          </div>
          <div className="remediation-refresh-steps">
            {run.refresh_steps.map((step) => (
              <div className="remediation-refresh-step" key={step.id}>
                <div
                  className={`remediation-step-icon remediation-step-icon-${toneForStatus(step.status)}`}
                >
                  {step.status === "completed" ? (
                    <CheckCircle2 size={15} />
                  ) : step.status === "blocked" ? (
                    <CircleAlert size={15} />
                  ) : step.status === "in_progress" ? (
                    <Clock3 size={15} />
                  ) : (
                    <SkipForward size={15} />
                  )}
                </div>
                <div>
                  <strong>{step.label}</strong>
                  <span>{step.detail}</span>
                  <small>
                    {step.status} ·{" "}
                    {formatNullableTime(step.completed_at ?? step.started_at)}
                  </small>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {run.excluded_repositories.length > 0 && (
        <section className="surface-panel remediation-exclusion-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Scope boundary</p>
              <h2>In-progress repositories excluded</h2>
            </div>
            <StatusPill tone="amber">Not scored here</StatusPill>
          </div>
          <div className="remediation-exclusion-list">
            {run.excluded_repositories.map((exclusion) => (
              <div
                className="remediation-exclusion"
                key={exclusion.repository_id}
              >
                <div>
                  <strong>{exclusion.repository_name}</strong>
                  <span>{exclusion.repository_path}</span>
                </div>
                <small>{exclusion.reason}</small>
              </div>
            ))}
          </div>
        </section>
      )}

      <div className="remediation-plan-layout">
        <section className="surface-panel remediation-plan-list-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Fleet plans</p>
              <h2>One plan per eligible repository</h2>
            </div>
            <span>{run.plans.length} plans</span>
          </div>
          {run.plans.length === 0 ? (
            <div className="surface-empty">
              <FileCheck2 size={18} />
              <span>Run the full refresh to generate repository plans.</span>
            </div>
          ) : (
            <div className="remediation-plan-list">
              {run.plans.map((plan) => (
                <button
                  className={`remediation-plan-row${
                    selectedPlan?.id === plan.id
                      ? " remediation-plan-row-selected"
                      : ""
                  }`}
                  type="button"
                  key={plan.id}
                  onClick={() => setSelectedPlanId(plan.id)}
                >
                  <div>
                    <strong>{plan.repository_name}</strong>
                    <span>
                      {plan.current_stage} · {plan.actions.length} actions
                    </span>
                  </div>
                  <div className="remediation-plan-row-meta">
                    <StatusPill tone={toneForStatus(plan.status)}>
                      {plan.status}
                    </StatusPill>
                    <strong>{Math.round(plan.progress.percentage)}%</strong>
                    <ChevronRight size={16} />
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>

        <section className="surface-panel remediation-plan-detail-panel">
          {selectedPlan ? (
            <>
              <div className="surface-heading remediation-detail-heading">
                <div>
                  <p className="eyebrow">Repository remediation plan</p>
                  <h2>{selectedPlan.repository_name}</h2>
                  <p>{selectedPlan.repository_path}</p>
                </div>
                <div className="remediation-detail-actions">
                  <StatusPill tone={toneForStatus(selectedPlan.status)}>
                    {selectedPlan.status}
                  </StatusPill>
                  {selectedRepository && (
                    <button
                      className="button button-secondary"
                      type="button"
                      onClick={() => onOpenRepository(selectedRepository)}
                    >
                      <ExternalLink size={14} />
                      Open repository
                    </button>
                  )}
                </div>
              </div>
              <div className="remediation-progress-block">
                <div>
                  <span>Weighted completion</span>
                  <strong>
                    {Math.round(selectedPlan.progress.percentage)}% ·{" "}
                    {selectedPlan.progress.verified_weight}/
                    {selectedPlan.progress.total_weight} points
                  </strong>
                </div>
                <div className="remediation-progress-track">
                  <span
                    style={{
                      width: `${Math.min(100, Math.max(0, selectedPlan.progress.percentage))}%`,
                    }}
                  />
                </div>
                <small>
                  Current stage: {selectedPlan.current_stage}
                  {selectedPlan.progress.deferred_weight > 0
                    ? ` · ${selectedPlan.progress.deferred_weight} points deferred`
                    : ""}
                </small>
              </div>
              <div className="remediation-track-list">
                {selectedPlan.tracks.map((track) => (
                  <div className="remediation-track" key={track.domain}>
                    <div>
                      <strong>{track.label}</strong>
                      <span>{track.action_ids.length} actions</span>
                    </div>
                    <StatusPill tone={toneForStatus(track.status)}>
                      {track.status}
                    </StatusPill>
                  </div>
                ))}
              </div>
              <div className="remediation-actions-list">
                {selectedPlan.actions.map((action) => (
                  <ActionRow
                    action={action}
                    key={action.id}
                    onUpdateStatus={onUpdateStatus}
                  />
                ))}
              </div>
            </>
          ) : (
            <div className="surface-empty remediation-detail-empty">
              <FileCheck2 size={18} />
              <span>Select a repository plan to inspect its actions.</span>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
