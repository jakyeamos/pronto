import { invoke } from "@tauri-apps/api/core";
import { emptyAnalytics } from "./apiDefaults";
import type { AnalyticsSnapshot, AnalyticsView } from "./types";

function isDesktopBridgeAvailable(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function getAnalytics(rangeDays = 30): Promise<AnalyticsSnapshot> {
  if (!isDesktopBridgeAvailable()) return emptyAnalytics;
  return invoke<AnalyticsSnapshot>("get_analytics", { rangeDays });
}

export async function saveAnalyticsView(
  view: AnalyticsView,
): Promise<AnalyticsView[]> {
  if (!isDesktopBridgeAvailable()) return [];
  return invoke("save_analytics_view", { view });
}

export async function deleteAnalyticsView(
  viewId: string,
): Promise<AnalyticsView[]> {
  if (!isDesktopBridgeAvailable()) return [];
  return invoke("delete_analytics_view", { viewId });
}

export async function setDefaultAnalyticsView(
  viewId: string,
): Promise<AnalyticsView[]> {
  if (!isDesktopBridgeAvailable()) return [];
  return invoke("set_default_analytics_view", { viewId });
}
