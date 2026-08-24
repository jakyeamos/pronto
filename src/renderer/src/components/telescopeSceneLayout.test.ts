import { describe, expect, it } from "vitest";
import {
  computeTelescopeSceneLayout,
  dimensionsForArchetype,
  routeRailWaypoints,
} from "./telescopeSceneLayout";
import { buildTelescopeScene } from "./telescopeSceneModel";
import { makeTelescopeSceneProjection } from "./telescopeSceneModel.test-support";

describe("telescopeSceneLayout", () => {
  it("gives each archetype a distinct visual footprint and routes rails through stable waypoints", () => {
    const dimensions = [
      dimensionsForArchetype("fin-row"),
      dimensionsForArchetype("tower"),
      dimensionsForArchetype("slab-stack"),
      dimensionsForArchetype("cube"),
      dimensionsForArchetype("low-slab"),
    ];
    expect(
      new Set(dimensions.map((value) => value.width + "x" + value.height)).size,
    ).toBe(5);

    const waypoints = routeRailWaypoints(
      { x: 10, y: 20 },
      { x: 300, y: 160 },
      2,
    );
    expect(waypoints[0]).toEqual({ x: 10, y: 20 });
    expect(waypoints.at(-1)).toEqual({ x: 300, y: 160 });
    expect(
      new Set(waypoints.map((point) => point.x + ":" + point.y)).size,
    ).toBe(waypoints.length);
  });

  it("produces deterministic compound district and building nodes", () => {
    const scene = buildTelescopeScene(
      makeTelescopeSceneProjection(12),
      "overview",
    );
    const layout = computeTelescopeSceneLayout(scene);
    const districtIds = new Set(scene.districts.map((district) => district.id));

    expect(layout.engine).toBe("grid-fallback");
    expect(
      layout.nodes.filter((node) => node.type === "telescopeGroup"),
    ).toHaveLength(scene.districts.length);
    expect(
      layout.nodes.filter((node) => node.type === "telescopeEntity"),
    ).toHaveLength(scene.buildings.length);
    for (const node of layout.nodes) {
      expect(Number.isFinite(node.position.x)).toBe(true);
      expect(Number.isFinite(node.position.y)).toBe(true);
      if (node.parentId) expect(districtIds.has(node.parentId)).toBe(true);
    }
    expect(layout.edges.map((edge) => edge.id)).toEqual(
      scene.rails.map((rail) => rail.id),
    );
  });

  it("keeps a large source scene finite after detail clustering", () => {
    const scene = buildTelescopeScene(
      makeTelescopeSceneProjection(206),
      "source",
    );
    const layout = computeTelescopeSceneLayout(scene);

    expect(layout.nodes.length).toBe(
      scene.districts.length + scene.buildings.length,
    );
    expect(layout.edges.length).toBe(scene.rails.length);
    expect(layout.nodes.every((node) => Number.isFinite(node.position.x))).toBe(
      true,
    );
    expect(layout.nodes.every((node) => Number.isFinite(node.position.y))).toBe(
      true,
    );
  });
});
