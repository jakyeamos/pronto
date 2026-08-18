import { useMemo, useState, type ReactElement } from "react";
import {
  AlertTriangle,
  Bot,
  ExternalLink,
  GitBranch,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import type {
  CiRunSnapshot,
  ProviderStatus,
  RemoteRepositorySnapshot,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

function failureRuns(repository: RemoteRepositorySnapshot): CiRunSnapshot[] {
  return repository.ci_runs.filter((run) => Boolean(run.failure_summary));
}

function runKey(
  repository: RemoteRepositorySnapshot,
  run: CiRunSnapshot,
): string {
  return `${repository.full_name}:${run.id}:${run.run_attempt}:${run.failure_signature ?? "unknown"}`;
}

function conclusionTone(run: CiRunSnapshot): string {
  return run.conclusion === "cancelled" ? "amber" : "coral";
}

export function CiTrackerSurface({
  status,
  repositories,
  isRefreshing,
  onRefresh,
  onStartCodex,
}: {
  status: ProviderStatus;
  repositories: RemoteRepositorySnapshot[];
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onStartCodex: (
    repository: RemoteRepositorySnapshot,
    run: CiRunSnapshot,
  ) => Promise<void>;
}): ReactElement {
  const [startedKeys, setStartedKeys] = useState<Set<string>>(() => new Set());
  const [startingKey, setStartingKey] = useState<string | null>(null);
  const [handoffError, setHandoffError] = useState<string | null>(null);
  const failingRepositories = useMemo(
    () =>
      repositories
        .map((repository) => ({ repository, runs: failureRuns(repository) }))
        .filter(({ runs }) => runs.length > 0),
    [repositories],
  );
  const failureCount = failingRepositories.reduce(
    (total, item) => total + item.runs.length,
    0,
  );

  const startCodex = async (
    repository: RemoteRepositorySnapshot,
    run: CiRunSnapshot,
  ): Promise<void> => {
    const key = runKey(repository, run);
    setStartingKey(key);
    setHandoffError(null);
    try {
      await onStartCodex(repository, run);
      setStartedKeys((current) => new Set(current).add(key));
    } catch (caught) {
      setHandoffError(
        caught instanceof Error
          ? caught.message
          : "Pronto could not start the Codex CI handoff.",
      );
    } finally {
      setStartingKey(null);
    }
  };

  return (
    <div className="ci-tracker-layout">
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">GitHub → Pronto → Codex</p>
            <h2>CI failure tracker</h2>
            <p>
              Failed workflow runs stay in GitHub. This view keeps a bounded
              local projection of the failure reason and lets you start a
              read-only Codex diagnosis when the prompt artifact is available.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh CI"}
          </button>
        </div>
        <div className="ci-tracker-summary">
          <div className="ci-tracker-summary-icon">
            <AlertTriangle size={18} />
          </div>
          <div>
            <div className="provider-status-heading">
              <strong>
                {failureCount === 0
                  ? "No failed runs in the current snapshot"
                  : `${failureCount} failed run${failureCount === 1 ? "" : "s"} need review`}
              </strong>
              <StatusPill tone={status.state === "Ready" ? "mint" : "amber"}>
                {status.state}
              </StatusPill>
            </div>
            <p>{status.message}</p>
            <small>
              {status.last_refresh_at
                ? `Last refresh ${formatTime(status.last_refresh_at)}`
                : "No successful GitHub refresh yet"}
            </small>
          </div>
        </div>
        {handoffError && (
          <div className="ci-tracker-error" role="alert">
            <AlertTriangle size={15} />
            <span>{handoffError}</span>
          </div>
        )}
      </section>

      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Actionable runs</p>
            <h2>{failingRepositories.length} repositories with failures</h2>
            <p>
              Matrix jobs remain grouped under one workflow run, so one Codex
              task can inspect the complete failure context.
            </p>
          </div>
        </div>
        {failingRepositories.length === 0 ? (
          <div className="surface-empty">
            <ShieldCheck size={18} />
            <span>
              Nothing failed in the stored GitHub run snapshot. Refresh CI to
              check for new workflow results.
            </span>
          </div>
        ) : (
          <div className="ci-tracker-repository-list">
            {failingRepositories.map(({ repository, runs }) => (
              <article className="ci-tracker-repository" key={repository.id}>
                <div className="ci-tracker-repository-heading">
                  <div>
                    <div className="remote-repository-heading">
                      <GitBranch size={14} />
                      <strong>{repository.full_name}</strong>
                      <StatusPill
                        tone={
                          repository.locality === "Local and remote"
                            ? "mint"
                            : "amber"
                        }
                      >
                        {repository.locality}
                      </StatusPill>
                    </div>
                    <small>
                      {repository.default_branch || "default branch unknown"} ·{" "}
                      {runs.length} failed run{runs.length === 1 ? "" : "s"}
                    </small>
                  </div>
                  <a
                    className="icon-button"
                    href={repository.html_url}
                    target="_blank"
                    rel="noreferrer"
                    aria-label={`Open ${repository.full_name} on GitHub`}
                  >
                    <ExternalLink size={14} />
                  </a>
                </div>
                <div className="ci-tracker-run-list">
                  {runs.map((run) => {
                    const key = runKey(repository, run);
                    const started = startedKeys.has(key);
                    const starting = startingKey === key;
                    const localCheckout =
                      repository.locality === "Local and remote";
                    const hasArtifact = Boolean(run.prompt_artifact);
                    const canStart = localCheckout && hasArtifact && !started;
                    return (
                      <div className="ci-tracker-run" key={key}>
                        <div className="ci-tracker-run-main">
                          <div className="ci-tracker-run-title">
                            <strong>{run.workflow_name}</strong>
                            <StatusPill tone={conclusionTone(run)}>
                              {run.conclusion || run.status}
                            </StatusPill>
                            {run.is_fork && (
                              <StatusPill tone="amber">
                                Fork · diagnosis only
                              </StatusPill>
                            )}
                          </div>
                          <p>
                            {run.failure_summary ||
                              "Failure details unavailable"}
                          </p>
                          <small>
                            Run #{run.run_number} · attempt {run.run_attempt} ·{" "}
                            {run.head_branch || "detached head"} · updated{" "}
                            {formatTime(run.updated_at)}
                          </small>
                          {run.jobs.filter(
                            (job) =>
                              job.conclusion && job.conclusion !== "success",
                          ).length > 0 && (
                            <div className="ci-tracker-job-list">
                              {run.jobs
                                .filter(
                                  (job) =>
                                    job.conclusion &&
                                    job.conclusion !== "success",
                                )
                                .slice(0, 3)
                                .map((job) => (
                                  <span key={job.id}>
                                    {job.name}
                                    {job.failed_steps.length > 0
                                      ? ` · ${job.failed_steps.join(", ")}`
                                      : ""}
                                  </span>
                                ))}
                            </div>
                          )}
                        </div>
                        <div className="ci-tracker-run-actions">
                          <a
                            className="button button-quiet"
                            href={run.html_url}
                            target="_blank"
                            rel="noreferrer"
                          >
                            View run
                          </a>
                          <button
                            className="button button-primary"
                            type="button"
                            disabled={!canStart || starting}
                            title={
                              !localCheckout
                                ? "A registered local checkout is required"
                                : !hasArtifact
                                  ? "The bridge workflow did not publish a prompt artifact"
                                  : undefined
                            }
                            onClick={() => void startCodex(repository, run)}
                          >
                            <Bot size={14} />
                            {started
                              ? "Started"
                              : starting
                                ? "Starting…"
                                : "Diagnose with Codex"}
                          </button>
                        </div>
                        {!started && !hasArtifact && (
                          <span className="ci-tracker-action-note">
                            Prompt artifact unavailable
                          </span>
                        )}
                        {!started && hasArtifact && !localCheckout && (
                          <span className="ci-tracker-action-note">
                            Remote-only repository · diagnosis stays in GitHub
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
