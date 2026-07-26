import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { CheckCircle2, RotateCcw, Save } from "lucide-react";
import type { ReleaseRecipeConfig, ReleaseRecipePreview } from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

const defaultRecipe: ReleaseRecipeConfig = {
  name: "Single repository release",
  validation_commands: [],
  release_commands: [],
  generated_paths: [],
  commit_message: "chore(release): prepare {version}",
};

function statusTone(status: string): string {
  if (
    status === "Passed" ||
    status === "Configured" ||
    status === "Ready for user review"
  ) {
    return "mint";
  }
  if (status === "Blocked") return "coral";
  if (status === "Needs configuration") return "amber";
  return "slate";
}

function lines(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export function ReleaseRecipePanel({
  recipe,
  configuredRecipe,
  candidateVersion,
  confirmedVersion,
  onSave,
  onConfirmVersion,
}: {
  recipe: ReleaseRecipePreview;
  configuredRecipe?: ReleaseRecipeConfig;
  candidateVersion?: string;
  confirmedVersion?: string;
  onSave: (recipe: ReleaseRecipeConfig | null) => Promise<void>;
  onConfirmVersion: (version: string | null) => Promise<void>;
}): ReactElement {
  const [name, setName] = useState(
    configuredRecipe?.name ?? defaultRecipe.name,
  );
  const [validationCommands, setValidationCommands] = useState(
    (configuredRecipe ?? defaultRecipe).validation_commands.join("\n"),
  );
  const [releaseCommands, setReleaseCommands] = useState(
    (configuredRecipe ?? defaultRecipe).release_commands.join("\n"),
  );
  const [generatedPaths, setGeneratedPaths] = useState(
    (configuredRecipe ?? defaultRecipe).generated_paths.join("\n"),
  );
  const [commitMessage, setCommitMessage] = useState(
    configuredRecipe?.commit_message ?? defaultRecipe.commit_message,
  );
  const [isSaving, setIsSaving] = useState(false);
  const [isConfirming, setIsConfirming] = useState(false);

  useEffect(() => {
    const next = configuredRecipe ?? defaultRecipe;
    setName(next.name);
    setValidationCommands(next.validation_commands.join("\n"));
    setReleaseCommands(next.release_commands.join("\n"));
    setGeneratedPaths(next.generated_paths.join("\n"));
    setCommitMessage(next.commit_message);
  }, [configuredRecipe]);

  const save = async (): Promise<void> => {
    if (!name.trim() || !commitMessage.trim()) return;
    setIsSaving(true);
    try {
      await onSave({
        name: name.trim(),
        validation_commands: lines(validationCommands),
        release_commands: lines(releaseCommands),
        generated_paths: lines(generatedPaths),
        commit_message: commitMessage.trim(),
      });
    } finally {
      setIsSaving(false);
    }
  };

  const confirmVersion = async (): Promise<void> => {
    if (!candidateVersion) return;
    setIsConfirming(true);
    try {
      await onConfirmVersion(candidateVersion);
    } finally {
      setIsConfirming(false);
    }
  };

  const clearVersion = async (): Promise<void> => {
    setIsConfirming(true);
    try {
      await onConfirmVersion(null);
    } finally {
      setIsConfirming(false);
    }
  };

  return (
    <div className="release-recipe">
      <div className="release-recipe-overview">
        <div>
          <span>Recipe status</span>
          <strong>{recipe.status}</strong>
        </div>
        <StatusPill tone={statusTone(recipe.status)}>
          {recipe.status}
        </StatusPill>
        <div>
          <span>Candidate</span>
          <strong>{recipe.candidate_version ?? "Unavailable"}</strong>
        </div>
        <div>
          <span>Version</span>
          <strong>{recipe.version_status}</strong>
        </div>
      </div>
      <p className="field-help">
        Configure the commands and exact generated paths that a user can review.
        Commands are never executed by this preview.
      </p>
      {recipe.reasons.length > 0 && (
        <ul className="preparation-reasons">
          {recipe.reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      )}
      <div className="release-recipe-steps">
        {recipe.steps.map((step) => (
          <div className="release-recipe-step" key={step.order}>
            <span className="release-recipe-step-order">{step.order}</span>
            <div>
              <strong>{step.label}</strong>
              <small>{step.detail}</small>
            </div>
            <StatusPill tone={statusTone(step.status)}>
              {step.status}
            </StatusPill>
          </div>
        ))}
      </div>
      <div className="release-version-actions">
        {candidateVersion && confirmedVersion !== candidateVersion && (
          <button
            className="button button-primary"
            type="button"
            onClick={() => void confirmVersion()}
            disabled={isConfirming}
          >
            <CheckCircle2 size={14} />
            {isConfirming ? "Confirming…" : "Confirm " + candidateVersion}
          </button>
        )}
        {confirmedVersion && (
          <button
            className="button button-quiet"
            type="button"
            onClick={() => void clearVersion()}
            disabled={isConfirming}
          >
            <RotateCcw size={14} />
            Clear version confirmation
          </button>
        )}
      </div>
      <form
        className="release-recipe-form"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <label className="field-label">
          Recipe name
          <input
            className="text-input"
            value={name}
            maxLength={80}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label className="field-label">
          Validation commands
          <textarea
            className="text-input release-recipe-textarea"
            rows={3}
            value={validationCommands}
            placeholder={"pnpm typecheck\npnpm lint"}
            onChange={(event) => setValidationCommands(event.target.value)}
          />
          <small className="field-help">One command per line.</small>
        </label>
        <label className="field-label">
          Release commands
          <textarea
            className="text-input release-recipe-textarea"
            rows={3}
            value={releaseCommands}
            placeholder={"pnpm changeset version\npnpm changeset publish"}
            onChange={(event) => setReleaseCommands(event.target.value)}
          />
          <small className="field-help">
            Stored for review; Pronto does not run shell commands.
          </small>
        </label>
        <label className="field-label">
          Exact generated paths
          <textarea
            className="text-input release-recipe-textarea"
            rows={3}
            value={generatedPaths}
            placeholder={"CHANGELOG.md\npackage.json"}
            onChange={(event) => setGeneratedPaths(event.target.value)}
          />
          <small className="field-help">
            Repository-relative paths only; these constrain the future diff.
          </small>
        </label>
        <label className="field-label">
          Commit message template
          <input
            className="text-input"
            value={commitMessage}
            maxLength={160}
            onChange={(event) => setCommitMessage(event.target.value)}
          />
          <small className="field-help">
            Use {"{version}"} for the confirmed candidate version.
          </small>
        </label>
        <div className="release-recipe-actions">
          <button
            className="button button-secondary"
            type="submit"
            disabled={isSaving || !name.trim() || !commitMessage.trim()}
          >
            <Save size={14} />
            {isSaving ? "Saving…" : "Save release recipe"}
          </button>
          {configuredRecipe && (
            <button
              className="button button-quiet"
              type="button"
              onClick={() => void onSave(null)}
              disabled={isSaving}
            >
              Clear recipe
            </button>
          )}
        </div>
      </form>
      <small className="release-recipe-generated">
        Preview generated {formatTime(recipe.generated_at)} · actions performed:{" "}
        {recipe.actions_performed ? "yes" : "no"}
      </small>
    </div>
  );
}
