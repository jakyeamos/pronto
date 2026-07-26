export interface RootConfig {
  id: string;
  path: string;
  label: string;
  ignore_patterns: string[];
  refresh_policy: "Manual" | "On open" | "Periodic" | string;
  background_monitoring: boolean;
  registered_at: string;
}

export interface ProductConfig {
  id: string;
  name: string;
  repository_ids: string[];
  release_mode: string;
  created_at: string;
  updated_at: string;
}

export interface GroupConfig {
  id: string;
  name: string;
  repository_ids: string[];
  created_at: string;
  updated_at: string;
}

export interface ProviderIdentity {
  id: string;
  provider: string;
  login: string;
  display_name?: string;
  organizations: string[];
  credential_state: string;
  updated_at: string;
}

export interface RemoteRepositorySnapshot {
  id: string;
  provider: string;
  full_name: string;
  name: string;
  owner: string;
  html_url: string;
  default_branch?: string;
  archived: boolean;
  locality: string;
  identity_id: string;
  last_refreshed_at: string;
}

export interface ProviderStatus {
  provider: string;
  state: string;
  message: string;
  last_refresh_at?: string;
  identity_count: number;
  repository_count: number;
}

export interface EvidenceItem {
  label: string;
  value: string;
  source: string;
  observed_at: string;
}

export interface Condition {
  id: string;
  kind: string;
  title: string;
  summary: string;
  priority: number;
  status: "Active" | "Expected" | "Resolved" | "Superseded";
  fingerprint: string;
  rule: string;
  evidence: EvidenceItem[];
  missing: string[];
  confidence?: string;
  freshness?: string;
}

export interface AgentManifest {
  task_id?: string;
  title?: string;
  target_branch?: string;
  agent_type?: string;
  start_time?: string;
  status?: string;
  source_session_id?: string;
}

export interface ActivitySignal {
  source: string;
  summary: string;
  confidence: string;
  observed_at: string;
  process_name?: string;
  process_id?: number;
  started_at?: string;
  working_directory?: string;
}

export interface WorkspaceActivity {
  state: string;
  confidence: string;
  signals: ActivitySignal[];
  manifest?: AgentManifest;
}

export interface WorkspaceSummary {
  id: string;
  path: string;
  is_primary: boolean;
  branch: string;
  dirty: boolean;
  added: number;
  removed: number;
  line_totals_partial: boolean;
  sync_state: string;
  remote_freshness: string;
  ahead: number;
  behind: number;
  upstream?: string;
  operation?: string;
  last_commit?: string;
  last_commit_at?: string;
  last_activity_at?: string;
  integration_state: string;
  target_branch?: string;
  target_confidence: string;
  role: string;
  role_confidence: string;
  activity: WorkspaceActivity;
}

export interface BranchSummary {
  name: string;
  role: string;
  role_confidence: string;
  target_branch?: string;
  target_confidence: string;
  ahead: number;
  behind: number;
  integration_state: string;
  workspace_id?: string;
  last_commit?: string;
  last_commit_at?: string;
}

export interface RepositorySnapshot {
  id: string;
  name: string;
  path: string;
  locality: string;
  lifecycle: string;
  lifecycle_candidate: string;
  remote_url?: string;
  provider_state: string;
  branch: string;
  default_branch?: string;
  workspace: WorkspaceSummary;
  workspaces: WorkspaceSummary[];
  branches: BranchSummary[];
  submodules: SubmoduleSummary[];
  conditions: Condition[];
  last_scan_at: string;
  last_fetch_at?: string;
  last_activity_at?: string;
}

export interface SubmoduleSummary {
  path: string;
  commit?: string;
  status: string;
}

export interface EventRecord {
  id: string;
  repository_id: string;
  kind: string;
  summary: string;
  fingerprint: string;
  created_at: string;
}

export interface ActionAudit {
  id: string;
  action: string;
  target_ids: string[];
  risk: string;
  status: string;
  summary: string;
  created_at: string;
  completed_at?: string | null;
}

export interface ActionPreflight {
  audit: ActionAudit;
  allowed: boolean;
  target_label: string;
}

export interface PortfolioSnapshot {
  roots: RootConfig[];
  repositories: RepositorySnapshot[];
  products: ProductConfig[];
  groups: GroupConfig[];
  events: EventRecord[];
  action_audits: ActionAudit[];
  provider_identities: ProviderIdentity[];
  remote_repositories: RemoteRepositorySnapshot[];
  provider_status: ProviderStatus;
  retention_days: number;
  generated_at: string;
  storage_path: string;
}
