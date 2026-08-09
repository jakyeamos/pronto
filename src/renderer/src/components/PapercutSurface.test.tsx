// quality-gate: allow static-ui-test: verifies the durable papercut contract and its explicit empty-state copy.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { PapercutSurface } from "./PapercutSurface";
import type { PapercutBacklog } from "../types";

function backlogFixture(
  overrides: Partial<PapercutBacklog> = {},
): PapercutBacklog {
  return {
    schema_version: "pronto-papercuts/v1",
    family: "design-audit",
    generated_at: "2026-08-08T16:00:00+00:00",
    papercuts: [
      {
        id: "papercut-1",
        title: "The empty state hides the next action",
        detail: "A first-time user has to infer where to begin.",
        family: "design-audit",
        surface: "Pronto UI",
        source: "design-friction",
        evidence_refs: ["screen:portfolio-empty"],
        impact: "Adds avoidable orientation cost.",
        priority: "P1",
        status: "open",
        next_action: "Exercise the empty-state flow after the copy change.",
        created_at: "2026-08-08T15:00:00+00:00",
        updated_at: "2026-08-08T15:00:00+00:00",
        resolved_at: null,
      },
    ],
    counts: {
      total: 1,
      open: 1,
      in_progress: 0,
      deferred: 0,
      resolved: 0,
    },
    ...overrides,
  };
}

describe("PapercutSurface", () => {
  it("keeps the per-turn audit boundary separate from durable capture", () => {
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture()}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("Design audit family");
    expect(markup).toContain("ephemeral per-turn sensor");
    expect(markup).toContain("durable capture point");
    expect(markup).toContain("Capture a papercut");
    expect(markup).toContain("Design-friction audit");
    expect(markup).toContain("The empty state hides the next action");
    expect(markup).toContain("Next validation step");
  });

  it("makes an empty top-level backlog actionable", () => {
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture({
          papercuts: [],
          counts: {
            total: 0,
            open: 0,
            in_progress: 0,
            deferred: 0,
            resolved: 0,
          },
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("No papercuts captured yet.");
    expect(markup).toContain(
      "When a design audit finds a repeatable small hurt",
    );
    expect(markup).toContain("Capture papercut");
  });
});
