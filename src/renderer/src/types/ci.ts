export interface CiJobSnapshot {
  id: number;
  name: string;
  status: string;
  conclusion?: string;
  html_url?: string;
  failed_steps: string[];
}

export interface CiPromptArtifactSnapshot {
  id: number;
  name: string;
  expired: boolean;
}

export interface CiRunSnapshot {
  id: number;
  workflow_name: string;
  workflow_path?: string;
  display_title: string;
  run_number: number;
  run_attempt: number;
  event: string;
  status: string;
  conclusion?: string;
  head_branch?: string;
  head_sha: string;
  html_url: string;
  created_at?: string;
  updated_at?: string;
  pull_request_number?: number;
  is_fork: boolean;
  jobs: CiJobSnapshot[];
  failure_summary?: string;
  failure_signature?: string;
  prompt_artifact?: CiPromptArtifactSnapshot;
  last_refreshed_at: string;
}

export interface CiCodexHandoffReceipt {
  schema_version: string;
  status: string;
  repository: string;
  run_id: number;
  run_attempt: number;
  failure_signature?: string;
  prompt_directory: string;
  started: boolean;
  message: string;
}
