import { useState, type ReactElement } from "react";
import {
  ClipboardList,
  Layers3,
  Lightbulb,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type {
  CreatePapercutInput,
  MultiplierProposalStatus,
  PapercutBacklog,
  PapercutStatus,
} from "../types";
import { formatExactTime, formatTime, StatusPill } from "./ConsolePrimitives";
import { PapercutPatternView } from "./PapercutPatternView";
import { papercutLabel, papercutStatusTone } from "./papercutPresentation";

type PapercutTab = "observations" | "patterns" | "digest";

export function PapercutSurface({
  backlog,
  isRefreshing,
  onRefresh,
  onCreate,
  onStatusChange,
  onProposalStatusChange,
}: {
  backlog: PapercutBacklog;
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onCreate: (input: CreatePapercutInput) => Promise<void>;
  onStatusChange: (papercutId: string, status: PapercutStatus) => Promise<void>;
  onProposalStatusChange: (
    proposalId: string,
    status: MultiplierProposalStatus,
  ) => Promise<void>;
}): ReactElement {
  const [activeTab, setActiveTab] = useState<PapercutTab>("patterns");
  const latestDigest = backlog.digests[0] ?? null;

  return (
    <div className="papercut-surface">
      <section className="surface-panel papercut-boundary">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Universal signal corpus</p>
            <h2>Turn friction into evidence, patterns, and multipliers.</h2>
            <p>
              Explicit corrections, dissatisfaction, failures, and
              evidence-backed suggestions enter as scoped observations.
              Recurrence—not sentiment—promotes them.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh corpus"}
          </button>
        </div>
        <div className="papercut-boundary-status">
          <StatusPill
            tone={backlog.health.status === "healthy" ? "mint" : "amber"}
            icon={<ShieldCheck size={12} />}
          >
            Capture {backlog.health.status}
          </StatusPill>
          <span>
            Local only · excerpts expire after{" "}
            {backlog.health.excerpt_retention_days} days
          </span>
          {backlog.health.spooled_events > 0 ? (
            <span>{backlog.health.spooled_events} signals awaiting flush</span>
          ) : null}
          <span>Generated {formatTime(backlog.generated_at)}</span>
        </div>
      </section>

      <nav className="papercut-tabs" aria-label="Papercuts corpus views">
        {(
          [
            ["observations", "Observations", ClipboardList],
            ["patterns", "Patterns", Layers3],
            ["digest", "Weekly digest", Lightbulb],
          ] as const
        ).map(([value, text, Icon]) => (
          <button
            className={activeTab === value ? "is-active" : ""}
            type="button"
            key={value}
            onClick={() => setActiveTab(value)}
          >
            <Icon size={14} />
            {text}
          </button>
        ))}
      </nav>

      <section
        className="papercut-overview-grid"
        aria-label="Papercut corpus counts"
      >
        <div className="papercut-card">
          <span>Observations</span>
          <strong>{backlog.counts.observations}</strong>
          <small>Explicit signals retained</small>
        </div>
        <div className="papercut-card">
          <span>Local patterns</span>
          <strong>{backlog.counts.local_patterns}</strong>
          <small>Recurring within one scope</small>
        </div>
        <div className="papercut-card">
          <span>Cross-scope</span>
          <strong>{backlog.counts.cross_scope_patterns}</strong>
          <small>Three signals across two scopes</small>
        </div>
        <div className="papercut-card">
          <span>Draft multipliers</span>
          <strong>{backlog.counts.draft_proposals}</strong>
          <small>Waiting for human review</small>
        </div>
      </section>

      {activeTab === "observations" ? (
        <section className="surface-panel papercut-corpus-panel">
          <div className="surface-heading">
            <div>
              <p className="eyebrow">Raw evidence</p>
              <h2>Scoped observations</h2>
              <p>
                Short excerpts expire; summaries and hashes remain for
                recurrence analysis.
              </p>
            </div>
            <ClipboardList size={18} aria-hidden="true" />
          </div>
          {backlog.observations.length === 0 ? (
            <div className="papercut-empty">
              <ClipboardList size={22} />
              <strong>No observations captured yet.</strong>
              <span>
                The passive route records only explicit interaction-quality
                signals.
              </span>
            </div>
          ) : (
            <div className="papercut-observation-list">
              {backlog.observations.map((observation) => (
                <article key={observation.id}>
                  <div>
                    <strong>{observation.summary}</strong>
                    <small>
                      {papercutLabel(observation.signal_kind)} ·{" "}
                      {papercutLabel(observation.target_kind)} ·{" "}
                      {observation.scope_id}
                    </small>
                  </div>
                  <StatusPill tone={observation.urgent ? "coral" : "violet"}>
                    {observation.priority}
                  </StatusPill>
                  {observation.excerpt ? (
                    <p>“{observation.excerpt}”</p>
                  ) : (
                    <p className="is-expired">
                      Excerpt expired; structured evidence retained.
                    </p>
                  )}
                  <footer>
                    <span>{formatExactTime(observation.observed_at)}</span>
                    <span>{observation.domain}</span>
                    <span>{observation.source}</span>
                  </footer>
                </article>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {activeTab === "patterns" ? (
        <PapercutPatternView
          backlog={backlog}
          isRefreshing={isRefreshing}
          onCreate={onCreate}
          onStatusChange={onStatusChange}
        />
      ) : null}

      {activeTab === "digest" ? (
        <div className="papercut-digest-layout">
          <section className="surface-panel papercut-digest-panel">
            <div className="surface-heading">
              <div>
                <p className="eyebrow">Sunday · 6 PM ET</p>
                <h2>Weekly digest</h2>
                <p>Deterministic evidence first; AI hypotheses second.</p>
              </div>
              <Sparkles size={18} />
            </div>
            {latestDigest ? (
              <div className="papercut-digest-body">
                <p>
                  {formatExactTime(latestDigest.week_start)} through{" "}
                  {formatExactTime(latestDigest.week_end)}
                </p>
                <div className="papercut-digest-metrics">
                  <strong>
                    {latestDigest.observation_count}
                    <small>observations</small>
                  </strong>
                  <strong>
                    {latestDigest.local_pattern_count}
                    <small>local patterns</small>
                  </strong>
                  <strong>
                    {latestDigest.cross_scope_pattern_count}
                    <small>cross-scope</small>
                  </strong>
                </div>
                <h3>Leading patterns</h3>
                {latestDigest.top_patterns.length === 0 ? (
                  <p>No patterns qualified this week.</p>
                ) : (
                  <ol>
                    {latestDigest.top_patterns.map((pattern) => (
                      <li key={pattern.id}>
                        <span>{pattern.title}</span>
                        <small>
                          {pattern.occurrence_count} occurrences ·{" "}
                          {pattern.scope_count} scopes
                        </small>
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            ) : (
              <div className="papercut-empty">
                <Sparkles size={22} />
                <strong>No weekly digest yet.</strong>
                <span>
                  The configured Codex automation will generate the first digest
                  Sunday at 6 PM ET.
                </span>
              </div>
            )}
          </section>
          <section className="surface-panel papercut-proposal-panel">
            <div className="surface-heading">
              <div>
                <p className="eyebrow">Human review required</p>
                <h2>Multiplier proposals</h2>
                <p>
                  Accepting records your judgment; it never starts
                  implementation.
                </p>
              </div>
              <Lightbulb size={18} />
            </div>
            {backlog.proposals.length === 0 ? (
              <div className="papercut-empty">
                <Lightbulb size={22} />
                <strong>No multiplier drafts yet.</strong>
                <span>
                  AI may draft causes and reusable prevention only from
                  sanitized digest evidence.
                </span>
              </div>
            ) : (
              <div className="papercut-proposal-list">
                {backlog.proposals.map((proposal) => (
                  <article key={proposal.id}>
                    <header>
                      <strong>{proposal.title}</strong>
                      <StatusPill tone={papercutStatusTone(proposal.status)}>
                        {papercutLabel(proposal.status)}
                      </StatusPill>
                    </header>
                    <p>{proposal.hypothesis}</p>
                    <dl>
                      <div>
                        <dt>Root cause</dt>
                        <dd>{proposal.root_cause}</dd>
                      </div>
                      <div>
                        <dt>Multiplier</dt>
                        <dd>{proposal.multiplier}</dd>
                      </div>
                    </dl>
                    <footer>
                      {(
                        [
                          "accepted",
                          "deferred",
                          "rejected",
                        ] as MultiplierProposalStatus[]
                      ).map((status) => (
                        <button
                          className="button button-secondary"
                          type="button"
                          key={status}
                          disabled={isRefreshing || proposal.status === status}
                          onClick={() =>
                            void onProposalStatusChange(proposal.id, status)
                          }
                        >
                          {papercutLabel(status)}
                        </button>
                      ))}
                    </footer>
                  </article>
                ))}
              </div>
            )}
          </section>
        </div>
      ) : null}
    </div>
  );
}
