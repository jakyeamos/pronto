import {
  Activity,
  FolderGit2,
  GitBranch,
  LayoutDashboard,
  PackageOpen,
  Settings2,
} from "lucide-react";

export type NavItem =
  "command" | "products" | "groups" | "remote" | "activity" | "settings";

export const navItems: Array<{
  id: NavItem;
  label: string;
  icon: typeof LayoutDashboard;
}> = [
  { id: "command", label: "Command center", icon: LayoutDashboard },
  { id: "products", label: "Products", icon: PackageOpen },
  { id: "groups", label: "Groups", icon: FolderGit2 },
  { id: "remote", label: "Remote catalog", icon: GitBranch },
  { id: "activity", label: "Activity", icon: Activity },
  { id: "settings", label: "Settings", icon: Settings2 },
];

export const pageCopy: Record<
  NavItem,
  { eyebrow: string; title: string; body: string }
> = {
  command: {
    eyebrow: "Local evidence",
    title: "Know what needs attention.",
    body: "A factual view of your projects, workspaces, and Git state—freshness included.",
  },
  products: {
    eyebrow: "Manual configuration",
    title: "Give the portfolio a shape.",
    body: "Name operational products, attach repositories intentionally, and keep release modes explicit.",
  },
  groups: {
    eyebrow: "Manual configuration",
    title: "Keep related work together.",
    body: "Use explicit labels across repositories without asking Pronto to guess your organization.",
  },
  remote: {
    eyebrow: "Read-only provider boundary",
    title: "Remote context comes second.",
    body: "The local portfolio is ready first; a read-only GitHub catalog will add remote context after durable state is in place.",
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
