import type { CiRunSnapshot } from "./ci";

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
  visibility?: GitHubVisibility;
  viewer_can_administer?: boolean;
  viewer_permission?: string;
  locality: string;
  identity_id: string;
  last_refreshed_at: string;
  pull_requests: PullRequestSnapshot[];
  releases: ReleaseSnapshot[];
  ci_checks: CheckSnapshot[];
  ci_branch?: string;
  ci_commit?: string;
  ci_runs: CiRunSnapshot[];
}

export type GitHubVisibility = "public" | "private" | "internal";

export interface CheckSnapshot {
  context: string;
  state: string;
  required: boolean;
  conclusion?: string;
  last_refreshed_at: string;
  html_url?: string;
  head_sha?: string;
}

export interface PullRequestSnapshot {
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

export interface ReleaseSnapshot {
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
