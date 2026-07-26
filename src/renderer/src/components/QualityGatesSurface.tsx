import type { ReactElement } from "react";
import {
  CheckCircle2,
  ClipboardCheck,
  GitBranch,
  ShieldCheck,
} from "lucide-react";
import type { PortfolioSnapshot, RepositorySnapshot } from "../types";
import {
  QualityFindingsSummary,
  QualityGateCell,
  QualityMaturitySummary,
} from "./QualityComponents";
import { formatTime, StatusPill } from "./ConsolePrimitives";

function gateColumns(repositories: RepositorySnapshot[]): string[] {
  const ids = new Set<string>();
  for (const repository of repositories) {
    for (const gate of repository.quality.gates) ids.add(gate.id);
  }
  return Array.from(ids).sort((left, right) => {
    const canonical = [
      "build",
      "runtime_smoke",
      "lint",
      "formatter",
      "typecheck",
      "dead_code",
    ];
    const leftIndex = canonical.indexOf(left);
    const rightIndex = canonical.indexOf(right);
    return (
      (leftIndex < 0 ? canonical.length : leftIndex) -
        (rightIndex < 0 ? canonical.length : rightIndex) ||
      left.localeCompare(right)
    );
  });
}

function totalHighFindings(repositories: RepositorySnapshot[]): number {
  return repositories.reduce(
    (total, repository) =>
      total + repository.quality.findings.high_severity_total,
    0,
  );
}

export function QualityGatesSurface({
  snapshot,
  repositories,
  onOpenRepository,
  onOpenReport,
}: {
  snapshot: PortfolioSnapshot;
  repositories: RepositorySnapshot[];
  onOpenRepository: (repository: RepositorySnapshot) => void;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const columns = gateColumns(repositories);
  const configuredGateCount = repositories.reduce(
    (total, repository) =>
      total +
      repository.quality.gates.filter((gate) => gate.evidence.length > 0)
        .length,
    0,
  );
  const highFindings = totalHighFindings(repositories);
  const portfolioQuality = snapshot.quality;
  return (
    <>
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
          <span>Imported gate evidence</span>
          <strong>{configuredGateCount}</strong>
          <small>CI, local, and QR sources kept separate</small>
          <CheckCircle2 size={18} />
        </div>
        <div className="quality-overview-card">
          <span>High-severity QR findings</span>
          <strong>{highFindings}</strong>
          <small>Critical and high items in current reports</small>
          <ShieldCheck size={18} />
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
          <StatusPill tone="slate">6 canonical · custom discovered</StatusPill>
        </div>
        {repositories.length === 0 ? (
          <div className="surface-empty quality-empty">
            <ShieldCheck size={18} />
            <span>Register a repository root to compare quality evidence.</span>
          </div>
        ) : (
          <div className="quality-matrix-scroll">
            <table className="quality-matrix">
              <thead>
                <tr>
                  <th scope="col">Repository</th>
                  <th scope="col">Maturity / audit</th>
                  <th scope="col">QR findings</th>
                  {columns.map((column) => (
                    <th scope="col" key={column}>
                      {repositories
                        .flatMap((repository) => repository.quality.gates)
                        .find((gate) => gate.id === column)?.label ?? column}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {repositories.map((repository) => (
                  <tr key={repository.id}>
                    <th scope="row" className="quality-repository-cell">
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
                    <td>
                      <QualityMaturitySummary
                        maturity={repository.quality.maturity}
                        compact
                        onOpenReport={onOpenReport}
                      />
                    </td>
                    <td>
                      <QualityFindingsSummary
                        findings={repository.quality.findings}
                        onOpenReport={onOpenReport}
                      />
                    </td>
                    {columns.map((column) => {
                      const gate = repository.quality.gates.find(
                        (candidate) => candidate.id === column,
                      );
                      return (
                        <td key={column}>
                          {gate ? (
                            <QualityGateCell
                              gate={gate}
                              compact
                              showLabel={false}
                              onOpenReport={onOpenReport}
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
