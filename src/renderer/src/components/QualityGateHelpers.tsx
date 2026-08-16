import { useState } from "react";
import { ShieldAlert } from "lucide-react";
import type { ReactElement } from "react";
import type { PortfolioSnapshot, RepositorySnapshot } from "../types";
import {
  qualityConfigurationSummary,
  qualityGateDisplayLabel,
} from "./QualityComponents";
import { StatusPill } from "./ConsolePrimitives";

export const CANONICAL_GATE_IDS = [
  "build",
  "runtime_smoke",
  "tests",
  "lint",
  "formatter",
  "typecheck",
  "dead_code",
  "secrets_scan",
] as const;

export const CONDITIONAL_GATE_IDS = [
  "dependency_audit",
  "web_readiness",
] as const;
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

export function customGateColumns(
  repositories: RepositorySnapshot[],
): string[] {
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

export function matrixGateColumns(
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

export function totalHighFindings(repositories: RepositorySnapshot[]): number {
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

export function readinessGapSummary(
  quality: PortfolioSnapshot["quality"],
): string {
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

export function BehaviorAssuranceSummary({
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

export function EdgeDurabilityPanel({
  repositories,
  onOpenRepository,
}: {
  repositories: RepositorySnapshot[];
  onOpenRepository: (repository: RepositorySnapshot) => void;
}): ReactElement {
  const [edgeFilter, setEdgeFilter] = useState<EdgeFleetFilter>("all");
  const edgeRepositories = repositories.filter((repository) =>
    matchesEdgeFilter(repository, edgeFilter),
  );
  const edgeCoverage = repositories.reduce(
    (total, repository) => {
      const coverage =
        repository.quality.behavior_assurance.coverage ?? EMPTY_EDGE_COVERAGE;
      for (const key of Object.keys(
        EMPTY_EDGE_COVERAGE,
      ) as (keyof typeof EMPTY_EDGE_COVERAGE)[])
        total[key] += coverage[key];
      return total;
    },
    { ...EMPTY_EDGE_COVERAGE },
  );
  return (
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
          <strong>{edgeCoverage.verified}</strong>/{edgeCoverage.total} verified
        </span>
        <span>
          <strong>{edgeCoverage.profiled}</strong>/{edgeCoverage.total} profiled
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
                {repository.quality.behavior_assurance.coverage?.verified ?? 0}/
                {repository.quality.behavior_assurance.coverage?.total ?? 0}{" "}
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
  );
}

export function EvidenceContractAlerts({
  contracts,
}: {
  contracts: NonNullable<PortfolioSnapshot["quality"]["evidence_contracts"]>;
}): ReactElement {
  return (
    <section
      className="quality-contract-alerts"
      aria-label="Evidence contract audits required"
    >
      {contracts.map((contract) => (
        <article className="quality-contract-alert" key={contract.contract_id}>
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
              {contract.current_repository_count}/{contract.repository_count}
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
  );
}

export function measurementConfidenceSummary(
  confidence: PortfolioSnapshot["quality"]["measurement_confidence"],
): string {
  if (!confidence) return "Measurement confidence unavailable";
  const level = `${confidence.level[0].toUpperCase()}${confidence.level.slice(1)}`;
  return `${level} measurement confidence · ${confidence.observed_repository_count}/${confidence.expected_repository_count} repositories measured`;
}

export function MeasurementConfidenceSummary({
  confidence,
}: {
  confidence: PortfolioSnapshot["quality"]["measurement_confidence"];
}): ReactElement {
  return (
    <>
      <small>{measurementConfidenceSummary(confidence)}</small>
      {confidence?.limitations.map((limitation) => (
        <small key={limitation}>{limitation.replaceAll("_", " ")}</small>
      ))}
    </>
  );
}
