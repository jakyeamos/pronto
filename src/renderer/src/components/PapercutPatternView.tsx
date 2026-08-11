import {
  useEffect,
  useMemo,
  useState,
  type FormEvent,
  type ReactElement,
} from "react";
import { Check, CircleDot, ListTodo, Plus } from "lucide-react";
import type {
  CreatePapercutInput,
  Papercut,
  PapercutBacklog,
  PapercutPriority,
  PapercutSource,
  PapercutStatus,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import { papercutLabel, papercutStatusTone } from "./papercutPresentation";

type ScopeFilter = "all" | "local" | "cross_scope";

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

function sourceLabel(source: string): string {
  if (source === "design-friction") return "Design-friction audit";
  if (source === "manual") return "Manual capture";
  return papercutLabel(source);
}

function PapercutRow({
  papercut,
  selected,
  scope,
  onSelect,
}: {
  papercut: Papercut;
  selected: boolean;
  scope: string;
  onSelect: () => void;
}): ReactElement {
  return (
    <button
      className={`papercut-row ${selected ? "is-selected" : ""}`}
      type="button"
      onClick={onSelect}
      aria-label={`Open papercut pattern ${papercut.title}`}
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
          {scope} · {sourceLabel(papercut.source)}
        </small>
      </span>
      <StatusPill tone={papercutStatusTone(papercut.status)}>
        {papercutLabel(papercut.status)}
      </StatusPill>
    </button>
  );
}

export function PapercutPatternView({
  backlog,
  isRefreshing,
  onCreate,
  onStatusChange,
}: {
  backlog: PapercutBacklog;
  isRefreshing: boolean;
  onCreate: (input: CreatePapercutInput) => Promise<void>;
  onStatusChange: (papercutId: string, status: PapercutStatus) => Promise<void>;
}): ReactElement {
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>("all");
  const [draft, setDraft] = useState<CreatePapercutInput>(emptyDraft);
  const [evidenceText, setEvidenceText] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const patternById = useMemo(
    () => new Map(backlog.patterns.map((pattern) => [pattern.id, pattern])),
    [backlog.patterns],
  );
  const filteredPapercuts = useMemo(
    () =>
      backlog.papercuts.filter((item) => {
        if (scopeFilter === "all") return true;
        return patternById.get(item.id)?.scope_kind === scopeFilter;
      }),
    [backlog.papercuts, patternById, scopeFilter],
  );
  const selected =
    filteredPapercuts.find((item) => item.id === selectedId) ?? null;
  const selectedPattern = selected
    ? (patternById.get(selected.id) ?? null)
    : null;

  useEffect(() => {
    if (!selected || selected.id !== selectedId) {
      setSelectedId(filteredPapercuts[0]?.id ?? null);
    }
  }, [filteredPapercuts, selected, selectedId]);

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
    <>
      <div className="papercut-scope-filter" aria-label="Pattern scope filter">
        {(["all", "local", "cross_scope"] as ScopeFilter[]).map((scope) => (
          <button
            type="button"
            className={scopeFilter === scope ? "is-active" : ""}
            key={scope}
            onClick={() => setScopeFilter(scope)}
          >
            {papercutLabel(scope)}
          </button>
        ))}
      </div>
      <div className="papercut-capture-layout">
        <section className="surface-panel papercut-capture-panel">
          <div className="surface-heading">
            <div>
              <p className="eyebrow">Manual evidence</p>
              <h2>Capture a papercut</h2>
              <p>
                Use this for a deliberate entry; automatic capture remains
                silent when healthy.
              </p>
            </div>
            <Plus size={18} />
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
                onChange={(event) =>
                  setDraft({ ...draft, title: event.target.value })
                }
                required
              />
            </label>
            <label className="field-label">
              Observed friction
              <textarea
                className="text-input papercut-textarea"
                value={draft.detail}
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
                    setDraft({ ...draft, source: event.target.value })
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
                onChange={(event) => setEvidenceText(event.target.value)}
              />
            </label>
            <label className="field-label">
              Impact
              <input
                className="text-input"
                value={draft.impact}
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
              <p className="eyebrow">Promoted evidence</p>
              <h2>
                {filteredPapercuts.length} pattern
                {filteredPapercuts.length === 1 ? "" : "s"}
              </h2>
              <p>
                Local recurrence stays local until cross-scope evidence earns
                promotion.
              </p>
            </div>
            <ListTodo size={18} />
          </div>
          {filteredPapercuts.length === 0 ? (
            <div className="papercut-empty">
              <ListTodo size={22} />
              <strong>No patterns in this scope.</strong>
              <span>
                Two matching observations create a local pattern; three across
                two scopes create a cross-scope candidate.
              </span>
            </div>
          ) : (
            <div className="papercut-list">
              {filteredPapercuts.map((papercut) => (
                <PapercutRow
                  key={papercut.id}
                  papercut={papercut}
                  selected={papercut.id === selectedId}
                  scope={papercutLabel(
                    patternById.get(papercut.id)?.scope_kind ?? "legacy",
                  )}
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
                {selectedPattern
                  ? papercutLabel(selectedPattern.evidence_tier)
                  : sourceLabel(selected.source)}{" "}
                · {selected.priority}
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
                    {papercutLabel(status)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="papercut-detail-grid">
            <div>
              <span>Scope</span>
              <strong>
                {selectedPattern
                  ? papercutLabel(selectedPattern.scope_kind)
                  : "legacy"}
              </strong>
            </div>
            <div>
              <span>Occurrences</span>
              <strong>{selectedPattern?.occurrence_count ?? 1}</strong>
            </div>
            <div>
              <span>Scopes</span>
              <strong>{selectedPattern?.scope_count ?? 1}</strong>
            </div>
            <div>
              <span>Last observed</span>
              <strong>
                {formatTime(
                  selectedPattern?.last_observed_at ?? selected.updated_at,
                )}
              </strong>
            </div>
          </div>
          <div className="papercut-detail-sections">
            <div>
              <span>Failure mode</span>
              <p>{selectedPattern?.failure_mode ?? "Legacy manual capture"}</p>
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
    </>
  );
}
