import { useMemo, useState, type ReactElement } from "react";
import {
  AlertTriangle,
  Bot,
  ChevronDown,
  ChevronRight,
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

interface FailureIssue {
  run: CiRunSnapshot;
  suppressedRunCount: number;
}

function runTimestamp(run: CiRunSnapshot): number {
  const timestamp = Date.parse(run.updated_at ?? run.created_at ?? "");
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

function failureIssueKey(run: CiRunSnapshot): string {
  return (
    run.failure_signature ??
    [run.workflow_path ?? run.workflow_name, run.failure_summary].join("|")
  );
}

function failureIssues(repository: RemoteRepositorySnapshot): FailureIssue[] {
  const runs = failureRuns(repository).sort(
    (left, right) =>
      runTimestamp(right) - runTimestamp(left) || right.id - left.id,
  );
  const issues: FailureIssue[] = [];
  for (const run of runs) {
    const previous = issues.at(-1);
    if (previous && failureIssueKey(previous.run) === failureIssueKey(run)) {
      previous.suppressedRunCount += 1;
      continue;
    }
    issues.push({ run, suppressedRunCount: 0 });
  }
  return issues;
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
  onOpenExternalUrl,
  onStartCodex,
}: {
  status: ProviderStatus;
  repositories: RemoteRepositorySnapshot[];
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onOpenExternalUrl: (url: string) => Promise<void>;
  onStartCodex: (
    repository: RemoteRepositorySnapshot,
    run: CiRunSnapshot,
  ) => Promise<void>;
}): ReactElement {
  const [startedKeys, setStartedKeys] = useState<Set<string>>(() => new Set());
  const [startingKey, setStartingKey] = useState<string | null>(null);
  const [handoffError, setHandoffError] = useState<string | null>(null);
  const [collapsedRepositories, setCollapsedRepositories] = useState<
    Set<string>
  >(() => new Set());
  const failingRepositories = useMemo(
    () =>
      repositories
        .map((repository) => {
          const issues = failureIssues(repository);
          return {
            repository,
            issues,
            suppressedRunCount: issues.reduce(
              (total, issue) => total + issue.suppressedRunCount,
              0,
            ),
          };
        })
        .filter(({ issues }) => issues.length > 0),
    [repositories],
  );
  const failureCount = failingRepositories.reduce(
    (total, item) => total + item.issues.length,
    0,
  );
  const suppressedRunCount = failingRepositories.reduce(
    (total, item) => total + item.suppressedRunCount,
    0,
  );
  const allRepositoriesCollapsed =
    failingRepositories.length > 0 &&
    failingRepositories.every(({ repository }) =>
      collapsedRepositories.has(repository.id),
    );

  const toggleAllRepositories = (): void => {
    setCollapsedRepositories(() => {
      if (allRepositoriesCollapsed) return new Set();
      return new Set(
        failingRepositories.map(({ repository }) => repository.id),
      );
    });
  };

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
              read-only Codex diagnosis for repositories with a registered local
              checkout.
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
                  : `${failureCount} failure issue${failureCount === 1 ? "" : "s"} need review`}
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
            {suppressedRunCount > 0 && (
              <small className="ci-tracker-dedup-note">
                {suppressedRunCount} repeated run
                {suppressedRunCount === 1 ? "" : "s"} hidden because the failure
                matched the previous run.
              </small>
            )}
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
              task can inspect the complete failure context. Consecutive runs
              with the same stable failure signature appear once, with the
              latest run retained for diagnosis.
            </p>
          </div>
          {failingRepositories.length > 0 && (
            <button
              className="button button-quiet"
              type="button"
              onClick={toggleAllRepositories}
              aria-label={`${allRepositoriesCollapsed ? "Expand" : "Collapse"} all repositories`}
            >
              {allRepositoriesCollapsed ? "Expand all" : "Collapse all"}
            </button>
          )}
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
            {failingRepositories.map(
              ({
                repository,
                issues,
                suppressedRunCount: repositorySuppressedRunCount,
              }) => {
                const collapsed = collapsedRepositories.has(repository.id);
                const runListId = `ci-tracker-runs-${repository.id.replace(
                  /[^a-zA-Z0-9_-]/g,
                  "-",
                )}`;
                return (
                  <article
                    className="ci-tracker-repository"
                    key={repository.id}
                  >
                    <div className="ci-tracker-repository-heading">
                      <button
                        className="ci-tracker-repository-toggle"
                        type="button"
                        aria-expanded={!collapsed}
                        aria-controls={runListId}
                        aria-label={`${collapsed ? "Expand" : "Collapse"} ${repository.full_name}`}
                        onClick={() =>
                          setCollapsedRepositories((current) => {
                            const next = new Set(current);
                            if (next.has(repository.id)) {
                              next.delete(repository.id);
                            } else {
                              next.add(repository.id);
                            }
                            return next;
                          })
                        }
                      >
                        <span
                          className="ci-tracker-repository-toggle-icon"
                          aria-hidden="true"
                        >
                          {collapsed ? (
                            <ChevronRight size={15} />
                          ) : (
                            <ChevronDown size={15} />
                          )}
                        </span>
                        <span className="ci-tracker-repository-toggle-copy">
                          <span className="remote-repository-heading">
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
                          </span>
                          <small>
                            {repository.default_branch ||
                              "default branch unknown"}{" "}
                            · {issues.length} failure issue
                            {issues.length === 1 ? "" : "s"}
                            {repositorySuppressedRunCount > 0 && (
                              <>
                                {" · "}
                                {repositorySuppressedRunCount} repeated run
                                {repositorySuppressedRunCount === 1
                                  ? ""
                                  : "s"}{" "}
                                suppressed
                              </>
                            )}
                          </small>
                        </span>
                      </button>
                      <a
                        className="icon-button"
                        href={repository.html_url}
                        target="_blank"
                        rel="noreferrer"
                        aria-label={`Open ${repository.full_name} on GitHub`}
                        onClick={(event) => {
                          event.preventDefault();
                          void onOpenExternalUrl(repository.html_url);
                        }}
                      >
                        <ExternalLink size={14} />
                      </a>
                    </div>
                    <div
                      className="ci-tracker-run-list"
                      id={runListId}
                      hidden={collapsed}
                    >
                      {issues.map(
                        ({
                          run,
                          suppressedRunCount: issueSuppressedRunCount,
                        }) => {
                          const key = runKey(repository, run);
                          const started = startedKeys.has(key);
                          const starting = startingKey === key;
                          const localCheckout =
                            repository.locality === "Local and remote";
                          const hasArtifact = Boolean(run.prompt_artifact);
                          const canStart = localCheckout && !started;
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
                                  Run #{run.run_number} · attempt{" "}
                                  {run.run_attempt} ·{" "}
                                  {run.head_branch || "detached head"} · updated{" "}
                                  {formatTime(run.updated_at)}
                                </small>
                                {run.jobs.filter(
                                  (job) =>
                                    job.conclusion &&
                                    job.conclusion !== "success",
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
                                  onClick={(event) => {
                                    event.preventDefault();
                                    void onOpenExternalUrl(run.html_url);
                                  }}
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
                                      : undefined
                                  }
                                  onClick={() =>
                                    void startCodex(repository, run)
                                  }
                                >
                                  <Bot size={14} />
                                  {started
                                    ? "Started"
                                    : starting
                                      ? "Starting…"
                                      : "Diagnose with Codex"}
                                </button>
                              </div>
                              {issueSuppressedRunCount > 0 && (
                                <span className="ci-tracker-repeat-note">
                                  Same issue as {issueSuppressedRunCount}{" "}
                                  earlier run
                                  {issueSuppressedRunCount === 1 ? "" : "s"} ·
                                  latest run shown
                                </span>
                              )}
                              {!started && !localCheckout && (
                                <span className="ci-tracker-action-note">
                                  Remote-only repository · diagnosis stays in
                                  GitHub
                                </span>
                              )}
                              {!started && localCheckout && !hasArtifact && (
                                <span className="ci-tracker-action-note">
                                  Prompt artifact unavailable · diagnosis will
                                  use fresh GitHub evidence
                                </span>
                              )}
                            </div>
                          );
                        },
                      )}
                    </div>
                  </article>
                );
              },
            )}
          </div>
        )}
      </section>
    </div>
  );
}
