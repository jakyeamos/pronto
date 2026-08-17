// quality-gate: allow static-ui-test: verifies the showcase ranking, excluded-work boundary, and missing-contract states remain explicit in the rendered surface.
// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type {
  ShowcasePortfolioSnapshot,
  ShowcaseProjectSnapshot,
} from "../types";
import { ShowcaseSurface } from "./ShowcaseSurface";

afterEach(cleanup);

function project(
  name: string,
  eligibility: ShowcaseProjectSnapshot["public_eligibility"],
  lane: ShowcaseProjectSnapshot["lane"],
): ShowcaseProjectSnapshot {
  return {
    repository_name: name,
    display_name: name,
    repository_id: null,
    repository_path: null,
    registration_status: "unregistered",
    public_eligibility: eligibility,
    disposition_source:
      eligibility === "private_client"
        ? "GitHub profile: Private / Client Product Work"
        : "Portfolio audit",
    work_disposition:
      eligibility === "private_client"
        ? "private_client"
        : "targeted_gap_closure",
    work_disposition_summary:
      eligibility === "private_client"
        ? "Private audit context only."
        : "The product exists; close one bounded demo and evidence path.",
    next_step_category: "demo_integration",
    product_readiness: {
      status: "assessed",
      score: 4.5,
      evidence: "reviewed",
    },
    demo_materials: {
      status: "assessed",
      score: eligibility === "private_client" ? 4.8 : 1.2,
      evidence: "reviewed",
    },
    career_signal: {
      status: "assessed",
      score: 5,
      evidence: "reviewed",
    },
    showcase_score: 3.18,
    priority_score: eligibility === "private_client" ? null : 4.67,
    lane,
    publishable: false,
    blockers: [],
    missing_materials: ["public no-auth project page"],
    next_step:
      eligibility === "private_client"
        ? "Keep private and never add it to the public showcase queue."
        : "Capture one safe end-to-end task.",
  };
}

function snapshot(): ShowcasePortfolioSnapshot {
  const unassessed = project("Unreviewed Repo", "unknown", "unknown");
  unassessed.product_readiness = {
    status: "unknown",
    score: null,
    evidence: "not assessed",
  };
  unassessed.demo_materials = {
    status: "unknown",
    score: null,
    evidence: "not assessed",
  };
  unassessed.career_signal = {
    status: "unknown",
    score: null,
    evidence: "not assessed",
  };
  unassessed.work_disposition = "unknown";
  unassessed.work_disposition_summary =
    "No reviewed Showcase work disposition exists.";
  unassessed.next_step_category = "evidence";
  unassessed.showcase_score = null;
  unassessed.priority_score = null;
  return {
    schema_version: "pronto-showcase/v2",
    status: "Ready",
    contract_path: ".pronto/showcase-goal.json",
    reviewed_at: "2026-08-12T00:00:00Z",
    quality_bar_source: "Authenticated Handshake AI Showcase audit",
    goal: {
      target_publishable_demo_count: 5,
      publishable_demo_count: 1,
      remaining_demo_count: 4,
      status: "In progress",
    },
    scoring: {
      product_weight: 0.6,
      materials_weight: 0.4,
      priority_career_weight: 0.5,
      priority_product_weight: 0.3,
      priority_materials_gap_weight: 0.2,
      publishable_product_minimum: 3.5,
      publishable_materials_minimum: 4,
    },
    public_queue: ["Mac Control"],
    private_client_count: 1,
    projects: [
      project("Mac Control", "public_showcase", "create_materials"),
      project("CrimClock", "private_client", "private_client"),
      project("Dotfiles", "not_applicable", "not_applicable"),
      unassessed,
    ],
    error: null,
  };
}

