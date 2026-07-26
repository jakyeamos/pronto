import type { ReactElement } from "react";
import {
  Activity,
  Database,
  FolderPlus,
  LockKeyhole,
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

export function SettingsSurface({
  roots,
  storagePath,
  generatedAt,
  onAddRoot,
}: {
  roots: RootConfig[];
  storagePath: string;
  generatedAt: string;
  onAddRoot: () => void;
}): ReactElement {
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
              <div className="settings-root" key={root.id}>
                <div>
                  <strong>{root.label}</strong>
                  <span>{root.path}</span>
                </div>
                <small>Added {formatTime(root.registered_at)}</small>
              </div>
            ))}
          </div>
        )}
      </section>
      <div className="surface-settings-grid">
        <section className="surface-panel surface-info-card">
          <Database size={17} />
          <div>
            <p className="eyebrow">Local storage</p>
            <h3>Snapshot location</h3>
            <p>{storagePath || "Created when the first snapshot is saved."}</p>
            <small>Last snapshot {formatTime(generatedAt)}</small>
          </div>
        </section>
        <section className="surface-panel surface-info-card">
          <LockKeyhole size={17} />
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
