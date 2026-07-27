import type { ReactElement } from "react";
import {
  AlertTriangle,
  Archive,
  ChevronDown,
  ChevronRight,
  Copy,
  FolderGit2,
  GitBranch,
  MoreHorizontal,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type {
  AnalyticsSnapshot,
  Condition,
  EventRecord,
  QualityPortfolioSnapshot,
  RepositorySnapshot,
} from "../types";
import { AnalyticsDashboardBand } from "./AnalyticsComponents";
import { EmptyState, NoMatchesState } from "./ConsolePrimitives";
import { AttentionQueue, RepositoryRow, Timeline } from "./PortfolioComponents";
import {
  qualityConfigurationSummary,
  qualityEvidenceSummary,
  qualityGateDisplayLabel,
} from "./QualityComponents";

export type Filter = "all" | "attention" | "dirty" | "sync";

export function CommandCenterSurface({
  activeConditionCount,
  dirtyCount,
  unsyncedCount,
  repositoryCount,
  rootCount,
  quality,
  analytics,
  repositories,
  allRepositories,
  events,
  filter,
  onFilterChange,
  onClearFilters,
  onAddRoot,
  onOpenRepository,
  onCondition,
  onOpenQualityReport,
}: {
  activeConditionCount: number;
  dirtyCount: number;
  unsyncedCount: number;
  repositoryCount: number;
  rootCount: number;
  quality: QualityPortfolioSnapshot;
  analytics: AnalyticsSnapshot;
  repositories: RepositorySnapshot[];
  allRepositories: RepositorySnapshot[];
  events: EventRecord[];
  filter: Filter;
  onFilterChange: (filter: Filter) => void;
  onClearFilters: () => void;
  onAddRoot: () => void;
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onCondition: (repository: RepositorySnapshot, condition: Condition) => void;
  onOpenQualityReport?: (reportPath: string) => void;
}): ReactElement {
  const configuration = qualityConfigurationSummary(quality);
  const evidence = qualityEvidenceSummary(quality);
  const openGateSummary = Object.entries(
    quality.ci_readiness_open_gate_counts ?? {},
  )
    .sort((left, right) => right[1] - left[1])
    .slice(0, 3)
    .map(([gateId, count]) => `${qualityGateDisplayLabel(gateId)} (${count})`)
    .join(" · ");

  return (
    <>
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
          <strong>{repositoryCount}</strong>
          <small>
            {rootCount} registered root
            {rootCount === 1 ? "" : "s"}
          </small>
          <Archive size={18} />
        </div>
      </section>
      <section
        className="command-quality-summary"
        aria-label="Quality configuration"
      >
        <div className="command-quality-summary-heading">
          <div>
            <p className="eyebrow">Quality evidence</p>
            <h2>CI configuration vs ideal</h2>
            <p>
              Imported maturity stays exact. This compares discovered gate
              configuration with each repository&apos;s recommended profile;
              execution evidence remains separate.
            </p>
          </div>
          <ShieldCheck size={19} />
        </div>
        <div className="command-quality-score-grid">
          <div>
            <span>Fleet maturity</span>
            <strong>
              {quality.maturity_score_display ?? "Not scored"}
              {quality.maturity_score_display && <small>/4</small>}
            </strong>
            <small>Quality Runner audit</small>
          </div>
          <div>
            <span>CI configuration</span>
            <strong>
              {configuration.ideal > 0
                ? `${configuration.configured}/${configuration.ideal}`
                : "Not assessed"}
            </strong>
            <small>
              {configuration.ideal === 0
                ? "No matched recommendation profile"
                : `${configuration.fullRepositories}/${configuration.repositories} repositories at ideal configuration${
                    configuration.unscoredRepositories > 0
                      ? ` · ${configuration.unscoredRepositories} not scored`
                      : ""
                  }`}
            </small>
          </div>
          <div>
            <span>Fresh passing evidence</span>
            <strong>
              {evidence.ideal > 0
                ? `${evidence.freshPassing}/${evidence.ideal}`
                : "Not assessed"}
            </strong>
            <small>
              {openGateSummary
                ? `Evidence updates: ${openGateSummary}`
                : "No open gate updates"}
            </small>
          </div>
        </div>
      </section>
      <AnalyticsDashboardBand analytics={analytics} />
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
              <button className="button button-quiet" type="button" disabled>
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
                    aria-pressed={filter === item}
                    onClick={() => onFilterChange(item)}
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
            allRepositories.length === 0 ? (
              <EmptyState onAddRoot={onAddRoot} hasRoots={rootCount > 0} />
            ) : (
              <NoMatchesState onClear={onClearFilters} />
            )
          ) : (
            <div className="repository-list">
              {repositories.map((repository) => (
                <RepositoryRow
                  key={repository.id}
                  repository={repository}
                  onOpen={() => onOpenRepository(repository)}
                  onCondition={(condition) =>
                    onCondition(repository, condition)
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
            repositories={allRepositories}
            onCondition={onCondition}
            onOpenRepository={onOpenRepository}
            onOpenReport={onOpenQualityReport}
          />
          <Timeline events={events} />
          <section className="provider-card">
            <div className="provider-icon">
              <Sparkles size={17} />
            </div>
            <div>
              <p className="eyebrow">Next boundary</p>
              <h3>GitHub stays explicit</h3>
              <p>
                Provider refresh, pull requests, and release rules will appear
                here once an identity is connected.
              </p>
            </div>
            <ChevronRight size={16} />
          </section>
        </aside>
      </div>
    </>
  );
}
