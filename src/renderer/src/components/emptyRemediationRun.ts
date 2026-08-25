import type { RemediationRun } from "../types/remediation";

export const EMPTY_REMEDIATION_RUN: RemediationRun = {
  schema_version: "unavailable",
  id: "unavailable",
  generated_at: "",
  status: "Unavailable",
  eligible_repository_ids: [],
  eligible_repository_paths: [],
  refresh_steps: [],
  excluded_repositories: [],
  closures: [],
  plans: [],
};
