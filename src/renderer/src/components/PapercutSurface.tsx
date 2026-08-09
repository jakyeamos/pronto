import { useEffect, useState, type FormEvent, type ReactElement } from "react";
import {
  Check,
  CircleDot,
  ListTodo,
  Plus,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import type {
  CreatePapercutInput,
  Papercut,
  PapercutBacklog,
  PapercutPriority,
  PapercutSource,
  PapercutStatus,
} from "../types";
import { formatExactTime, formatTime, StatusPill } from "./ConsolePrimitives";

const statusOptions: PapercutStatus[] = [
  "open",
  "in_progress",
  "deferred",
  "resolved",
];

const priorityOptions: PapercutPriority[] = ["P0", "P1", "P2", "P3"];

const sourceOptions: Array<{ value: PapercutSource; label: string }> = [
  { value: "manual", label: "Manual capture" },
  { value: "design-friction", label: "Design-friction audit" },
];

const emptyDraft: CreatePapercutInput = {
  title: "",
  detail: "",
  surface: "Pronto UI",
  source: "manual",
  priority: "P2",
  evidenceRefs: [],
  impact: "",
  nextAction: "",
};

function statusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

function statusTone(status: string): string {
  if (status === "resolved") return "mint";
  if (status === "in_progress") return "blue";
  if (status === "deferred") return "amber";
  return "coral";
}

function sourceLabel(source: string): string {
  return source === "design-friction"
    ? "Design-friction audit"
    : "Manual capture";
}

function PapercutRow({
  papercut,
  selected,
  onSelect,
}: {
  papercut: Papercut;
  selected: boolean;
  onSelect: () => void;
}): ReactElement {
  return (
    <button
      className={`papercut-row ${selected ? "is-selected" : ""}`}
      type="button"
      onClick={onSelect}
      aria-label={`Open papercut ${papercut.title}`}
    >
      <span className="papercut-row-icon">
        {papercut.status === "resolved" ? (
          <Check size={14} />
        ) : (
          <CircleDot size={14} />
        )}
      </span>
      <span className="papercut-row-main">
        <strong>{papercut.title}</strong>
        <small>
          {papercut.surface} · {sourceLabel(papercut.source)}
        </small>
      </span>
      <StatusPill tone={statusTone(papercut.status)}>
        {statusLabel(papercut.status)}
      </StatusPill>
    </button>
  );
}

export function PapercutSurface({
  backlog,
  isRefreshing,
  onRefresh,
  onCreate,
  onStatusChange,
}: {
  backlog: PapercutBacklog;
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onCreate: (input: CreatePapercutInput) => Promise<void>;
  onStatusChange: (papercutId: string, status: PapercutStatus) => Promise<void>;
}): ReactElement {
  const [draft, setDraft] = useState<CreatePapercutInput>(emptyDraft);
  const [evidenceText, setEvidenceText] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected =
    backlog.papercuts.find((item) => item.id === selectedId) ?? null;

  useEffect(() => {
    if (!selected || selected.id !== selectedId) {
      setSelectedId(backlog.papercuts[0]?.id ?? null);
    }
  }, [backlog.papercuts, selected, selectedId]);

  async function handleSubmit(
    event: FormEvent<HTMLFormElement>,
  ): Promise<void> {
    event.preventDefault();
    if (!draft.title.trim() || !draft.detail.trim() || isRefreshing) return;
    await onCreate({
      ...draft,
      evidenceRefs: evidenceText
        .split("\n")
        .map((value) => value.trim())
        .filter(Boolean),
    });
    setDraft(emptyDraft);
    setEvidenceText("");
  }

  return (
    <div className="papercut-surface">
      <section className="surface-panel papercut-boundary">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Design audit family</p>
            <h2>Turn small friction into a visible backlog.</h2>
            <p>
              Design-friction audit stays an ephemeral per-turn sensor. This
              skill is the durable capture point: nothing enters the backlog
              unless you explicitly record it here.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh backlog"}
          </button>
        </div>
        <div className="papercut-boundary-status">
          <StatusPill tone="violet" icon={<Sparkles size={12} />}>
            Shared family: design audit
          </StatusPill>
          <span>Local only · generated {formatTime(backlog.generated_at)}</span>
        </div>
      </section>

      <section className="papercut-overview-grid" aria-label="Papercut counts">
        <div className="papercut-card">
          <span>Open</span>
          <strong>{backlog.counts.open}</strong>
          <small>Needs triage or a next step</small>
        </div>
        <div className="papercut-card">
          <span>In progress</span>
          <strong>{backlog.counts.in_progress}</strong>
          <small>Being worked</small>
        </div>
        <div className="papercut-card">
          <span>Deferred</span>
          <strong>{backlog.counts.deferred}</strong>
          <small>Kept visible with a pause</small>
        </div>
        <div className="papercut-card">
          <span>Resolved</span>
          <strong>{backlog.counts.resolved}</strong>
          <small>Retained as audit history</small>
        </div>
      </section>

      <div className="papercut-capture-layout">
        <section className="surface-panel papercut-capture-panel">
          <div className="surface-heading">
            <div>
              <p className="eyebrow">Explicit capture</p>
              <h2>Capture a papercut</h2>
              <p>
                Record the symptom and the next validation step—not a prompt
                transcript.
              </p>
            </div>
            <Plus size={18} aria-hidden="true" />
          </div>
          <form
            className="papercut-form"
            onSubmit={(event) => void handleSubmit(event)}
          >
            <label className="field-label">
              Title
              <input
                className="text-input"
                value={draft.title}
                placeholder="e.g. The empty state hides the next action"
                onChange={(event) =>
                  setDraft({ ...draft, title: event.target.value })
                }
                required
              />
            </label>
            <label className="field-label">
              What feels harder than it should?
              <textarea
                className="text-input papercut-textarea"
                value={draft.detail}
                placeholder="Describe the observed friction and where it occurs."
                onChange={(event) =>
                  setDraft({ ...draft, detail: event.target.value })
                }
                required
              />
            </label>
            <div className="papercut-form-grid">
              <label className="field-label">
                Surface
                <input
                  className="text-input"
                  value={draft.surface}
                  onChange={(event) =>
                    setDraft({ ...draft, surface: event.target.value })
                  }
                />
              </label>
              <label className="field-label">
                Source
                <select
                  className="text-input"
                  value={draft.source}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      source: event.target.value as PapercutSource,
                    })
                  }
                >
                  {sourceOptions.map((option) => (
                    <option value={option.value} key={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field-label">
                Priority
                <select
                  className="text-input"
                  value={draft.priority}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      priority: event.target.value as PapercutPriority,
                    })
                  }
                >
                  {priorityOptions.map((priority) => (
                    <option value={priority} key={priority}>
                      {priority}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <label className="field-label">
              Evidence references
              <textarea
                className="text-input papercut-textarea papercut-textarea-short"
                value={evidenceText}
                placeholder="One screen, route, file, or observation per line"
                onChange={(event) => setEvidenceText(event.target.value)}
              />
            </label>
            <label className="field-label">
              Impact
              <input
                className="text-input"
                value={draft.impact}
                placeholder="Who loses time or confidence?"
                onChange={(event) =>
                  setDraft({ ...draft, impact: event.target.value })
                }
              />
            </label>
            <label className="field-label">
              Next validation step
              <input
                className="text-input"
                value={draft.nextAction}
                placeholder="What would make this ready to resolve?"
                onChange={(event) =>
                  setDraft({ ...draft, nextAction: event.target.value })
                }
              />
            </label>
            <button
              className="button button-primary"
              type="submit"
              disabled={isRefreshing}
            >
              <Plus size={15} />
              Capture papercut
            </button>
          </form>
        </section>

        <section className="surface-panel papercut-list-panel">
          <div className="surface-heading">
            <div>
              <p className="eyebrow">Durable backlog</p>
              <h2>
                {backlog.counts.total} papercut
                {backlog.counts.total === 1 ? "" : "s"}
              </h2>
              <p>
                Review, defer, or resolve without changing the audit sensor.
              </p>
            </div>
            <ListTodo size={18} aria-hidden="true" />
          </div>
          {backlog.papercuts.length === 0 ? (
            <div className="papercut-empty">
              <ListTodo size={22} />
              <strong>No papercuts captured yet.</strong>
              <span>
                When a design audit finds a repeatable small hurt, capture it
                here explicitly.
              </span>
            </div>
          ) : (
            <div className="papercut-list">
              {backlog.papercuts.map((papercut) => (
                <PapercutRow
                  key={papercut.id}
                  papercut={papercut}
                  selected={papercut.id === selectedId}
                  onSelect={() => setSelectedId(papercut.id)}
                />
              ))}
            </div>
          )}
        </section>
      </div>

      {selected && (
        <section className="surface-panel papercut-detail-panel">
          <div className="papercut-detail-heading">
            <div>
              <p className="eyebrow">
                {sourceLabel(selected.source)} · {selected.priority}
              </p>
              <h2>{selected.title}</h2>
              <p>{selected.detail}</p>
            </div>
            <label className="field-label papercut-status-field">
              Status
              <select
                className="text-input"
                aria-label={`Status for ${selected.title}`}
                value={selected.status}
                disabled={isRefreshing}
                onChange={(event) =>
                  void onStatusChange(
                    selected.id,
                    event.target.value as PapercutStatus,
                  )
                }
              >
                {statusOptions.map((status) => (
                  <option value={status} key={status}>
                    {statusLabel(status)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="papercut-detail-grid">
            <div>
              <span>Surface</span>
              <strong>{selected.surface}</strong>
            </div>
            <div>
              <span>Captured</span>
              <strong>{formatExactTime(selected.created_at)}</strong>
            </div>
            <div>
              <span>Updated</span>
              <strong>{formatTime(selected.updated_at)}</strong>
            </div>
            <div>
              <span>Family</span>
              <strong>{selected.family}</strong>
            </div>
          </div>
          <div className="papercut-detail-sections">
            <div>
              <span>Impact</span>
              <p>{selected.impact || "Not recorded."}</p>
            </div>
            <div>
              <span>Next validation step</span>
              <p>{selected.next_action}</p>
            </div>
            <div>
              <span>Evidence references</span>
              {selected.evidence_refs.length === 0 ? (
                <p>None recorded.</p>
              ) : (
                <ul className="papercut-evidence-list">
                  {selected.evidence_refs.map((reference) => (
                    <li key={reference}>
                      <code>{reference}</code>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
