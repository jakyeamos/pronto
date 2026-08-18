import type { Edge, Node } from "@xyflow/react";
import type { TelescopeProjection } from "../types/telescope";

interface TelescopeNodeData extends Record<string, unknown> {
  telescopeId: string;
  label: string;
  kind: string;
  technology: string;
  confidence: string;
}

export interface TelescopeLayout {
  nodes: Array<Node<TelescopeNodeData>>;
  edges: Edge[];
  engine?: "elk" | "grid-fallback";
  warning?: string;
}

export async function layoutTelescope(
  projection: TelescopeProjection,
  collapsedGroupIds: string[] = [],
): Promise<TelescopeLayout> {
  const { computeTelescopeLayout } = await import("./telescopeLayoutCore");
  return computeTelescopeLayout(projection, collapsedGroupIds);
}
