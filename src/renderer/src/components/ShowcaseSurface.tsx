import type { ReactElement } from "react";
import { Film, Flag, LockKeyhole, Sparkles } from "lucide-react";
import type {
  RepositorySnapshot,
  ShowcasePortfolioSnapshot,
  ShowcaseProjectSnapshot,
} from "../types";
import { formatExactTime } from "./ConsolePrimitives";

const laneLabels: Record<ShowcaseProjectSnapshot["lane"], string> = {
  publish_ready: "Publish ready",
  create_materials: "Create materials",
  product_first: "Product first",
  private_client: "Private / client",
  blocked: "Blocked",
  unknown: "Unknown",
  not_applicable: "Not applicable",
};

function scoreLabel(value: number | null | undefined): string {
  return value == null ? "—" : `${value.toFixed(1)}/5`;
}

function projectKey(project: ShowcaseProjectSnapshot): string {
  return (
    project.repository_id ??
    `${project.repository_name}:${project.registration_status}`
  );
}

export function ShowcaseSurface({
  showcase,
  repositories,
  onOpenRepository,
}: {
  showcase: ShowcasePortfolioSnapshot;
  repositories: RepositorySnapshot[];
  onOpenRepository: (repository: RepositorySnapshot) => void;
}): ReactElement {
  if (showcase.status !== "Ready") {
    return (
      <section className="showcase-surface" aria-label="AI showcase readiness">
        <div className="showcase-heading">
          <div>
            <p className="eyebrow">AI showcase</p>
            <h2>Demo readiness is {showcase.status.toLowerCase()}</h2>
            <p>
              {showcase.error ??
                `Add one ${showcase.contract_path} fleet contract to separate product readiness, demo materials, and public eligibility.`}
            </p>
          </div>
          <Film size={20} />
        </div>
      </section>
    );
  }

  const projects = [...showcase.projects].sort((left, right) => {
    if (left.showcase_score == null && right.showcase_score == null) {
      return left.display_name.localeCompare(right.display_name);
    }
    if (left.showcase_score == null) return 1;
    if (right.showcase_score == null) return -1;
    return (
      right.showcase_score - left.showcase_score ||
      left.display_name.localeCompare(right.display_name)
    );
  });
  const readinessRanks = new Map(
    projects
      .filter((project) => project.showcase_score != null)
      .map((project, index) => [projectKey(project), index + 1]),
  );
  const scoredCount = readinessRanks.size;
  const needsAuditCount = projects.length - scoredCount;
  const openProject = (project: ShowcaseProjectSnapshot): void => {
    const repository = repositories.find(
      (candidate) => candidate.id === project.repository_id,
    );
    if (repository) onOpenRepository(repository);
  };

  return (
    <section className="showcase-surface" aria-label="AI showcase readiness">
      <div className="showcase-heading">
        <div>
          <p className="eyebrow">AI showcase</p>
          <h2>Build five recruiter-ready public demos</h2>
          <p>
            Product readiness and demo materials stay separate. Client work is
            audited privately but cannot enter the public goal or publishing
            queue.
          </p>
        </div>
        <div className="showcase-heading-meta">
          <span>{showcase.goal.status}</span>
          <small>
            Reviewed {formatExactTime(showcase.reviewed_at ?? undefined)}
          </small>
          <small title={showcase.quality_bar_source ?? undefined}>
            Handshake audit · six quality gates
          </small>
        </div>
      </div>

      <div className="showcase-metrics">
        <div className="showcase-metric showcase-metric-accent">
          <Flag size={17} />
          <span>Public demo goal</span>
          <strong>
            {showcase.goal.publishable_demo_count}/
            {showcase.goal.target_publishable_demo_count}
          </strong>
          <small>{showcase.goal.remaining_demo_count} remaining</small>
        </div>
        <div className="showcase-metric">
          <Film size={17} />
          <span>Fleet projects</span>
          <strong>{projects.length}</strong>
          <small>Every registered repo plus audited entries</small>
        </div>
        <div className="showcase-metric">
          <Sparkles size={17} />
          <span>Readiness scored</span>
          <strong>{scoredCount}</strong>
          <small>{needsAuditCount} need evidence before ranking</small>
        </div>
        <div className="showcase-metric showcase-metric-private">
          <LockKeyhole size={17} />
          <span>Client excluded</span>
          <strong>{showcase.private_client_count}</strong>
          <small>Hard public-publishing boundary</small>
        </div>
      </div>

      <div className="showcase-queue">
        <div className="showcase-queue-heading">
          <div>
            <h3>Full fleet readiness ranking</h3>
            <p>
              Every repository is visible. Assessed work is ranked by product
              and demo readiness; unknown evidence stays explicitly unranked.
              Client work can receive a private readiness score but never a
              public-publishing priority.
            </p>
          </div>
          <div className="showcase-formulas">
            <span className="showcase-formula">
              Readiness · 60% product · 40% materials
            </span>
            <span className="showcase-formula">
              Public priority · 50% signal · 30% product · 20% gap
            </span>
          </div>
        </div>
        <div className="showcase-project-list">
          {projects.map((project) => (
            <article className="showcase-project" key={projectKey(project)}>
              <span className="showcase-rank">
                {readinessRanks.get(projectKey(project)) ?? "—"}
              </span>
              <div className="showcase-project-copy">
                <div>
                  <h4>{project.display_name}</h4>
                  <span
                    className={`showcase-lane showcase-lane-${project.lane}`}
                  >
                    {laneLabels[project.lane]}
                  </span>
                </div>
                <p>{project.next_step}</p>
                <small>
                  {project.public_eligibility === "private_client"
                    ? "Private audit only · never eligible for public publishing"
                    : `Missing: ${project.missing_materials.join(" · ") || "nothing"}`}
                </small>
              </div>
              <div className="showcase-project-scores">
                <span>
                  Product{" "}
                  <strong>{scoreLabel(project.product_readiness.score)}</strong>
                </span>
                <span>
                  Materials{" "}
                  <strong>{scoreLabel(project.demo_materials.score)}</strong>
                </span>
                <span>
                  Readiness{" "}
                  <strong>{scoreLabel(project.showcase_score)}</strong>
                </span>
                <span>
                  Public priority{" "}
                  <strong>{scoreLabel(project.priority_score)}</strong>
                </span>
                {project.repository_id && (
                  <button
                    type="button"
                    className="button button-quiet"
                    onClick={() => openProject(project)}
                  >
                    Open repository
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
