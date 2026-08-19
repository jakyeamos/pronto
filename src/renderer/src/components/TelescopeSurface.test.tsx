// @vitest-environment happy-dom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RemediationRun } from "../types/remediation";
import type { TelescopeProjection } from "../types/telescope";
import { makeRepository } from "./QualityComponents.test-support";
import { TelescopeSurface } from "./TelescopeSurface";

vi.mock("./telescopeLayout", () => ({
  layoutTelescope: vi.fn(async (projection: TelescopeProjection) => ({
    nodes: projection.nodes.map((node, index) => ({
      id: node.id,
      type: "telescopeEntity",
      position: { x: index * 220, y: 80 },
      data: {
        telescopeId: node.id,
        label: node.label,
        kind: node.kind,
        technology: node.technology,
        confidence: node.confidence,
      },
    })),
    edges: projection.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "telescopeFlow",
      data: { telescopeId: edge.id, inferred: edge.inferred },
    })),
  })),
}));

const projection: TelescopeProjection = {
  schema_version: "pronto-telescope/v1",
  repository_id: "repo-1",
  repository_name: "pronto",
  binding: {
    workspace_id: "workspace-1",
    branch: "codex/telescope",
    commit: "0123456789abcdef",
    dirty: true,
    dirty_state_fingerprint: "dirty-1",
    workspace_fingerprint: "workspace-1",
    generated_at: "2026-08-18T16:00:00Z",
  },
  freshness: { state: "fresh", cache: "miss", reason: "generated" },
  coverage: {
    discovered_source_files: 2,
    examined_source_files: 2,
    supported_source_files: 2,
    partial_source_files: 0,
    skipped_large_files: 0,
    truncated: false,
    resolved_relationships: 1,
    inferred_relationships: 0,
    confidence: "high",
  },
  groups: [
    {
      id: "group-app",
      label: "Application",
      kind: "subsystem",
      summary: "Application modules",
      confidence: "high",
    },
  ],
  nodes: [
    {
      id: "node-api",
      group_id: "group-app",
      label: "API",
      kind: "route",
      technology: "TypeScript",
      semantic_summary: "Receives repository requests.",
      implementation_summary: "A typed Tauri bridge.",
      summary_status: "derived",
      confidence: "high",
      provenance: ["static-source"],
      source_anchors: [{ path: "src/api.ts", line: 12, provenance: "symbol" }],
      symbols: ["getRepositoryTelescope"],
      data_shapes: ["TelescopeProjection"],
    },
    {
      id: "node-store",
      group_id: "group-app",
      label: "Store",
      kind: "store",
      technology: "Rust",
      semantic_summary: "Stores local projections.",
      implementation_summary: "SQLite cache keyed by fingerprint.",
      summary_status: "derived",
      confidence: "high",
      provenance: ["static-source"],
      source_anchors: [
        { path: "src/store.rs", line: 30, provenance: "symbol" },
      ],
      symbols: ["cache_projection"],
      data_shapes: ["TelescopeProjection"],
    },
  ],
  edges: [
    {
      id: "edge-api-store",
      source: "node-api",
      target: "node-store",
      kind: "import",
      direction: "forward",
      label: "persists",
      confidence: "high",
      provenance: "static-import",
      inferred: false,
    },
  ],
  flows: [
    {
      id: "flow-request",
      label: "Repository request",
      kind: "control",
      node_ids: ["node-api", "node-store"],
      edge_ids: ["edge-api-store"],
      data_shape: "TelescopeProjection",
      confidence: "high",
      provenance: "static-route",
    },
  ],
  actions: [
    {
      id: "inspect-request",
      label: "Inspect the repository request",
      verb: "Inspect",
      category: "evidence",
      what_it_does: "Focuses the city on the request path.",
      how_its_built: "Uses the mapped request flow and its source anchors.",
      node_ids: ["node-api", "node-store"],
      edge_ids: ["edge-api-store"],
      flow_id: "flow-request",
      behavior_id: "behavior-feed-projection",
      scenario_ids: ["missing-invalid-stale-never-green"],
      behavior_state: "declared",
      behavior_verification: "automated",
      source_anchors: [{ path: "src/api.ts", line: 12, provenance: "source" }],
      status: "reviewed",
      confidence: "high",
      provenance: "authored-action-inventory",
      read_only: true,
      guarded: true,
    },
  ],
  action_coverage: {
    inventory_status: "reviewed",
    total: 1,
    authored: 1,
    inferred: 0,
    partial: 0,
    mapped: 1,
    unmapped: 0,
    behavior_backed: 1,
    unprofiled: 0,
  },
  warnings: [],
  enrichment: {
    enabled: false,
    source_content_transmitted: false,
    status: "disabled",
  },
};

const remediation: RemediationRun = {
  schema_version: "pronto-remediation/v3",
  id: "run-1",
  generated_at: "2026-08-18T16:00:00Z",
  status: "ready",
  eligible_repository_ids: ["repo-1"],
  eligible_repository_paths: ["/tmp/pronto"],
  refresh_steps: [],
  excluded_repositories: [],
  closures: [],
  plans: [],
};

beforeEach(() => {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
});

afterEach(cleanup);

