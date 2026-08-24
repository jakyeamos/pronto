import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { emptySnapshot } from "./apiDefaults";
export {
  emptyAnalytics,
  emptyPapercutBacklog,
  emptyPromotionInbox,
  emptySkills,
  emptySnapshot,
} from "./apiDefaults";
export {
  deleteAnalyticsView,
  getAnalytics,
  saveAnalyticsView,
  setDefaultAnalyticsView,
} from "./analyticsApi";
export * from "./apiReviews";
import type {
  AiPayloadPreview,
  ExternalTool,
  PortfolioSnapshot,
  RemediationActionStatus,
  RemediationExport,
  RepositoryPreparation,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
  CiCodexHandoffReceipt,
} from "./types";
import type { TelescopeProjection } from "./types/telescope";

function isDesktopBridgeAvailable(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

interface RefreshBatchReport {
  snapshot: PortfolioSnapshot;
}

export async function getSnapshot(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    return emptySnapshot;
  }
  return invoke<PortfolioSnapshot>("get_snapshot");
}

export async function getRepositoryTelescope(
  repositoryId: string,
): Promise<TelescopeProjection> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Telescope is available in the Pronto desktop app.");
  }
  return invoke<TelescopeProjection>("get_repository_telescope", {
    repositoryId,
  });
}

export async function refreshRepositoryTelescope(
  repositoryId: string,
): Promise<TelescopeProjection> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Telescope refresh is available in the Pronto desktop app.",
    );
  }
  return invoke<TelescopeProjection>("refresh_repository_telescope", {
    repositoryId,
  });
}

export async function cancelRepositoryTelescopeRefresh(
  repositoryId: string,
): Promise<boolean> {
  if (!isDesktopBridgeAvailable()) return false;
  return invoke<boolean>("cancel_repository_telescope_refresh", {
    repositoryId,
  });
}

export async function refresh(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Open Pronto as a desktop app to scan local repositories.");
  }
  const report = await invoke<RefreshBatchReport>("refresh_batch", {
    target: null,
    parallelism: null,
  });
  return report.snapshot;
}

export async function refreshQuality(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Quality refresh is available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("refresh_quality");
}

export async function refreshGithub(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("GitHub refresh is available in the Pronto desktop app.");
  }
  return invoke<PortfolioSnapshot>("refresh_github");
}

export async function startCiCodexHandoff(
  repository: string,
  runId: number,
  runAttempt: number,
): Promise<CiCodexHandoffReceipt> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Codex CI handoff is available in the Pronto desktop app.");
  }
  return invoke<CiCodexHandoffReceipt>("start_ci_codex_handoff", {
    repository,
    runId,
    runAttempt,
  });
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
    actionId,
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
    outputDir: outputDir ?? null,
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
    repositoryId,
    workspaceId,
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
    repositoryId,
    workspaceId,
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
    repositoryId,
    releaseRule,
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
    repositoryId,
    releaseRecipe,
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
    repositoryId,
    releaseVersion,
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
    repositoryId,
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
    repositoryId,
    workspaceId,
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
    reportPath,
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
    repositoryId,
    conditionId,
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
    repositoryId,
    conditionId,
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
    rootId,
    ignorePatterns,
    refreshPolicy,
    backgroundMonitoring,
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
    repositoryId,
    lifecycle,
  });
}

export async function setRepositoryTargetBranch(
  repositoryId: string,
  targetBranch: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Target branch settings are available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("set_repository_target_branch", {
    repositoryId,
    targetBranch,
  });
}

export async function refreshRepositoryTargetEvidence(
  repositoryId: string,
  targetBranch: string,
): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Target evidence refresh is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("refresh_repository_target_evidence", {
    repositoryId,
    targetBranch,
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
    retentionDays,
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
    productId,
    name,
    repositoryIds,
    releaseMode,
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
  return invoke<PortfolioSnapshot>("delete_product", { productId });
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
    groupId,
    name,
    repositoryIds,
  });
}

export async function deleteGroup(groupId: string): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Group configuration is available in the Pronto desktop app.",
    );
  }
  return invoke<PortfolioSnapshot>("delete_group", { groupId });
}