describe("ShowcaseSurface", () => {
  it("ranks eligible work while hiding client and not-applicable entries", () => {
    render(
      <ShowcaseSurface
        showcase={snapshot()}
        repositories={[]}
        onOpenRepository={() => undefined}
      />,
    );

    expect(
      screen.getByText("Create materials for every showcase project"),
    ).toBeTruthy();
    expect(screen.getByText("Full showcase goal")).toBeTruthy();
    expect(screen.getByText("Mac Control")).toBeTruthy();
    expect(screen.getByText("Targeted gap closure")).toBeTruthy();
    expect(screen.getByText(/Demo integration:/)).toBeTruthy();
    expect(screen.getByText("Unreviewed Repo")).toBeTruthy();
    expect(screen.queryByText("CrimClock")).toBeNull();
    expect(screen.queryByText("Dotfiles")).toBeNull();
    expect(
      screen.getByText("Readiness · 60% product · 40% materials"),
    ).toBeTruthy();
    expect(
      document.querySelectorAll("details.showcase-score-disclosure"),
    ).toHaveLength(4);
    expect(screen.getAllByText("Product readiness")).toHaveLength(4);
    expect(screen.getAllByText("Materials gap")).toHaveLength(2);
    expect(screen.getByText("1 need evidence before ranking")).toBeTruthy();
  });

  it("surfaces a missing contract without inventing scores", () => {
    const missing = snapshot();
    missing.status = "Missing";
    missing.projects = [];
    missing.public_queue = [];

    render(
      <ShowcaseSurface
        showcase={missing}
        repositories={[]}
        onOpenRepository={() => undefined}
      />,
    );

    expect(screen.getByText("Demo readiness is missing")).toBeTruthy();
    expect(
      screen.getByText(/Add one \.pronto\/showcase-goal\.json/),
    ).toBeTruthy();
  });

  it("renders every DevOps tooling sprint target from the Showcase projection", () => {
    const targetNames = [
      "quality-lens",
      "debug-trail",
      "quality-setup",
      "rule-lab",
      "evidence-replay",
      "workflow-gateboard",
      "failure-capsule",
      "change-radius",
      "behavior-coverage-atlas",
      "automation-flight-recorder",
      "remediation-canvas",
      "contract-watch",
      "review-attention-map",
      "review-sandbox",
      "change-integration-simulator",
      "deletion-proof-workbench",
      "readiness-inspector",
      "fleet-radar",
    ];
    const ready = snapshot();
    ready.goal = {
      target_publishable_demo_count: 34,
      publishable_demo_count: 0,
      remaining_demo_count: 34,
      status: "In progress",
    };
    ready.projects = targetNames.map((name) =>
      project(
        name,
        "public_showcase",
        name === "rule-lab" ? "create_materials" : "product_first",
      ),
    );

    render(
      <ShowcaseSurface
        showcase={ready}
        repositories={[]}
        onOpenRepository={() => undefined}
      />,
    );

    for (const name of targetNames) {
      expect(screen.getByText(name)).toBeTruthy();
    }
    expect(screen.getAllByText("18").length).toBeGreaterThan(0);
  });

  it("opens the Quality Runner case as an ordered, bounded evidence story", () => {
    const qualityRunner = project(
      "quality-runner",
      "public_showcase",
      "create_materials",
    );
    qualityRunner.display_name = "Quality Runner";
    const ready = snapshot();
    ready.projects.unshift(qualityRunner);

    render(
      <ShowcaseSurface
        showcase={ready}
        repositories={[]}
        onOpenRepository={() => undefined}
      />,
    );

    fireEvent.click(screen.getByText("View Tenure case study"));

    expect(
      screen.getByRole("heading", {
        name: "4,022 findings, driven by what this codebase values.",
      }),
    ).toBeTruthy();
    expect(screen.getByText("8 of 12 packs selected")).toBeTruthy();
    expect(screen.getByText("50 coverage entries")).toBeTruthy();
    expect(
      screen.getByRole("heading", {
        name: "The same standards that guide your agents can audit what they produce.",
      }),
    ).toBeTruthy();
    expect(screen.getByText("What this means for your team")).toBeTruthy();
    expect(screen.getByText("Make the failure explicit")).toBeTruthy();
    expect(screen.getByText("Measure the gap in the repo")).toBeTruthy();
    expect(
      screen.getByText("Reviewed compilation, not arbitrary execution."),
    ).toBeTruthy();
    expect(screen.getByText("What drives a finding")).toBeTruthy();
    expect(screen.getByText("Values become inspectable evidence")).toBeTruthy();
    expect(screen.getAllByText("537").length).toBeGreaterThan(0);
    expect(screen.getByText("open actionable findings")).toBeTruthy();
    expect(screen.getByText("Why review remains essential")).toBeTruthy();
    expect(
      screen.getByText((_, element) => element?.textContent === "15 errors"),
    ).toBeTruthy();
    expect(screen.getByText("Consent not supplied")).toBeTruthy();
    expect(screen.getByText("Responsive render")).toBeTruthy();
    expect(screen.getByText("Not verified")).toBeTruthy();
    expect(screen.getByText("Critical boundary")).toBeTruthy();
    expect(screen.getByText("fixtures/corpus/partial-js")).toBeTruthy();

    fireEvent.click(screen.getByText("All showcase projects"));
    expect(
      screen.getByText("Create materials for every showcase project"),
    ).toBeTruthy();
  });
});
