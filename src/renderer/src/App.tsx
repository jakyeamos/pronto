import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";
import {
  AlertTriangle,
  ChevronRight,
  Command,
  FolderPlus,
  GitBranch,
  LoaderCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import * as api from "./api";
import {
  CommandCenterSurface,
  type Filter,
} from "./components/CommandCenterSurface";
import { AppOverlays } from "./components/AppOverlays";
import { AppSidebar } from "./components/AppSidebar";
import {
  formatTime,
  IconButton,
  StatusPill,
} from "./components/ConsolePrimitives";
import { PortfolioConfigSurface } from "./components/PortfolioConfigSurface";
import { RemoteCatalogSurface } from "./components/RemoteCatalogSurface";
import {
  ActivitySurface,
  DeferredSurface,
  SettingsSurface,
} from "./components/WorkspaceSurfaces";
import { navItems, pageCopy, type NavItem } from "./navigation";
import type {
  Condition,
  ExternalTool,
  PortfolioSnapshot,
  RepositoryPreparation,
  RepositorySnapshot,
  ReleaseRuleConfig,
} from "./types";
import "./styles.css";

export function App(): ReactElement {
  const [snapshot, setSnapshot] = useState<PortfolioSnapshot>({
    roots: [],
    repositories: [],
    events: [],
    action_audits: [],
    products: [],
    groups: [],
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
    retention_days: 90,
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
  const [selectedPreparation, setSelectedPreparation] = useState<{
    repository: RepositorySnapshot;
    preparation: RepositoryPreparation;
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
        else if (selectedPreparation) setSelectedPreparation(null);
        else if (selectedRepository) setSelectedRepository(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedEvidence, selectedPreparation, selectedRepository]);

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

  const handleSaveRoot = useCallback(
    async (
      rootId: string,
      ignorePatterns: string[],
      refreshPolicy: string,
      backgroundMonitoring: boolean,
    ): Promise<void> => {
      await loadSnapshot(() =>
        api.updateRootSettings(
          rootId,
          ignorePatterns,
          refreshPolicy,
          backgroundMonitoring,
        ),
      );
    },
    [loadSnapshot],
  );

  const handleSaveRetention = useCallback(
    async (retentionDays: number): Promise<void> => {
      await loadSnapshot(() => api.setRetentionDays(retentionDays));
    },
    [loadSnapshot],
  );

  const handleSaveProduct = useCallback(
    async (
      productId: string | null,
      name: string,
      repositoryIds: string[],
      releaseMode: string,
    ): Promise<void> => {
      await loadSnapshot(() =>
        api.upsertProduct(productId, name, repositoryIds, releaseMode),
      );
    },
    [loadSnapshot],
  );

  const handleDeleteProduct = useCallback(
    async (productId: string): Promise<void> => {
      await loadSnapshot(() => api.deleteProduct(productId));
    },
    [loadSnapshot],
  );

  const handleSaveGroup = useCallback(
    async (
      groupId: string | null,
      name: string,
      repositoryIds: string[],
    ): Promise<void> => {
      await loadSnapshot(() => api.upsertGroup(groupId, name, repositoryIds));
    },
    [loadSnapshot],
  );

  const handleDeleteGroup = useCallback(
    async (groupId: string): Promise<void> => {
      await loadSnapshot(() => api.deleteGroup(groupId));
    },
    [loadSnapshot],
  );

  const handleRefreshGithub = useCallback(async (): Promise<void> => {
    await loadSnapshot(api.refreshGithub);
  }, [loadSnapshot]);

  const handleOpenWorkspace = useCallback(
    async (workspaceId: string, tool: ExternalTool): Promise<void> => {
      if (!selectedRepository) return;
      try {
        await api.openWorkspace(selectedRepository.id, workspaceId, tool);
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not open that external tool.",
        );
      }
    },
    [selectedRepository],
  );

  const handlePrepareRepository = useCallback(
    async (workspaceId?: string): Promise<void> => {
      if (!selectedRepository) return;
      try {
        const preparation = await api.prepareRepository(
          selectedRepository.id,
          workspaceId,
        );
        setSelectedPreparation({
          repository: selectedRepository,
          preparation,
        });
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not prepare that evidence preview.",
        );
      }
    },
    [selectedRepository],
  );

  const handleSaveReleaseRule = useCallback(
    async (releaseRule: ReleaseRuleConfig | null): Promise<void> => {
      if (!selectedRepository) return;
      await loadSnapshot(() =>
        api.setReleaseRule(selectedRepository.id, releaseRule),
      );
      setSelectedPreparation(null);
      setSelectedRepository(null);
    },
    [loadSnapshot, selectedRepository],
  );

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
      <AppSidebar
        activeNav={activeNav}
        activeConditionCount={activeConditionCount}
        rootCount={snapshot.roots.length}
        onNavigate={(nav) => {
          setActiveNav(nav);
          setSelectedRepository(null);
          setSelectedEvidence(null);
          setSelectedPreparation(null);
        }}
      />
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
            <ActivitySurface
              events={snapshot.events}
              actionAudits={snapshot.action_audits}
            />
          ) : activeNav === "settings" ? (
            <SettingsSurface
              roots={snapshot.roots}
              storagePath={snapshot.storage_path}
              generatedAt={snapshot.generated_at}
              retentionDays={snapshot.retention_days}
              onAddRoot={handleAddRoot}
              onSaveRoot={handleSaveRoot}
              onSaveRetention={handleSaveRetention}
            />
          ) : activeNav === "products" ? (
            <PortfolioConfigSurface
              kind="product"
              items={snapshot.products}
              repositories={snapshot.repositories}
              onSave={handleSaveProduct}
              onDelete={handleDeleteProduct}
            />
          ) : activeNav === "groups" ? (
            <PortfolioConfigSurface
              kind="group"
              items={snapshot.groups}
              repositories={snapshot.repositories}
              onSave={handleSaveGroup}
              onDelete={handleDeleteGroup}
            />
          ) : activeNav === "remote" ? (
            <RemoteCatalogSurface
              status={snapshot.provider_status}
              identities={snapshot.provider_identities}
              repositories={snapshot.remote_repositories}
              isRefreshing={isRefreshing}
              onRefresh={handleRefreshGithub}
            />
          ) : (
            <DeferredSurface
              eyebrow="Accepted provider decision"
              title="Read-only GitHub comes later."
              body="Remote context will be additive and read-only at first. No credentials, network refresh, pull request mutation, or release publishing is active in this local slice."
              icon={<GitBranch size={19} />}
              details={[
                { label: "Permission", value: "Read-only" },
                { label: "Prerequisite", value: "Read-only provider contract" },
              ]}
            />
          )}
        </div>
      </main>
      <AppOverlays
        selectedRepository={selectedRepository}
        selectedEvidence={selectedEvidence}
        selectedPreparation={selectedPreparation}
        onCloseRepository={() => {
          setSelectedRepository(null);
          setSelectedPreparation(null);
        }}
        onOpenWorkspace={handleOpenWorkspace}
        onPrepareRepository={handlePrepareRepository}
        onSaveReleaseRule={handleSaveReleaseRule}
        onLifecycleChange={async (lifecycle) => {
          if (!selectedRepository) return;
          await loadSnapshot(() =>
            api.setRepositoryLifecycle(selectedRepository.id, lifecycle),
          );
          setSelectedRepository(null);
        }}
        onCondition={(condition) => {
          if (!selectedRepository) return;
          setSelectedRepository(null);
          handleCondition(selectedRepository, condition);
        }}
        onCloseEvidence={() => setSelectedEvidence(null)}
        onExpected={() => void handleExpected()}
        onClosePreparation={() => setSelectedPreparation(null)}
      />
    </div>
  );
}
