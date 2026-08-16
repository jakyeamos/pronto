import type {
  QualityPortfolioSnapshot,
  QualitySnapshot,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
} from "./types/quality";
import type { RemediationRun } from "./types/remediation";
import type { ShowcasePortfolioSnapshot } from "./types/showcase";

export * from "./types/quality";
export * from "./types/remediation";
export * from "./types/insights";
export * from "./types/promotion";
export * from "./types/papercuts";
export * from "./types/showcase";

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
  pull_requests: PullRequestSnapshot[];
  releases: ReleaseSnapshot[];
  ci_checks: CheckSnapshot[];
  ci_branch?: string;
  ci_commit?: string;
}

interface CheckSnapshot {
  context: string;
  state: string;
  required: boolean;
  conclusion?: string;
  last_refreshed_at: string;
  html_url?: string;
  head_sha?: string;
}

interface PullRequestSnapshot {
  id: string;
  provider: string;
  repository_id: string;
  number: number;
  html_url: string;
  title: string;
  head_branch: string;
  base_branch: string;
  state: string;
  draft: boolean;
  checks_state: string;
  reviews_state: string;
  mergeability: string;
  checks: CheckSnapshot[];
  last_refreshed_at: string;
  head_commit?: string;
}

interface ReleaseSnapshot {
  id: string;
  provider: string;
  repository_id: string;
  tag: string;
  name: string;
  target_commit?: string;
  published_at?: string;
  draft: boolean;
  prerelease: boolean;
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

interface EvidenceItem {
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

interface AgentManifest {
  task_id?: string;
  title?: string;
  target_branch?: string;
  agent_type?: string;
  start_time?: string;
  status?: string;
  source_session_id?: string;
}

interface ActivitySignal {
  source: string;
  summary: string;
  confidence: string;
  observed_at: string;
  process_name?: string;
  process_id?: number;
  started_at?: string;
  working_directory?: string;
}

interface WorkspaceActivity {
  state: string;
  confidence: string;
  signals: ActivitySignal[];
  manifest?: AgentManifest;
}

interface WorkspaceSyncDetail {
  reason: string;
  evidence_observed_at?: string;
  evidence_expires_at?: string;
  evidence_window_minutes: number;
  next_safe_action: string;
  scoped_refresh_command: string;
  authorization: string;
}

export type ExternalTool =
  "file_browser" | "terminal" | "editor" | "git_client";

export interface WorkspaceSummary {
  id: string;
  path: string;
  is_primary: boolean;
  branch: string;
  status_available?: boolean;
  status_error?: string;
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
  sync_detail?: WorkspaceSyncDetail;
}

interface BranchSummary {
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
  target_branch?: string;
  target_branch_configured: boolean;
  workspace: WorkspaceSummary;
  workspaces: WorkspaceSummary[];
  branches: BranchSummary[];
  submodules: SubmoduleSummary[];
  pull_requests: PullRequestSnapshot[];
  releases: ReleaseSnapshot[];
  quality: QualitySnapshot;
  project_compass: ProjectCompassSummary;
  release_rule?: ReleaseRuleConfig;
  release_recipe?: ReleaseRecipeConfig;
  confirmed_release_version?: string;
  ai_permission: string;
  conditions: Condition[];
  last_scan_at: string;
  last_fetch_at?: string;
  last_activity_at?: string;
}

export interface ProjectCompassTargetSummary {
  progress_percent: number | null;
  scored_outcome_count: number;
  covered_pillar_count: number;
  total_pillar_count: number;
  confidence: string;
  confidence_percent: number;
}

interface ProjectCompassBlockerSummary {
  outcome_id: string;
  outcome_name: string;
  kind: string;
  summary: string;
}

interface ProjectCompassDriftSummary {
  kind: string;
  summary: string;
  observed_at: string;
}

interface ProjectCompassSummary {
  status: "Ready" | "Missing" | "Invalid";
  contract_path: string;
  revision: number | null;
  updated_at: string | null;
  project_name: string | null;
  identity: string | null;
  audience: string | null;
  mvp: ProjectCompassTargetSummary;
  complete_product: ProjectCompassTargetSummary;
  open_blockers: number;
  open_drift: number;
  open_blocker_items: ProjectCompassBlockerSummary[];
  open_drift_items: ProjectCompassDriftSummary[];
  error: string | null;
}

interface PullRequestPreparation {
  repository_id: string;
  workspace_id: string;
  head_branch: string;
  base_branch?: string;
  commit_count: number;
  status_available?: boolean;
  status_error?: string;
  dirty: boolean;
  ahead: number;
  behind: number;
  upstream?: string;
  provider_state: string;
  checks_state: string;
  reviews_state: string;
  mergeability: string;
  status: string;
  reasons: string[];
  evidence: EvidenceItem[];
  existing_pull_request?: PullRequestSnapshot;
}

interface ReleaseCommitSummary {
  sha: string;
  subject: string;
  category: string;
  bump?: string;
  committed_at: string;
}

interface ReleaseNoteSection {
  category: string;
  commits: ReleaseCommitSummary[];
}

interface ReleaseRuleTrace {
  label: string;
  status: string;
  value: string;
  source: string;
}

interface ReleasePreparation {
  repository_id: string;
  target_branch?: string;
  baseline_status: string;
  baseline?: ReleaseSnapshot;
  commits_since_baseline: ReleaseCommitSummary[];
  rule_status: string;
  threshold_label?: string;
  rule_trace: ReleaseRuleTrace[];
  candidate_bump?: string;
  candidate_version?: string;
  version_status: string;
  release_boundary_status?: string;
  notes: ReleaseNoteSection[];
  status: string;
  reasons: string[];
  evidence: EvidenceItem[];
}

interface ReleaseRecipeStep {
  order: number;
  label: string;
  status: string;
  detail: string;
}

export interface ReleaseRecipePreview {
  repository_id: string;
  recipe_name: string;
  candidate_version?: string;
  version_status: string;
  status: string;
  reasons: string[];
  steps: ReleaseRecipeStep[];
  actions_performed: boolean;
  generated_at: string;
}

export interface RepositoryPreparation {
  repository_id: string;
  pull_request: PullRequestPreparation;
  release: ReleasePreparation;
  recipe: ReleaseRecipePreview;
  generated_at: string;
}

interface SubmoduleSummary {
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
  quality: QualityPortfolioSnapshot;
  remediation: RemediationRun;
  showcase: ShowcasePortfolioSnapshot;
  retention_days: number;
  generated_at: string;
  storage_path: string;
}
