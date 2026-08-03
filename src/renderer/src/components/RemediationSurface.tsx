import { type ReactElement, useEffect, useMemo, useRef, useState } from "react";
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Clock3,
  ExternalLink,
  FileCheck2,
  SkipForward,
  X,
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

function remediationActionSummary(
  actions: RemediationRun["plans"][number]["actions"],
): string {
  const active = actions.filter((action) =>
    ["open", "in_progress", "blocked"].includes(action.status),
  ).length;
  const verified = actions.filter(
    (action) => action.status === "verified",
  ).length;
  const parts = [`${active} active`];
  if (verified > 0) parts.push(`${verified} verified`);
  return parts.join(" · ");
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
  const [detailRevealed, setDetailRevealed] = useState(false);
  const detailPanelRef = useRef<HTMLElement>(null);
  const planRowRefs = useRef(new Map<string, HTMLButtonElement>());
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
      setDetailRevealed(false);
    }
  }, [run.plans, selectedPlanId]);

  useEffect(() => {
    if (!detailRevealed) return undefined;
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      setDetailRevealed(false);
      planRowRefs.current.get(selectedPlanId ?? "")?.focus();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [detailRevealed, selectedPlanId]);

  const revealPlan = (planId: string): void => {
    setSelectedPlanId(planId);
    setDetailRevealed(true);
    window.requestAnimationFrame(() => detailPanelRef.current?.focus());
  };

  const closeDetail = (): void => {
    setDetailRevealed(false);
    planRowRefs.current.get(selectedPlanId ?? "")?.focus();
  };

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
                  ref={(element) => {
                    if (element) planRowRefs.current.set(plan.id, element);
                    else planRowRefs.current.delete(plan.id);
                  }}
                  aria-controls="remediation-plan-detail"
                  aria-current={selectedPlan?.id === plan.id}
                  onClick={() => revealPlan(plan.id)}
                >
                  <div>
                    <strong>
                      #{index + 1} · {plan.repository_name}
                    </strong>
                    <span>
                      {plan.goal.label} · {plan.current_stage} ·{" "}
                      {remediationActionSummary(plan.actions)}
                    </span>
                  </div>
                  <div className="remediation-plan-row-meta">
                    <StatusPill tone={remediationStatusTone(plan.status)}>
                      {plan.status}
                    </StatusPill>
                    <strong>{Math.round(plan.progress.percentage)}%</strong>
                    <ChevronRight size={16} aria-hidden="true" />
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>

        {detailRevealed && (
          <button
            className="remediation-plan-detail-scrim"
            type="button"
            aria-label="Close remediation detail"
            onClick={closeDetail}
          />
        )}
        <section
          className={`surface-panel remediation-plan-detail-panel${
            detailRevealed ? " remediation-plan-detail-panel-open" : ""
          }`}
          id="remediation-plan-detail"
          ref={detailPanelRef}
          tabIndex={-1}
          aria-labelledby={selectedPlan ? "remediation-plan-title" : undefined}
        >
          {selectedPlan ? (
            <>
              <div className="surface-heading remediation-detail-heading">
                <div>
                  <p className="eyebrow">Repository remediation plan</p>
                  <h2 id="remediation-plan-title">
                    {selectedPlan.repository_name}
                  </h2>
                  <p>{selectedPlan.repository_path}</p>
                </div>
                <div className="remediation-detail-actions">
                  <button
                    className="icon-button remediation-detail-close"
                    type="button"
                    aria-label="Close remediation detail"
                    onClick={closeDetail}
                  >
                    <X size={15} />
                  </button>
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
              {selectedPlan.explanation && (
                <section
                  className="remediation-explanation-block"
                  aria-label="Remediation path"
                >
                  <div className="remediation-explanation-heading">
                    <div>
                      <span>Remediation path</span>
                      <strong>
                        {selectedPlan.explanation.phases.length} phase
                        {selectedPlan.explanation.phases.length === 1
                          ? ""
                          : "s"}{" "}
                        remaining
                      </strong>
                    </div>
                    <small>{selectedPlan.explanation.summary}</small>
                  </div>
                  <div className="remediation-phase-list">
                    {selectedPlan.explanation.phases.map((phase, index) => (
                      <article className="remediation-phase" key={phase.id}>
                        <div className="remediation-phase-heading">
                          <div>
                            <span>Phase {index + 1}</span>
                            <h3>{phase.title}</h3>
                          </div>
                          <StatusPill
                            tone={remediationStatusTone(phase.status)}
                          >
                            {remediationStatusLabel(phase.status)}
                          </StatusPill>
                        </div>
                        <p>{phase.summary}</p>
                        <ol className="remediation-phase-steps">
                          {phase.steps.map((step) => (
                            <li key={step.action_id}>
                              <div>
                                <strong>{step.title}</strong>
                                <span>{step.summary}</span>
                              </div>
                              <div className="remediation-phase-step-meta">
                                <span>{step.priority}</span>
                                <StatusPill
                                  tone={remediationStatusTone(step.status)}
                                >
                                  {remediationStatusLabel(step.status)}
                                </StatusPill>
                              </div>
                              {step.completion_criteria.length > 0 && (
                                <details>
                                  <summary>What done means</summary>
                                  <ul>
                                    {step.completion_criteria.map(
                                      (criterion) => (
                                        <li key={criterion}>{criterion}</li>
                                      ),
                                    )}
                                  </ul>
                                </details>
                              )}
                            </li>
                          ))}
                        </ol>
                        <div className="remediation-phase-exit">
                          <CheckCircle2 size={13} />
                          <span>{phase.completion_criterion}</span>
                        </div>
                      </article>
                    ))}
                  </div>
                  <div className="remediation-explanation-footnotes">
                    <details>
                      <summary>
                        Already healthy ·{" "}
                        {selectedPlan.explanation.healthy_surfaces.length}{" "}
                        surfaces
                      </summary>
                      <div className="remediation-healthy-list">
                        {selectedPlan.explanation.healthy_surfaces.map(
                          (surface) => (
                            <div key={surface.surface}>
                              <CheckCircle2 size={12} />
                              <span>
                                <strong>{surface.label}</strong>
                                {surface.detail}
                              </span>
                            </div>
                          ),
                        )}
                      </div>
                    </details>
                    <details>
                      <summary>What closes this plan</summary>
                      <ul>
                        {selectedPlan.explanation.closure_requirements.map(
                          (requirement) => (
                            <li key={requirement}>{requirement}</li>
                          ),
                        )}
                      </ul>
                    </details>
                    <small>{selectedPlan.explanation.authority}</small>
                  </div>
                </section>
              )}
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
