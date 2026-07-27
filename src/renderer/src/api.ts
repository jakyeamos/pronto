import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AiPayloadPreview,
  AnalyticsSnapshot,
  ConnectionInput,
  ConnectionNodeInput,
  ExternalTool,
  PortfolioSnapshot,
  RemediationActionStatus,
  RemediationExport,
  RepositoryPreparation,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
  WorkflowInput,
} from "./types";

const fallbackConnectionAdapters = [
  {
    id: "static-discovery",
    enabled: true,
    freshness: "Not run",
    permission_state: "Local read-only",
  },
  {
    id: "deep-code",
    enabled: false,
    freshness: "Not analyzed",
    permission_state: "Opt-in local read-only",
    failure_message:
      "Only validated JavaScript/TypeScript, Rust, and Python fixtures are analyzed.",
  },
  {
    id: "runtime-provider",
    enabled: false,
    freshness: "Not enabled",
    permission_state: "Opt-in provider/runtime read-only",
    failure_message: "Network and runtime queries stay off by default.",
  },
];

export const emptySnapshot: PortfolioSnapshot = {
  roots: [],
  repositories: [],
  products: [],
  groups: [],
  events: [],
  action_audits: [],
  provider_identities: [],
  remote_repositories: [],
  provider_status: {
    provider: "GitHub",
    state: "Not connected",
    message:
      "Connect GitHub through the existing credential manager to load remote context.",
    identity_count: 0,
    repository_count: 0,
  },
  quality: {
    audit_status: "Not configured",
    matched_repository_count: 0,
  },
  connections: {
    nodes: [],
    connections: [],
    workflows: [],
    adapters: fallbackConnectionAdapters,
    generated_at: new Date().toISOString(),
  },
  remediation: {
    schema_version: "pronto-remediation/v1",
    id: "",
    generated_at: new Date().toISOString(),
    source_refresh_id: null,
    status: "not_run",
    message: null,
    eligible_repository_ids: [],
    eligible_repository_paths: [],
    refresh_steps: [],
    excluded_repositories: [],
    plans: [],
  },
  retention_days: 90,
  generated_at: new Date().toISOString(),
  storage_path: "",
};

export const emptyAnalytics: AnalyticsSnapshot = {
  schema_version: "pronto-analytics/v1",
  generated_at: new Date().toISOString(),
  source: "Local refresh snapshots",
  freshness: "Unavailable until the first local refresh",
  range_days: 30,
  retention_days: 90,
  portfolio_samples: [],
  repositories: [],
};

function isDesktopBridgeAvailable(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function getSnapshot(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    return emptySnapshot;
  }
  return invoke<PortfolioSnapshot>("get_snapshot");
}

export async function getAnalytics(): Promise<AnalyticsSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    return emptyAnalytics;
  }
  return invoke<AnalyticsSnapshot>("get_analytics");
}

export async function refresh(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Open Pronto as a desktop app to scan local repositories.");
  }
  return invoke<PortfolioSnapshot>("refresh");
}

export async function refreshConnections(
  repositoryId?: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Connections refresh is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("refresh_connections", {
    repository_id: repositoryId ?? null,
  });
}

export async function upsertConnectionNode(
  input: ConnectionNodeInput,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual connection nodes require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("upsert_connection_node", { input });
}

export async function deleteConnectionNode(
  nodeId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual connection nodes require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("delete_connection_node", {
    node_id: nodeId,
  });
}

export async function upsertConnection(
  input: ConnectionInput,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual connections require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("upsert_connection", { input });
}

export async function deleteConnection(
  connectionId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual connections require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("delete_connection", {
    connection_id: connectionId,
  });
}

export async function upsertWorkflow(
  input: WorkflowInput,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual workflows require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("upsert_workflow", { input });
}

export async function deleteWorkflow(
  workflowId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Manual workflows require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("delete_workflow", {
    workflow_id: workflowId,
  });
}

export async function setConnectionReview(
  recordType: "node" | "connection" | "workflow",
  recordId: string,
  reviewState: "Suggested" | "Confirmed" | "Overridden" | "Hidden",
  label?: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Connection review requires the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("set_connection_review", {
    record_type: recordType,
    record_id: recordId,
    review_state: reviewState,
    label: label ?? null,
  });
}

export async function setConnectionAdapterEnabled(
  adapterId: string,
  enabled: boolean,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Connection adapters require the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("set_connection_adapter_enabled", {
    adapter_id: adapterId,
    enabled,
  });
}

