import type { ReactElement } from "react";
import { AlertTriangle, BellOff, Check, Clock3, X } from "lucide-react";
import type { Condition, RepositorySnapshot } from "../types";
import { ConditionPill, formatTime, IconButton } from "./ConsolePrimitives";

export function EvidenceDrawer({
  repository,
  condition,
  onClose,
  onExpected,
}: {
  repository: RepositorySnapshot;
  condition: Condition;
  onClose: () => void;
  onExpected: () => void;
}): ReactElement {
  const isExpected = condition.status === "Expected";
  return (
    <div className="drawer-layer" role="presentation">
      <button
        className="drawer-scrim"
        aria-label="Close evidence"
        type="button"
        onClick={onClose}
      />
      <aside
        className="evidence-drawer"
        aria-label={`${condition.title} evidence`}
      >
        <div className="drawer-header">
          <div>
            <p className="eyebrow">Why this is here</p>
            <h2>{condition.title}</h2>
          </div>
          <IconButton label="Close evidence" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        <div className="evidence-hero">
          <ConditionPill condition={condition} />
          <p>{condition.summary}</p>
          <span>Repository · {repository.name}</span>
        </div>
        <div className="evidence-block">
          <h3>Rule</h3>
          <p>{condition.rule}</p>
        </div>
        <div className="evidence-block">
          <h3>Evidence</h3>
          <div className="evidence-list">
            {condition.evidence.map((item) => (
              <div className="evidence-row" key={`${item.label}-${item.value}`}>
                <span>{item.label}</span>
                <strong>{item.value || "Not available"}</strong>
                <small>
                  {item.source} · {formatTime(item.observed_at)}
                </small>
              </div>
            ))}
          </div>
        </div>
        <div className="evidence-block">
          <h3>Missing or bounded</h3>
          {condition.missing.length === 0 ? (
            <p className="evidence-positive">
              <Check size={15} />
              No missing facts recorded for this classification.
            </p>
          ) : (
            <ul className="evidence-missing">
              {condition.missing.map((item) => (
                <li key={item}>
                  <AlertTriangle size={14} />
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
        {condition.freshness && (
          <div className="freshness-note">
            <Clock3 size={15} />
            <span>
              <strong>Freshness</strong>
              {condition.freshness}
            </span>
          </div>
        )}
        <div className="drawer-footer">
          <button
            className="button button-secondary"
            type="button"
            onClick={onExpected}
          >
            <BellOff size={15} />
            {isExpected
              ? "Return to active queue"
              : "Mark current state expected"}
          </button>
          <span className="drawer-footnote">
            Expected state is attached to this exact evidence fingerprint.
          </span>
        </div>
      </aside>
    </div>
  );
}
