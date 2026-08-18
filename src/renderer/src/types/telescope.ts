export interface TelescopeBinding {
  workspace_id: string;
  branch: string;
  commit?: string | null;
  dirty: boolean;
  dirty_state_fingerprint: string;
  workspace_fingerprint: string;
  generated_at: string;
}

export interface TelescopeFreshness {
  state: string;
  cache: string;
  reason: string;
}

export interface TelescopeCoverage {
  discovered_source_files: number;
  examined_source_files: number;
  supported_source_files: number;
  partial_source_files: number;
  skipped_large_files: number;
  truncated: boolean;
  resolved_relationships: number;
  inferred_relationships: number;
  confidence: string;
}

export interface TelescopeGroup {
  id: string;
  label: string;
  kind: string;
  parent_id?: string | null;
  summary: string;
  confidence: string;
}

export interface TelescopeAnchor {
  path: string;
  line?: number | null;
  symbol?: string | null;
  provenance: string;
}

export interface TelescopeNode {
  id: string;
  group_id: string;
  label: string;
  kind: string;
  technology: string;
  semantic_summary: string;
  implementation_summary: string;
  summary_status: string;
  confidence: string;
  provenance: string[];
  source_anchors: TelescopeAnchor[];
  symbols: string[];
  data_shapes: string[];
}

export interface TelescopeEdge {
  id: string;
  source: string;
  target: string;
  kind: string;
  direction: string;
  label: string;
  confidence: string;
  provenance: string;
  inferred: boolean;
  source_anchor?: TelescopeAnchor | null;
}

export interface TelescopeFlow {
  id: string;
  label: string;
  kind: string;
  node_ids: string[];
  edge_ids: string[];
  data_shape?: string | null;
  confidence: string;
  provenance: string;
}

export interface TelescopeWarning {
  code: string;
  message: string;
  scope: string;
}

export interface TelescopeProjection {
  schema_version: "pronto-telescope/v1";
  repository_id: string;
  repository_name: string;
  binding: TelescopeBinding;
  freshness: TelescopeFreshness;
  coverage: TelescopeCoverage;
  groups: TelescopeGroup[];
  nodes: TelescopeNode[];
  edges: TelescopeEdge[];
  flows: TelescopeFlow[];
  warnings: TelescopeWarning[];
  enrichment: {
    enabled: boolean;
    provider?: string | null;
    model?: string | null;
    source_content_transmitted: boolean;
    status: string;
  };
}

export type TelescopeLens =
  | "architecture"
  | "changes"
  | "quality"
  | "remediation"
  | "delivery"
  | "activity"
  | "intent";
