import { describe, expect, it } from "vitest";
import {
  buildTelescopeScene,
  MAX_SOURCE_DETAIL_BUILDINGS,
} from "./telescopeSceneModel";
import { makeTelescopeSceneProjection } from "./telescopeSceneModel.test-support";

describe("buildTelescopeScene", () => {
  it("keeps overview comprehensible without losing source-node coverage", () => {
    const projection = makeTelescopeSceneProjection(32);
    const scene = buildTelescopeScene(projection, "overview");
    const covered = new Set(
      scene.buildings.flatMap((building) => building.sourceNodeIds),
    );

    expect(scene.buildings.length).toBeLessThanOrEqual(24);
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

  it("bounds source detail while retaining every source entity in inspectable clusters", () => {
    const projection = makeTelescopeSceneProjection(206);
    const first = buildTelescopeScene(projection, "source");
    const second = buildTelescopeScene(projection, "source");
    const covered = new Set(
      first.buildings.flatMap((building) => building.sourceNodeIds),
    );

    expect(first).toEqual(second);
    expect(first.buildings.length).toBeLessThanOrEqual(
      MAX_SOURCE_DETAIL_BUILDINGS,
    );
    expect(first.clusteredSourceNodeCount).toBeGreaterThan(0);
    expect(first.sourceDetailBuildingLimit).toBe(MAX_SOURCE_DETAIL_BUILDINGS);
    expect(covered).toEqual(new Set(projection.nodes.map((node) => node.id)));
    expect(
      first.buildings.some((building) =>
        building.label.startsWith("More District"),
      ),
    ).toBe(true);
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
