import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Save, X } from "lucide-react";
import { AiSummaryPreview } from "./AiSummaryPreview";
import { ReleaseRecipePanel } from "./ReleaseRecipePanel";
import type {
  AiPayloadPreview,
  ReleaseRuleConfig,
  ReleaseRecipeConfig,
  RepositoryPreparation,
  RepositorySnapshot,
} from "../types";
import { formatTime, IconButton, StatusPill } from "./ConsolePrimitives";

function PreparationStatus({ value }: { value: string }): ReactElement {
  return (
    <StatusPill
      tone={
        value === "Evidence ready"
          ? "mint"
          : value === "Blocked"
            ? "coral"
            : "amber"
      }
    >
      {value}
    </StatusPill>
  );
}

function PreparationEvidence({
  items,
}: {
  items: RepositoryPreparation["pull_request"]["evidence"];
}): ReactElement {
  return (
    <div className="preparation-evidence">
      {items.map((item) => (
        <div
          className="preparation-evidence-row"
          key={`${item.label}-${item.value}`}
        >
          <span>{item.label}</span>
          <strong>{item.value}</strong>
          <small>
            {item.source} · {formatTime(item.observed_at)}
          </small>
        </div>
      ))}
    </div>
  );
}

function PreparationReasons({
  reasons,
}: {
  reasons: string[];
}): ReactElement | null {
  if (reasons.length === 0) return null;
  return (
    <ul className="preparation-reasons">
      {reasons.map((reason) => (
        <li key={reason}>{reason}</li>
      ))}
    </ul>
  );
}

function traceTone(status: string): string {
  if (status === "Passed") return "mint";
  if (status === "Failed") return "coral";
  return "amber";
}

function ReleaseRuleEditor({
  rule,
  onSave,
}: {
  rule?: ReleaseRuleConfig;
  onSave: (rule: ReleaseRuleConfig | null) => Promise<void>;
}): ReactElement {
  const [name, setName] = useState(rule?.name ?? "Release threshold");
  const [operator, setOperator] = useState(rule?.operator ?? "AND");
  const [minCommits, setMinCommits] = useState(
    rule?.min_commits?.toString() ?? "",
  );
  const [minElapsedDays, setMinElapsedDays] = useState(
    rule?.min_elapsed_days?.toString() ?? "",
  );
  const [commitTypes, setCommitTypes] = useState(
    rule?.required_commit_types.join(", ") ?? "",
  );
  const [allowFirstRelease, setAllowFirstRelease] = useState(
    rule?.allow_first_release ?? false,
  );
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    setName(rule?.name ?? "Release threshold");
    setOperator(rule?.operator ?? "AND");
    setMinCommits(rule?.min_commits?.toString() ?? "");
    setMinElapsedDays(rule?.min_elapsed_days?.toString() ?? "");
    setCommitTypes(rule?.required_commit_types.join(", ") ?? "");
    setAllowFirstRelease(rule?.allow_first_release ?? false);
  }, [rule]);

  const normalizedTypes = commitTypes
    .split(",")
    .map((value) => value.trim().toLowerCase())
    .filter(Boolean);
  const hasClause = Boolean(
    minCommits.trim() || minElapsedDays.trim() || normalizedTypes.length > 0,
  );

  const save = async (): Promise<void> => {
    if (!name.trim() || !hasClause) return;
    setIsSaving(true);
    try {
      const parsedCommits = Number.parseInt(minCommits, 10);
      const parsedElapsedDays = Number.parseInt(minElapsedDays, 10);
      await onSave({
        name: name.trim(),
        operator,
        min_commits:
          Number.isFinite(parsedCommits) && parsedCommits > 0
            ? parsedCommits
            : undefined,
        min_elapsed_days:
          Number.isFinite(parsedElapsedDays) && parsedElapsedDays > 0
            ? parsedElapsedDays
            : undefined,
        required_commit_types: normalizedTypes,
        allow_first_release: allowFirstRelease,
      });
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <form
      className="release-rule-form"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label className="field-label">
        Rule name
        <input
          className="text-input"
          value={name}
          maxLength={80}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="release-rule-grid">
        <label className="field-label">
          Combine clauses
          <select
            className="text-input"
            value={operator}
            onChange={(event) => setOperator(event.target.value)}
          >
            <option value="AND">All clauses (AND)</option>
            <option value="OR">Any clause (OR)</option>
          </select>
        </label>
        <label className="field-label">
          Minimum commits
          <input
            className="text-input"
            type="number"
            min={1}
            value={minCommits}
            placeholder="Optional"
            onChange={(event) => setMinCommits(event.target.value)}
          />
        </label>
        <label className="field-label">
          Minimum elapsed days
          <input
            className="text-input"
            type="number"
            min={1}
            value={minElapsedDays}
            placeholder="Optional"
            onChange={(event) => setMinElapsedDays(event.target.value)}
          />
        </label>
      </div>
      <label className="field-label">
        Commit types present
        <input
          className="text-input"
          value={commitTypes}
          placeholder="feat, fix, perf"
          onChange={(event) => setCommitTypes(event.target.value)}
        />
        <small className="field-help">
          Use conventional types: breaking, feat, fix, perf, docs, refactor,
          test, or chore.
        </small>
      </label>
      <label className="checkbox-label">
        <input
          type="checkbox"
          checked={allowFirstRelease}
          onChange={(event) => setAllowFirstRelease(event.target.checked)}
        />
        <span>Allow this rule to evaluate without a published baseline</span>
      </label>
      <div className="release-rule-actions">
        <button
          className="button button-primary"
          type="submit"
          disabled={isSaving || !name.trim() || !hasClause}
        >
          <Save size={14} />
          {isSaving ? "Saving…" : "Save deterministic rule"}
        </button>
        {rule && (
          <button
            className="button button-quiet"
            type="button"
            onClick={() => void onSave(null)}
            disabled={isSaving}
          >
            Clear rule
          </button>
        )}
      </div>
    </form>
  );
}

