import { describe, expect, it } from "vitest";
import { buildTelescopeScene } from "./telescopeSceneModel";
import { makeTelescopeSceneProjection } from "./telescopeSceneModel.test-support";

describe("buildTelescopeScene", () => {
  it("keeps every authored overview building instead of enforcing an arbitrary quota", () => {
    const projection = makeTelescopeSceneProjection(32);
    const scene = buildTelescopeScene(projection, "overview");
    const covered = new Set(
      scene.buildings.flatMap((building) => building.sourceNodeIds),
    );

    expect(scene.buildings.length).toBe(32);
    expect(covered.size).toBe(projection.nodes.length);
    expect(scene.primaryFlowId).toBe("flow-primary");
    expect(scene.primaryRailIds.length).toBeGreaterThan(0);
    expect(scene.primaryBuildingIds.length).toBeGreaterThan(0);
  });

  it("uses authored building identities and measured archetype fallbacks deterministically", () => {
    const projection = makeTelescopeSceneProjection(8);
    const first = buildTelescopeScene(projection, "subsystems");
    const second = buildTelescopeScene(projection, "subsystems");

    expect(first).toEqual(second);
    expect(first.buildings.map((building) => building.id)).toContain(
      "building-" + stableId("authored|authored-building-0"),
    );
    expect(first.buildings.map((building) => building.archetype)).toEqual(
      expect.arrayContaining(["tower", "fin-row", "slab-stack", "low-slab"]),
    );
    expect(first.rails.map((rail) => rail.railKind)).toEqual(
      expect.arrayContaining(["control", "import"]),
    );
  });

  it("scopes source detail to a selected building and its immediate handoffs", () => {
    const projection = makeTelescopeSceneProjection(206);
    const scope = { selectedNodeIds: ["node-100"] };
    const first = buildTelescopeScene(projection, "source", scope);
    const second = buildTelescopeScene(projection, "source", scope);
    const covered = new Set(
      first.buildings.flatMap((building) => building.sourceNodeIds),
    );

    expect(first).toEqual(second);
    expect(first.buildings.length).toBe(3);
    expect(first.scopedSourceNodeCount).toBe(3);
    expect(first.hiddenSourceNodeCount).toBe(203);
    expect(covered).toEqual(new Set(["node-99", "node-100", "node-101"]));
  });
});

function stableId(value: string): string {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
