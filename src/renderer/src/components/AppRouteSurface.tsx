import type { ComponentProps, ReactElement } from "react";
import type {
  AnalyticsSnapshot,
  CiRunSnapshot,
  Condition,
  ExternalTool,
  PortfolioSnapshot,
  PromotionInbox,
  RemediationActionStatus,
  RemoteRepositorySnapshot,
  RepositorySnapshot,
} from "../types";
import { type NavItem } from "../navigation";
import {
  CommandCenterSurface,
  type Filter,
} from "./CommandCenterSurface";
import { RepositoryDetailSurface } from "./Drawers";
import { PortfolioCollectionsSurface } from "./PortfolioCollectionsSurface";
import { ShowcaseSurface } from "./ShowcaseSurface";
import { RemediationSurface } from "./RemediationSurface";
import { PromotionSurface } from "./PromotionSurface";
import { AnalyticsRoute } from "./AnalyticsRoute";
import { SkillsSurface } from "./SkillsComponents";
import { ActivitySurface, SettingsSurface } from "./WorkspaceSurfaces";
import { RemoteCatalogSurface } from "./RemoteCatalogSurface";
import { CiTrackerSurface } from "./CiTrackerSurface";
import { RemoteDeferredSurface } from "./RemoteDeferredSurface";
import { QualityGatesSurface } from "./QualityGatesSurface";
import type { SkillsSurfaceProps } from "./SkillsPresentation";

