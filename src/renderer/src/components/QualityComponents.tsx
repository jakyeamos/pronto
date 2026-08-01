import type { ReactElement } from "react";
import { ExternalLink, FileSearch, ShieldAlert } from "lucide-react";
import type {
  QualityEvidence,
  QualityFreshness,
  QualityGate,
  QualityGateStatus,
  QualityMaturity,
  QualityPortfolioSnapshot,
  QualityReadiness,
  RepositorySnapshot,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

export { QualityFindingsSummary } from "./QualityFindingsSummary";

function qualityStatusTone(status: QualityGateStatus): string {
  if (status === "Passed") return "mint";
  if (status === "Failed") return "coral";
  if (status === "Blocked") return "red";
  return "slate";
}

function qualityFreshnessTone(freshness: QualityFreshness): string {
  if (freshness === "Fresh") return "mint";
  if (freshness === "Stale" || freshness === "Conflicted") return "amber";
  return "slate";
}

export function QualityGateStatusPill({
  status,
  freshness,
}: {
  status: QualityGateStatus;
  freshness?: QualityFreshness;
}): ReactElement {
  return (
    <span className="quality-status-stack">
      <StatusPill tone={qualityStatusTone(status)}>{status}</StatusPill>
      {freshness && freshness !== "Unknown" && (
        <StatusPill tone={qualityFreshnessTone(freshness)}>
          {freshness}
        </StatusPill>
      )}
    </span>
  );
}

export function QualityTraceStatusPill({
  value,
}: {
  value: string;
}): ReactElement {
  const status = (
    ["Passed", "Failed", "Blocked", "Not configured"] as const
  ).find(
    (candidate) => value === candidate || value.startsWith(`${candidate} ·`),
  );
  if (!status) {
    return (
      <StatusPill tone={value === "Unknown" ? "slate" : "amber"}>
        {value}
      </StatusPill>
    );
  }
  const freshness = value.includes(" · ")
    ? value.split(" · ").slice(1).join(" · ")
    : undefined;
  return (
    <QualityGateStatusPill
      status={status}
      freshness={
        freshness === "Fresh" ||
        freshness === "Stale" ||
        freshness === "Unknown" ||
        freshness === "Conflicted"
          ? freshness
          : undefined
      }
    />
  );
}

function EvidenceAction({
  evidence,
  onOpenReport,
}: {
  evidence: QualityEvidence;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement | null {
  if (evidence.report_path && onOpenReport) {
    return (
      <button
        className="quality-report-link"
        type="button"
        onClick={() => onOpenReport(evidence.report_path as string)}
      >
        <FileSearch size={12} />
        Detailed report
      </button>
    );
  }
  if (evidence.report_url) {
    return (
      <a
        className="quality-report-link"
        href={evidence.report_url}
        target="_blank"
        rel="noreferrer"
      >
        <ExternalLink size={12} />
        Open source
      </a>
    );
  }
  return null;
}

export function QualityEvidenceList({
  evidence,
  onOpenReport,
}: {
  evidence: QualityEvidence[];
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  return (
    <div className="quality-evidence-list">
      {evidence.map((item, index) => (
        <div
          className="quality-evidence-row"
          key={`${item.id}-${item.source}-${item.observed_at ?? "unknown"}-${index}`}
        >
          <div className="quality-evidence-heading">
            <strong>
              {item.source} · {item.source_label}
            </strong>
            <QualityGateStatusPill
              status={item.status}
              freshness={item.freshness}
            />
          </div>
          <span>{item.detail || "No additional result detail"}</span>
          <small>
            {item.command ? `${item.command} · ` : ""}
            {item.scanned_commit
              ? `commit ${item.scanned_commit.slice(0, 8)}`
              : item.scanned_branch
                ? `branch ${item.scanned_branch}`
                : "Reference unavailable"}
            {" · "}
            {formatTime(item.observed_at)}
          </small>
          <EvidenceAction evidence={item} onOpenReport={onOpenReport} />
        </div>
      ))}
    </div>
  );
}

export function QualityGateCell({
  gate,
  configured = false,
  compact = false,
  showLabel = true,
  onOpenReport,
}: {
  gate: QualityGate;
  configured?: boolean;
  compact?: boolean;
  showLabel?: boolean;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const configuredWithoutEvidence =
    configured && gate.status === "Not configured";
  return (
    <div
      className={`quality-gate-cell${compact ? " quality-gate-cell-compact" : ""}`}
    >
      {showLabel && (
        <strong className="quality-gate-label">{gate.label}</strong>
      )}
      {configuredWithoutEvidence ? (
        <StatusPill tone="slate">Configured</StatusPill>
      ) : (
        <QualityGateStatusPill
          status={gate.status}
          freshness={gate.freshness}
        />
      )}
      <span className="quality-gate-evidence-count">
        {gate.evidence.length === 0
          ? "No evidence"
          : `${gate.evidence.length} source${gate.evidence.length === 1 ? "" : "s"}`}
      </span>
      {gate.evidence.length > 0 && (
        <details className="quality-evidence-disclosure">
          <summary>Expand evidence</summary>
          <QualityEvidenceList
            evidence={gate.evidence}
            onOpenReport={onOpenReport}
          />
        </details>
      )}
    </div>
  );
}

const QUALITY_GATE_LABELS: Record<string, string> = {
  build: "Build",
  runtime_smoke: "Smoke",
  tests: "Tests",
  lint: "Lint",
  formatter: "Formatter",
  typecheck: "Typecheck",
  dead_code: "Dead-code",
  secrets_scan: "Secrets scan",
  dependency_audit: "Dependency audit",
};

function readinessOpenGateIds(readiness: QualityReadiness): string[] {
  return Array.from(
    new Set([
      ...readiness.missing_gate_ids,
      ...readiness.stale_gate_ids,
      ...readiness.failed_gate_ids,
      ...readiness.blocked_gate_ids,
    ]),
  );
}

export function qualityGateDisplayLabel(gateId: string): string {
  return (
    QUALITY_GATE_LABELS[gateId] ??
    gateId
      .replace(/^custom:/, "")
      .split("_")
      .filter(Boolean)
      .map((part) => `${part[0]?.toUpperCase() ?? ""}${part.slice(1)}`)
      .join(" ")
  );
}

export function qualityConfigurationSummary(
  quality: QualityPortfolioSnapshot,
): {
  configured: number;
  ideal: number;
  fullRepositories: number;
  repositories: number;
  unscoredRepositories: number;
} {
  return {
    configured: quality.ci_configuration_configured_gate_count ?? 0,
    ideal: quality.ci_configuration_ideal_gate_count ?? 0,
    fullRepositories: quality.ci_configuration_full_repository_count ?? 0,
    repositories: quality.ci_configuration_repository_count ?? 0,
    unscoredRepositories:
      quality.ci_configuration_unscored_repository_count ?? 0,
  };
}

export function qualityEvidenceSummary(quality: QualityPortfolioSnapshot): {
  freshPassing: number;
  ideal: number;
} {
  return {
    freshPassing: quality.ci_evidence_fresh_passing_gate_count ?? 0,
    ideal: quality.ci_evidence_ideal_gate_count ?? 0,
  };
}

function QualityReadinessSummary({
  readiness,
  compact = false,
}: {
  readiness: QualityReadiness;
  compact?: boolean;
}): ReactElement {
  const openGateIds = readinessOpenGateIds(readiness);
  const configuredGateIds = readiness.configured_gate_ids ?? [];
  const unconfiguredGateIds = readiness.unconfigured_gate_ids ?? [];
  const applicableGateCount = readiness.applicable_gate_ids.length;
  const evidenceGateCount = readiness.covered_gate_ids.length;
  const freshPassingGateCount = readiness.fresh_passing_gate_ids ?? [];
  return (
    <div
      className={`quality-readiness${compact ? " quality-readiness-compact" : ""}`}
    >
      <div className="quality-readiness-heading">
        <span>CI configuration</span>
        <strong>
          {readiness.configuration_score == null
            ? "—"
            : `${configuredGateIds.length}/${applicableGateCount}`}
        </strong>
      </div>
      {readiness.configuration_score == null ? (
        <small>No matched recommendation profile</small>
      ) : (
        <>
          <small>
            {configuredGateIds.length}/{applicableGateCount} ideal gates
            configured
          </small>
          <small>
            Fresh passing evidence: {freshPassingGateCount.length}/
            {applicableGateCount}
          </small>
          <small>
            Imported evidence: {evidenceGateCount}/{applicableGateCount}
          </small>
          {unconfiguredGateIds.length > 0 ? (
            <details className="quality-readiness-disclosure">
              <summary>
                {unconfiguredGateIds.length} gate configuration update
                {unconfiguredGateIds.length === 1 ? "" : "s"} needed
              </summary>
              <span>
                {unconfiguredGateIds.map(qualityGateDisplayLabel).join(", ")}
              </span>
            </details>
          ) : openGateIds.length > 0 ? (
            <details className="quality-readiness-disclosure">
              <summary>
                {openGateIds.length} gate evidence update
                {openGateIds.length === 1 ? "" : "s"} needed
              </summary>
              <span>{openGateIds.map(qualityGateDisplayLabel).join(", ")}</span>
            </details>
          ) : null}
        </>
      )}
    </div>
  );
}

export function QualityMaturitySummary({
  maturity,
  readiness,
  compact = false,
  onOpenReport,
}: {
  maturity: QualityMaturity;
  readiness: QualityReadiness;
  compact?: boolean;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  return (
    <div
      className={`quality-maturity${compact ? " quality-maturity-compact" : ""}`}
    >
      <strong>{maturity.score_display ?? "Not scored"}</strong>
      <span>
        {maturity.score_display ? "/ 4 maturity" : "Audit unavailable"}
      </span>
      <small>
        {maturity.scored_dimension_count
          ? `${maturity.scored_dimension_count} dimensions · `
          : ""}
        {maturity.audit_id ?? "No audit run"} · {maturity.freshness}
      </small>
      {(maturity.gaps ?? []).length > 0 && (
        <ul className="quality-maturity-gaps" aria-label="Maturity gaps">
          {(maturity.gaps ?? []).slice(0, compact ? 2 : 4).map((gap) => (
            <li key={`${gap.dimension}-${gap.status}`}>
              <strong>{gap.dimension.replaceAll("_", " ")}</strong>
              <span>
                {gap.score === undefined ? "unknown" : `${gap.score}/4`} ·{" "}
                {gap.message}
              </span>
            </li>
          ))}
        </ul>
      )}
      <QualityReadinessSummary readiness={readiness} compact={compact} />
      {maturity.report_path && onOpenReport && (
        <button
          className="quality-report-link"
          type="button"
          onClick={() => onOpenReport(maturity.report_path as string)}
        >
          <FileSearch size={12} />
          Audit finding
        </button>
      )}
    </div>
  );
}

export interface QualityAttentionItem {
  kind: "gate" | "findings";
  label: string;
  detail: string;
  gate?: QualityGate;
}

export function qualityAttentionItems(
  repository: RepositorySnapshot,
): QualityAttentionItem[] {
  const requiredGateIds = new Set(
    (repository.release_rule?.required_quality_gates ?? []).map(
      (requirement) => requirement.gate_id,
    ),
  );
  const configuredGateIds = new Set(
    repository.quality.ci_readiness.configured_gate_ids,
  );
  const items: QualityAttentionItem[] = [];
  for (const gate of repository.quality.gates) {
    const required = requiredGateIds.has(gate.id);
    const configuredWithoutEvidence =
      configuredGateIds.has(gate.id) && gate.status === "Not configured";
    const needsAttention =
      gate.status === "Failed" ||
      gate.status === "Blocked" ||
      gate.freshness === "Stale" ||
      gate.freshness === "Conflicted" ||
      (required && gate.status === "Not configured");
    if (needsAttention) {
      items.push({
        kind: "gate",
        label: `${gate.label}${required ? " · release required" : ""}`,
        detail: configuredWithoutEvidence
          ? "Configured · no evidence"
          : `${gate.status} · ${gate.freshness}`,
        gate,
      });
    }
  }
  if (repository.quality.findings.high_severity_total > 0) {
    items.push({
      kind: "findings",
      label: "High-severity QR findings",
      detail: `${repository.quality.findings.high_severity_total} critical or high finding${repository.quality.findings.high_severity_total === 1 ? "" : "s"}`,
    });
  }
  return items;
}

export function QualityAttentionList({
  repository,
  onOpenRepository,
  onOpenReport,
}: {
  repository: RepositorySnapshot;
  onOpenRepository: () => void;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  return (
    <div className="quality-attention-list">
      {qualityAttentionItems(repository).map((item) => (
        <div
          className="quality-attention-item"
          key={`${item.kind}-${item.label}`}
        >
          <button
            className="quality-attention-main"
            type="button"
            onClick={onOpenRepository}
          >
            <ShieldAlert size={14} />
            <span>
              <strong>{item.label}</strong>
              <small>{item.detail}</small>
            </span>
          </button>
          {item.gate && (
            <QualityGateStatusPill
              status={item.gate.status}
              freshness={item.gate.freshness}
            />
          )}
          {item.gate?.evidence[0] && (
            <EvidenceAction
              evidence={item.gate.evidence[0]}
              onOpenReport={onOpenReport}
            />
          )}
          {item.kind === "findings" &&
            repository.quality.findings.report_path && (
              <button
                className="quality-report-link"
                type="button"
                onClick={() =>
                  onOpenReport?.(
                    repository.quality.findings.report_path as string,
                  )
                }
              >
                <FileSearch size={12} />
                Report
              </button>
            )}
        </div>
      ))}
    </div>
  );
}
