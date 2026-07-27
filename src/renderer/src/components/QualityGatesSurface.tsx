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
  QualityMaturitySummary,
  qualityConfigurationSummary,
  qualityEvidenceSummary,
  qualityGateDisplayLabel,
} from "./QualityComponents";
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

const CONDITIONAL_GATE_IDS = ["dependency_audit"] as const;

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
    repositories.some((repository) =>
      repository.quality.gates.some((gate) => gate.id === id),
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
      total +
      repository.quality.gates.filter((gate) => gate.evidence.length > 0)
        .length,
    0,
  );
  const highFindings = totalHighFindings(repositories);
  const portfolioQuality = snapshot.quality;
  const configuration = qualityConfigurationSummary(portfolioQuality);
  const evidence = qualityEvidenceSummary(portfolioQuality);
  return (
    <>
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
                ? `${portfolioQuality.scored_dimension_count} dimensions · `
                : ""}
              {portfolioQuality.audit_status}
            </small>
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
            <span>High-severity QR findings</span>
            <strong>{highFindings}</strong>
            <small>Critical and high items in current reports</small>
            <ShieldCheck size={18} />
          </div>
        </section>
      )}

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
                {repositories.map((repository) => (
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
                          <GitBranch size={11} /> {repository.branch} · scanned{" "}
                          {formatTime(repository.last_scan_at)}
                        </small>
                      </button>
                    </th>
                    <td className="quality-matrix-sticky quality-matrix-maturity-column">
                      <QualityMaturitySummary
                        maturity={repository.quality.maturity}
                        readiness={repository.quality.ci_readiness}
                        compact
                        onOpenReport={onOpenReport}
                      />
                    </td>
                    <td className="quality-matrix-sticky quality-matrix-findings-column">
                      <QualityFindingsSummary
                        findings={repository.quality.findings}
                        onOpenReport={onOpenReport}
                      />
                    </td>
                    {columns.map((column) => {
                      const gate = repository.quality.gates.find(
                        (candidate) => candidate.id === column,
                      );
                      const optionalColumn = !CANONICAL_GATE_IDS.includes(
                        column as (typeof CANONICAL_GATE_IDS)[number],
                      );
                      return (
                        <td key={column} className="quality-matrix-gate-column">
                          {gate ? (
                            <QualityGateCell
                              gate={gate}
                              compact
                              showLabel={false}
                              onOpenReport={onOpenReport}
                            />
                          ) : optionalColumn ? (
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
                              compact
                              showLabel={false}
                            />
                          )}
                        </td>
                      );
                    })}
                  </tr>
                ))}
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
