import { invoke } from "@tauri-apps/api/core";
import { normalizeSkillsSnapshot } from "./skillsSnapshot";
import {
  emptyPapercutBacklog,
  emptyPromotionInbox,
  emptySkills,
} from "./apiDefaults";
import type {
  CreatePapercutInput,
  MultiplierProposal,
  MultiplierProposalStatus,
  PapercutBacklog,
  PapercutStatus,
  PromotionDecision,
  PromotionInbox,
  SkillsSnapshot,
} from "./types";

function isDesktopBridgeAvailable(): boolean {
  return "__TAURI_INTERNALS__" in window;
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

export async function setMultiplierProposalStatus(
  proposalId: string,
  status: MultiplierProposalStatus,
): Promise<MultiplierProposal> {
  if (!isDesktopBridgeAvailable()) {
    throw new Error(
      "Multiplier proposal review is available in the Pronto desktop app.",
    );
  }
  return invoke<MultiplierProposal>("set_multiplier_proposal_status", {
    proposalId,
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
