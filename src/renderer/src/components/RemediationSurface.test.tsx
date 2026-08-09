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
import type {
  RemediationAction,
  RemediationRun,
  RepositorySnapshot,
} from "../types";
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
        explanation: {
          authority:
            "Advisory only: this explanation orders evidence-backed work but does not authorize Git, provider, publication, release, or pruning mutations.",
          summary:
            "1 ordered remediation phase remains across 1 active action.",
          phases: [
            {
              id: "preserve_and_reconcile",
              title: "Preserve and reconcile repository work",
              summary:
                "Protect active work and make the repository state intentional.",
              status: "open",
              steps: [
                {
                  action_id: activeAction.id,
                  title: activeAction.title,
                  summary: activeAction.summary,
                  status: activeAction.status,
                  priority: activeAction.priority,
                  completion_criteria: activeAction.acceptance_criteria,
                },
              ],
              completion_criterion:
                "Every scoped workspace and branch action is verified.",
            },
          ],
          healthy_surfaces: [
            {
              surface: "quality_evidence",
              label: "Quality evidence",
              status: "clear",
              detail: "Required evidence is fresh.",
            },
          ],
          closure_requirements: [
            "Fresh release evidence passes.",
            "A final scoped refresh reports no open or blocked actions.",
          ],
        },
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

function repositoryWithTarget(): RepositorySnapshot {
  return {
    id: "repo-active",
    name: "tenure",
    path: "/tmp/active",
    branch: "dev",
    default_branch: "main",
    target_branch: "main",
    target_branch_configured: true,
    workspace: {
      branch: "dev",
      last_commit: "dev-head-1234567890",
    },
    branches: [
      {
        name: "main",
        role: "Production",
        role_confidence: "High",
        target_confidence: "High",
        ahead: 0,
        behind: 0,
        integration_state: "Synced",
        last_commit: "target-head-1234567890",
      },
    ],
  } as RepositorySnapshot;
}

function runWithEvidence(
  scanned_branch?: string,
  scanned_commit?: string,
): RemediationRun {
  const targetedRun = run();
  targetedRun.plans[0].actions[0] = {
    ...targetedRun.plans[0].actions[0],
    evidence: targetedRun.plans[0].actions[0].evidence.map((item) => ({
      ...item,
      scanned_branch,
      scanned_commit,
    })),
  };
  return targetedRun;
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
    expect(markup).toContain("Scope exclusions");
    expect(markup).toContain("All registered repositories are eligible");
    expect(markup).not.toContain("Soundscape and Tenure stay out of the plan");
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
    expect(markup).toContain("Remediation path");
    expect(markup).toContain("1 phase");
    expect(markup).toContain("Preserve and reconcile repository work");
    expect(markup).toContain("Preserve the dirty workspace");
    expect(markup).toContain("What done means");
    expect(markup).toContain("Already healthy ·");
    expect(markup).toContain("Required evidence is fresh");
    expect(markup).toContain("What closes this plan");
    expect(markup).toContain("does not authorize Git");
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

  it("keeps remediation evidence scoped to the selected target branch and head", () => {
    const repository = repositoryWithTarget();
    const mismatch = renderToStaticMarkup(
      <RemediationSurface
        run={runWithEvidence("dev", "stale-head-1234567890")}
        repositories={[repository]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(mismatch).toContain("Evidence target");
    expect(mismatch).toContain("main @ target-h");
    expect(mismatch).toContain("Target evidence unavailable");
    expect(mismatch).toContain("Raw remediation evidence");
    expect(mismatch).toContain(
      "Branch/head provenance does not match the selected target.",
    );

    const matching = renderToStaticMarkup(
      <RemediationSurface
        run={runWithEvidence("main", "target-head-1234567890")}
        repositories={[repository]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(matching).toContain("Fresh target evidence");
    expect(matching).toContain("Evidence");
    expect(matching).not.toContain("Target evidence unavailable");
    expect(matching).not.toContain("Raw remediation evidence");

    const staleRepository = {
      ...repository,
      branch: "dev",
      target_branch: "dev",
      workspace: {
        branch: "dev",
        last_commit: "target-head-1234567890",
      },
      branches: [
        {
          name: "dev",
          role: "Development",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "Synced",
          last_commit: "target-head-1234567890",
        },
      ],
    } as RepositorySnapshot;
    const stale = renderToStaticMarkup(
      <RemediationSurface
        run={runWithEvidence("dev", "stale-head-1234567890")}
        repositories={[staleRepository]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(stale).toContain("Evidence target");
    expect(stale).toContain("dev @ target-h");
    expect(stale).toContain("Stale branch evidence");
    expect(stale).toContain(
      "The selected branch matches, but this evidence predates the selected target head.",
    );
    expect(stale).not.toContain("Target evidence unavailable");
    expect(stale).not.toContain("Raw remediation evidence");

    const ambiguous = renderToStaticMarkup(
      <RemediationSurface
        run={runWithEvidence()}
        repositories={[repository]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(ambiguous).toContain("Unscoped evidence");
    expect(ambiguous).toContain(
      "Branch/head provenance is not recorded for this evidence.",
    );
    expect(ambiguous).not.toContain("Target evidence unavailable");
    expect(ambiguous).not.toContain("Raw remediation evidence");
  });

  it("renders every backend-defined remediation phase without a four-phase ceiling", () => {
    const expandedRun = run();
    const basePhase = expandedRun.plans[0].explanation.phases[0];
    expandedRun.plans[0].explanation.summary =
      "5 ordered remediation phases remain across 5 active actions.";
    expandedRun.plans[0].explanation.phases = Array.from(
      { length: 5 },
      (_, index) => ({
        ...basePhase,
        id: `phase-${index + 1}`,
        title: `Repository phase ${index + 1}`,
        steps: basePhase.steps.map((step) => ({
          ...step,
          action_id: `${step.action_id}-${index + 1}`,
        })),
      }),
    );

    const markup = renderToStaticMarkup(
      <RemediationSurface
        run={expandedRun}
        repositories={[]}
        isRefreshing={false}
        onRefresh={noop}
        onExport={noop}
        onUpdateStatus={noop}
        onOpenRepository={() => undefined}
      />,
    );

    expect(markup).toContain("5 phases remaining");
    expect(markup).toContain("Phase 5");
    expect(markup).toContain("Repository phase 5");
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
