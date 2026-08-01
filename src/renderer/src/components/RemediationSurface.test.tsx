// quality-gate: allow static-ui-test: verifies verified closures leave the ranked active queue while retained evidence and coverage remain visible.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { RemediationAction, RemediationRun } from "../types";
import { RemediationSurface } from "./RemediationSurface";

const noop = async (): Promise<void> => undefined;

function action(): RemediationAction {
  return {
    id: "action-preserve",
    stable_key: "branch_hygiene:dirty:workspace",
    repository_id: "repo-active",
    domain: "branch_hygiene",
    title: "Preserve the dirty workspace",
    summary: "Review and preserve the current coherent slice.",
    severity: "workspace",
    priority: "P1",
    weight: 2,
    status: "open",
    acceptance_criteria: ["The work is intentionally preserved."],
    evidence: [
      {
        source: "Pronto",
        label: "Dirty workspace",
        status: "Dirty",
        freshness: "Fresh",
        observed_at: "2026-07-29T12:00:00Z",
        detail: "The workspace has uncommitted changes.",
      },
    ],
    related_finding_ids: [],
    source_run_id: null,
    updated_at: "2026-07-29T12:00:00Z",
    completed_at: null,
    notes: null,
  };
}

function run(): RemediationRun {
  const activeAction = action();
  return {
    schema_version: "pronto-remediation/v3",
    id: "run-1",
    generated_at: "2026-07-29T13:00:00Z",
    source_refresh_id: "refresh-1",
    status: "completed",
    message: null,
    eligible_repository_ids: ["repo-active", "repo-closed"],
    eligible_repository_paths: ["/tmp/active", "/tmp/closed"],
    refresh_steps: [],
    excluded_repositories: [],
    closures: [
      {
        id: "closure-closed",
        repository_id: "repo-closed",
        repository_name: "closed-repo",
        repository_path: "/tmp/closed",
        plan_id: "plan-closed",
        target_state: "clean_only",
        goal_source: "repository_contract",
        closed_at: "2026-07-29T12:30:00Z",
        source_refresh_id: "refresh-1",
        disposition: "verified",
        summary: "Fresh evidence removed the repository from the queue.",
        resolved_action_count: 3,
        verified_action_count: 3,
        deferred_action_count: 0,
        last_evidence_at: "2026-07-29T12:20:00Z",
      },
    ],
    plans: [
      {
        schema_version: "pronto-remediation/v3",
        id: "plan-active",
        repository_id: "repo-active",
        repository_name: "active-repo",
        repository_path: "/tmp/active",
        generated_at: "2026-07-29T13:00:00Z",
        source_refresh_id: "refresh-1",
        goal: {
          schema_version: "pronto-remediation-goal/v1",
          target_state: "public_release",
          label: "Public release",
          source: "repository_contract",
          confidence: "High",
          reason: "This repository is distributed publicly.",
          contract_path: ".pronto/remediation-goal.json",
          required_gate_ids: ["build", "tests", "secrets_scan"],
          optional_gate_ids: [],
          evidence_max_age_days: 7,
          closure_criteria: ["Fresh release evidence passes."],
          error: null,
        },
        current_stage: "branch_hygiene",
        status: "open",
        progress: {
          verified_weight: 0,
          total_weight: 2,
          deferred_weight: 0,
          percentage: 0,
        },
        coverage: [
          {
            surface: "project_compass",
            label: "Project Compass",
            status: "attention",
            detail: "Status: Missing · blockers: 0 · drift: 0.",
            action_ids: [activeAction.id],
          },
        ],
        tracks: [
          {
            domain: "branch_hygiene",
            label: "Branch hygiene",
            status: "open",
            action_ids: [activeAction.id],
            verified_weight: 0,
            total_weight: 2,
          },
        ],
        actions: [activeAction],
      },
    ],
  };
}

describe("remediation active queue", () => {
  it("renders ranked active work separately from retained closures", () => {
    const markup = renderToStaticMarkup(
      <RemediationSurface
        run={run()}
        repositories={[]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(markup).toContain("Active repository remediation");
    expect(markup).toContain("#1 · active-repo");
    expect(markup).toContain("Public release");
    expect(markup).toContain("repository contract");
    expect(markup).toContain("Goal-specific closure contract");
    expect(markup).toContain("UI tracking coverage");
    expect(markup).toContain("Project Compass");
    expect(markup).toContain("attention");
    expect(markup).toContain("Repositories removed from the active queue");
    expect(markup).toContain("closed-repo");
    expect(markup).toContain("clean only");
  });
});
