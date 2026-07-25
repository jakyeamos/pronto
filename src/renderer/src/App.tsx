import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import {
  Activity,
  AlertTriangle,
  Archive,
  ChevronDown,
  ChevronRight,
  Command,
  Copy,
  FolderGit2,
  FolderPlus,
  GitBranch,
  LayoutDashboard,
  LoaderCircle,
  MoreHorizontal,
  PackageOpen,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  X,
} from "lucide-react";
import * as api from "./api";
import { DetailDrawer, EvidenceDrawer } from "./components/Drawers";
import {
  EmptyState,
  IconButton,
  StatusPill,
} from "./components/ConsolePrimitives";
import {
  AttentionQueue,
  RepositoryRow,
  Timeline,
} from "./components/PortfolioComponents";
import type { Condition, PortfolioSnapshot, RepositorySnapshot } from "./types";
import "./styles.css";

type Filter = "all" | "attention" | "dirty" | "sync";
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
              onClick={() => setActiveNav(id)}
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
            <strong>Command center</strong>
          </div>
          <div className="topbar-actions">
            <label className="search-box">
              <Search size={15} />
              <input
                aria-label="Search repositories"
                placeholder="Search repos, branches, paths"
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
              <p className="eyebrow">Saturday · July 25, 2026</p>
              <h1>Know what needs attention.</h1>
              <p className="intro-copy">
                A factual view of your projects, workspaces, and Git
                state—freshness included.
              </p>
            </div>
            <div className="intro-actions">
              <StatusPill tone="mint" icon={<ShieldCheck size={12} />}>
                No cloud account
              </StatusPill>
              <button
                className="button button-secondary"
                type="button"
                onClick={handleAddRoot}
              >
                <FolderPlus size={15} />
                Add root
              </button>
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
          <section className="metric-grid">
            <div className="metric-card metric-card-accent">
              <span>Active conditions</span>
              <strong>{activeConditionCount}</strong>
              <small>Grouped by repository</small>
              <AlertTriangle size={18} />
            </div>
            <div className="metric-card">
              <span>Dirty workspaces</span>
              <strong>{dirtyCount}</strong>
              <small>Aggregate line deltas only</small>
              <FolderGit2 size={18} />
            </div>
            <div className="metric-card">
              <span>Unsynced branches</span>
              <strong>{unsyncedCount}</strong>
              <small>Remote freshness visible</small>
              <GitBranch size={18} />
            </div>
            <div className="metric-card">
              <span>Repositories</span>
              <strong>{snapshot.repositories.length}</strong>
              <small>
                {snapshot.roots.length} registered root
                {snapshot.roots.length === 1 ? "" : "s"}
              </small>
              <Archive size={18} />
            </div>
          </section>
          <div className="content-grid">
            <section className="portfolio-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Portfolio first</p>
                  <h2>
                    Repository portfolio <span>{repositories.length}</span>
                  </h2>
                </div>
                <div className="panel-heading-actions">
                  <button
                    className="button button-quiet"
                    type="button"
                    disabled
                  >
                    <MoreHorizontal size={16} />
                    Customize
                  </button>
                </div>
              </div>
              <div className="filter-bar">
                <div className="filter-tabs">
                  {(["all", "attention", "dirty", "sync"] as Filter[]).map(
                    (item) => (
                      <button
                        className={
                          filter === item
                            ? "filter-tab filter-tab-active"
                            : "filter-tab"
                        }
                        type="button"
                        key={item}
                        onClick={() => setFilter(item)}
                      >
                        {item === "all"
                          ? "All repositories"
                          : item === "attention"
                            ? "Needs attention"
                            : item === "dirty"
                              ? "Dirty"
                              : "Sync state"}
                      </button>
                    ),
                  )}
                </div>
                <button className="sort-button" type="button" disabled>
                  <span>Priority</span>
                  <ChevronDown size={14} />
                </button>
              </div>
              {repositories.length === 0 ? (
                <EmptyState
                  onAddRoot={handleAddRoot}
                  hasRoots={snapshot.roots.length > 0}
                />
              ) : (
                <div className="repository-list">
                  {repositories.map((repository) => (
                    <RepositoryRow
                      key={repository.id}
                      repository={repository}
                      onOpen={() => setSelectedRepository(repository)}
                      onCondition={(condition) =>
                        handleCondition(repository, condition)
                      }
                    />
                  ))}
                </div>
              )}
              <div className="panel-footnote">
                <Copy size={13} />
                Facts are scanned locally. Pronto never shows filenames or
                uncommitted diff content in this view.
              </div>
            </section>
            <aside className="right-rail">
              <AttentionQueue
                repositories={snapshot.repositories}
                onCondition={handleCondition}
              />
              <Timeline events={snapshot.events} />
              <section className="provider-card">
                <div className="provider-icon">
                  <Sparkles size={17} />
                </div>
                <div>
                  <p className="eyebrow">Next boundary</p>
                  <h3>GitHub stays explicit</h3>
                  <p>
                    Provider refresh, pull requests, and release rules will
                    appear here once an identity is connected.
                  </p>
                </div>
                <ChevronRight size={16} />
              </section>
            </aside>
          </div>
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
