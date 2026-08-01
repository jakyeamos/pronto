import { type ReactElement, useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Clock3,
  ExternalLink,
  FileCheck2,
  SkipForward,
} from "lucide-react";
import type {
  RemediationActionStatus,
  RemediationRun,
  RepositorySnapshot,
} from "../types";
import { StatusPill } from "./ConsolePrimitives";
import {
  formatRemediationTime,
  RemediationActionRow,
  remediationStatusLabel,
  remediationStatusTone,
} from "./RemediationActionRow";
import {
  remediationMaturityPolicySummary,
  RemediationMaturityPolicyCriteria,
  RemediationMaturityPolicyMeta,
} from "./RemediationMaturityPolicy";
import { RemediationRunOverview } from "./RemediationRunOverview";

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
  const closures = run.closures ?? [];
  const selectedPlan = useMemo(
    () => run.plans.find((plan) => plan.id === selectedPlanId) ?? run.plans[0],
    [run.plans, selectedPlanId],
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
      <RemediationRunOverview
        run={run}
        isRefreshing={isRefreshing}
        onRefresh={onRefresh}
        onExport={onExport}
      />

      {run.refresh_steps.length > 0 && (
        <section className="surface-panel remediation-refresh-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Run ledger</p>
              <h2>What was refreshed</h2>
            </div>
            <StatusPill tone={remediationStatusTone(run.status)}>
              {run.status}
            </StatusPill>
          </div>
          <div className="remediation-refresh-steps">
            {run.refresh_steps.map((step) => (
              <div className="remediation-refresh-step" key={step.id}>
                <div
                  className={`remediation-step-icon remediation-step-icon-${remediationStatusTone(step.status)}`}
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
                    {formatRemediationTime(
                      step.completed_at ?? step.started_at,
                    )}
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

      {closures.length > 0 && (
        <section className="surface-panel remediation-closure-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Closure ledger</p>
              <h2>Repositories removed from the active queue</h2>
            </div>
            <StatusPill tone="mint">{closures.length} retained</StatusPill>
          </div>
          <div className="remediation-closure-list">
            {closures.map((closure) => (
              <div className="remediation-closure" key={closure.id}>
                <div>
                  <strong>{closure.repository_name}</strong>
                  <span>{closure.summary}</span>
                </div>
                <small>
                  {remediationStatusLabel(closure.target_state)} ·{" "}
                  {remediationStatusLabel(closure.goal_source)} ·{" "}
                  {closure.disposition} ·{" "}
                  {formatRemediationTime(closure.closed_at)}
                  {closure.maturity_policy &&
                    ` · ${remediationMaturityPolicySummary(closure.maturity_policy)}`}
                </small>
              </div>
            ))}
          </div>
        </section>
      )}

      <div className="remediation-plan-layout">
        <section className="surface-panel remediation-plan-list-panel">
          <div className="surface-heading compact-heading">
            <div>
              <p className="eyebrow">Ranked fleet queue</p>
              <h2>Active repository remediation</h2>
            </div>
            <span>{run.plans.length} active</span>
          </div>
          {run.plans.length === 0 ? (
            <div className="surface-empty">
              <CheckCircle2 size={18} />
              <span>
                No active remediation remains. Refresh scoped evidence before
                treating this as current.
              </span>
            </div>
          ) : (
            <div className="remediation-plan-list">
              {run.plans.map((plan, index) => (
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
                    <strong>
                      #{index + 1} · {plan.repository_name}
                    </strong>
                    <span>
                      {plan.goal.label} · {plan.current_stage} ·{" "}
                      {plan.actions.length} actions
                    </span>
                  </div>
                  <div className="remediation-plan-row-meta">
                    <StatusPill tone={remediationStatusTone(plan.status)}>
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
                  <StatusPill tone={remediationStatusTone(selectedPlan.status)}>
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
              <div
                className={`remediation-goal-block${
                  selectedPlan.goal.source === "repository_contract"
                    ? ""
                    : " remediation-goal-block-inferred"
                }`}
              >
                <div className="remediation-goal-heading">
                  <div>
                    <span>Target outcome</span>
                    <strong>{selectedPlan.goal.label}</strong>
                  </div>
                  <StatusPill
                    tone={
                      selectedPlan.goal.source === "repository_contract"
                        ? "mint"
                        : "amber"
                    }
                  >
                    {remediationStatusLabel(selectedPlan.goal.source)}
                  </StatusPill>
                </div>
                <p>{selectedPlan.goal.reason}</p>
                {selectedPlan.goal.error && (
                  <p className="remediation-goal-error">
                    {selectedPlan.goal.error}
                  </p>
                )}
                <div className="remediation-goal-meta">
                  <span>
                    Evidence fresh for {selectedPlan.goal.evidence_max_age_days}{" "}
                    days
                  </span>
                  <span>
                    {selectedPlan.goal.required_gate_ids.length} required gates
                  </span>
                  <code>{selectedPlan.goal.contract_path}</code>
                  {selectedPlan.goal.maturity_policy && (
                    <RemediationMaturityPolicyMeta
                      policy={selectedPlan.goal.maturity_policy}
                    />
                  )}
                </div>
                <details>
                  <summary>Goal-specific closure contract</summary>
                  <ul>
                    {selectedPlan.goal.closure_criteria.map((criterion) => (
                      <li key={criterion}>{criterion}</li>
                    ))}
                    {selectedPlan.goal.maturity_policy && (
                      <RemediationMaturityPolicyCriteria
                        policy={selectedPlan.goal.maturity_policy}
                      />
                    )}
                  </ul>
                </details>
              </div>
              <div className="remediation-coverage-block">
                <div className="remediation-coverage-heading">
                  <div>
                    <span>UI tracking coverage</span>
                    <strong>
                      {(selectedPlan.coverage ?? []).length} repository surfaces
                    </strong>
                  </div>
                  <small>
                    Every repo-level surface visible in Pronto is classified
                    here.
                  </small>
                </div>
                <div className="remediation-coverage-grid">
                  {(selectedPlan.coverage ?? []).map((entry) => (
                    <div
                      className="remediation-coverage-item"
                      key={entry.surface}
                    >
                      <div>
                        <strong>{entry.label}</strong>
                        <span>{entry.detail}</span>
                        {entry.action_ids.length > 0 && (
                          <small>
                            {entry.action_ids.length} linked action
                            {entry.action_ids.length === 1 ? "" : "s"}
                          </small>
                        )}
                      </div>
                      <StatusPill tone={remediationStatusTone(entry.status)}>
                        {remediationStatusLabel(entry.status)}
                      </StatusPill>
                    </div>
                  ))}
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
                    <StatusPill tone={remediationStatusTone(track.status)}>
                      {track.status}
                    </StatusPill>
                  </div>
                ))}
              </div>
              <div className="remediation-actions-list">
                {selectedPlan.actions.map((action) => (
                  <RemediationActionRow
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
