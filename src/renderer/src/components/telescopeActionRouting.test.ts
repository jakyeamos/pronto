import { describe, expect, it } from "vitest";
import type { TelescopeAction } from "../types/telescope";
import { routeTelescopeActions } from "./telescopeActionRouting";

function makeAction(overrides: Partial<TelescopeAction> = {}): TelescopeAction {
  return {
    id: "action",
    label: "Inspect an action",
    verb: "Inspect",
    category: "inspect",
    what_it_does: "Explains the selected workflow.",
    how_its_built: "Uses mapped source evidence.",
    node_ids: [],
    edge_ids: [],
    source_anchors: [],
    status: "reviewed",
    confidence: "high",
    provenance: "authored-action-inventory",
    read_only: true,
    guarded: false,
    ...overrides,
  };
}

describe("routeTelescopeActions", () => {
  it("routes a conversational search question to the search action", () => {
    const matches = routeTelescopeActions("how does search work", [
      makeAction({
        id: "find-action",
        label: "Find an action",
        verb: "Find",
        category: "navigate",
        what_it_does:
          "Searches the canonical action projection and focuses the city.",
      }),
      makeAction({
        id: "refresh-map",
        label: "Refresh the workspace map",
        verb: "Refresh",
        category: "freshness",
        what_it_does: "Regenerates the city from the active worktree.",
      }),
    ]);

    expect(matches[0]?.action.id).toBe("find-action");
    expect(matches[0]?.relationship).toBe("direct");
    expect(matches[0]?.matchedTerms).toContain("search");
  });

  it("keeps multiple related actions available for an exploratory question", () => {
    const matches = routeTelescopeActions("where can I find proof", [
      makeAction({
        id: "inspect-source",
        label: "Inspect source evidence",
        category: "evidence",
        what_it_does: "Explains which source files support the architecture.",
      }),
      makeAction({
        id: "inspect-behavior",
        label: "Inspect behavior evidence",
        category: "behavior",
        what_it_does: "Separates verification receipts from behavior state.",
      }),
    ]);

    expect(matches).toHaveLength(2);
    expect(matches.every((match) => match.action.id)).toBe(true);
    expect(matches.some((match) => match.relationship === "related")).toBe(
      true,
    );
  });

  it("returns the authored catalog order when no question has been entered", () => {
    const actions = [makeAction({ id: "first" }), makeAction({ id: "second" })];

    expect(
      routeTelescopeActions("", actions).map((match) => match.action.id),
    ).toEqual(["first", "second"]);
  });
});