describe("TelescopeSurface", () => {
  it("synchronizes navigator selection with semantic and implementation inspection", async () => {
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    const navigatorEntity = screen.getByRole("button", { name: /API route/i });
    fireEvent.click(navigatorEntity);
    expect(screen.getByText("Receives repository requests.")).toBeTruthy();
    expect(navigatorEntity.className).toContain("active");

    fireEvent.click(screen.getByRole("button", { name: "How it’s built" }));
    expect(screen.getByText("A typed Tauri bridge.")).toBeTruthy();
    expect(screen.getByText("src/api.ts:12")).toBeTruthy();

    await waitFor(() => expect(navigatorEntity.className).toContain("active"));
  });

  it("keeps workflow lenses independent and inspects flow data without runtime values", () => {
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Quality" }));
    expect(screen.getByText(/actionable findings/i)).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Flow: Repository request" }),
    );
    expect(screen.getByText("TelescopeProjection")).toBeTruthy();
    expect(
      screen.getByText(/no runtime payload values are captured/i),
    ).toBeTruthy();
  });

  it("searches the action catalog and focuses the city with canonical behavior evidence", () => {
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    const search = screen.getByRole("textbox", {
      name: "Find a Telescope action",
    });
    fireEvent.change(search, {
      target: { value: "how does the repository request work" },
    });
    expect(
      screen.getByText(/Direct match on repository, request/i),
    ).toBeTruthy();
    fireEvent.keyDown(search, { key: "Enter" });

    expect(
      screen.getByText("Focuses the city on the request path."),
    ).toBeTruthy();
    expect(screen.getByLabelText(/guided city story/i)).toBeTruthy();
    expect(screen.getByText(/Behavior contract unavailable/i)).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Subsystems" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "How it’s built" }));
    expect(screen.getByText("behavior-feed-projection")).toBeTruthy();
    expect(screen.getByText(/no mutation is performed/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Show full city" }));
    expect(
      screen
        .getByRole("button", { name: "Overview" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("respects reduced motion while preserving a static inspectable token and keyboard selection", async () => {
    vi.mocked(window.matchMedia).mockReturnValue({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    } as unknown as MediaQueryList);
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    expect(screen.getByText(/Flow motion is reduced/i)).toBeTruthy();
    fireEvent.keyDown(screen.getByLabelText("pronto Telescope"), {
      key: "ArrowRight",
    });
    expect(screen.getByText("Receives repository requests.")).toBeTruthy();
  });

  it("can focus the remediation lens on source-matched architecture", async () => {
    const remediationWithAction = {
      ...remediation,
      plans: [
        {
          repository_id: "repo-1",
          progress: { percentage: 10 },
          actions: [
            {
              title: "Repair API route",
              summary: "Update src/api.ts request handling",
              status: "open",
            },
          ],
        },
      ],
    } as RemediationRun;
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediationWithAction}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Remediation" }));
    const focus = screen.getByRole("button", { name: "Show affected only" });
    expect(focus).not.toHaveProperty("disabled", true);
    fireEvent.click(focus);
    expect(focus.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByText("1 affected entity shown")).toBeTruthy();
    await waitFor(() => {
      const unrelated = document.querySelector<HTMLElement>(
        '.react-flow__node[data-id="node-store"]',
      );
      expect(unrelated).toBeNull();
    });
  });

  it("routes incomplete meaning through the guarded Map Workshop", async () => {
    const onPrepareRepository = vi.fn(async () => undefined);
    const incompleteProjection: TelescopeProjection = {
      ...projection,
      schema_version: "pronto-telescope/v2",
      map_readiness: {
        state: "needs_information",
        reason: "A primary actor is still unknown.",
        requirements: [],
        blocking_gap_keys: ["telescope-readiness:actors"],
        enhancement_gap_keys: [],
      },
      blocking_gaps: [],
      enhancement_gaps: [],
      knowledge_tasks: [
        {
          id: "knowledge-task-actors",
          stable_gap_key: "telescope-readiness:actors",
          domain: "telescope_readiness",
          status: "open",
          title: "Complete Telescope knowledge: actors",
          question: "Who enters this city, and which gates do they use?",
          summary: "Entrypoints do not establish the people using them.",
          priority: "P1",
          dependency_order: 2,
          depends_on: ["telescope-readiness:identity"],
          unlocks: ["People and crews"],
          candidate_answers: ["Repository operator"],
          allowed_responses: ["confirm", "edit", "unknown"],
          completion_criteria: ["actors contains explicit draft evidence."],
          manifest_fields: ["actors"],
          evidence: [{ path: "src/api.ts", line: 12, provenance: "symbol" }],
          freshness: "current-workspace",
          provenance: "telescope-readiness-to-remediation-projection",
          read_only: true,
          guarded_handoff: true,
        },
      ],
    };
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={incompleteProjection}
        onOpenWorkspace={async () => undefined}
        onPrepareRepository={onPrepareRepository}
      />,
    );

    expect(screen.getByText("Who enters this city, and which gates do they use?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Answer next question" }));
    await waitFor(() => expect(onPrepareRepository).toHaveBeenCalledTimes(1));
  });

  it("turns Source detail into a building-local evidence workspace", async () => {
    render(
      <TelescopeSurface
        repository={makeRepository()}
        remediation={remediation}
        events={[]}
        initialProjection={projection}
        onOpenWorkspace={async () => undefined}
      />,
    );

    const sourceDetailButton = screen.getByRole("button", {
      name: "Source detail",
    });
    fireEvent.click(sourceDetailButton);
    await waitFor(() =>
      expect(sourceDetailButton.getAttribute("aria-pressed")).toBe("true"),
    );
    await waitFor(() =>
      expect(document.querySelector(".telescope-source-detail")).toBeTruthy(),
    );
    expect(screen.getByText("Immediate handoffs")).toBeTruthy();
    expect(screen.getByText("Symbols and data")).toBeTruthy();
    expect(screen.getAllByText(/src\/api\.ts/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/repository-wide file graph/i)).toBeNull();
  });
});