export async function refreshGithub(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("GitHub refresh is available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("refresh_github");
}

export async function refreshRemediation(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Remediation refresh is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("refresh_remediation");
}

export async function setRemediationActionStatus(
  actionId: string,
  status: RemediationActionStatus,
  notes?: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Remediation status updates are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_remediation_action_status", {
    action_id: actionId,
    status,
    notes: notes ?? null,
  });
}

export async function exportRemediation(
  outputDir?: string,
): Promise<RemediationExport> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Remediation exports are available in the Pronto desktop app.",
    );
  }
  return invoke<RemediationExport>("export_remediation", {
    output_dir: outputDir ?? null,
  });
}

export async function openWorkspace(
  repositoryId: string,
  workspaceId: string,
  tool: ExternalTool,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("External handoff is available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("open_workspace", {
    repository_id: repositoryId,
    workspace_id: workspaceId,
    tool,
  });
}

export async function prepareRepository(
  repositoryId: string,
  workspaceId?: string,
): Promise<RepositoryPreparation> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Preparation previews are available in the Pronto desktop app.",
    );
  }
  return invoke<RepositoryPreparation>("prepare_repository", {
    repository_id: repositoryId,
    workspace_id: workspaceId,
  });
}

export async function setReleaseRule(
  repositoryId: string,
  releaseRule: ReleaseRuleConfig | null,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Release rules are available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("set_release_rule", {
    repository_id: repositoryId,
    release_rule: releaseRule,
  });
}

export async function setReleaseRecipe(
  repositoryId: string,
  releaseRecipe: ReleaseRecipeConfig | null,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Release recipes are available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("set_release_recipe", {
    repository_id: repositoryId,
    release_recipe: releaseRecipe,
  });
}

export async function setReleaseVersion(
  repositoryId: string,
  releaseVersion: string | null,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Release version confirmation is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_release_version", {
    repository_id: repositoryId,
    release_version: releaseVersion,
  });
}

export async function setAiPermission(
  repositoryId: string,
  permission: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("AI permissions are available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("set_ai_permission", {
    repository_id: repositoryId,
    permission,
  });
}

export async function previewAiSummary(
  repositoryId: string,
  workspaceId?: string,
): Promise<AiPayloadPreview> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "AI payload previews are available in the Pronto desktop app.",
    );
  }
  return invoke<AiPayloadPreview>("preview_ai_summary", {
    repository_id: repositoryId,
    workspace_id: workspaceId,
  });
}

export async function pickRoot(): Promise<string | null> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Folder selection is available in the Pronto desktop app.");
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Choose a repository discovery root",
  });
  return typeof selected === "string" ? selected : null;
}

export async function openQualityReport(
  reportPath: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Quality reports are available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("open_quality_report", {
    report_path: reportPath,
  });
}

export async function registerRoot(path: string): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Registering a repository root is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("register_root", { path });
}

export async function markExpected(
  repositoryId: string,
  conditionId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Expected conditions are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("mark_condition_expected", {
    repository_id: repositoryId,
    condition_id: conditionId,
  });
}

export async function clearExpected(
  repositoryId: string,
  conditionId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Expected conditions are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("clear_condition_expected", {
    repository_id: repositoryId,
    condition_id: conditionId,
  });
}

export async function updateRootSettings(
  rootId: string,
  ignorePatterns: string[],
  refreshPolicy: string,
  backgroundMonitoring: boolean,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Root settings are available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("update_root_settings", {
    root_id: rootId,
    ignore_patterns: ignorePatterns,
    refresh_policy: refreshPolicy,
    background_monitoring: backgroundMonitoring,
  });
}

export async function setRepositoryLifecycle(
  repositoryId: string,
  lifecycle: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Lifecycle settings are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_repository_lifecycle", {
    repository_id: repositoryId,
    lifecycle,
  });
}

export async function setRetentionDays(
  retentionDays: number,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Retention settings are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_retention_days", {
    retention_days: retentionDays,
  });
}

export async function upsertProduct(
  productId: string | null,
  name: string,
  repositoryIds: string[],
  releaseMode: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Product configuration is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("upsert_product", {
    product_id: productId,
    name,
    repository_ids: repositoryIds,
    release_mode: releaseMode,
  });
}

export async function deleteProduct(
  productId: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Product configuration is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("delete_product", { product_id: productId });
}

export async function upsertGroup(
  groupId: string | null,
  name: string,
  repositoryIds: string[],
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Group configuration is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("upsert_group", {
    group_id: groupId,
    name,
    repository_ids: repositoryIds,
  });
}

export async function deleteGroup(groupId: string): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Group configuration is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("delete_group", { group_id: groupId });
}
