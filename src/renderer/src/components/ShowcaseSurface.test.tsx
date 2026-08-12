// quality-gate: allow static-ui-test: verifies the showcase ranking, private-client exclusion, and missing-contract states remain explicit in the rendered surface.
// @vitest-environment happy-dom
import { cleanup, render, screen } from "@testing-library/react";
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
    missing_materials: ["45-90 second captioned demo recording and shot list"],
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
  unassessed.showcase_score = null;
  unassessed.priority_score = null;
  return {
    schema_version: "pronto-showcase/v1",
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
      unassessed,
    ],
    error: null,
  };
}

describe("ShowcaseSurface", () => {
  it("ranks the full fleet while preserving client and unknown boundaries", () => {
    render(
      <ShowcaseSurface
        showcase={snapshot()}
        repositories={[]}
        onOpenRepository={() => undefined}
      />,
    );

    expect(
      screen.getByText("Build five recruiter-ready public demos"),
    ).toBeTruthy();
    expect(screen.getByText("Mac Control")).toBeTruthy();
    expect(screen.getByText("CrimClock")).toBeTruthy();
    expect(screen.getByText("Unreviewed Repo")).toBeTruthy();
    expect(
      screen.getByText(
        "Private audit only · never eligible for public publishing",
      ),
    ).toBeTruthy();
    expect(
      screen.getByText("Readiness · 60% product · 40% materials"),
    ).toBeTruthy();
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
});
