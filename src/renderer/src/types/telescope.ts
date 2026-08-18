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
  provenance?: string[];
  source_file_count?: number;
  measured_lines?: number;
  visual_archetype?: string;
  visual_override_provenance?: string;
  narrative_status?: string;
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
  source_paths?: string[];
  measured_lines?: number;
  source_file_count?: number;
  visual_building_id?: string | null;
  visual_archetype?: string;
  visual_override_provenance?: string;
  narrative_status?: string;
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
  rail_kind?: string;
  visual_override_provenance?: string;
  narrative_status?: string;
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
  narrative_status?: string;
  primary?: boolean;
}

export interface TelescopeAction {
  id: string;
  label: string;
  verb: string;
  category: string;
  what_it_does: string;
  how_its_built: string;
  node_ids: string[];
  edge_ids: string[];
  flow_id?: string | null;
  behavior_id?: string | null;
  scenario_ids?: string[];
  behavior_state?: string;
  behavior_verification?: string;
  source_anchors: TelescopeAnchor[];
  status: string;
  confidence: string;
  provenance: string;
  read_only: boolean;
  guarded: boolean;
}

export interface TelescopeActionCoverage {
  inventory_status: string;
  total: number;
  authored: number;
  inferred: number;
  partial: number;
  mapped: number;
  unmapped: number;
  behavior_backed?: number;
  unprofiled?: number;
}

export interface TelescopeWarning {
  code: string;
  message: string;
  scope: string;
}

export interface TelescopeNarrativeCoverage {
  authored_source_files: number;
  mapped_source_files: number;
  unmapped_source_files: string[];
  coverage_percent: number;
}

export interface TelescopeNarrativeGroup {
  id: string;
  label: string;
  kind?: string;
  summary?: string;
  pathPrefixes?: string[];
  visualArchetype?: string;
  status?: string;
}

export interface TelescopeNarrativeNode {
  id: string;
  label: string;
  groupId?: string;
  whatItDoes?: string;
  howItsBuilt?: string;
  files?: string[];
  visualArchetype?: string;
  status?: string;
}

export interface TelescopeNarrativeEdge {
  id: string;
  sourceFile: string;
  targetFile: string;
  kind?: string;
  label?: string;
  railKind?: string;
  status?: string;
}

export interface TelescopeNarrativeFlow {
  id: string;
  label: string;
  kind?: string;
  nodeIds: string[];
  edgeIds: string[];
  dataShape?: string | null;
  status?: string;
  primary?: boolean;
}

export interface TelescopeNarrativeAction {
  id: string;
  label: string;
  verb?: string;
  category?: string;
  whatItDoes?: string;
  howItsBuilt?: string;
  files?: string[];
  nodeIds?: string[];
  edgeIds?: string[];
  flowId?: string | null;
  behaviorId?: string | null;
  scenarioIds?: string[];
  status?: string;
  readOnly?: boolean;
  guarded?: boolean;
}

export interface TelescopeNarrative {
  manifest_path?: string;
  status?: "missing" | "draft" | "reviewed" | "stale" | string;
  manifest_fingerprint?: string | null;
  measured_fingerprint?: string | null;
  visual_model_version?: string;
  primary_flow_id?: string | null;
  authored_groups?: TelescopeNarrativeGroup[];
  authored_nodes?: TelescopeNarrativeNode[];
  authored_edges?: TelescopeNarrativeEdge[];
  authored_flows?: TelescopeNarrativeFlow[];
  authored_actions?: TelescopeNarrativeAction[];
  coverage?: TelescopeNarrativeCoverage;
  drift_warnings?: TelescopeWarning[];
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
  actions: TelescopeAction[];
  action_coverage: TelescopeActionCoverage;
  warnings: TelescopeWarning[];
  enrichment: {
    enabled: boolean;
    provider?: string | null;
    model?: string | null;
    source_content_transmitted: boolean;
    status: string;
  };
  narrative?: TelescopeNarrative;
}

export type TelescopeLens =
  | "architecture"
  | "changes"
  | "quality"
  | "remediation"
  | "delivery"
  | "activity"
  | "intent";
