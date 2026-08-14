import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { normalizeSkillsSnapshot } from "./skillsSnapshot";
import type {
  AiPayloadPreview,
  AnalyticsSnapshot,
  AnalyticsView,
  SkillsSnapshot,
  ExternalTool,
  PortfolioSnapshot,
  RemediationActionStatus,
  RemediationExport,
  RepositoryPreparation,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
  PromotionDecision,
  PromotionInbox,
  CreatePapercutInput,
  PapercutBacklog,
  PapercutStatus,
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
  remediation: {
    schema_version: "pronto-remediation/v3",
    id: "",
    generated_at: new Date().toISOString(),
    source_refresh_id: null,
    status: "not_run",
    message: null,
    eligible_repository_ids: [],
    eligible_repository_paths: [],
    refresh_steps: [],
    excluded_repositories: [],
    closures: [],
    github_only_candidates: [],
    plans: [],
  },
  showcase: {
    schema_version: "pronto-showcase/v2",
    status: "Missing",
    contract_path: ".pronto/showcase-goal.json",
    reviewed_at: null,
    quality_bar_source: null,
    goal: {
      target_publishable_demo_count: 0,
      publishable_demo_count: 0,
      remaining_demo_count: 0,
      status: "Not configured",
    },
    scoring: null,
    public_queue: [],
    private_client_count: 0,
    projects: [],
    error: null,
  },
  retention_days: 90,
  generated_at: new Date().toISOString(),
  storage_path: "",
};

export const emptyAnalytics: AnalyticsSnapshot = {
  schema_version: "pronto-analytics/v2",
  generated_at: new Date().toISOString(),
  source: "Local refresh snapshots",
  freshness: "Unavailable until the first local refresh",
  range_days: 30,
  retention_days: 90,
  portfolio_samples: [],
  repositories: [],
  metric_catalog: [],
  findings: [],
  views: [],
  default_view_id: "curated",
};

export const emptySkills: SkillsSnapshot = {
  schema_version: "pronto-skills/v4",
  generated_at: new Date().toISOString(),
  freshness: "Unavailable until the first skills refresh",
  source: "Local skill roots; usage requires structured provider telemetry",
  recent_days: 30,
  roots: [],
  skills: [],
  telemetry_gap: "No skills refresh has been recorded.",
};

export const emptyPromotionInbox: PromotionInbox = {
  schema_version: "leverage-promotion-inbox/v1",
  visibility: "private",
  generated_at: new Date().toISOString(),
  source_root: "",
  candidates: [],
  counts: {
    total: 0,
    pending: 0,
    deferred: 0,
    rejected: 0,
    accepted: 0,
    complete: 0,
    drafts: 0,
  },
  coverage: null,
  discovery: null,
  errors: [],
  manual_review_required: true,
  jas_mutation: false,
  status: "unavailable",
  provenance_hash: "",
  message: "Promotion review is available in the Pronto desktop app.",
  jas_admission: null,
};

export const emptyPapercutBacklog: PapercutBacklog = {
  schema_version: "pronto-papercuts/v1",
  family: "design-audit",
  generated_at: new Date().toISOString(),
  papercuts: [],
  counts: {
    total: 0,
    open: 0,
    in_progress: 0,
    deferred: 0,
    resolved: 0,
  },
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

export async function getAnalytics(rangeDays = 30): Promise<AnalyticsSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    return emptyAnalytics;
  }
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

export async function getSkills(): Promise<SkillsSnapshot> {
  if (!isDesktopBridgeAvailable()) return emptySkills;
  return normalizeSkillsSnapshot(await invoke<unknown>("get_skills"));
}

export async function getPromotionInbox(): Promise<PromotionInbox> {
  if (!isDesktopBridgeAvailable()) return emptyPromotionInbox;
  return invoke<PromotionInbox>("get_promotion_inbox");
}

export async function refreshPromotionInbox(): Promise<PromotionInbox> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Promotion review is available in the Pronto desktop app.");
  }
  return invoke<PromotionInbox>("get_promotion_inbox");
}

export async function decidePromotion(
  candidateId: string,
  decision: PromotionDecision,
  reason?: string,
): Promise<PromotionInbox> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Promotion decisions are available in the Pronto desktop app.",
    );
  }
  return invoke<PromotionInbox>("decide_promotion", {
    candidateId,
    decision,
    reason: reason?.trim() || null,
  });
}

export async function getPapercutBacklog(): Promise<PapercutBacklog> {
  if (!isDesktopBridgeAvailable()) return emptyPapercutBacklog;
  return invoke<PapercutBacklog>("get_papercut_backlog");
}

export async function refreshPapercutBacklog(): Promise<PapercutBacklog> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Papercut review is available in the Pronto desktop app.");
  }
  return invoke<PapercutBacklog>("get_papercut_backlog");
}

export async function createPapercut(
  input: CreatePapercutInput,
): Promise<PapercutBacklog> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Papercut capture is available in the Pronto desktop app.");
  }
  return invoke<PapercutBacklog>("create_papercut", {
    title: input.title.trim(),
    detail: input.detail.trim(),
    surface: input.surface.trim(),
    source: input.source,
    priority: input.priority,
    evidenceRefs: input.evidenceRefs
      .map((value) => value.trim())
      .filter(Boolean),
    impact: input.impact.trim(),
    nextAction: input.nextAction.trim(),
  });
}

export async function setPapercutStatus(
  papercutId: string,
  status: PapercutStatus,
): Promise<PapercutBacklog> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Papercut status updates are available in the Pronto desktop app.",
    );
  }
  return invoke<PapercutBacklog>("set_papercut_status", {
    papercutId,
    status,
  });
}

export async function refreshSkills(): Promise<SkillsSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Skills refresh is available in the Pronto desktop app.");
  }
  return normalizeSkillsSnapshot(await invoke<unknown>("refresh_skills"));
}

export async function openSkillSource(path: string): Promise<void> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Opening skill sources is available in the Pronto desktop app.",
    );
  }
  await invoke("open_skill_source", { path });
}

export async function refresh(): Promise<PortfolioSnapshot> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error("Open Pronto as a desktop app to scan local repositories.");
  }
  return invoke<PortfolioSnapshot>("refresh");
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
