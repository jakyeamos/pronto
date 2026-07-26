import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ActionPreflight,
  AiPayloadPreview,
  ExternalTool,
  PortfolioSnapshot,
  RepositoryPreparation,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
} from "./types";

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
  retention_days: 90,
  generated_at: new Date().toISOString(),
  storage_path: "",
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

export async function refresh(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Open Pronto as a desktop app to scan local repositories.");
  }
  return invoke<PortfolioSnapshot>("refresh");
}

export async function refreshGithub(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("GitHub refresh is available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("refresh_github");
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

export async function preflightAction(
  action: string,
  repositoryId?: string,
): Promise<ActionPreflight> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Action preflight is available in the Pronto desktop app.");
  }
  return invoke<ActionPreflight>("preflight_action", {
    action,
    repository_id: repositoryId ?? null,
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

export async function pickMaturityAuditRoot(): Promise<string | null> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Folder selection is available in the Pronto desktop app.");
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Choose a maturity audit root",
  });
  return typeof selected === "string" ? selected : null;
}

export async function setMaturityAuditRoot(
  auditRoot: string | null,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Maturity audit settings are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_maturity_audit_root", {
    audit_root: auditRoot,
  });
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
