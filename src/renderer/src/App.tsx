import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";
import {
  AlertTriangle,
  FolderPlus,
  GitBranch,
  ShieldCheck,
  X,
} from "lucide-react";
import * as api from "./api";
import { AnalyticsSurface } from "./components/AnalyticsComponents";
import { SkillsSurface } from "./components/SkillsComponents";
import {
  CommandCenterSurface,
  type Filter,
} from "./components/CommandCenterSurface";
import { AppOverlays } from "./components/AppOverlays";
import { AppSidebar } from "./components/AppSidebar";
import { AppTopbar } from "./components/AppTopbar";
import { formatTime, StatusPill } from "./components/ConsolePrimitives";
import { RepositoryDetailSurface } from "./components/Drawers";
import { PortfolioCollectionsSurface } from "./components/PortfolioCollectionsSurface";
import { QualityGatesSurface } from "./components/QualityGatesSurface";
import { RemediationSurface } from "./components/RemediationSurface";
import { RemoteCatalogSurface } from "./components/RemoteCatalogSurface";
import { RefreshConfirmationDialog } from "./components/RefreshConfirmationDialog";
import { useEvidenceActions } from "./hooks/useEvidenceActions";
import { useAppKeyboardShortcuts } from "./hooks/useAppKeyboardShortcuts";
import { usePortfolioController } from "./hooks/usePortfolioController";
import { usePreparationActions } from "./hooks/usePreparationActions";
import {
  countActiveConditions,
  countDirtyRepositories,
  countUnsyncedRepositories,
  currentDateLabel,
} from "./portfolioSelectors";
import {
  ActivitySurface,
  DeferredSurface,
  SettingsSurface,
} from "./components/WorkspaceSurfaces";
import { navItems, pageCopy, type NavItem } from "./navigation";
import type {
  Condition,
  ExternalTool,
  RepositoryPreparation,
  RepositorySnapshot,
} from "./types";
import "./styles.css";