export function AppRouteSurface({
  activeNav,
  selectedRepository,
  snapshot,
  analytics,
  promotionInbox,
  isRefreshing,
  repositories,
  filter,
  activeConditionCount,
  dirtyCount,
  unsyncedCount,
  skills,
  onBack,
  onOpenWorkspace,
  onPrepareRepository,
  onTargetBranchChange,
  onLifecycleChange,
  onCondition,
  onOpenReport,
  onFilterChange,
  onClearFilters,
  onAddRoot,
  onOpenRepository,
  onRefreshQuality,
  onShowAttention,
  onOpenAnalytics,
  promotionActions,
  onRefreshRemediation,
  onExportRemediation,
  onUpdateRemediationStatus,
  onRefreshGithub,
  onStartCiCodex,
  settingsActions,
  collectionActions,
}: {
  activeNav: NavItem;
  selectedRepository: RepositorySnapshot | null;
  snapshot: PortfolioSnapshot;
  analytics: AnalyticsSnapshot;
  promotionInbox: PromotionInbox;
  isRefreshing: boolean;
  repositories: RepositorySnapshot[];
  filter: Filter;
  activeConditionCount: number;
  dirtyCount: number;
  unsyncedCount: number;
  skills: SkillsSurfaceProps;
  onBack: () => void;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onTargetBranchChange: (targetBranch: string) => Promise<void>;
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (repository: RepositorySnapshot, condition: Condition) => void;
  onOpenReport: (reportPath: string) => void;
  onFilterChange: (nextFilter: Filter) => void;
  onClearFilters: () => void;
  onAddRoot: () => void;
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onRefreshQuality: () => void;
  onShowAttention: () => void;
  onOpenAnalytics: () => void;
  promotionActions: Pick<
    ComponentProps<typeof PromotionSurface>,
    "onRefresh" | "onDecide"
  >;
  onRefreshRemediation: () => Promise<void>;
  onExportRemediation: () => Promise<void>;
  onUpdateRemediationStatus: (
    actionId: string,
    status: RemediationActionStatus,
  ) => Promise<void>;
  onRefreshGithub: () => Promise<void>;
  onStartCiCodex: (
    repository: RemoteRepositorySnapshot,
    run: CiRunSnapshot,
  ) => Promise<void>;
  settingsActions: Pick<
    ComponentProps<typeof SettingsSurface>,
    "onSaveRoot" | "onSaveRetention"
  >;
  collectionActions: Pick<
    ComponentProps<typeof PortfolioCollectionsSurface>,
    "onSaveGroup" | "onDeleteGroup" | "onSaveProduct" | "onDeleteProduct"
  >;
}): ReactElement {
  const showingRepositoryDetail = Boolean(
    selectedRepository &&
      activeNav !== "remediation" &&
      activeNav !== "promotions",
  );

  return (
    <>
      {showingRepositoryDetail && selectedRepository ? (
        <RepositoryDetailSurface
          repository={selectedRepository}
          remediation={snapshot.remediation}
          events={snapshot.events}
          backLabel={
            activeNav === "showcase"
              ? "Back to AI showcase"
              : "Back to Portfolio"
          }
          analytics={analytics}
          isRefreshing={isRefreshing}
          onBack={onBack}
          onOpenWorkspace={onOpenWorkspace}
          onPrepareRepository={onPrepareRepository}
          onTargetBranchChange={onTargetBranchChange}
          onLifecycleChange={onLifecycleChange}
          onCondition={(condition) => onCondition(selectedRepository, condition)}
          onOpenReport={onOpenReport}
        />
      ) : activeNav === "portfolio" ? (
        <>
          <CommandCenterSurface
            activeConditionCount={activeConditionCount}
            dirtyCount={dirtyCount}
            unsyncedCount={unsyncedCount}
            repositoryCount={snapshot.repositories.length}
            rootCount={snapshot.roots.length}
            quality={snapshot.quality}
            snapshotGeneratedAt={snapshot.generated_at}
            isRefreshing={isRefreshing}
            repositories={repositories}
            allRepositories={snapshot.repositories}
            events={snapshot.events}
            filter={filter}
            onFilterChange={onFilterChange}
            onClearFilters={onClearFilters}
            onAddRoot={onAddRoot}
            onOpenRepository={onOpenRepository}
            onCondition={onCondition}
            onRefreshQuality={onRefreshQuality}
            onShowAttention={onShowAttention}
            onOpenAnalytics={onOpenAnalytics}
          />
          <QualityGatesSurface
            snapshot={snapshot}
            repositories={snapshot.repositories}
            showOverview={false}
            onOpenRepository={onOpenRepository}
            onOpenReport={onOpenReport}
          />
        </>
      ) : activeNav === "showcase" ? (
        <ShowcaseSurface
          showcase={snapshot.showcase}
          repositories={snapshot.repositories}
          onOpenRepository={onOpenRepository}
        />
      ) : activeNav === "remediation" ? (
        <RemediationSurface
          run={snapshot.remediation}
          repositories={snapshot.repositories}
          isRefreshing={isRefreshing}
          onRefresh={onRefreshRemediation}
          onExport={onExportRemediation}
          onUpdateStatus={onUpdateRemediationStatus}
          onOpenRepository={onOpenRepository}
        />
      ) : activeNav === "promotions" ? (
        <PromotionSurface
          inbox={promotionInbox}
          isRefreshing={isRefreshing}
          onRefresh={promotionActions.onRefresh}
          onDecide={promotionActions.onDecide}
        />
      ) : activeNav === "analytics" ? (
        <AnalyticsRoute
          analytics={analytics}
          repositories={snapshot.repositories}
          groups={snapshot.groups}
          products={snapshot.products}
        />
      ) : activeNav === "skills" ? (
        <SkillsSurface {...skills} />
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
          onAddRoot={onAddRoot}
          onSaveRoot={settingsActions.onSaveRoot}
          onSaveRetention={settingsActions.onSaveRetention}
          quality={snapshot.quality}
        />
      ) : activeNav === "groups" ? (
        <PortfolioCollectionsSurface
          groups={snapshot.groups}
          products={snapshot.products}
          repositories={snapshot.repositories}
          onSaveGroup={collectionActions.onSaveGroup}
          onDeleteGroup={collectionActions.onDeleteGroup}
          onSaveProduct={collectionActions.onSaveProduct}
          onDeleteProduct={collectionActions.onDeleteProduct}
        />
      ) : activeNav === "remote" ? (
        <RemoteCatalogSurface
          status={snapshot.provider_status}
          identities={snapshot.provider_identities}
          repositories={snapshot.remote_repositories}
          isRefreshing={isRefreshing}
          onRefresh={onRefreshGithub}
        />
      ) : activeNav === "ci" ? (
        <CiTrackerSurface
          status={snapshot.provider_status}
          repositories={snapshot.remote_repositories}
          isRefreshing={isRefreshing}
          onRefresh={onRefreshGithub}
          onStartCodex={onStartCiCodex}
        />
      ) : (
        <RemoteDeferredSurface />
      )}
    </>
  );
}
