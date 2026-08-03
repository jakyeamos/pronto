// @vitest-environment happy-dom
// quality-gate: allow static-ui-test: verifies verified closures leave the ranked active queue while retained evidence and coverage remain visible.
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { RemediationAction, RemediationRun } from "../types";
import { RemediationSurface } from "./RemediationSurface";

const noop = async (): Promise<void> => undefined;

afterEach(cleanup);

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
        target_state: "active_maintained",
        goal_source: "repository_contract",
        maturity_policy: {
          minimum_closure_score: 3,
          ideal_score: 4,
          scoring_owner: "Quality Runner canonical maturity feed",
          improvement_rule:
            "Continue material, evidence-backed improvements toward 4.0/4 when applicable.",
          integrity_rule:
            "Do not add or accept superficial documentation solely to raise the score.",
        },
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
          maturity_policy: {
            minimum_closure_score: 3,
            ideal_score: 4,
            scoring_owner: "Quality Runner canonical maturity feed",
            improvement_rule:
              "Continue material, evidence-backed improvements toward 4.0/4 when applicable.",
            integrity_rule:
              "Do not add or accept superficial documentation solely to raise the score.",
          },
          error: null,
        },
        current_stage: "branch_hygiene",
        status: "open",
        integration_only_remaining: false,
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

    expect(markup).toContain("Refresh state");
    expect(markup).toContain("Actions to work");
    expect(markup).toContain("Run full refresh");
    expect(markup).toContain("Active repository remediation");
    expect(markup).toContain("#1 · active-repo");
    expect(markup).toContain("Public release");
    expect(markup).toContain("repository contract");
    expect(markup).toContain("Goal-specific closure contract");
    expect(markup).toContain("Maturity 3.0/4 minimum");
    expect(markup).toContain("4.0/4 evidence-backed ideal");
    expect(markup).toContain("Do not add or accept superficial documentation");
    expect(markup).toContain("UI tracking coverage");
    expect(markup).toContain("Project Compass");
    expect(markup).toContain("attention");
    expect(markup).toContain("Repositories removed from the active queue");
    expect(markup).toContain("maturity 3.0/4 minimum · 4.0/4 ideal");
    expect(markup).toContain("closed-repo");
    expect(markup).toContain("active maintained");
  });

  it("renders the backend weighted percentage without replacing it with zero", () => {
    const weightedRun = run();
    weightedRun.plans[0].progress = {
      verified_weight: 4,
      total_weight: 47,
      deferred_weight: 6,
      percentage: 9,
    };

    const markup = renderToStaticMarkup(
      <RemediationSurface
        run={weightedRun}
        repositories={[]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(markup).toContain("9%");
    expect(markup).toContain("4/47 points");
  });

  it("separates active actions from retained verified history in the queue row", () => {
    const mixedRun = run();
    const verifiedAction = {
      ...action(),
      id: "action-verified",
      stable_key: "product_truth:resolved",
      domain: "product_truth",
      status: "verified" as const,
      completed_at: "2026-07-29T12:30:00Z",
    };
    mixedRun.plans[0].actions.push(verifiedAction);

    const markup = renderToStaticMarkup(
      <RemediationSurface
        run={mixedRun}
        repositories={[]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(markup).toContain("1 active · 1 verified");
    expect(markup).not.toContain("2 actions");
  });

  it("reveals and focuses plan detail, then restores row focus when closed", async () => {
    render(
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

    const planRow = screen.getByRole("button", { name: /#1 · active-repo/ });
    fireEvent.click(planRow);

    const detail = document.getElementById("remediation-plan-detail");
    expect(
      detail?.classList.contains("remediation-plan-detail-panel-open"),
    ).toBe(true);
    await waitFor(() => expect(document.activeElement).toBe(detail));

    fireEvent.click(
      screen.getAllByRole("button", {
        name: "Close remediation detail",
      })[1],
    );

    expect(
      detail?.classList.contains("remediation-plan-detail-panel-open"),
    ).toBe(false);
    expect(document.activeElement).toBe(planRow);
  });
});