export function App(): ReactElement {
  const {
    snapshot,
    setSnapshot,
    analytics,
    skills,
    isRefreshing,
    error,
    setError,
    notice,
    setNotice,
    loadSnapshot,
    handleRefreshSkills,
    handleOpenSkillSource,
    handleAddRoot,
    handleSaveRoot,
    handleSaveRetention,
    handleSaveProduct,
    handleDeleteProduct,
    handleSaveGroup,
    handleDeleteGroup,
    handleRefreshGithub,
    handleRefreshRemediation,
    handleExportRemediation,
    handleRemediationStatus,
    handleConfirmRefresh,
    handleOpenQualityReport,
  } = usePortfolioController();
  const [activeNav, setActiveNav] = useState<NavItem>("portfolio");
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [selectedRepositoryId, setSelectedRepositoryId] = useState<
    string | null
  >(null);
  const [selectedEvidence, setSelectedEvidence] = useState<{
    repository: RepositorySnapshot;
    condition: Condition;
  } | null>(null);
  const [selectedPreparation, setSelectedPreparation] = useState<{
    repository: RepositorySnapshot;
    preparation: RepositoryPreparation;
  } | null>(null);
  const [isRefreshConfirmationOpen, setIsRefreshConfirmationOpen] =
    useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const selectedRepository = useMemo(
    () =>
      selectedRepositoryId
        ? (snapshot.repositories.find(
            (repository) => repository.id === selectedRepositoryId,
          ) ?? null)
        : null,
    [selectedRepositoryId, snapshot.repositories],
  );

  useEffect(() => {
    if (
      selectedRepositoryId &&
      !snapshot.repositories.some(
        (repository) => repository.id === selectedRepositoryId,
      )
    ) {
      setSelectedRepositoryId(null);
      setSelectedEvidence(null);
      setSelectedPreparation(null);
      setError("That repository is no longer in the current local snapshot.");
    }
  }, [selectedRepositoryId, snapshot.repositories]);

  useAppKeyboardShortcuts(searchInputRef, () => {
    if (selectedEvidence) setSelectedEvidence(null);
    else if (selectedPreparation) setSelectedPreparation(null);
    else if (selectedRepositoryId) setSelectedRepositoryId(null);
  });

  const handleConfirmLocalRefresh = useCallback(async (): Promise<void> => {
    await handleConfirmRefresh();
    setIsRefreshConfirmationOpen(false);
  }, [handleConfirmRefresh]);

  const handleOpenRepository = useCallback(
    (repository: RepositorySnapshot): void => {
      setActiveNav("portfolio");
      setSelectedRepositoryId(repository.id);
      setSelectedEvidence(null);
      setSelectedPreparation(null);
    },
    [],
  );

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

  const handleLifecycleChange = useCallback(
    async (lifecycle: string): Promise<void> => {
      if (!selectedRepository) return;
      await loadSnapshot(() =>
        api.setRepositoryLifecycle(selectedRepository.id, lifecycle),
      );
    },
    [loadSnapshot, selectedRepository],
  );

  const {
    handleSaveReleaseRule,
    handleSaveReleaseRecipe,
    handleConfirmReleaseVersion,
    handleSaveAiPermission,
    handlePreviewAiSummary,
  } = usePreparationActions({
    selectedRepository,
    selectedPreparation,
    loadSnapshot,
    setSnapshot,
    setSelectedRepositoryId,
    setSelectedPreparation,
    setError,
  });
  const { handleCondition, handleExpected } = useEvidenceActions({
    selectedEvidence,
    loadSnapshot,
    setSelectedRepositoryId,
    setSelectedEvidence,
  });

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

  const activeConditionCount = countActiveConditions(snapshot.repositories);
  const dirtyCount = countDirtyRepositories(snapshot.repositories);
  const unsyncedCount = countUnsyncedRepositories(snapshot.repositories);
  const activePage = pageCopy[activeNav];
  const activeNavLabel = navItems.find((item) => item.id === activeNav)?.label;
  const dateLabel = currentDateLabel();
  const isPortfolio = activeNav === "portfolio";
  const showingRepositoryDetail = Boolean(
    selectedRepository && activeNav !== "remediation",
  );

  return (
    <div className="app-shell">
      <AppSidebar
        activeNav={activeNav}
        activeConditionCount={activeConditionCount}
        repositories={snapshot.repositories}
        remediation={snapshot.remediation}
        selectedRepositoryId={selectedRepositoryId}
        onNavigate={(nav) => {
          setActiveNav(nav);
          setSelectedRepositoryId(null);
          setSelectedEvidence(null);
          setSelectedPreparation(null);
        }}
        onOpenRepository={handleOpenRepository}
      />
      <main className="main-content">
        <AppTopbar
          activeNavLabel={activeNavLabel}
          isPortfolio={isPortfolio}
          repositoryName={selectedRepository?.name}
          query={query}
          searchInputRef={searchInputRef}
          isRefreshing={isRefreshing}
          onQueryChange={setQuery}
          onRefresh={() => {
            if (activeNav === "remediation") {
              void handleRefreshRemediation();
            } else {
              setIsRefreshConfirmationOpen(true);
            }
          }}
          onBackToPortfolio={() => setSelectedRepositoryId(null)}
        />
        <div className="content-scroll">
          {!showingRepositoryDetail && (
            <section className="page-intro">
              <div>
                <p className="eyebrow">
                  {isPortfolio ? dateLabel : activePage.eyebrow}
                </p>
                <h1>{activePage.title}</h1>
                <p className="intro-copy">{activePage.body}</p>
              </div>
              <div className="intro-actions">
                {isPortfolio && (
                  <span className="snapshot-freshness">
                    Snapshot {formatTime(snapshot.generated_at)}
                  </span>
                )}
                <StatusPill tone="mint" icon={<ShieldCheck size={12} />}>
                  Local only
                </StatusPill>
                {(isPortfolio || activeNav === "settings") && (
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
          )}
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
          {notice && (
            <div className="success-banner" role="status">
              <ShieldCheck size={16} />
              <span>{notice}</span>
              <button
                type="button"
                onClick={() => setNotice(null)}
                aria-label="Dismiss notice"
              >
                <X size={14} />
              </button>
            </div>
          )}
          {showingRepositoryDetail && selectedRepository ? (
            <RepositoryDetailSurface
              repository={selectedRepository}
              analytics={analytics}
              onBack={() => setSelectedRepositoryId(null)}
              onOpenWorkspace={handleOpenWorkspace}
              onPrepareRepository={handlePrepareRepository}
              onLifecycleChange={handleLifecycleChange}
              onCondition={(condition) =>
                handleCondition(selectedRepository, condition)
              }
              onOpenReport={(reportPath) =>
                void handleOpenQualityReport(reportPath)
              }
            />
          ) : isPortfolio ? (
            <>
              <CommandCenterSurface
                activeConditionCount={activeConditionCount}
                dirtyCount={dirtyCount}
                unsyncedCount={unsyncedCount}
                repositoryCount={snapshot.repositories.length}
                rootCount={snapshot.roots.length}
                quality={snapshot.quality}
                analytics={analytics}
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
                onOpenRepository={handleOpenRepository}
                onCondition={handleCondition}
                onOpenQualityReport={(reportPath) =>
                  void handleOpenQualityReport(reportPath)
                }
              />
              <QualityGatesSurface
                snapshot={snapshot}
                repositories={snapshot.repositories}
                showOverview={false}
                onOpenRepository={handleOpenRepository}
                onOpenReport={(reportPath) =>
                  void handleOpenQualityReport(reportPath)
                }
              />
            </>
          ) : activeNav === "remediation" ? (
            <RemediationSurface
              run={snapshot.remediation}
              repositories={snapshot.repositories}
              isRefreshing={isRefreshing}
              onRefresh={handleRefreshRemediation}
              onExport={handleExportRemediation}
              onUpdateStatus={handleRemediationStatus}
              onOpenRepository={handleOpenRepository}
            />
          ) : activeNav === "analytics" ? (
            <AnalyticsSurface
              analytics={analytics}
              repositories={snapshot.repositories}
            />
          ) : activeNav === "skills" ? (
            <SkillsSurface
              snapshot={skills}
              isRefreshing={isRefreshing}
              onRefresh={() => void handleRefreshSkills()}
              onOpenSource={(path) => void handleOpenSkillSource(path)}
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
              quality={snapshot.quality}
            />
          ) : activeNav === "groups" ? (
            <PortfolioCollectionsSurface
              groups={snapshot.groups}
              products={snapshot.products}
              repositories={snapshot.repositories}
              onSaveGroup={handleSaveGroup}
              onDeleteGroup={handleDeleteGroup}
              onSaveProduct={handleSaveProduct}
              onDeleteProduct={handleDeleteProduct}
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
      {isRefreshConfirmationOpen && (
        <RefreshConfirmationDialog
          rootCount={snapshot.roots.length}
          repositoryCount={snapshot.repositories.length}
          isRefreshing={isRefreshing}
          onCancel={() => setIsRefreshConfirmationOpen(false)}
          onConfirm={handleConfirmLocalRefresh}
        />
      )}
      <AppOverlays
        selectedEvidence={selectedEvidence}
        selectedPreparation={selectedPreparation}
        onSaveReleaseRule={handleSaveReleaseRule}
        onSaveReleaseRecipe={handleSaveReleaseRecipe}
        onConfirmReleaseVersion={handleConfirmReleaseVersion}
        onSaveAiPermission={handleSaveAiPermission}
        onPreviewAiSummary={handlePreviewAiSummary}
        onCloseEvidence={() => setSelectedEvidence(null)}
        onExpected={() => void handleExpected()}
        onClosePreparation={() => setSelectedPreparation(null)}
      />
    </div>
  );
}
