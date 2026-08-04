import { useMemo, useState } from "react";
import type { ReactElement } from "react";
import { ChevronRight, GitBranch, Search } from "lucide-react";
import { qualityAttentionItems } from "./QualityComponents";
import { navItems, type NavItem } from "../navigation";
import type { RemediationRun, RepositorySnapshot } from "../types";

type RemediationPlan = RemediationRun["plans"][number];

function repositoryStatus(
  repository: RepositorySnapshot,
  remediationPlan?: RemediationPlan,
): "attention" | "stale" | "opportunity" | "watch" | "ready" {
  const activeConditions = repository.conditions.filter(
    (condition) => condition.status === "Active",
  );
  const qualityAttention = qualityAttentionItems(repository);
  const workspaceNeedsReview =
    repository.workspace.dirty || repository.workspace.sync_state !== "Synced";
  const hasStaleness =
    activeConditions.some((condition) => condition.kind === "remote-stale") ||
    qualityAttention.some((item) => item.staleOnly);
  const hasIntegrationCondition = activeConditions.some(
    (condition) => condition.kind === "integration-eligible",
  );
  const hasIntegrationOpportunity =
    remediationPlan?.integration_only_remaining === true;
  const hasUnconfirmedIntegration =
    hasIntegrationCondition && remediationPlan === undefined;
  const hasRemediationAffliction = Boolean(
    remediationPlan &&
    (remediationPlan.status === "blocked" ||
      !remediationPlan.integration_only_remaining),
  );
  const hasAffliction =
    hasRemediationAffliction ||
    hasUnconfirmedIntegration ||
    activeConditions.some(
      (condition) =>
        condition.kind !== "remote-stale" &&
        condition.kind !== "integration-eligible",
    ) ||
    qualityAttention.some((item) => !item.staleOnly);
  const advisorySignalCount = [
    hasStaleness,
    hasIntegrationOpportunity,
    workspaceNeedsReview,
  ].filter(Boolean).length;

  if (hasAffliction || advisorySignalCount > 1) {
    return "attention";
  }
  if (hasStaleness) {
    return "stale";
  }
  if (hasIntegrationOpportunity) {
    return "opportunity";
  }
  if (workspaceNeedsReview) {
    return "watch";
  }
  return "ready";
}

export function AppSidebar({
  activeNav,
  activeConditionCount,
  repositories,
  remediation,
  selectedRepositoryId,
  onNavigate,
  onOpenRepository,
}: {
  activeNav: NavItem;
  activeConditionCount: number;
  repositories: RepositorySnapshot[];
  remediation: RemediationRun;
  selectedRepositoryId: string | null;
  onNavigate: (nav: NavItem) => void;
  onOpenRepository: (repository: RepositorySnapshot) => void;
}): ReactElement {
  const [repositoryQuery, setRepositoryQuery] = useState("");
  const remediationByRepositoryId = useMemo(
    () =>
      new Map(
        remediation.plans.map((plan) => [plan.repository_id, plan] as const),
      ),
    [remediation.plans],
  );
  const excludedRepositoryIds = useMemo(
    () =>
      new Set(
        remediation.excluded_repositories.map(
          (exclusion) => exclusion.repository_id,
        ),
      ),
    [remediation.excluded_repositories],
  );
  const excludedRepositoryPaths = useMemo(
    () =>
      new Set(
        remediation.excluded_repositories.map(
          (exclusion) => exclusion.repository_path,
        ),
      ),
    [remediation.excluded_repositories],
  );
  const eligibleRepositories = useMemo(
    () =>
      repositories.filter(
        (repository) =>
          !excludedRepositoryIds.has(repository.id) &&
          !excludedRepositoryPaths.has(repository.path),
      ),
    [excludedRepositoryIds, excludedRepositoryPaths, repositories],
  );
  const filteredRepositories = useMemo(() => {
    const normalizedQuery = repositoryQuery.trim().toLowerCase();
    if (!normalizedQuery) return eligibleRepositories;
    return eligibleRepositories.filter((repository) =>
      repository.name.toLowerCase().includes(normalizedQuery),
    );
  }, [eligibleRepositories, repositoryQuery]);

  return (
    <aside className="sidebar">
      <nav className="primary-nav" aria-label="Primary navigation">
        {navItems.map(({ id, label, icon: Icon }) => (
          <button
            className={`nav-item ${activeNav === id ? "nav-item-active" : ""}`}
            type="button"
            key={id}
            title={label}
            aria-current={activeNav === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
          >
            <Icon size={17} />
            <span>{label}</span>
            {id === "portfolio" && activeConditionCount > 0 && (
              <span className="nav-count">{activeConditionCount}</span>
            )}
          </button>
        ))}
      </nav>
      <section
        className="sidebar-repositories"
        aria-labelledby="sidebar-repositories-title"
      >
        <div className="sidebar-section-heading">
          <div>
            <p className="eyebrow" id="sidebar-repositories-title">
              Repositories
            </p>
            <span>{eligibleRepositories.length} eligible local</span>
          </div>
          <GitBranch size={14} />
        </div>
        <label className="sidebar-search">
          <Search size={13} />
          <input
            aria-label="Filter repositories in sidebar"
            placeholder="Find a repository"
            value={repositoryQuery}
            onChange={(event) => setRepositoryQuery(event.target.value)}
          />
        </label>
        <div className="sidebar-repository-list">
          {filteredRepositories.length === 0 ? (
            <span className="sidebar-empty">
              {eligibleRepositories.length === 0
                ? repositories.length === 0
                  ? "No local repositories"
                  : "No eligible repositories"
                : "No matching repositories"}
            </span>
          ) : (
            filteredRepositories.map((repository) => {
              const status = repositoryStatus(
                repository,
                remediationByRepositoryId.get(repository.id),
              );
              return (
                <button
                  className={`sidebar-repository ${
                    selectedRepositoryId === repository.id
                      ? "sidebar-repository-active"
                      : ""
                  }`}
                  type="button"
                  key={repository.id}
                  onClick={() => onOpenRepository(repository)}
                >
                  <span
                    className={`sidebar-repository-status sidebar-repository-status-${status}`}
                    title={
                      status === "attention"
                        ? "Needs attention"
                        : status === "stale"
                          ? "Stale evidence only"
                          : status === "opportunity"
                            ? "Integration is the only remaining remediation"
                            : status === "watch"
                              ? "Workspace needs review"
                              : "No active conditions"
                    }
                  />
                  <span className="sidebar-repository-name">
                    {repository.name}
                  </span>
                  <ChevronRight size={12} />
                </button>
              );
            })
          )}
        </div>
      </section>
    </aside>
  );
}
