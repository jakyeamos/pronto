import { useState, type ReactElement } from "react";
import { Film, Flag, LockKeyhole, Sparkles } from "lucide-react";
import type {
  RepositorySnapshot,
  ShowcasePortfolioSnapshot,
  ShowcaseProjectSnapshot,
} from "../types";
import { ExpandableScore, formatExactTime } from "./ConsolePrimitives";
import { QualityRunnerCaseStudy } from "./QualityRunnerCaseStudy";

const laneLabels: Record<ShowcaseProjectSnapshot["lane"], string> = {
  publish_ready: "Publish ready",
  create_materials: "Create materials",
  product_first: "Product first",
  private_client: "Private / client",
  blocked: "Blocked",
  unknown: "Unknown",
  not_applicable: "Not applicable",
};

const workDispositionLabels: Record<
  ShowcaseProjectSnapshot["work_disposition"],
  string
> = {
  largely_product_ready: "Largely product-ready",
  targeted_gap_closure: "Targeted gap closure",
  material_build_or_restoration: "Material build or restoration",
  conditional_gate: "Conditional gate",
  private_client: "Private / client",
  not_applicable: "Not applicable",
  blocked: "Blocked",
  unknown: "Disposition unknown",
};

const nextStepCategoryLabels: Record<
  ShowcaseProjectSnapshot["next_step_category"],
  string
> = {
  product: "Product",
  demo_integration: "Demo integration",
  evidence: "Evidence",
  content: "Content",
  packaging: "Packaging",
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
  const [activeCaseStudy, setActiveCaseStudy] = useState<string | null>(null);

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

  const projects = showcase.projects
    .filter(
      (project) =>
        project.public_eligibility !== "not_applicable" &&
        project.public_eligibility !== "private_client",
    )
    .sort((left, right) => {
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
  const scoring = showcase.scoring;
  const productWeight = scoring?.product_weight ?? 0.6;
  const materialsWeight = scoring?.materials_weight ?? 0.4;
  const careerWeight = scoring?.priority_career_weight ?? 0.5;
  const priorityProductWeight = scoring?.priority_product_weight ?? 0.3;
  const materialsGapWeight = scoring?.priority_materials_gap_weight ?? 0.2;

  if (activeCaseStudy === "quality-runner") {
    return <QualityRunnerCaseStudy onBack={() => setActiveCaseStudy(null)} />;
  }

  return (
    <section className="showcase-surface" aria-label="AI showcase readiness">
      <div className="showcase-heading">
        <div>
          <p className="eyebrow">AI showcase</p>
          <h2>Create materials for every showcase project</h2>
          <p>
            The full eligible tab is now the goal. Product-first and blocked
            projects must clear their evidence gates before final materials;
            client work remains outside the public queue.
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
          <span>Full showcase goal</span>
          <strong>
            {showcase.goal.publishable_demo_count}/
            {showcase.goal.target_publishable_demo_count}
          </strong>
          <small>{showcase.goal.remaining_demo_count} remaining</small>
        </div>
        <div className="showcase-metric">
          <Film size={17} />
          <span>Visible projects</span>
          <strong>{projects.length}</strong>
          <small>
            Eligible demo candidates; support and client work hidden
          </small>
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
            <h3>Showcase candidate ranking</h3>
            <p>
              Eligible work is ranked by product and demo readiness. Supporting
              repositories, upstream/provenance carriers, and client work stay
              out of this tab; unknown evidence remains explicitly unranked.
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
                  <span className="showcase-work-disposition">
                    {workDispositionLabels[project.work_disposition]}
                  </span>
                </div>
                <p>{project.work_disposition_summary}</p>
                <p>
                  <strong>
                    {nextStepCategoryLabels[project.next_step_category]}:
                  </strong>{" "}
                  {project.next_step}
                </p>
                <small>
                  {project.public_eligibility === "private_client"
                    ? "Private audit only · never eligible for public publishing"
                    : `Missing: ${project.missing_materials.join(" · ") || "nothing"}`}
                </small>
              </div>
              <div className="showcase-project-scores">
                <span>
                  <span title="Reviewed product readiness input">Product</span>{" "}
                  <strong>{scoreLabel(project.product_readiness.score)}</strong>
                </span>
                <span>
                  <span title="Reviewed demo-materials input">Materials</span>{" "}
                  <strong>{scoreLabel(project.demo_materials.score)}</strong>
                </span>
                <ExpandableScore
                  className="showcase-score-disclosure"
                  label="Readiness"
                  value={scoreLabel(project.showcase_score)}
                  description="Combined from product readiness and demo materials."
                  title="Open to see the reviewed inputs and their weights."
                  breakdown={[
                    {
                      id: "showcase-product",
                      label: "Product readiness",
                      value: scoreLabel(project.product_readiness.score),
                      detail: `${Math.round(productWeight * 100)}% weight`,
                    },
                    {
                      id: "showcase-materials",
                      label: "Demo materials",
                      value: scoreLabel(project.demo_materials.score),
                      detail: `${Math.round(materialsWeight * 100)}% weight`,
                    },
                  ]}
                />
                <ExpandableScore
                  className="showcase-score-disclosure"
                  label="Public priority"
                  value={scoreLabel(project.priority_score)}
                  description="A contract-defined ranking that combines career signal, product readiness, and the remaining materials gap."
                  title="Open to see the inputs and weights used by the Showcase contract."
                  breakdown={[
                    {
                      id: "priority-career-signal",
                      label: "Career signal",
                      value: scoreLabel(project.career_signal.score),
                      detail: `${Math.round(careerWeight * 100)}% weight`,
                    },
                    {
                      id: "priority-product",
                      label: "Product readiness",
                      value: scoreLabel(project.product_readiness.score),
                      detail: `${Math.round(priorityProductWeight * 100)}% weight`,
                    },
                    {
                      id: "priority-materials-gap",
                      label: "Materials gap",
                      value:
                        project.demo_materials.score == null
                          ? "—"
                          : `${(5 - project.demo_materials.score).toFixed(1)}/5`,
                      detail: `${Math.round(materialsGapWeight * 100)}% weight · 5 − materials score`,
                    },
                  ]}
                />
                {project.repository_id && (
                  <button
                    type="button"
                    className="button button-quiet"
                    onClick={() => openProject(project)}
                  >
                    Open repository
                  </button>
                )}
                {project.repository_name === "quality-runner" && (
                  <button
                    type="button"
                    className="button button-quiet showcase-case-button"
                    onClick={() => setActiveCaseStudy("quality-runner")}
                  >
                    View Tenure case study
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
