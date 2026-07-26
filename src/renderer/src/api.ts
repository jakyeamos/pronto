import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ActionPreflight, PortfolioSnapshot } from "./types";

const emptySnapshot: PortfolioSnapshot = {
  roots: [],
  repositories: [],
  products: [],
  groups: [],
  events: [],
  action_audits: [],
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
