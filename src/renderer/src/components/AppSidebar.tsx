import { useMemo, useState } from "react";
import type { ReactElement } from "react";
import { ChevronRight, GitBranch, Search, ShieldCheck } from "lucide-react";
import { qualityAttentionItems } from "./QualityComponents";
import { navItems, type NavItem } from "../navigation";
import type { RepositorySnapshot } from "../types";

function repositoryStatus(
  repository: RepositorySnapshot,
): "attention" | "watch" | "ready" {
  if (
    repository.conditions.some((condition) => condition.status === "Active") ||
    qualityAttentionItems(repository).length > 0
  ) {
    return "attention";
  }
  if (
    repository.workspace.dirty ||
    repository.workspace.sync_state !== "Synced"
  ) {
    return "watch";
  }
  return "ready";
}

export function AppSidebar({
  activeNav,
  activeConditionCount,
  rootCount,
  repositories,
  selectedRepositoryId,
  onNavigate,
  onOpenRepository,
}: {
  activeNav: NavItem;
  activeConditionCount: number;
  rootCount: number;
  repositories: RepositorySnapshot[];
  selectedRepositoryId: string | null;
  onNavigate: (nav: NavItem) => void;
  onOpenRepository: (repository: RepositorySnapshot) => void;
}): ReactElement {
  const [repositoryQuery, setRepositoryQuery] = useState("");
  const filteredRepositories = useMemo(() => {
    const normalizedQuery = repositoryQuery.trim().toLowerCase();
    if (!normalizedQuery) return repositories;
    return repositories.filter((repository) =>
      repository.name.toLowerCase().includes(normalizedQuery),
    );
  }, [repositories, repositoryQuery]);

  return (
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
            <span>{repositories.length} local</span>
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
              {repositories.length === 0
                ? "No local repositories"
                : "No matching repositories"}
            </span>
          ) : (
            filteredRepositories.map((repository) => {
              const status = repositoryStatus(repository);
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
      <div className="sidebar-bottom">
        <div className="local-status">
          <span className="status-beacon" />
          <div>
            <strong>Local evidence only</strong>
            <span>
              {rootCount} discovery root{rootCount === 1 ? "" : "s"}
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
  );
}
