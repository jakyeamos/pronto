import type { ReactElement } from "react";
import {
  Activity,
  BellOff,
  Check,
  ChevronDown,
  ChevronRight,
  Clock3,
  GitBranch,
  ShieldCheck,
} from "lucide-react";
import type { Condition, EventRecord, RepositorySnapshot } from "../types";
import {
  ConditionPill,
  formatTime,
  StatusPill,
  toneForCondition,
} from "./ConsolePrimitives";

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
              {workspace.branch}
            </span>
            <span className={workspace.dirty ? "fact-warn" : ""}>
              {workspace.dirty
                ? workspace.line_totals_partial
                  ? "Dirty · totals partial"
                  : `Dirty · +${workspace.added} / −${workspace.removed}`
                : "Clean"}
            </span>
            <span
              className={workspace.sync_state !== "Synced" ? "fact-warn" : ""}
            >
              {workspace.sync_state}
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
          <StatusPill tone="violet" icon={<BellOff size={11} />}>
            {expectedCount} expected
          </StatusPill>
        ) : (
          <StatusPill tone="mint" icon={<Check size={11} />}>
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
  onCondition,
}: {
  repositories: RepositorySnapshot[];
  onCondition: (repository: RepositorySnapshot, condition: Condition) => void;
}): ReactElement {
  const attention = repositories
    .map((repository) => ({
      repository,
      conditions: repository.conditions.filter(
        (condition) => condition.status === "Active",
      ),
    }))
    .filter((group) => group.conditions.length > 0);
  return (
    <section className="rail-section">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Evidence-led queue</p>
          <h2>Attention queue</h2>
        </div>
        <span className="section-count">
          {attention.reduce(
            (total, group) => total + group.conditions.length,
            0,
          )}
        </span>
      </div>
      {attention.length === 0 ? (
        <div className="rail-empty">
          <ShieldCheck size={18} />
          <span>No active conditions in the current snapshot.</span>
        </div>
      ) : (
        <div className="attention-list">
          {attention.map(({ repository, conditions }) => (
            <details className="attention-group" key={repository.id} open>
              <summary>
                <span className="summary-repo">
                  <span className="tiny-repo-mark">
                    <GitBranch size={12} />
                  </span>
                  {repository.name}
                </span>
                <span className="summary-count">
                  {conditions.length}
                  <ChevronDown size={13} />
                </span>
              </summary>
              <div className="attention-conditions">
                {conditions.map((condition) => (
                  <button
                    className="attention-item"
                    type="button"
                    key={condition.id}
                    onClick={() => onCondition(repository, condition)}
                  >
                    <span
                      className={`attention-dot attention-dot-${toneForCondition(condition)}`}
                    />
                    <span className="attention-copy">
                      <strong>{condition.title}</strong>
                      <span>{condition.summary}</span>
                    </span>
                    <ChevronRight size={14} />
                  </button>
                ))}
              </div>
            </details>
          ))}
        </div>
      )}
    </section>
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
