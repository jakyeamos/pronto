import type { ReactElement } from "react";
import {
  Activity,
  BellOff,
  Check,
  ChevronRight,
  Clock3,
  Compass,
  GitBranch,
  ShieldCheck,
} from "lucide-react";
import type { Condition, EventRecord, RepositorySnapshot } from "../types";
import { ConditionPill, formatTime, StatusPill } from "./ConsolePrimitives";
import { qualityAttentionItems } from "./QualityComponents";
import {
  projectCompassCoverageIsIncomplete,
  projectCompassCoverageLabel,
  projectCompassProgressLabel,
} from "../projectCompass";

export function RepositoryRow({
  repository,
  onOpen,
  onCondition,
}: {
  repository: RepositorySnapshot;
  onOpen: () => void;
  onCondition: (condition: Condition) => void;
}): ReactElement {
  const activeCondition = repository.conditions.find(
    (condition) => condition.status === "Active",
  );
  const expectedCount = repository.conditions.filter(
    (condition) => condition.status === "Expected",
  ).length;
  const workspace = repository.workspace;
  const gitStatusUnavailable = workspace.status_available === false;
  const compassReady = repository.project_compass.status === "Ready";
  const mvpCoverageIncomplete =
    compassReady &&
    projectCompassCoverageIsIncomplete(repository.project_compass.mvp);
  return (
    <article className="repository-row">
      <button className="repository-main" type="button" onClick={onOpen}>
        <div className="repo-mark">
          <GitBranch size={17} />
        </div>
        <div className="repo-identity">
          <div className="repo-title-line">
            <h3>{repository.name}</h3>
            <StatusPill
              tone={repository.locality === "Connected" ? "blue" : "slate"}
            >
              {repository.locality}
            </StatusPill>
            {repository.lifecycle !== "Unconfirmed" && (
              <StatusPill tone="slate">{repository.lifecycle}</StatusPill>
            )}
          </div>
          <p className="repo-path">{repository.path}</p>
          <div className="repo-facts">
            <span>
              <GitBranch size={13} />
              {gitStatusUnavailable
                ? "Git status unavailable"
                : workspace.branch}
            </span>
            <span
              className={
                gitStatusUnavailable || workspace.dirty ? "fact-warn" : ""
              }
            >
              {gitStatusUnavailable
                ? "Git status unavailable"
                : workspace.dirty
                  ? workspace.line_totals_partial
                    ? "Dirty · totals partial"
                    : `Dirty · +${workspace.added} / −${workspace.removed}`
                  : "Clean"}
            </span>
            <span
              className={
                gitStatusUnavailable || workspace.sync_state !== "Synced"
                  ? "fact-warn"
                  : ""
              }
            >
              {gitStatusUnavailable
                ? "Git status unavailable"
                : workspace.sync_state}
            </span>
            <span
              className={
                !compassReady || mvpCoverageIncomplete ? "fact-warn" : ""
              }
              title={
                compassReady
                  ? projectCompassCoverageLabel(repository.project_compass.mvp)
                  : undefined
              }
            >
              <Compass size={13} />
              {compassReady
                ? projectCompassProgressLabel(repository.project_compass.mvp)
                : `Compass ${repository.project_compass.status.toLowerCase()}`}
            </span>
          </div>
        </div>
      </button>
      <div className="repo-condition-cell">
        {activeCondition ? (
          <button
            className="condition-button"
            type="button"
            onClick={() => onCondition(activeCondition)}
          >
            <ConditionPill condition={activeCondition} />
            <ChevronRight size={14} />
          </button>
        ) : expectedCount > 0 ? (
          <StatusPill
            tone="violet"
            icon={<BellOff size={11} />}
            title="Expected conditions are tracked context, not unresolved active work."
          >
            {expectedCount} expected
          </StatusPill>
        ) : (
          <StatusPill
            tone="mint"
            icon={<Check size={11} />}
            title="No unresolved active condition records were found for this repository."
          >
            No active conditions
          </StatusPill>
        )}
      </div>
      <div className="repo-meta">
        <div>
          <span>Last scan</span>
          <strong>{formatTime(repository.last_scan_at)}</strong>
        </div>
        <ChevronRight className="row-arrow" size={16} />
      </div>
    </article>
  );
}

