import {
  AlertTriangle,
  FolderSearch,
  LoaderCircle,
  ShieldCheck,
  X,
} from "lucide-react";
import { useEffect, useRef } from "react";
import type { ReactElement } from "react";

export function RefreshConfirmationDialog({
  rootCount,
  repositoryCount,
  isRefreshing,
  onCancel,
  onConfirm,
}: {
  rootCount: number;
  repositoryCount: number;
  isRefreshing: boolean;
  onCancel: () => void;
  onConfirm: () => Promise<void>;
}): ReactElement {
  const cancelButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (isRefreshing) return;
    cancelButtonRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isRefreshing, onCancel]);

  const repositoryLabel =
    repositoryCount === 1 ? "registered repository" : "registered repositories";
  const rootLabel = rootCount === 1 ? "registered root" : "registered roots";

  return (
    <div className="confirmation-layer">
      <button
        className="confirmation-scrim"
        type="button"
        aria-label="Cancel local refresh"
        onClick={onCancel}
        disabled={isRefreshing}
      />
      <section
        className="confirmation-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="refresh-dialog-title"
        aria-describedby="refresh-dialog-description"
      >
        <div className="confirmation-dialog-header">
          <div className="confirmation-dialog-icon">
            {isRefreshing ? (
              <LoaderCircle className="spin" size={21} />
            ) : (
              <AlertTriangle size={21} />
            )}
          </div>
          <div>
            <p className="eyebrow">Portfolio-wide operation</p>
            <h2 id="refresh-dialog-title">
              {isRefreshing
                ? "Refreshing local evidence…"
                : "Refresh local evidence?"}
            </h2>
          </div>
          {!isRefreshing && (
            <button
              className="icon-button confirmation-close"
              type="button"
              aria-label="Cancel local refresh"
              onClick={onCancel}
            >
              <X size={16} />
            </button>
          )}
        </div>
        <p id="refresh-dialog-description" className="confirmation-dialog-copy">
          {isRefreshing
            ? "Pronto is scanning the registered roots now. Keep the app open while it reads the local portfolio."
            : "This can take several minutes for a large portfolio. Pronto will scan every registered discovery root before updating the snapshot."}
        </p>
        <div className="confirmation-dialog-facts">
          <div className="confirmation-dialog-fact">
            <FolderSearch size={16} />
            <div>
              <strong>
                {repositoryCount} {repositoryLabel} · {rootCount} {rootLabel}
              </strong>
              <span>Git state, branches, worktrees, and local conditions</span>
            </div>
          </div>
          <div className="confirmation-dialog-fact">
            <ShieldCheck size={16} />
            <div>
              <strong>Reads existing evidence only</strong>
              <span>
                Quality-runner reports and the configured maturity audit are
                reimported
              </span>
            </div>
          </div>
        </div>
        {!isRefreshing && (
          <p className="confirmation-dialog-note">
            No tests, builds, QR scans, audits, fetches, or file changes will be
            started.
          </p>
        )}
        <div className="confirmation-dialog-actions">
          <button
            className="button button-secondary"
            type="button"
            onClick={onCancel}
            disabled={isRefreshing}
            ref={cancelButtonRef}
            autoFocus={!isRefreshing}
          >
            Cancel
          </button>
          <button
            className="button button-primary"
            type="button"
            onClick={() => void onConfirm()}
            disabled={isRefreshing}
          >
            {isRefreshing && <LoaderCircle className="spin" size={15} />}
            {isRefreshing ? "Scanning…" : "Start refresh"}
          </button>
        </div>
      </section>
    </div>
  );
}
