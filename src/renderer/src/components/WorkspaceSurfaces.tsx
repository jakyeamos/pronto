import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import {
  Activity,
  Database,
  FolderPlus,
  LockKeyhole,
  Save,
  ShieldCheck,
} from "lucide-react";
import type { ActionAudit, EventRecord, RootConfig } from "../types";
import { formatTime } from "./ConsolePrimitives";

export function DeferredSurface({
  eyebrow,
  title,
  body,
  icon,
  details,
}: {
  eyebrow: string;
  title: string;
  body: string;
  icon: ReactElement;
  details: Array<{ label: string; value: string }>;
}): ReactElement {
  return (
    <section className="surface-panel surface-boundary">
      <div className="surface-boundary-icon">{icon}</div>
      <div className="surface-copy">
        <p className="eyebrow">{eyebrow}</p>
        <h2>{title}</h2>
        <p>{body}</p>
      </div>
      <div className="surface-detail-grid">
        {details.map((detail) => (
          <div className="surface-detail" key={detail.label}>
            <span>{detail.label}</span>
            <strong>{detail.value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}

export function ActivitySurface({
  events,
  actionAudits,
}: {
  events: EventRecord[];
  actionAudits: ActionAudit[];
}): ReactElement {
  const hasActivity = events.length > 0 || actionAudits.length > 0;
  return (
    <section className="surface-panel">
      <div className="surface-heading">
        <div>
          <p className="eyebrow">Local action history</p>
          <h2>Activity</h2>
          <p>Safe local actions and meaningful state changes are retained.</p>
        </div>
        <Activity size={18} className="muted-icon" />
      </div>
      {!hasActivity ? (
        <div className="surface-empty">
          <ShieldCheck size={18} />
          <span>
            Activity will appear after a local refresh or a repository state
            transition.
          </span>
        </div>
      ) : (
        <div className="surface-event-list">
          {actionAudits.map((audit) => (
            <article className="surface-event" key={audit.id}>
              <span className="timeline-node" />
              <div>
                <strong>{audit.summary}</strong>
                <p>
                  {audit.action} · {audit.status.toLowerCase()} · {audit.risk}
                </p>
              </div>
              <time>{formatTime(audit.created_at)}</time>
            </article>
          ))}
          {events.map((event) => (
            <article className="surface-event" key={event.id}>
              <span className="timeline-node" />
              <div>
                <strong>{event.summary}</strong>
                <p>{event.kind.replaceAll("-", " ")}</p>
              </div>
              <time>{formatTime(event.created_at)}</time>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function RootSettingsCard({
  root,
  onSave,
}: {
  root: RootConfig;
  onSave: (
    rootId: string,
    ignorePatterns: string[],
    refreshPolicy: string,
    backgroundMonitoring: boolean,
  ) => Promise<void>;
}): ReactElement {
  const [patterns, setPatterns] = useState(root.ignore_patterns.join(", "));
  const [refreshPolicy, setRefreshPolicy] = useState(root.refresh_policy);
  const [backgroundMonitoring, setBackgroundMonitoring] = useState(
    root.background_monitoring,
  );
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    setPatterns(root.ignore_patterns.join(", "));
    setRefreshPolicy(root.refresh_policy);
    setBackgroundMonitoring(root.background_monitoring);
  }, [root]);

  const save = async (): Promise<void> => {
    setIsSaving(true);
    try {
      await onSave(
        root.id,
        patterns
          .split(",")
          .map((pattern) => pattern.trim())
          .filter(Boolean),
        refreshPolicy,
        backgroundMonitoring,
      );
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <article className="settings-root settings-root-editable">
      <div className="settings-root-heading">
        <div>
          <strong>{root.label}</strong>
          <span>{root.path}</span>
        </div>
        <small>Added {formatTime(root.registered_at)}</small>
      </div>
      <div className="root-settings-grid">
        <label className="field-label">
          Ignore names or suffix patterns
          <input
            className="text-input"
            value={patterns}
            onChange={(event) => setPatterns(event.target.value)}
            placeholder="target, *.tmp"
          />
          <small className="field-help">
            Comma-separated names; nested paths are rejected.
          </small>
        </label>
        <label className="field-label">
          Fetch policy
          <select
            className="text-input"
            value={refreshPolicy}
            onChange={(event) => setRefreshPolicy(event.target.value)}
          >
            <option>On open</option>
            <option>Manual</option>
            <option>Periodic</option>
          </select>
        </label>
      </div>
      <div className="settings-root-actions">
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={backgroundMonitoring}
            onChange={(event) => setBackgroundMonitoring(event.target.checked)}
          />
          <span>Allow background monitoring for this root</span>
        </label>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => void save()}
          disabled={isSaving}
        >
          <Save size={14} />
          {isSaving ? "Saving…" : "Save root settings"}
        </button>
      </div>
    </article>
  );
}

export function SettingsSurface({
  roots,
  storagePath,
  generatedAt,
  onAddRoot,
  retentionDays,
  onSaveRoot,
  onSaveRetention,
}: {
  roots: RootConfig[];
  storagePath: string;
  generatedAt: string;
  onAddRoot: () => void;
  retentionDays: number;
  onSaveRoot: (
    rootId: string,
    ignorePatterns: string[],
    refreshPolicy: string,
    backgroundMonitoring: boolean,
  ) => Promise<void>;
  onSaveRetention: (retentionDays: number) => Promise<void>;
}): ReactElement {
  const [retentionInput, setRetentionInput] = useState(String(retentionDays));
  const [isSavingRetention, setIsSavingRetention] = useState(false);

  useEffect(() => {
    setRetentionInput(String(retentionDays));
  }, [retentionDays]);

  const saveRetention = async (): Promise<void> => {
    const value = Number.parseInt(retentionInput, 10);
    if (!Number.isFinite(value)) return;
    setIsSavingRetention(true);
    try {
      await onSaveRetention(value);
    } finally {
      setIsSavingRetention(false);
    }
  };

  return (
    <div className="surface-settings">
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Local configuration</p>
            <h2>Discovery roots</h2>
            <p>Pronto only scans folders you explicitly register.</p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={onAddRoot}
          >
            <FolderPlus size={15} />
            Add root
          </button>
        </div>
        {roots.length === 0 ? (
          <div className="surface-empty">
            <FolderPlus size={18} />
            <span>No local discovery roots are registered yet.</span>
          </div>
        ) : (
          <div className="settings-root-list">
            {roots.map((root) => (
              <RootSettingsCard key={root.id} root={root} onSave={onSaveRoot} />
            ))}
          </div>
        )}
      </section>
      <div className="surface-settings-grid">
        <section className="surface-panel surface-info-card">
          <Database size={17} />
          <div>
            <p className="eyebrow">History retention</p>
            <h3>Keep transition evidence</h3>
            <div className="retention-control">
              <input
                className="text-input retention-input"
                type="number"
                min={1}
                max={3650}
                value={retentionInput}
                onChange={(event) => setRetentionInput(event.target.value)}
                aria-label="Event retention days"
              />
              <span>days</span>
              <button
                className="icon-button"
                type="button"
                aria-label="Save history retention"
                onClick={() => void saveRetention()}
                disabled={isSavingRetention}
              >
                <Save size={14} />
              </button>
            </div>
            <small>Current state is independent from retained history.</small>
          </div>
        </section>
        <section className="surface-panel surface-info-card">
          <LockKeyhole size={17} />
          <div>
            <p className="eyebrow">Local storage</p>
            <h3>Snapshot location</h3>
            <p>{storagePath || "Created when the first snapshot is saved."}</p>
            <small>Last snapshot {formatTime(generatedAt)}</small>
          </div>
        </section>
        <section className="surface-panel surface-info-card">
          <ShieldCheck size={17} />
          <div>
            <p className="eyebrow">Privacy boundary</p>
            <h3>Private by default</h3>
            <p>Repository paths and aggregate Git facts stay on this device.</p>
            <small>Provider sync is not connected.</small>
          </div>
        </section>
      </div>
    </div>
  );
}
