import type { ReactElement, ReactNode } from "react";
import { CircleDot, FolderPlus, SearchX } from "lucide-react";
import type { Condition } from "../types";

export function formatTime(value?: string): string {
  if (!value) return "Not recorded";
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return value;
  const minutes = Math.max(0, Math.round((Date.now() - timestamp) / 60_000));
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

export function toneForCondition(condition: Condition): string {
  if (condition.kind === "integration-eligible") return "mint";
  if (condition.kind === "remote-stale" || condition.kind === "behind-remote")
    return "amber";
  if (condition.kind === "dirty-workspace") return "coral";
  if (
    condition.kind === "unavailable" ||
    condition.kind === "interrupted-operation"
  )
    return "red";
  return "blue";
}

export function IconButton({
  label,
  onClick,
  children,
  disabled = false,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
}): ReactElement {
  return (
    <button
      className="icon-button"
      type="button"
      aria-label={label}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}

export function StatusPill({
  children,
  tone = "slate",
  icon,
}: {
  children: ReactNode;
  tone?: string;
  icon?: ReactNode;
}): ReactElement {
  return (
    <span className={`status-pill status-pill-${tone}`}>
      {icon}
      {children}
    </span>
  );
}

export function ConditionPill({
  condition,
}: {
  condition: Condition;
}): ReactElement {
  return (
    <StatusPill
      tone={toneForCondition(condition)}
      icon={<CircleDot size={11} strokeWidth={2.5} />}
    >
      {condition.status === "Expected" ? "Expected" : condition.title}
    </StatusPill>
  );
}

export function EmptyState({
  onAddRoot,
  hasRoots,
}: {
  onAddRoot: () => void;
  hasRoots: boolean;
}): ReactElement {
  return (
    <div className="empty-state">
      <div className="empty-state-icon">
        <FolderPlus size={22} />
      </div>
      <div>
        <p className="eyebrow">Start with local evidence</p>
        <h2>
          {hasRoots
            ? "No repositories found in these roots"
            : "Register your first repository root"}
        </h2>
        <p>
          {hasRoots
            ? "Pronto did not find a Git repository beneath the registered folders. Add another root or check the folder contents."
            : "Choose a parent folder and Pronto will discover Git repositories without uploading source or diff content."}
        </p>
      </div>
      <button
        className="button button-primary"
        type="button"
        onClick={onAddRoot}
      >
        <FolderPlus size={16} />
        Add discovery root
      </button>
    </div>
  );
}

export function NoMatchesState({
  onClear,
}: {
  onClear: () => void;
}): ReactElement {
  return (
    <div className="empty-state empty-state-compact">
      <div className="empty-state-icon">
        <SearchX size={22} />
      </div>
      <div>
        <p className="eyebrow">No matching evidence</p>
        <h2>Nothing matches this view.</h2>
        <p>Try a different search or return to the full local portfolio.</p>
      </div>
      <button
        className="button button-secondary"
        type="button"
        onClick={onClear}
      >
        Clear filters
      </button>
    </div>
  );
}
