import { useState } from "react";
import type { ReactElement } from "react";
import {
  CheckCircle2,
  ClipboardCheck,
  GitBranch,
  MoveHorizontal,
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
  macControlFreshnessLabel,
} from "./QualityComponents";
import { QualityMaturityWithCacheSummary } from "./CacheDesignSummary";
import { targetScopeForRepository } from "../branchEvidence";
import { formatTime, StatusPill } from "./ConsolePrimitives";

import {
  BehaviorAssuranceSummary,
  EvidenceContractAlerts,
  CANONICAL_GATE_IDS,
  CONDITIONAL_GATE_IDS,
  EdgeDurabilityPanel,
  MeasurementConfidenceSummary,
  customGateColumns,
  matrixGateColumns,
  readinessGapSummary,
  totalHighFindings,
} from "./QualityGateHelpers";
export { measurementConfidenceSummary } from "./QualityGateHelpers";

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
  const maturityCheckpoint = portfolioQuality.maturity_checkpoint;
  const staleEvidenceContracts = (
    portfolioQuality.evidence_contracts ?? []
  ).filter((contract) => contract.status !== "current");
  return (
    <>
      {showOverview && (
        <EvidenceContractAlerts contracts={staleEvidenceContracts} />
      )}
      \n{" "}
      {showOverview && (
        <section className="quality-overview-grid">
          <div className="quality-overview-card quality-overview-card-accent">
            <span>Consolidated fleet maturity</span>
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
            <MeasurementConfidenceSummary
              confidence={portfolioQuality.measurement_confidence}
            />
            <small>
              QR + Mac Control checkpoint:{" "}
              {maturityCheckpoint?.status ?? "Legacy separate"}
              {maturityCheckpoint?.observed_at
                ? ` · ${formatTime(maturityCheckpoint.observed_at)}`
                : ""}
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
            <span>High-severity detector findings in imported scans</span>
            <strong>{highFindings}</strong>
            <small>Target matching is shown per repository below</small>
            <ShieldCheck size={18} />
          </div>
        </section>
      )}
      <EdgeDurabilityPanel
        repositories={repositories}
        onOpenRepository={onOpenRepository}
      />
      \n\n{" "}
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
                    Detector findings
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
