import type { Edge, Node } from "@xyflow/react";
import type { TelescopeProjection } from "../types/telescope";
import type { TelescopeSceneModel } from "./telescopeSceneModel";

export interface TelescopeLayout {
  nodes: Array<Node>;
  edges: Edge[];
  engine?: "elk" | "grid-fallback";
  warning?: string;
}

export async function layoutTelescope(
  projection: TelescopeProjection,
  collapsedGroupIds: string[] = [],
  scene?: TelescopeSceneModel,
): Promise<TelescopeLayout> {
  if (scene) {
    const { computeTelescopeSceneLayout } =
      await import("./telescopeSceneLayout");
    return computeTelescopeSceneLayout(scene);
  }
  const { computeTelescopeLayout } = await import("./telescopeLayoutCore");
  return computeTelescopeLayout(projection, collapsedGroupIds);
}
