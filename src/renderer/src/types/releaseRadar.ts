export interface ReleaseRadarCounts {
  total: number;
  local_only: number;
  github_only: number;
  local_and_remote: number;
  public_fresh: number;
  private_or_internal_fresh: number;
  unknown_or_blocked: number;
  public_framing_eligible: number;
  profile_needs_sync: number;
}

export interface ReleaseRadarRow {
  id: string;
  name: string;
  full_name?: string;
  locality: string;
  github_url?: string;
  visibility: string;
  visibility_freshness: string;
  public_framing: string;
  profile_link_sync: string;
  product_readiness: string;
  demo_materials: string;
  release_readiness: string;
  next_action: string;
  blocker?: string;
  repository_id?: string;
  remote_repository_id?: string;
}

export interface ReleaseRadarSnapshot {
  schema_version: "pronto-release-radar/v1";
  generated_at: string;
  provider_state: string;
  provider_last_refresh_at?: string | null;
  profile_state: string;
  counts: ReleaseRadarCounts;
  rows: ReleaseRadarRow[];
}
