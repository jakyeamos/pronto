// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  makeGate,
  makeReadiness,
  makeQuality,
  makeRepository,
  noopReport,
  analyticsSnapshot,
  RepositoryRow,
  RepositoryDetailSurface,
  ProjectCompassDetail,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("repository detail surfaces", () => {
  it("renders repository detail as a full page with quality, maturity, QR, and release context", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 3,
        updated_at: "2026-07-28T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 75,
          scored_outcome_count: 3,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "high",
          confidence_percent: 100,
        },
        complete_product: {
          progress_percent: 50,
          scored_outcome_count: 4,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "medium",
          confidence_percent: 60,
        },
        open_blockers: 2,
        open_drift: 1,
        open_blocker_items: [
          {
            outcome_id: "release-preparation",
            outcome_name: "A release candidate can be inspected",
            kind: "verification",
            summary: "Provider-native release proof is missing.",
          },
          {
            outcome_id: "release-safety",
            outcome_name: "Release mutation remains governed",
            kind: "evidence",
            summary: "The release boundary still needs fresh evidence.",
          },
        ],
        open_drift_items: [
          {
            kind: "verification-gap",
            summary:
              "Release evidence trails the intended product finish line.",
            observed_at: "2026-07-28T00:00:00Z",
          },
        ],
        error: null,
      },
      quality: makeQuality({
        gates: [makeGate("build", "Build")],
        ci_readiness: makeReadiness({
          applicable_gate_ids: ["build", "dependency_audit"],
          configured_gate_ids: ["build", "dependency_audit"],
        }),
      }),
      release_rule: {
        name: "Ready to release",
        operator: "AND",
        required_commit_types: [],
        allow_first_release: false,
        required_quality_gates: [{ gate_id: "build", source: "CI" }],
      },
      target_branch: "develop",
      target_branch_configured: true,
      branches: [
        {
          name: "main",
          role: "Production",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
        },
        {
          name: "develop",
          role: "Integration",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
        },
      ],
    });
    const markup = renderToStaticMarkup(
      <RepositoryDetailSurface
        repository={repository}
        analytics={analyticsSnapshot}
        isRefreshing={false}
        onBack={() => undefined}
        onOpenWorkspace={async () => undefined}
        onPrepareRepository={async () => undefined}
        onTargetBranchChange={async () => undefined}
        onLifecycleChange={async () => undefined}
        onCondition={() => undefined}
      />,
    );
    expect(markup).toContain("Back to Portfolio");
    expect(markup).toContain("Target branch");
    expect(markup).toContain("Pronto override · Git default: main");
    expect(markup).toContain(
      "Selecting a branch or refreshing evidence checks existing target evidence first, then runs QR quality and fleet audits in a clean disposable worktree when the target head changed or matching evidence is unavailable; your active workspace is not switched.",
    );
    expect(markup).toContain('aria-label="Target branch for pronto"');
    expect(markup).toContain('aria-label="Refresh target evidence for pronto"');
    expect(markup).toContain(
      '<option value="develop" selected="">develop</option>',
    );
    expect(markup).toContain("/tmp/pronto");
    expect(markup).toContain("Quality gates");
    expect(markup).toContain("Project Compass");
    expect(markup).toContain("Contract valid");
    expect(markup).toContain("Provider-native release proof is missing.");
    expect(markup).toContain(
      "Release evidence trails the intended product finish line.",
    );
    expect(markup).toContain("Verification Gap");
    expect(markup).toContain("MVP");
    expect(markup).toContain("75%");
    expect(markup).toContain("Complete product");
    expect(markup).toContain("Release rule trace");
    expect(markup).toContain("Workspaces");
    expect(markup).toContain("Conditions");
    expect(markup).toContain("Branches");
    expect(markup).toContain("Not scored");
    expect(markup).toContain("Configured");
    expect(markup).toContain("Dependency audit");
    expect(markup).toContain("Awaiting evidence");
    expect(markup).not.toContain("drawer-layer");
    expect(markup).not.toContain("drawer-scrim");
  });

  it("shows a loading state and withholds old quality projections during a branch refresh", () => {
    const repository = makeRepository({
      target_branch: "main",
      target_branch_configured: true,
      branches: [
        {
          name: "main",
          role: "Production",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
          last_commit: "main-commit",
        },
        {
          name: "dev",
          role: "Integration",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
          last_commit: "dev-commit",
        },
      ],
      quality: makeQuality({
        ingestion_status: "Available",
        ingestion_message: "Existing main evidence",
        gates: [makeGate("build", "Build", "Failed", "Fresh")],
      }),
    });

    function Harness() {
      const [isRefreshing, setIsRefreshing] = useState(false);
      return (
        <RepositoryDetailSurface
          repository={repository}
          analytics={analyticsSnapshot}
          isRefreshing={isRefreshing}
          onBack={() => undefined}
          onOpenWorkspace={async () => undefined}
          onPrepareRepository={async () => undefined}
          onTargetBranchChange={async () => setIsRefreshing(true)}
          onLifecycleChange={async () => undefined}
          onCondition={() => undefined}
          onOpenReport={noopReport}
        />
      );
    }

    render(<Harness />);
    fireEvent.change(
      screen.getByRole("combobox", { name: "Target branch for pronto" }),
      { target: { value: "dev" } },
    );

    expect(screen.getByText("Refreshing evidence for dev…")).toBeTruthy();
    expect(
      screen.getByText(
        "Resolving the target head and checking existing evidence. A QR audit runs only when the target head changed or matching evidence is unavailable. Existing evidence is held until the check completes.",
      ),
    ).toBeTruthy();
    expect(screen.getByText("Ingesting dev evidence…")).toBeTruthy();
    expect(
      screen.getByText("Refreshing release remediation evidence…"),
    ).toBeTruthy();
    expect(screen.queryByText("Existing main evidence")).toBeNull();
    expect(
      screen.getByRole("combobox", { name: "Target branch for pronto" }),
    ).toHaveProperty("disabled", true);
  });

  it("refreshes target evidence without requiring a branch value change", () => {
    const repository = makeRepository({
      target_branch: "main",
      target_branch_configured: true,
      branches: [
        {
          name: "main",
          role: "Production",
          role_confidence: "High",
          target_confidence: "High",
          ahead: 0,
          behind: 0,
          integration_state: "No unique commits",
          last_commit: "main-commit",
        },
      ],
    });
    const requestedBranches: string[] = [];

    render(
      <RepositoryDetailSurface
        repository={repository}
        analytics={analyticsSnapshot}
        isRefreshing={false}
        onBack={() => undefined}
        onOpenWorkspace={async () => undefined}
        onPrepareRepository={async () => undefined}
        onTargetBranchChange={async (branch) => {
          requestedBranches.push(branch);
        }}
        onLifecycleChange={async () => undefined}
        onCondition={() => undefined}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: "Refresh target evidence for pronto",
      }),
    );

    expect(requestedBranches).toEqual(["main"]);
  });

  it("makes missing Compass item details explicit for legacy snapshots", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 6,
        updated_at: "2026-07-28T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 79,
          scored_outcome_count: 4,
          covered_pillar_count: 3,
          total_pillar_count: 4,
          confidence: "high",
          confidence_percent: 100,
        },
        complete_product: {
          progress_percent: 75,
          scored_outcome_count: 5,
          covered_pillar_count: 4,
          total_pillar_count: 4,
          confidence: "high",
          confidence_percent: 100,
        },
        open_blockers: 2,
        open_drift: 1,
        open_blocker_items: [],
        open_drift_items: [],
        error: null,
      },
    });

    const markup = renderToStaticMarkup(
      <ProjectCompassDetail repository={repository} />,
    );

    expect(markup).toContain("Contract valid");
    expect(markup).not.toContain(">Ready<");
    expect(markup).toContain(
      "2 blocker descriptions are unavailable in this snapshot",
    );
    expect(markup).toContain(
      "1 drift description is unavailable in this snapshot",
    );
    expect(markup).toContain("Refresh the repository to load the details");
    expect(markup).toContain("Refresh the repository to load the detail");
  });

  it("shows Compass coverage beside progress instead of implying complete evidence", () => {
    const repository = makeRepository({
      project_compass: {
        status: "Ready",
        contract_path: ".project-compass/contract.json",
        revision: 7,
        updated_at: "2026-08-08T00:00:00Z",
        project_name: "Pronto",
        identity: "A local-first portfolio command center",
        audience: "Developers with many active repositories",
        mvp: {
          progress_percent: 50,
          scored_outcome_count: 1,
          covered_pillar_count: 1,
          total_pillar_count: 2,
          confidence: "medium",
          confidence_percent: 60,
        },
        complete_product: {
          progress_percent: 75,
          scored_outcome_count: 3,
          covered_pillar_count: 2,
          total_pillar_count: 2,
          confidence: "high",
          confidence_percent: 100,
        },
        open_blockers: 0,
        open_drift: 0,
        open_blocker_items: [],
        open_drift_items: [],
        error: null,
      },
    });

    const row = renderToStaticMarkup(
      <RepositoryRow
        repository={repository}
        onOpen={() => undefined}
        onCondition={() => undefined}
      />,
    );
    expect(row).toContain("MVP 50% · 1/2 pillars");
    expect(row).toContain(
      'title="Coverage incomplete · 1 scoped outcome · 1/2 pillars covered"',
    );

    const detail = renderToStaticMarkup(
      <ProjectCompassDetail repository={repository} />,
    );
    expect(detail).toContain(
      "Coverage incomplete · 1 scoped outcome · 1/2 pillars covered",
    );
    expect(detail).toContain("3 scoped outcomes · 2/2 pillars covered");
  });
});
