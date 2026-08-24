import {
  Activity,
  ChartNoAxesCombined,
  ClipboardCheck,
  CircleAlert,
  FolderGit2,
  GitBranch,
  Inbox,
  LayoutDashboard,
  Film,
  Settings2,
  Sparkles,
} from "lucide-react";

export type NavItem =
  | "portfolio"
  | "showcase"
  | "remediation"
  | "promotions"
  | "analytics"
  | "skills"
  | "groups"
  | "remote"
  | "ci"
  | "activity"
  | "settings";

export const navItems: Array<{
  id: NavItem;
  label: string;
  icon: typeof LayoutDashboard;
}> = [
  { id: "portfolio", label: "Portfolio", icon: LayoutDashboard },
  { id: "showcase", label: "AI showcase", icon: Film },
  { id: "remediation", label: "Remediation", icon: ClipboardCheck },
  { id: "promotions", label: "Promotion inbox", icon: Inbox },
  { id: "analytics", label: "Analytics", icon: ChartNoAxesCombined },
  { id: "skills", label: "Skills", icon: Sparkles },
  { id: "groups", label: "Groups", icon: FolderGit2 },
  { id: "remote", label: "Remote catalog", icon: GitBranch },
  { id: "ci", label: "CI tracker", icon: CircleAlert },
  { id: "activity", label: "Activity", icon: Activity },
  { id: "settings", label: "Settings", icon: Settings2 },
];

export const pageCopy: Record<
  NavItem,
  { eyebrow: string; title: string; body: string }
> = {
  portfolio: {
    eyebrow: "Local evidence",
    title: "Know what needs attention.",
    body: "A factual view of your projects, workspaces, and Git state—freshness included.",
  },
  showcase: {
    eyebrow: "Recruiter-facing proof",
    title: "Turn the full fleet into a publishing plan.",
    body: "Rank every registered repository by product and demo readiness, then prioritize public materials without exposing client work.",
  },
  remediation: {
    eyebrow: "Fresh evidence → focused work",
    title: "Turn the latest scan into a plan.",
    body: "Review per-repository actions from current QR, maturity, CI ideal-state, provider, branch, and local evidence.",
  },
  promotions: {
    eyebrow: "AWL → JAS handoff",
    title: "Choose what earns a place in JAS.",
    body: "Review evidence-backed candidates from ai-workflow-leverage before they enter the public base or private overlay.",
  },
  groups: {
    eyebrow: "Manual configuration",
    title: "Keep related work together.",
    body: "Use explicit labels across repositories without asking Pronto to guess your organization.",
  },
  analytics: {
    eyebrow: "Local refresh history",
    title: "See the portfolio move.",
    body: "Read-only trends for health, delivery, quality, workspace friction, and release readiness from local refresh evidence.",
  },
  skills: {
    eyebrow: "Provider-neutral corpus",
    title: "Know what your skills are doing.",
    body: "Inspect sources, provider compatibility, hosting, and verified usage availability without treating prompts or skill catalogs as telemetry.",
  },
  remote: {
    eyebrow: "Read-only provider boundary",
    title: "Remote context comes second.",
    body: "The local portfolio is ready first; a read-only GitHub catalog will add remote context after durable state is in place.",
  },
  ci: {
    eyebrow: "Read-only CI evidence",
    title: "See failures, then start the right task.",
    body: "GitHub remains the source of truth. Pronto groups recent failed runs, explains the failing jobs, and hands a bounded prompt to Codex only when you ask.",
  },
  activity: {
    eyebrow: "Local action history",
    title: "See what changed.",
    body: "Pronto records safe local actions and meaningful state transitions, not a noisy scan log.",
  },
  settings: {
    eyebrow: "Local configuration",
    title: "Keep the boundary visible.",
    body: "Manage discovery roots and understand where Pronto keeps its private local snapshot.",
  },
};
