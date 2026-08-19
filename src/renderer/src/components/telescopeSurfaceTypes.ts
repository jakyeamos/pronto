import type { EventRecord, ExternalTool, RepositorySnapshot } from "../types";
import type { RemediationRun } from "../types/remediation";
import type {
  TelescopeAction,
  TelescopeEdge,
  TelescopeFlow,
  TelescopeGroup,
  TelescopeNode,
  TelescopeProjection,
} from "../types/telescope";

export type Selection =
  | { kind: "node"; id: string }
  | { kind: "group"; id: string }
  | { kind: "edge"; id: string }
  | { kind: "flow"; id: string }
  | { kind: "action"; id: string }
  | null;

export type SelectedItem =
  | { kind: "node"; value: TelescopeNode }
  | { kind: "group"; value: TelescopeGroup }
  | { kind: "edge"; value: TelescopeEdge }
  | { kind: "flow"; value: TelescopeFlow }
  | { kind: "action"; value: TelescopeAction }
  | null;

export interface LensSummary {
  label: string;
  detail: string;
  tone: "neutral" | "blue" | "mint" | "amber" | "coral";
}

export interface TelescopeWorkspaceProps {
  repository: RepositorySnapshot;
  remediation: RemediationRun;
  events: EventRecord[];
  initialProjection?: TelescopeProjection;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository?: (workspaceId?: string) => Promise<void>;
}
