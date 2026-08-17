import type { ReactElement } from "react";
import {
  AlertTriangle,
  Archive,
  ChevronDown,
  ChevronRight,
  Copy,
  FolderGit2,
  GitBranch,
  LoaderCircle,
  MoreHorizontal,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type {
  Condition,
  EventRecord,
  QualityPortfolioSnapshot,
  RepositorySnapshot,
} from "../types";
import {
  EmptyState,
  ExpandableScore,
  formatExactTime,
  NoMatchesState,
} from "./ConsolePrimitives";
import { AttentionQueue, RepositoryRow, Timeline } from "./PortfolioComponents";
import {
  QualityOutcomeSummary,
  macControlFreshnessLabel,
  qualityConfigurationSummary,
  qualityEvidenceSummary,
  qualityGateDisplayLabel,
  qualityProfileSummary,
} from "./QualityComponents";

export type Filter = "all" | "attention" | "dirty" | "sync";

export function CommandCenterSurface({
  activeConditionCount,
  dirtyCount,
  unsyncedCount,
  repositoryCount,
  rootCount,
  quality,
  snapshotGeneratedAt,
  isRefreshing,
  repositories,
  allRepositories,
  events,
  filter,
  onFilterChange,
  onClearFilters,
  onAddRoot,
  onOpenRepository,
  onCondition,
  onRefreshQuality,
  onShowAttention,
  onOpenAnalytics = () => undefined,
}: {
  activeConditionCount: number;
  dirtyCount: number;
  unsyncedCount: number;
  repositoryCount: number;
  rootCount: number;
  quality: QualityPortfolioSnapshot;
  snapshotGeneratedAt: string;
  isRefreshing: boolean;
  repositories: RepositorySnapshot[];
  allRepositories: RepositorySnapshot[];
  events: EventRecord[];
  filter: Filter;
  onFilterChange: (filter: Filter) => void;
  onClearFilters: () => void;
  onAddRoot: () => void;
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onCondition: (repository: RepositorySnapshot, condition: Condition) => void;
  onRefreshQuality: () => void;
  onShowAttention?: () => void;
  onOpenAnalytics?: () => void;
  analytics?: unknown;
}): ReactElement {
  const configuration = qualityConfigurationSummary(quality);
  const evidence = qualityEvidenceSummary(quality);
  const macControl = quality.mac_control_ideal_state;
  const openGateSummary = Object.entries(
    quality.ci_readiness_open_gate_counts ?? {},
  )
    .sort((left, right) => right[1] - left[1])
    .slice(0, 3)
    .map(([gateId, count]) => `${qualityGateDisplayLabel(gateId)} (${count})`)
    .join(" · ");
  const maturityBreakdown = [
    ...(quality.maturity_pillars ?? []).map((pillar) => ({
      id: `maturity-${pillar.id}`,
      label: pillar.label,
      value: pillar.score === undefined ? "Unknown" : `${pillar.score}/4`,
      detail: `${pillar.assessed_repository_count} repositories assessed`,
    })),
    {
      id: "maturity-evidence",
      label: "Evidence coverage",
      value:
        quality.maturity_evidence_coverage === undefined
          ? "Unknown"
          : `${Math.round(quality.maturity_evidence_coverage * 100)}%`,
      detail:
        quality.maturity_fresh_evidence_coverage === undefined
          ? "Freshness not reported"
          : `${Math.round(quality.maturity_fresh_evidence_coverage * 100)}% fresh`,
    },
    {
      id: "maturity-disposition",
      label: "Audit disposition",
      value: quality.audit_status,
      detail: `${quality.maturity_provisional_repository_count ?? 0} provisional · ${quality.maturity_capped_repository_count ?? 0} capped`,
    },
  ];

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
              Quality Runner&apos;s source score stays visible. Pronto
              consolidates it with current local evidence while keeping CI
              configuration and execution evidence separate.
            </p>
          </div>
          <div className="command-quality-summary-heading-actions">
            <button
              className="button button-quiet"
              type="button"
              onClick={onRefreshQuality}
              disabled={isRefreshing}
            >
              {isRefreshing ? (
                <LoaderCircle className="spin" size={15} />
              ) : (
                <RefreshCw size={15} />
              )}
              Refresh quality
            </button>
            <ShieldCheck size={19} />
          </div>
        </div>
        <QualityOutcomeSummary quality={quality} />
        <div
          className="command-quality-provenance"
          aria-label="Quality evidence provenance"
        >
          <div>
            <span>Quality audit</span>
            <strong>{quality.latest_audit_id ?? "No audit imported"}</strong>
          </div>
          <div>
            <span>Audit observed</span>
            <strong>{formatExactTime(quality.latest_audit_at)}</strong>
          </div>
          <div>
            <span>QR + Mac Control checkpoint</span>
            <strong>
              {quality.maturity_checkpoint?.status ?? "Legacy separate"}
            </strong>
            <small>
              {formatExactTime(quality.maturity_checkpoint?.observed_at)}
            </small>
          </div>
          <div>
            <span>Pronto snapshot</span>
            <strong>{formatExactTime(snapshotGeneratedAt)}</strong>
          </div>
        </div>
        <div className="command-quality-score-grid">
          <ExpandableScore
            className="command-quality-score-card"
            label="Consolidated fleet maturity"
            value={
              <>
                {quality.maturity_score_display ?? "Not scored"}
                {quality.maturity_score_display && <small>/4</small>}
              </>
            }
            description="Pronto's evidence-governed score remains separate from the Quality Runner source score."
            title="Open to see the assessed pillars, evidence coverage, and audit disposition."
            breakdown={maturityBreakdown}
          />
          {quality.source_maturity_score_display && (
            <small className="command-quality-score-source">
              QR source {quality.source_maturity_score_display}/4
            </small>
          )}
          <ExpandableScore
            className="command-quality-score-card"
            label="CI configuration"
            value={
              configuration.ideal > 0
                ? `${configuration.configured}/${configuration.ideal}`
                : "Not assessed"
            }
            description="Configured gates are compared with the recommendation profile; this is configuration coverage, not passing execution evidence."
            title="Open to separate configured gates, repositories at the ideal profile, and unscored repositories."
            breakdown={[
              {
                id: "configured-gates",
                label: "Configured gates",
                value:
                  configuration.ideal > 0
                    ? `${configuration.configured}/${configuration.ideal}`
                    : "Not assessed",
              },
              {
                id: "ideal-repositories",
                label: "Repositories at ideal",
                value: `${configuration.fullRepositories}/${configuration.repositories}`,
              },
              {
                id: "unscored-repositories",
                label: "Repositories not scored",
                value: configuration.unscoredRepositories,
                detail:
                  configuration.ideal === 0
                    ? qualityProfileSummary(quality)
                    : `${configuration.fullRepositories}/${configuration.repositories} repositories at ideal configuration`,
              },
            ]}
          />
          <ExpandableScore
            className="command-quality-score-card"
            label="Fresh passing evidence"
            value={
              evidence.ideal > 0
                ? `${evidence.freshPassing}/${evidence.ideal}`
                : "Not assessed"
            }
            description="Fresh passing evidence is kept separate from configuration so a declared gate cannot be mistaken for a verified pass."
            title="Open to see the evidence total and the most common open gate updates."
            breakdown={[
              {
                id: "fresh-passing-gates",
                label: "Fresh passing gates",
                value:
                  evidence.ideal > 0
                    ? `${evidence.freshPassing}/${evidence.ideal}`
                    : "Not assessed",
              },
              {
                id: "open-gate-updates",
                label: "Open gate updates",
                value: openGateSummary || "None",
              },
            ]}
          />
          {macControl ? (
            <ExpandableScore
              className="command-quality-score-card"
              label="Mac Control semantic evidence"
              value={
                <>
                  {macControl.implementation_criteria_passed_count ?? 0}/
                  {macControl.implementation_criteria_total ?? 0}
                  <small> dimensions</small>
                </>
              }
              description="Source-grounded semantics and live task evidence are separate inputs to the Mac Control ideal state."
              title="Open to see semantic source evidence, live task evidence, and freshness."
              breakdown={[
                {
                  id: "semantic-source-evidence",
                  label: "Semantic source evidence",
                  value: `${macControl.implementation_criteria_passed_count ?? 0}/${macControl.implementation_criteria_total ?? 0}`,
                  detail: `${macControl.implementation_score_display ?? "Not scored"}/4 · ${macControl.implementation_status ?? macControl.status} · ${macControlFreshnessLabel(macControl.freshness)}`,
                },
                {
                  id: "live-task-evidence",
                  label: "Live task evidence",
                  value: `${macControl.measured_task_count ?? 0}/${macControl.live_task_count ?? 0}`,
                  detail: `Live tasks: ${macControl.measured_task_count ?? 0}/${macControl.live_task_count ?? 0} measured · ${macControl.live_status ?? "Not assessed"} (${macControl.live_score_display ?? "Not scored"}/4)`,
                },
                {
                  id: "ideal-state",
                  label: "Ideal state",
                  value: macControl.ideal_state ? "Pass" : macControl.status,
                  detail:
                    (macControl.implementation_declaration_criteria_count ?? 0)
                      ? `Legacy declarations: ${macControl.implementation_declaration_criteria_count} recorded · non-scoring until v4 source evidence is established`
                      : undefined,
                },
              ]}
            />
          ) : null}
        </div>
      </section>
      <div className="portfolio-analytics-link">
        <span>Need historical trends, comparisons, or custom views?</span>
        <button type="button" className="text-link" onClick={onOpenAnalytics}>
          Open Analytics
          <ChevronRight size={14} />
        </button>
      </div>
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
            onShowAttention={onShowAttention}
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