export function PreparationDrawer({
  repository,
  preparation,
  onClose,
  onSaveReleaseRule,
  onSaveReleaseRecipe,
  onConfirmReleaseVersion,
  onSaveAiPermission,
  onPreviewAiSummary,
}: {
  repository: RepositorySnapshot;
  preparation: RepositoryPreparation;
  onClose: () => void;
  onSaveReleaseRule: (rule: ReleaseRuleConfig | null) => Promise<void>;
  onSaveReleaseRecipe: (recipe: ReleaseRecipeConfig | null) => Promise<void>;
  onConfirmReleaseVersion: (version: string | null) => Promise<void>;
  onSaveAiPermission: (permission: string) => Promise<void>;
  onPreviewAiSummary: () => Promise<AiPayloadPreview>;
}): ReactElement {
  const pullRequest = preparation.pull_request;
  const release = preparation.release;
  return (
    <div className="drawer-layer drawer-layer-front" role="presentation">
      <button
        className="drawer-scrim"
        aria-label="Close preparation preview"
        type="button"
        onClick={onClose}
      />
      <aside
        className="detail-drawer preparation-drawer"
        aria-label="Preparation preview"
      >
        <div className="drawer-header">
          <div>
            <p className="eyebrow">Read-only preparation preview</p>
            <h2>{repository.name}</h2>
          </div>
          <IconButton label="Close preparation preview" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        <p className="drawer-path">
          No PR, branch, worktree, commit, push, or release publication is
          performed.
        </p>

        <div className="drawer-section">
          <div className="drawer-section-title">
            <div>
              <h3>Release recipe preview</h3>
              <small>
                Version confirmation and a deterministic handoff plan with every
                mutating step held for user review.
              </small>
            </div>
            <StatusPill
              tone={preparation.recipe.status === "Blocked" ? "coral" : "amber"}
            >
              {preparation.recipe.status}
            </StatusPill>
          </div>
          <ReleaseRecipePanel
            recipe={preparation.recipe}
            configuredRecipe={repository.release_recipe}
            candidateVersion={release.candidate_version}
            confirmedVersion={repository.confirmed_release_version}
            onSave={onSaveReleaseRecipe}
            onConfirmVersion={onConfirmReleaseVersion}
          />
        </div>

        <div className="drawer-section">
          <div className="drawer-section-title">
            <div>
              <h3>Pull request evidence</h3>
              <small>
                Exact head, base, push state, and provider uncertainty.
              </small>
            </div>
            <PreparationStatus value={pullRequest.status} />
          </div>
          <div className="preparation-fact-grid">
            <div>
              <span>Head</span>
              <strong>{pullRequest.head_branch}</strong>
            </div>
            <div>
              <span>Base</span>
              <strong>{pullRequest.base_branch ?? "Unknown"}</strong>
            </div>
            <div>
              <span>Unique commits</span>
              <strong>{pullRequest.commit_count}</strong>
            </div>
            <div>
              <span>Workspace</span>
              <strong>{pullRequest.dirty ? "Dirty" : "Clean"}</strong>
            </div>
            <div>
              <span>Push state</span>
              <strong>{pullRequest.upstream ?? "No upstream"}</strong>
            </div>
            <div>
              <span>Provider</span>
              <strong>{pullRequest.provider_state}</strong>
            </div>
          </div>
          <PreparationReasons reasons={pullRequest.reasons} />
          {pullRequest.existing_pull_request && (
            <div className="preparation-existing">
              <span>Existing pull request</span>
              <strong>
                #{pullRequest.existing_pull_request.number} ·{" "}
                {pullRequest.existing_pull_request.title}
              </strong>
              <small>
                Checks: {pullRequest.checks_state} · Reviews:{" "}
                {pullRequest.reviews_state} · Mergeability:{" "}
                {pullRequest.mergeability}
              </small>
              {pullRequest.existing_pull_request.html_url && (
                <a
                  href={pullRequest.existing_pull_request.html_url}
                  target="_blank"
                  rel="noreferrer"
                >
                  Open on GitHub
                </a>
              )}
            </div>
          )}
          {!pullRequest.existing_pull_request && (
            <div className="preparation-existing">
              <span>Pull request snapshot</span>
              <strong>Not available</strong>
              <small>
                Checks, reviews, and mergeability remain unknown until the
                provider supplies a snapshot.
              </small>
            </div>
          )}
          <PreparationEvidence items={pullRequest.evidence} />
        </div>

        <div className="drawer-section">
          <div className="drawer-section-title">
            <div>
              <h3>Release evidence</h3>
              <small>
                Published baseline, deterministic commit grouping, and candidate
                version.
              </small>
            </div>
            <PreparationStatus value={release.status} />
          </div>
          <div className="preparation-fact-grid">
            <div>
              <span>Target</span>
              <strong>{release.target_branch ?? "Unknown"}</strong>
            </div>
            <div>
              <span>Baseline</span>
              <strong>
                {release.baseline?.tag ?? release.baseline_status}
              </strong>
            </div>
            <div>
              <span>Commits since baseline</span>
              <strong>{release.commits_since_baseline.length}</strong>
            </div>
            <div>
              <span>Rule</span>
              <strong>{release.rule_status}</strong>
            </div>
            <div>
              <span>Candidate bump</span>
              <strong>{release.candidate_bump ?? "Unavailable"}</strong>
            </div>
            <div>
              <span>Candidate version</span>
              <strong>{release.candidate_version ?? "Unavailable"}</strong>
            </div>
          </div>
          <PreparationReasons reasons={release.reasons} />
          {release.rule_trace.length > 0 && (
            <div className="preparation-trace">
              {release.rule_trace.map((trace) => (
                <div className="preparation-trace-row" key={trace.label}>
                  <span>{trace.label}</span>
                  <StatusPill tone={traceTone(trace.status)}>
                    {trace.status}
                  </StatusPill>
                  <strong>{trace.value}</strong>
                  <small>{trace.source}</small>
                </div>
              ))}
            </div>
          )}
          <div className="preparation-existing">
            <span>Version confirmation</span>
            <strong>{release.version_status}</strong>
          </div>
          {release.notes.length > 0 && (
            <div className="preparation-notes">
              {release.notes.map((section) => (
                <div key={section.category}>
                  <strong>{section.category}</strong>
                  {section.commits.map((commit) => (
                    <span key={commit.sha}>
                      {commit.sha.slice(0, 7)} · {commit.subject}
                    </span>
                  ))}
                </div>
              ))}
            </div>
          )}
          <PreparationEvidence items={release.evidence} />
        </div>

        <div className="drawer-section">
          <div className="drawer-section-title">
            <div>
              <h3>Deterministic release rule</h3>
              <small>
                Configure local clauses; Pronto never turns an unconfigured rule
                into a release recommendation.
              </small>
            </div>
            <StatusPill tone={repository.release_rule ? "mint" : "slate"}>
              {repository.release_rule ? "Configured" : "Not configured"}
            </StatusPill>
          </div>
          <ReleaseRuleEditor
            rule={repository.release_rule}
            onSave={onSaveReleaseRule}
          />
        </div>

        <div className="drawer-section">
          <div className="drawer-section-title">
            <div>
              <h3>Optional AI summary payload</h3>
              <small>
                Preview committed evidence locally before any future provider
                request. AI cannot choose readiness, version, or actions.
              </small>
            </div>
            <StatusPill
              tone={repository.ai_permission === "Disabled" ? "slate" : "amber"}
            >
              {repository.ai_permission}
            </StatusPill>
          </div>
          <AiSummaryPreview
            permission={repository.ai_permission}
            onSavePermission={onSaveAiPermission}
            onPreview={onPreviewAiSummary}
          />
        </div>

        <div className="drawer-footer">
          <StatusPill tone="slate">
            Generated {formatTime(preparation.generated_at)}
          </StatusPill>
          <button
            className="button button-secondary"
            type="button"
            onClick={onClose}
          >
            Close preview
          </button>
        </div>
      </aside>
    </div>
  );
}
