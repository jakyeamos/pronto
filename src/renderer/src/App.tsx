import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";
import {
  AlertTriangle,
  Activity,
  ChevronRight,
  Command,
  FolderGit2,
  FolderPlus,
  GitBranch,
  LayoutDashboard,
  LoaderCircle,
  PackageOpen,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  X,
} from "lucide-react";
import * as api from "./api";
import {
  CommandCenterSurface,
  type Filter,
} from "./components/CommandCenterSurface";
import { DetailDrawer, EvidenceDrawer } from "./components/Drawers";
import {
  formatTime,
  IconButton,
  StatusPill,
} from "./components/ConsolePrimitives";
import {
  ActivitySurface,
  DeferredSurface,
  SettingsSurface,
} from "./components/WorkspaceSurfaces";
import type { Condition, PortfolioSnapshot, RepositorySnapshot } from "./types";
import "./styles.css";

type NavItem =
  "command" | "products" | "groups" | "remote" | "activity" | "settings";

const navItems: Array<{
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

const pageCopy: Record<
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
    body: "Products will group repositories by the work you choose to name and maintain.",
  },
  groups: {
    eyebrow: "Manual configuration",
    title: "Keep related work together.",
    body: "Groups will provide an intentional view across repositories without guessing your organization.",
  },
  remote: {
    eyebrow: "Read-only provider boundary",
    title: "Remote context comes second.",
    body: "The local portfolio is ready first; a read-only GitHub catalog will add remote context after durable state is in place.",
  },
  activity: {
    eyebrow: "Transition-only history",
    title: "See what changed.",
    body: "Pronto records meaningful local state transitions, not a noisy scan log.",
  },
  settings: {
    eyebrow: "Local configuration",
    title: "Keep the boundary visible.",
    body: "Manage discovery roots and understand where Pronto keeps its private local snapshot.",
  },
};

