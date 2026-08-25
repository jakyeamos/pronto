import {
  AlertTriangle,
  Building2,
  Globe2,
  LoaderCircle,
  LockKeyhole,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import type { GitHubVisibility, RemoteRepositorySnapshot } from "../types";

const visibilityOptions: Array<{
  value: GitHubVisibility;
  label: string;
  description: string;
}> = [
  {
    value: "public",
    label: "Public",
    description: "Anyone on the internet can view the code.",
  },
  {
    value: "private",
    label: "Private",
    description: "Only people with explicit access can view the code.",
  },
  {
    value: "internal",
    label: "Internal",
    description:
      "Organization members can view it, subject to organization policy.",
  },
];

function visibilityIcon(visibility: GitHubVisibility): ReactElement {
  if (visibility === "public") return <Globe2 size={16} />;
  if (visibility === "internal") return <Building2 size={16} />;
  return <LockKeyhole size={16} />;
}

function consequenceCopy(visibility: GitHubVisibility): string {
  if (visibility === "public") {
    return "Public repositories expose their code, issues, and pull requests to anyone. GitHub may detach private forks and reset stars or watchers when visibility changes.";
  }
  if (visibility === "internal") {
    return "Internal repositories are visible to organization members subject to organization policy. Existing forks and outside collaborators may be affected.";
  }
  return "Private repositories remove public access. Existing public forks can be detached, and outside collaborators may lose access.";
}

function messageFromCaught(caught: unknown): string {
  if (caught instanceof Error && caught.message) return caught.message;
  if (typeof caught === "string" && caught.trim()) return caught;
  return "Pronto could not change the GitHub repository visibility.";
}

export function GitHubVisibilityDialog({
  repository,
  onCancel,
  onConfirm,
}: {
  repository: RemoteRepositorySnapshot;
  onCancel: () => void;
  onConfirm: (
    targetVisibility: GitHubVisibility,
    confirmation: string,
  ) => Promise<void>;
}): ReactElement {
  const currentVisibility = repository.visibility;
  const [targetVisibility, setTargetVisibility] = useState<GitHubVisibility>(
    currentVisibility ?? "private",
  );
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelButtonRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape" && !isSubmitting) onCancel();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isSubmitting, onCancel]);

  const isConfirmed = confirmation === repository.full_name;
  const canSubmit =
    !isSubmitting &&
    Boolean(currentVisibility) &&
    targetVisibility !== currentVisibility &&
    isConfirmed;

  const handleSubmit = async (): Promise<void> => {
    if (!canSubmit) return;
    setError(null);
    setIsSubmitting(true);
    try {
      await onConfirm(targetVisibility, confirmation);
      onCancel();
    } catch (caught) {
      setError(messageFromCaught(caught));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="confirmation-layer">
      <button
        className="confirmation-scrim"
        type="button"
        aria-label="Cancel GitHub visibility change"
        onClick={onCancel}
        disabled={isSubmitting}
      />
      <section
        className="confirmation-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="github-visibility-dialog-title"
        aria-describedby="github-visibility-dialog-description"
      >
        <div className="confirmation-dialog-header">
          <div className="confirmation-dialog-icon">
            {isSubmitting ? (
              <LoaderCircle className="spin" size={21} />
            ) : (
              <AlertTriangle size={21} />
            )}
          </div>
          <div>
            <p className="eyebrow">Approval-gated provider action</p>
            <h2 id="github-visibility-dialog-title">
              {isSubmitting ? "Changing visibility…" : "Change visibility?"}
            </h2>
          </div>
          {!isSubmitting && (
            <button
              className="icon-button confirmation-close"
              type="button"
              aria-label="Cancel GitHub visibility change"
              onClick={onCancel}
            >
              <X size={16} />
            </button>
          )}
        </div>
        <p
          id="github-visibility-dialog-description"
          className="confirmation-dialog-copy"
        >
          {repository.full_name} is currently{" "}
          <strong>{currentVisibility ?? "unknown"}</strong>. Pronto will
          re-check the repository, require administration permission, submit the
          change through GitHub CLI with the provider consequence
          acknowledgement, and read the result back before it updates local
          evidence.
        </p>
        <label className="visibility-dialog-field">
          <span>New visibility</span>
          <select
            className="visibility-dialog-select"
            value={targetVisibility}
            onChange={(event) =>
              setTargetVisibility(event.target.value as GitHubVisibility)
            }
            disabled={isSubmitting}
          >
            {visibilityOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label} — {option.description}
              </option>
            ))}
          </select>
        </label>
        <div className="confirmation-dialog-facts">
          <div className="confirmation-dialog-fact">
            {visibilityIcon(targetVisibility)}
            <div>
              <strong>
                {visibilityOptions.find(
                  (option) => option.value === targetVisibility,
                )?.label ?? targetVisibility}
              </strong>
              <span>{consequenceCopy(targetVisibility)}</span>
            </div>
          </div>
        </div>
        <label className="visibility-dialog-field">
          <span>
            Type <code>{repository.full_name}</code> to confirm
          </span>
          <input
            className="visibility-dialog-input"
            type="text"
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            placeholder={repository.full_name}
            autoComplete="off"
            spellCheck={false}
            disabled={isSubmitting}
          />
        </label>
        {error && (
          <p className="visibility-dialog-error" role="alert">
            {error}
          </p>
        )}
        <p className="confirmation-dialog-note">
          This is a provider write. Pronto keeps the credential boundary in
          GitHub CLI and records the action audit locally.
        </p>
        <div className="confirmation-dialog-actions">
          <button
            className="button button-secondary"
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            ref={cancelButtonRef}
          >
            Cancel
          </button>
          <button
            className="button button-primary"
            type="button"
            onClick={() => void handleSubmit()}
            disabled={!canSubmit}
          >
            {isSubmitting ? "Applying…" : "Change visibility"}
          </button>
        </div>
      </section>
    </div>
  );
}
