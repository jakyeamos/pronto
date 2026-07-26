import type { ReactElement } from "react";
import { X } from "lucide-react";
import type { RepositoryPreparation, RepositorySnapshot } from "../types";
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

export function PreparationDrawer({
  repository,
  preparation,
  onClose,
}: {
  repository: RepositorySnapshot;
  preparation: RepositoryPreparation;
  onClose: () => void;
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