export function App(): ReactElement {
  const [snapshot, setSnapshot] = useState<PortfolioSnapshot>({
    roots: [],
    repositories: [],
    events: [],
    generated_at: "",
    storage_path: "",
  });
  const [activeNav, setActiveNav] = useState<NavItem>("command");
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [selectedRepository, setSelectedRepository] =
    useState<RepositorySnapshot | null>(null);
  const [selectedEvidence, setSelectedEvidence] = useState<{
    repository: RepositorySnapshot;
    condition: Condition;
  } | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const loadSnapshot = useCallback(
    async (operation: () => Promise<PortfolioSnapshot>): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      try {
        setSnapshot(await operation());
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not update the portfolio.",
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  useEffect(() => {
    void loadSnapshot(api.getSnapshot);
  }, [loadSnapshot]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchInputRef.current?.focus();
      }
      if (event.key === "Escape") {
        if (selectedEvidence) setSelectedEvidence(null);
        else if (selectedRepository) setSelectedRepository(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedEvidence, selectedRepository]);

  const handleAddRoot = useCallback(async (): Promise<void> => {
    try {
      const root = await api.pickRoot();
      if (root) await loadSnapshot(() => api.registerRoot(root));
    } catch (caught) {
      setError(
        caught instanceof Error
          ? caught.message
          : "Pronto could not register that root.",
      );
    }
  }, [loadSnapshot]);

  const repositories = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return snapshot.repositories
      .filter((repository) => {
        if (!normalizedQuery) return true;
        return [
          repository.name,
          repository.path,
          repository.branch,
          ...repository.conditions.map((condition) => condition.title),
        ].some((value) => value.toLowerCase().includes(normalizedQuery));
      })
      .filter((repository) => {
        if (filter === "attention")
          return repository.conditions.some(
            (condition) => condition.status === "Active",
          );
        if (filter === "dirty") return repository.workspace.dirty;
        if (filter === "sync")
          return repository.workspace.sync_state !== "Synced";
        return true;
      });
  }, [filter, query, snapshot.repositories]);

  const activeConditionCount = snapshot.repositories.reduce(
    (total, repository) =>
      total +
      repository.conditions.filter((condition) => condition.status === "Active")
        .length,
    0,
  );
  const dirtyCount = snapshot.repositories.filter(
    (repository) => repository.workspace.dirty,
  ).length;
  const unsyncedCount = snapshot.repositories.filter(
    (repository) => repository.workspace.sync_state !== "Synced",
  ).length;
  const activePage = pageCopy[activeNav];
  const activeNavLabel = navItems.find((item) => item.id === activeNav)?.label;
  const dateLabel = new Intl.DateTimeFormat("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).format(new Date());
  const isCommandCenter = activeNav === "command";

  const handleCondition = (
    repository: RepositorySnapshot,
    condition: Condition,
  ): void => setSelectedEvidence({ repository, condition });
  const handleExpected = async (): Promise<void> => {
    if (!selectedEvidence) return;
    const { repository, condition } = selectedEvidence;
    await loadSnapshot(() =>
      condition.status === "Expected"
        ? api.clearExpected(repository.id, condition.id)
        : api.markExpected(repository.id, condition.id),
    );
    setSelectedEvidence(null);
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">P</div>
          <div>
            <strong>Pronto</strong>
            <span>Portfolio command center</span>
          </div>
        </div>
        <div className="sidebar-rule" />
        <nav className="primary-nav" aria-label="Primary navigation">
          {navItems.map(({ id, label, icon: Icon }) => (
            <button
              className={`nav-item ${activeNav === id ? "nav-item-active" : ""}`}
              type="button"
              key={id}
              aria-current={activeNav === id ? "page" : undefined}
              onClick={() => {
                setActiveNav(id);
                setSelectedRepository(null);
                setSelectedEvidence(null);
              }}
            >
              <Icon size={17} />
              <span>{label}</span>
              {id === "command" && activeConditionCount > 0 && (
                <span className="nav-count">{activeConditionCount}</span>
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-status">
            <span className="status-beacon" />
            <div>
              <strong>Local evidence only</strong>
              <span>
                {snapshot.roots.length} discovery root
                {snapshot.roots.length === 1 ? "" : "s"}
              </span>
            </div>
          </div>
          <div className="privacy-card">
            <ShieldCheck size={16} />
            <p>
              <strong>Private by default</strong>
              <span>Source and uncommitted diff content stay local.</span>
            </p>
          </div>
        </div>
      </aside>
      <main className="main-content">
        <header className="topbar">
          <div className="breadcrumbs">
            <span>Workspace</span>
            <ChevronRight size={13} />
            <strong>{activeNavLabel}</strong>
          </div>
          <div className="topbar-actions">
            <label className="search-box">
              <Search size={15} />
              <input
                ref={searchInputRef}
                aria-label="Search repositories"
                placeholder={
                  isCommandCenter
                    ? "Search repos, branches, paths"
                    : "Search local portfolio"
                }
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
              <kbd>
                <Command size={11} />K
              </kbd>
            </label>
            <IconButton
              label="Refresh local evidence"
              onClick={() => void loadSnapshot(api.refresh)}
              disabled={isRefreshing}
            >
              {isRefreshing ? (
                <LoaderCircle className="spin" size={17} />
              ) : (
                <RefreshCw size={17} />
              )}
            </IconButton>
            <div className="avatar">JA</div>
          </div>
        </header>
        <div className="content-scroll">
          <section className="page-intro">
            <div>
              <p className="eyebrow">
                {isCommandCenter ? dateLabel : activePage.eyebrow}
              </p>
              <h1>{activePage.title}</h1>
              <p className="intro-copy">{activePage.body}</p>
            </div>
            <div className="intro-actions">
              {isCommandCenter && (
                <span className="snapshot-freshness">
                  Snapshot {formatTime(snapshot.generated_at)}
                </span>
              )}
              <StatusPill tone="mint" icon={<ShieldCheck size={12} />}>
                Local only
              </StatusPill>
              {(isCommandCenter || activeNav === "settings") && (
                <button
                  className="button button-secondary"
                  type="button"
                  onClick={handleAddRoot}
                >
                  <FolderPlus size={15} />
                  Add root
                </button>
              )}
            </div>
          </section>
          {error && (
            <div className="error-banner" role="alert">
              <AlertTriangle size={16} />
              <span>{error}</span>
              <button
                type="button"
                onClick={() => setError(null)}
                aria-label="Dismiss error"
              >
                <X size={14} />
              </button>
            </div>
          )}
          {isCommandCenter ? (
            <CommandCenterSurface
              activeConditionCount={activeConditionCount}
              dirtyCount={dirtyCount}
              unsyncedCount={unsyncedCount}
              repositoryCount={snapshot.repositories.length}
              rootCount={snapshot.roots.length}
              repositories={repositories}
              allRepositories={snapshot.repositories}
              events={snapshot.events}
              filter={filter}
              onFilterChange={setFilter}
              onClearFilters={() => {
                setQuery("");
                setFilter("all");
              }}
              onAddRoot={handleAddRoot}
              onOpenRepository={setSelectedRepository}
              onCondition={handleCondition}
            />
          ) : activeNav === "activity" ? (
            <ActivitySurface events={snapshot.events} />
          ) : activeNav === "settings" ? (
            <SettingsSurface
              roots={snapshot.roots}
              storagePath={snapshot.storage_path}
              generatedAt={snapshot.generated_at}
              onAddRoot={handleAddRoot}
            />
          ) : activeNav === "products" ? (
            <DeferredSurface
              eyebrow="Accepted product decision"
              title="Products will be manual first."
              body="Pronto will let you name the work that matters to you and attach repositories intentionally. Inference can follow once the durable model is stable."
              icon={<PackageOpen size={19} />}
              details={[
                { label: "Authority", value: "User-defined" },
                { label: "State", value: "Planned after SQLite" },
              ]}
            />
          ) : activeNav === "groups" ? (
            <DeferredSurface
              eyebrow="Accepted group decision"
              title="Groups will stay explicit."
              body="Related repositories will be grouped by configuration, not inferred silently. The first release will favor predictable structure over automation."
              icon={<FolderGit2 size={19} />}
              details={[
                { label: "Authority", value: "User-defined" },
                { label: "State", value: "Planned after SQLite" },
              ]}
            />
          ) : (
            <DeferredSurface
              eyebrow="Accepted provider decision"
              title="Read-only GitHub comes later."
              body="Remote context will be additive and read-only at first. No credentials, network refresh, pull request mutation, or release publishing is active in this local slice."
              icon={<GitBranch size={19} />}
              details={[
                { label: "Permission", value: "Read-only" },
                { label: "Prerequisite", value: "SQLite state" },
              ]}
            />
          )}
        </div>
      </main>
      {selectedRepository && (
        <DetailDrawer
          repository={selectedRepository}
          onClose={() => setSelectedRepository(null)}
          onCondition={(condition) => {
            setSelectedRepository(null);
            handleCondition(selectedRepository, condition);
          }}
        />
      )}
      {selectedEvidence && (
        <EvidenceDrawer
          repository={selectedEvidence.repository}
          condition={selectedEvidence.condition}
          onClose={() => setSelectedEvidence(null)}
          onExpected={() => void handleExpected()}
        />
      )}
    </div>
  );
}
