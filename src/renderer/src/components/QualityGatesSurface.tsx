import { useState } from "react";
import type { ReactElement } from "react";
import {
  CheckCircle2,
  ClipboardCheck,
  GitBranch,
  MoveHorizontal,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import type { PortfolioSnapshot, RepositorySnapshot } from "../types";
import {
  QualityFindingsSummary,
  QualityGateCell,
  QualityOutcomeSummary,
  projectQualityReadinessForTarget,
  qualityConfigurationSummary,
  qualityEvidenceSummary,
  qualityGateDisplayLabel,
  macControlFreshnessLabel,
} from "./QualityComponents";
import { QualityMaturityWithCacheSummary } from "./CacheDesignSummary";
import { targetScopeForRepository } from "../branchEvidence";
import { formatTime, StatusPill } from "./ConsolePrimitives";

const CANONICAL_GATE_IDS = [
  "build",
  "runtime_smoke",
  "tests",
  "lint",
  "formatter",
  "typecheck",
  "dead_code",
  "secrets_scan",
] as const;

const CONDITIONAL_GATE_IDS = ["dependency_audit", "web_readiness"] as const;
const EMPTY_EDGE_COVERAGE = {
  total: 0,
  profiled: 0,
  verified: 0,
  stale: 0,
  failed: 0,
  blocked: 0,
  unknown: 0,
};
type EdgeFleetFilter =
  | "all"
  | "missing"
  | "legacy"
  | "unprofiled"
  | "partially_verified"
  | "stale"
  | "failed"
  | "blocked"
  | "unknown"
  | "current"
  | "not_applicable";

function customGateColumns(repositories: RepositorySnapshot[]): string[] {
  const knownGateIds = new Set<string>([
    ...CANONICAL_GATE_IDS,
    ...CONDITIONAL_GATE_IDS,
  ]);
  const ids = new Set<string>();
  for (const repository of repositories) {
    for (const gate of repository.quality.gates) {
      if (!knownGateIds.has(gate.id)) ids.add(gate.id);
    }
  }
  return Array.from(ids).sort((left, right) => left.localeCompare(right));
}

function matrixGateColumns(
  repositories: RepositorySnapshot[],
  includeCustomGates: boolean,
): string[] {
  const conditional = CONDITIONAL_GATE_IDS.filter((id) =>
    repositories.some(
      (repository) =>
        repository.quality.gates.some((gate) => gate.id === id) ||
        repository.quality.ci_readiness.applicable_gate_ids.includes(id),
    ),
  );
  return [
    ...CANONICAL_GATE_IDS,
    ...conditional,
    ...(includeCustomGates ? customGateColumns(repositories) : []),
  ];
}

function totalHighFindings(repositories: RepositorySnapshot[]): number {
  return repositories.reduce(
    (total, repository) =>
      total + repository.quality.findings.high_severity_total,
    0,
  );
}

function matchesEdgeFilter(
  repository: RepositorySnapshot,
  filter: EdgeFleetFilter,
): boolean {
  const assurance = repository.quality.behavior_assurance;
  const state = behaviorAssuranceState(assurance);
  if (filter === "all") return true;
  if (filter === "missing") return state === "missing_contract";
  if (filter === "legacy") return state === "legacy_v1";
  if (filter === "unprofiled") return state === "unprofiled";
  if (filter === "partially_verified") return state === "partially_verified";
  if (filter === "stale") return state === "stale";
  if (filter === "failed") return state === "failed";
  if (filter === "blocked") return state === "blocked";
  if (filter === "unknown") return state === "unknown";
  if (filter === "current") return state === "current";
  return state === "not_applicable";
}

function behaviorAssuranceState(
  assurance: RepositorySnapshot["quality"]["behavior_assurance"],
): string {
  if (assurance.state) return assurance.state;
  if (
    assurance.contract_status === "missing" &&
    assurance.gaps.some((gap) => gap.kind === "contract_missing")
  )
    return "missing_contract";
  if (
    assurance.contract_schema === "pronto-behavior-assurance/v1" ||
    assurance.edge_profile_status === "legacy"
  )
    return "legacy_v1";
  if (assurance.applicability === "not_applicable") return "not_applicable";
  if (
    assurance.contract_status === "invalid" ||
    assurance.result_status === "blocked"
  )
    return "blocked";
  if (assurance.result_status === "failed") return "failed";
  if (assurance.freshness === "stale") return "stale";
  if (
    assurance.passed_scenario_count > 0 &&
    assurance.passed_scenario_count < assurance.required_scenario_count
  )
    return "partially_verified";
  if (
    assurance.edge_profile_status === "missing" ||
    assurance.edge_profile_status === "unprofiled" ||
    assurance.edge_profile_status === "partially_profiled"
  )
    return "unprofiled";
  if (assurance.release_ready) return "current";
  return "unknown";
}

function behaviorAssuranceStateLabel(
  assurance: RepositorySnapshot["quality"]["behavior_assurance"],
): string {
  const labels: Record<string, string> = {
    missing_contract: "Missing contract",
    legacy_v1: "Legacy v1",
    unprofiled: "Unprofiled",
    partially_verified: "Partially verified",
    stale: "Stale",
    failed: "Failed",
    blocked: "Blocked",
    unknown: "Unknown",
    current: "Current",
    not_applicable: "Not applicable",
  };
  return labels[behaviorAssuranceState(assurance)] ?? "Unknown";
}

function readinessGapSummary(quality: PortfolioSnapshot["quality"]): string {
  const configuration = qualityConfigurationSummary(quality);
  if (configuration.ideal === 0) {
    return "CI configuration profile not available";
  }
  const entries = Object.entries(quality.ci_readiness_open_gate_counts ?? {})
    .sort(
      ([leftId, leftCount], [rightId, rightCount]) =>
        rightCount - leftCount || leftId.localeCompare(rightId),
    )
    .slice(0, 3);
  return entries.length > 0
    ? `Evidence updates: ${entries
        .map(
          ([gateId, count]) => `${qualityGateDisplayLabel(gateId)} (${count})`,
        )
        .join(" · ")}`
    : "No open gate updates";
}

function BehaviorAssuranceSummary({
  repository,
}: {
  repository: RepositorySnapshot;
}): ReactElement {
  const assurance = repository.quality.behavior_assurance;
  const coverage = assurance.coverage ?? EMPTY_EDGE_COVERAGE;
  const state = behaviorAssuranceState(assurance);
  const status = behaviorAssuranceStateLabel(assurance);
  return (
    <div className="quality-maturity-summary">
      <StatusPill
        tone={
          state === "current" || state === "not_applicable"
            ? "mint"
            : state === "failed" || state === "blocked"
              ? "coral"
              : "amber"
        }
      >
        {status}
      </StatusPill>
      <strong>
        Release {assurance.passed_scenario_count}/
        {assurance.required_scenario_count}
      </strong>
      <small>required Tier-0 scenarios</small>
      <strong>
        Edge {coverage.verified}/{coverage.total}
      </strong>
      <small>
        verified · {coverage.profiled}/{coverage.total} profiled
      </small>
      {(coverage.failed > 0 ||
        coverage.blocked > 0 ||
        coverage.stale > 0 ||
        coverage.unknown > 0) && (
        <small>
          {coverage.failed} failed · {coverage.blocked} blocked ·{" "}
          {coverage.stale} stale · {coverage.unknown} unknown
        </small>
      )}
      {assurance.coverage?.category_gaps.slice(0, 2).map((gap) => (
        <small key={gap.category}>
          {gap.category.replaceAll("_", " ")} · {gap.scenario_count} gap
          {gap.scenario_count === 1 ? "" : "s"}
        </small>
      ))}
      {assurance.gaps[0] && <small>{assurance.gaps[0].message}</small>}
      {assurance.gaps.length > 1 && (
        <small>{assurance.gaps.length - 1} more gaps</small>
      )}
    </div>
  );
}

export function QualityGatesSurface({
  snapshot,
  repositories,
  onOpenRepository,
  onOpenReport,
  showOverview = true,
}: {
  snapshot: PortfolioSnapshot;
  repositories: RepositorySnapshot[];
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onOpenReport?: (reportPath: string) => void;
  showOverview?: boolean;
}): ReactElement {
  const [showCustomGates, setShowCustomGates] = useState(false);
  const [edgeFilter, setEdgeFilter] = useState<EdgeFleetFilter>("all");
  const discoveredCustomGates = customGateColumns(repositories);
  const customGateCountLabel = `${discoveredCustomGates.length} custom gate${
    discoveredCustomGates.length === 1 ? "" : "s"
  }`;
  const columns = matrixGateColumns(repositories, showCustomGates);
  const canonicalGateCount = CANONICAL_GATE_IDS.length;
  const conditionalGateCount = columns.filter((column) =>
    CONDITIONAL_GATE_IDS.includes(
      column as (typeof CONDITIONAL_GATE_IDS)[number],
    ),
  ).length;
  const configuredGateCount = repositories.reduce(
    (total, repository) =>
      total + repository.quality.ci_readiness.configured_gate_ids.length,
    0,
  );
  const highFindings = totalHighFindings(repositories);
  const portfolioQuality = snapshot.quality;
  const configuration = qualityConfigurationSummary(portfolioQuality);
  const evidence = qualityEvidenceSummary(portfolioQuality);
  const macControl = portfolioQuality.mac_control_ideal_state;
  const staleEvidenceContracts = (
    portfolioQuality.evidence_contracts ?? []
  ).filter((contract) => contract.status !== "current");
  const edgeRepositories = repositories.filter((repository) =>
    matchesEdgeFilter(repository, edgeFilter),
  );
  const edgeCoverage = repositories.reduce(
    (total, repository) => {
      const coverage =
        repository.quality.behavior_assurance.coverage ?? EMPTY_EDGE_COVERAGE;
      for (const key of Object.keys(
        EMPTY_EDGE_COVERAGE,
      ) as (keyof typeof EMPTY_EDGE_COVERAGE)[]) {
        total[key] += coverage[key];
      }
      return total;
    },
    { ...EMPTY_EDGE_COVERAGE },
  );
  return (
    <>
      {showOverview && staleEvidenceContracts.length > 0 && (
        <section
          className="quality-contract-alerts"
          aria-label="Evidence contract audits required"
        >
          {staleEvidenceContracts.map((contract) => (
            <article
              className="quality-contract-alert"
              key={contract.contract_id}
            >
              <ShieldAlert size={20} />
              <div>
                <p className="eyebrow">Evidence contract changed</p>
                <h2>Full fleet audit required</h2>
                <strong>{contract.label}</strong>
                <p>{contract.message}</p>
                <small>{contract.next_safe_step}</small>
              </div>
              <div className="quality-contract-coverage">
                <strong>
                  {contract.current_repository_count}/
                  {contract.repository_count}
                </strong>
                <span>current</span>
                <small>
                  {contract.legacy_repository_count} legacy ·{" "}
                  {contract.missing_repository_count} missing
                </small>
              </div>
            </article>
          ))}
        </section>
      )}
      {showOverview && (
        <section className="quality-overview-grid">
          <div className="quality-overview-card quality-overview-card-accent">
            <span>Fleet maturity</span>
            <strong>
              {portfolioQuality.maturity_score_display ?? "—"}
              <small>/4</small>
            </strong>
            <small>
              {portfolioQuality.scored_dimension_count
                ? `${portfolioQuality.scored_dimension_count} pillar assessments · `
                : ""}
              {portfolioQuality.audit_status}
            </small>
            {portfolioQuality.source_maturity_score_display && (
              <small>
                {portfolioQuality.feed_schema ===
                "quality-runner-maturity-feed/v2"
                  ? "QR source holistic"
                  : "Legacy QR dimension mean"}{" "}
                {portfolioQuality.source_maturity_score_display}/4
              </small>
            )}
            {(portfolioQuality.maturity_pillars ?? []).length > 0 && (
              <div
                className="portfolio-maturity-pillars"
                aria-label="Fleet maturity pillars"
              >
                {(portfolioQuality.maturity_pillars ?? []).map((pillar) => (
                  <span key={pillar.id} title={pillar.label}>
                    <b>{pillar.label}</b>
                    {pillar.score === undefined
                      ? "Unknown"
                      : `${pillar.score}/4`}
                  </span>
                ))}
              </div>
            )}
            {portfolioQuality.maturity_evidence_coverage !== undefined && (
              <small>
                {Math.round(portfolioQuality.maturity_evidence_coverage * 100)}%
                evidence ·{" "}
                {Math.round(
                  (portfolioQuality.maturity_fresh_evidence_coverage ?? 0) *
                    100,
                )}
                % fresh ·{" "}
                {portfolioQuality.maturity_provisional_repository_count ?? 0}
                {" provisional · "}
                {portfolioQuality.maturity_capped_repository_count ?? 0} capped
              </small>
            )}
            <small>
              Product readiness and Project Compass progress are reported
              separately from repository maturity.
            </small>
            <QualityOutcomeSummary quality={portfolioQuality} />
            <div className="quality-overview-secondary">
              <span>CI configuration</span>
              <strong>
                {configuration.ideal > 0
                  ? `${configuration.configured}/${configuration.ideal}`
                  : "—"}
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
              <small>{readinessGapSummary(portfolioQuality)}</small>
            </div>
            <ClipboardCheck size={18} />
          </div>
          <div className="quality-overview-card">
            <span>Mac Control ideal state</span>
            <strong>
              {macControl?.ideal_state ? "Pass" : (macControl?.status ?? "—")}
            </strong>
            <small>
              {macControl
                ? `${macControl.applicable_repository_count} applicable repositories · ${macControlFreshnessLabel(macControl.freshness)}`
                : "Not configured"}
            </small>
            <small>
              {macControl &&
              typeof macControl.implementation_criteria_passed_count ===
                "number" &&
              typeof macControl.implementation_criteria_total === "number"
                ? "Semantic source evidence: " +
                  macControl.implementation_criteria_passed_count +
                  "/" +
                  macControl.implementation_criteria_total +
                  " dimensions · " +
                  (macControl.implementation_score_display ?? "—") +
                  "/4 · " +
                  (macControl.implementation_status ?? "—")
                : "Semantic source-evidence lane not reported"}
            </small>
            {macControl &&
            (macControl.implementation_declaration_criteria_count ?? 0) > 0 ? (
              <small>
                Legacy declarations:{" "}
                {macControl.implementation_declaration_criteria_count} recorded
                · non-scoring until v4 source evidence is established
              </small>
            ) : null}
            <small>
              {macControl &&
              typeof macControl.measured_task_count === "number" &&
              typeof macControl.live_task_count === "number"
                ? "Live tasks: " +
                  macControl.measured_task_count +
                  "/" +
                  macControl.live_task_count +
                  " measured · " +
                  (macControl.live_score_display ?? "—") +
                  "/4 · " +
                  (macControl.live_status ?? "—")
                : "Live task lane not reported"}
            </small>
            <small>
              Source-grounded semantics and live task evidence are both required
              for the 4.0/4.0 maturity ideal
            </small>
            <ShieldCheck size={18} />
          </div>
          <div className="quality-overview-card">
            <span>Repositories matched</span>
            <strong>{portfolioQuality.matched_repository_count}</strong>
            <small>
              {portfolioQuality.latest_audit_at
                ? `Audit ${formatTime(portfolioQuality.latest_audit_at)}`
                : "No audit run imported"}
            </small>
            <GitBranch size={18} />
          </div>
          <div className="quality-overview-card">
            <span>Fresh passing evidence</span>
            <strong>
              {evidence.ideal > 0
                ? `${evidence.freshPassing}/${evidence.ideal}`
                : "—"}
            </strong>
            <small>
              {evidence.ideal > 0
                ? "Fresh CI, local, and QR passes"
                : "No matched recommendation profile"}
            </small>
            <CheckCircle2 size={18} />
          </div>
          <div className="quality-overview-card">
            <span>High-severity QR findings in imported scans</span>
            <strong>{highFindings}</strong>
            <small>Target matching is shown per repository below</small>
            <ShieldCheck size={18} />
          </div>
        </section>
      )}

      <section className="surface-panel quality-edge-panel">
        <div className="surface-heading quality-edge-heading">
          <div>
            <p className="eyebrow">Whole-inventory assurance</p>
            <h2>Edge durability</h2>
            <p>
              Release assurance remains Tier 0. This view tracks every declared
              scenario without turning the wider inventory into a release gate.
            </p>
          </div>
          <label className="quality-edge-filter">
            <span>Show repositories</span>
            <select
              aria-label="Filter edge durability repositories"
              value={edgeFilter}
              onChange={(event) =>
                setEdgeFilter(event.target.value as EdgeFleetFilter)
              }
            >
              <option value="all">All</option>
              <option value="missing">Missing contracts</option>
              <option value="legacy">Legacy v1</option>
              <option value="unprofiled">Unprofiled scenarios</option>
              <option value="partially_verified">Partially verified</option>
              <option value="stale">Stale receipts</option>
              <option value="failed">Reproducible failures</option>
              <option value="blocked">Blocked</option>
              <option value="unknown">Unknown</option>
              <option value="current">Current</option>
              <option value="not_applicable">Not applicable</option>
            </select>
          </label>
        </div>
        <div className="quality-edge-metrics">
          <span>
            <strong>{edgeCoverage.verified}</strong>/{edgeCoverage.total}{" "}
            verified
          </span>
          <span>
            <strong>{edgeCoverage.profiled}</strong>/{edgeCoverage.total}{" "}
            profiled
          </span>
          <span>{edgeCoverage.stale} stale</span>
          <span>{edgeCoverage.failed} failed</span>
          <span>{edgeCoverage.blocked} blocked</span>
          <span>{edgeCoverage.unknown} unknown</span>
        </div>
        <div className="quality-edge-repositories">
          {edgeRepositories.length === 0 ? (
            <p>No repositories match this edge-durability filter.</p>
          ) : (
            edgeRepositories.map((repository) => (
              <button
                className="quality-edge-repository"
                key={repository.id}
                type="button"
                onClick={() => onOpenRepository(repository)}
              >
                <strong>{repository.name}</strong>
                <span>
                  {behaviorAssuranceStateLabel(
                    repository.quality.behavior_assurance,
                  )}
                </span>
                <small>
                  {repository.quality.behavior_assurance.coverage?.verified ??
                    0}
                  /{repository.quality.behavior_assurance.coverage?.total ?? 0}{" "}
                  verified ·{" "}
                  {repository.quality.behavior_assurance.edge_profile_status ??
                    "unprofiled"}{" "}
                  profile
                </small>
              </button>
            ))
          )}
        </div>
      </section>

      <section className="surface-panel quality-matrix-panel">
        <div className="surface-heading quality-matrix-heading">
          <div>
            <p className="eyebrow">Repository comparison</p>
            <h2>Quality gate matrix</h2>
            <p>
              Evidence is imported from existing CI, local, or QR artifacts. No
              command runs from this surface.
            </p>
          </div>
          <div className="quality-matrix-heading-meta">
            <div className="quality-matrix-controls">
              <StatusPill tone="slate">
                {canonicalGateCount} canonical
                {conditionalGateCount > 0 ? " · dependency audit applies" : ""}
              </StatusPill>
              {discoveredCustomGates.length > 0 && (
                <button
                  className="button button-secondary quality-matrix-custom-toggle"
                  type="button"
                  aria-pressed={showCustomGates}
                  onClick={() => setShowCustomGates((visible) => !visible)}
                >
                  {showCustomGates
                    ? "Hide custom gates"
                    : `Show ${customGateCountLabel}`}
                </button>
              )}
            </div>
            <span>
              {showCustomGates
                ? `${columns.length} gates visible · horizontal comparison`
                : "Canonical release gates shown by default"}
            </span>
          </div>
        </div>
        {repositories.length === 0 ? (
          <div className="surface-empty quality-empty">
            <ShieldCheck size={18} />
            <span>Register a repository root to compare quality evidence.</span>
          </div>
        ) : (
          <div
            className="quality-matrix-scroll"
            role="region"
            aria-label="Quality gate comparison"
            tabIndex={0}
          >
            <div className="quality-matrix-scroll-hint">
              <MoveHorizontal size={14} />
              <span>
                {showCustomGates
                  ? `Scroll horizontally to compare all ${columns.length} gates.`
                  : "Custom gates are hidden until you choose to compare them."}
              </span>
            </div>
            <table className="quality-matrix">
              <colgroup>
                <col className="quality-matrix-repository-column" />
                <col className="quality-matrix-maturity-column" />
                <col className="quality-matrix-findings-column" />
                <col className="quality-matrix-gate-column" />
                {columns.map((column) => (
                  <col className="quality-matrix-gate-column" key={column} />
                ))}
              </colgroup>
              <thead>
                <tr>
                  <th
                    scope="col"
                    className="quality-matrix-sticky quality-matrix-repository-column"
                  >
                    Repository
                  </th>
                  <th
                    scope="col"
                    className="quality-matrix-sticky quality-matrix-maturity-column"
                  >
                    Maturity / audit
                  </th>
                  <th
                    scope="col"
                    className="quality-matrix-sticky quality-matrix-findings-column"
                  >
                    QR findings
                  </th>
                  <th scope="col" className="quality-matrix-gate-column">
                    <span className="quality-matrix-gate-heading">
                      Behavior assurance
                    </span>
                  </th>
                  {columns.map((column) => (
                    <th
                      scope="col"
                      className="quality-matrix-gate-column"
                      key={column}
                    >
                      <span
                        className="quality-matrix-gate-heading"
                        title={column}
                      >
                        {repositories
                          .flatMap((repository) => repository.quality.gates)
                          .find((gate) => gate.id === column)?.label ??
                          (column === "tests"
                            ? "Tests"
                            : column === "secrets_scan"
                              ? "Secrets scan"
                              : column === "dependency_audit"
                                ? "Dependency audit"
                                : column)}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {repositories.map((repository) => {
                  const target = targetScopeForRepository(repository);
                  const targetReadiness = projectQualityReadinessForTarget(
                    repository.quality.ci_readiness,
                    repository.quality.gates,
                    target.branch,
                    target.commit,
                  );
                  return (
                    <tr key={repository.id}>
                      <th
                        scope="row"
                        className="quality-repository-cell quality-matrix-sticky quality-matrix-repository-column"
                      >
                        <button
                          className="quality-repository-button"
                          type="button"
                          onClick={() => onOpenRepository(repository)}
                        >
                          <strong>{repository.name}</strong>
                          <span>{repository.path}</span>
                          <small>
                            <GitBranch size={11} /> {repository.branch} ·
                            scanned {formatTime(repository.last_scan_at)}
                          </small>
                          <small>
                            Target {target.branch ?? "unavailable"}
                            {target.commit
                              ? ` @ ${target.commit.slice(0, 8)}`
                              : " · head unavailable"}
                          </small>
                        </button>
                      </th>
                      <td className="quality-matrix-sticky quality-matrix-maturity-column">
                        <QualityMaturityWithCacheSummary
                          maturity={repository.quality.maturity}
                          readiness={targetReadiness.readiness}
                          targetBranch={target.branch}
                          targetCommit={target.commit}
                          targetReadinessState={targetReadiness.state}
                          compact
                          onOpenReport={onOpenReport}
                        />
                      </td>
                      <td className="quality-matrix-sticky quality-matrix-findings-column">
                        <QualityFindingsSummary
                          findings={repository.quality.findings}
                          targetBranch={
                            repository.target_branch ??
                            repository.default_branch
                          }
                          targetCommit={target.commit}
                          onOpenReport={onOpenReport}
                        />
                      </td>
                      <td className="quality-matrix-gate-column">
                        <BehaviorAssuranceSummary repository={repository} />
                      </td>
                      {columns.map((column) => {
                        const gate = repository.quality.gates.find(
                          (candidate) => candidate.id === column,
                        );
                        const optionalColumn = !CANONICAL_GATE_IDS.includes(
                          column as (typeof CANONICAL_GATE_IDS)[number],
                        );
                        const applicableColumn =
                          repository.quality.ci_readiness.applicable_gate_ids.includes(
                            column,
                          );
                        return (
                          <td
                            key={column}
                            className="quality-matrix-gate-column"
                          >
                            {gate ? (
                              <QualityGateCell
                                gate={gate}
                                configured={repository.quality.ci_readiness.configured_gate_ids.includes(
                                  gate.id,
                                )}
                                compact
                                showLabel={false}
                                onOpenReport={onOpenReport}
                                targetBranch={target.branch}
                                targetCommit={target.commit}
                              />
                            ) : optionalColumn && !applicableColumn ? (
                              <span
                                className="quality-matrix-empty-cell"
                                aria-label="Not applicable for this repository"
                              />
                            ) : (
                              <QualityGateCell
                                gate={{
                                  id: column,
                                  label: column,
                                  status: "Not configured",
                                  freshness: "Unknown",
                                  evidence: [],
                                }}
                                configured={repository.quality.ci_readiness.configured_gate_ids.includes(
                                  column,
                                )}
                                compact
                                showLabel={false}
                                targetBranch={target.branch}
                                targetCommit={target.commit}
                              />
                            )}
                          </td>
                        );
                      })}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
        {repositories.length > 0 && configuredGateCount === 0 && (
          <div className="quality-inline-empty">
            <ShieldCheck size={15} />
            No CI, local, or QR gate evidence has been imported yet. Refreshing
            only reads existing artifacts.
          </div>
        )}
      </section>
    </>
  );
}