export function AttentionQueue({
  repositories,
  onShowAttention,
}: {
  repositories: RepositorySnapshot[];
  onShowAttention?: () => void;
}): ReactElement {
  const attention = repositories
    .map((repository) => ({
      repository,
      conditions: repository.conditions.filter(
        (condition) => condition.status === "Active",
      ),
    }))
    .filter((group) => group.conditions.length > 0);
  const qualityAttention = repositories
    .map((repository) => ({
      repository,
      items: qualityAttentionItems(repository),
    }))
    .filter((group) => group.items.length > 0);
  const attentionCount =
    attention.reduce((total, group) => total + group.conditions.length, 0) +
    qualityAttention.reduce((total, group) => total + group.items.length, 0);
  const qualityAttentionCount = qualityAttention.reduce(
    (total, group) => total + group.items.length,
    0,
  );
  return (
    <details className="rail-section attention-queue">
      <summary className="section-heading attention-queue-summary">
        <div>
          <p className="eyebrow">Evidence-led queue</p>
          <h2 title="Unresolved local conditions and quality follow-ups from the current snapshot">
            Attention queue
          </h2>
        </div>
        <span
          className="section-count"
          aria-label={`${attentionCount} attention items`}
        >
          {attentionCount}
        </span>
      </summary>
      {attention.length === 0 && qualityAttention.length === 0 ? (
        <div className="rail-empty">
          <ShieldCheck size={18} />
          <span>
            No active conditions or quality follow-ups in the current snapshot.
          </span>
        </div>
      ) : (
        <div className="attention-summary">
          <div className="attention-summary-grid">
            <div className="attention-summary-stat">
              <strong>{attention.length}</strong>
              <span>repositories with active conditions</span>
            </div>
            <div className="attention-summary-stat">
              <strong>{attentionCount - qualityAttentionCount}</strong>
              <span>active conditions</span>
            </div>
            <div className="attention-summary-stat">
              <strong>{qualityAttention.length}</strong>
              <span>repositories with quality follow-up</span>
            </div>
            <div className="attention-summary-stat">
              <strong>{qualityAttentionCount}</strong>
              <span>quality follow-ups</span>
            </div>
          </div>
          <p
            className="attention-summary-copy"
            title="Active conditions are unresolved condition records with status Active. Expected conditions are intentionally excluded."
          >
            Active conditions are unresolved local evidence signals. The
            Repository portfolio is the canonical repository list; this rail
            only summarizes what needs review. Use the Quality gate matrix below
            when you need per-repository gate comparisons.
          </p>
          {attention.length > 0 && onShowAttention ? (
            <button
              className="button button-secondary attention-summary-action"
              type="button"
              onClick={onShowAttention}
            >
              Show active-condition repositories
            </button>
          ) : null}
        </div>
      )}
    </details>
  );
}

export function Timeline({ events }: { events: EventRecord[] }): ReactElement {
  return (
    <section className="rail-section timeline-section">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Transition-only history</p>
          <h2>Recent activity</h2>
        </div>
        <Activity size={17} className="muted-icon" />
      </div>
      {events.length === 0 ? (
        <div className="rail-empty">
          <Clock3 size={18} />
          <span>
            Meaningful state transitions will appear here after the first scan.
          </span>
        </div>
      ) : (
        <div className="timeline-list">
          {events.slice(0, 6).map((event) => (
            <div className="timeline-item" key={event.id}>
              <span className="timeline-node" />
              <div>
                <p>{event.summary}</p>
                <span>{formatTime(event.created_at)}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
