import type { ReactElement } from "react";
import { FileSearch, ShieldAlert } from "lucide-react";
import type { QualityGate, RepositorySnapshot } from "../types";
import {
  branchEvidenceStatus,
  hasTargetScope,
  targetEvidenceState,
  targetScopeForRepository,
  type TargetEvidenceState,
} from "../branchEvidence";
import { projectQualityGateForTarget } from "./QualityProjection";
import { EvidenceAction, QualityGateStatusPill } from "./QualityComponents";
import { StatusPill } from "./ConsolePrimitives";
export interface QualityAttentionItem {
  kind: "gate" | "findings" | "contract";
  label: string;
  detail: string;
  staleOnly: boolean;
  targetVerified: boolean;
  targetEvidenceState: TargetEvidenceState;
  gate?: QualityGate;
}

export function qualityAttentionItems(
  repository: RepositorySnapshot,
): QualityAttentionItem[] {
  const requiredGateIds = new Set(
    (repository.release_rule?.required_quality_gates ?? []).map(
      (requirement) =>
        (requirement.policy ?? "block") === "block" ? requirement.gate_id : "",
    ),
  );
  requiredGateIds.delete("");
  const configuredGateIds = new Set(
    repository.quality.ci_readiness.configured_gate_ids,
  );
  const target = targetScopeForRepository(repository);
  const targetMode = hasTargetScope(target.branch, target.commit);
  const items: QualityAttentionItem[] = [];
  for (const contract of repository.quality.evidence_contracts ?? []) {
    if (contract.status === "current") continue;
    items.push({
      kind: "contract",
      label: `${contract.label} · re-audit required`,
      detail: contract.observed_schema
        ? `${contract.observed_schema} observed · ${contract.target_schema} required`
        : `No contract schema observed · ${contract.target_schema} required`,
      staleOnly: false,
      targetVerified: false,
      targetEvidenceState: "unavailable",
    });
  }
  for (const gate of repository.quality.gates) {
    const projection = projectQualityGateForTarget(
      gate,
      target.branch,
      target.commit,
    );
    const displayedGate = projection.gate;
    const required = requiredGateIds.has(gate.id);
    const configuredWithoutEvidence =
      configuredGateIds.has(gate.id) && gate.status === "Not configured";
    const rawNeedsAttention =
      gate.status === "Failed" ||
      gate.status === "Blocked" ||
      gate.freshness === "Stale" ||
      gate.freshness === "Conflicted" ||
      (required && gate.status === "Not configured");
    const targetEvidenceUnavailable =
      targetMode &&
      projection.state === "unavailable" &&
      gate.evidence.length > 0;
    const targetEvidenceNeedsReview =
      targetMode &&
      (projection.state === "stale" || projection.state === "unscoped");
    const needsAttention =
      rawNeedsAttention ||
      targetEvidenceUnavailable ||
      targetEvidenceNeedsReview;
    if (needsAttention) {
      const staleOnly =
        !targetEvidenceUnavailable &&
        (gate.freshness === "Stale" || projection.state === "stale") &&
        gate.status !== "Failed" &&
        gate.status !== "Blocked" &&
        !(required && gate.status === "Not configured");
      items.push({
        kind: "gate",
        label: `${gate.label}${required ? " · release required" : ""}`,
        detail: targetEvidenceUnavailable
          ? "Target evidence unavailable · raw evidence is not a target result"
          : projection.state === "stale"
            ? "Stale branch evidence · scan predates the selected target head"
            : projection.state === "unscoped"
              ? "Unscoped evidence · branch/head provenance is not recorded"
              : configuredWithoutEvidence
                ? "Configured · no evidence"
                : `${gate.status} · ${gate.freshness}`,
        staleOnly,
        targetVerified: projection.verified,
        targetEvidenceState: projection.state,
        gate: displayedGate,
      });
    }
  }
  const findingsTargetVerified =
    !targetMode ||
    branchEvidenceStatus({
      targetBranch: target.branch,
      targetCommit: target.commit,
      scannedBranch: repository.quality.findings.scanned_branch,
      scannedCommit: repository.quality.findings.scanned_commit,
    }) === "verified";
  const findingsEvidenceState = targetMode
    ? targetEvidenceState({
        targetBranch: target.branch,
        targetCommit: target.commit,
        scannedBranch: repository.quality.findings.scanned_branch,
        scannedCommit: repository.quality.findings.scanned_commit,
      })
    : "verified";
  if (repository.quality.findings.high_severity_total > 0) {
    const findingsLabel = findingsTargetVerified
      ? "High-severity QR findings"
      : findingsEvidenceState === "stale"
        ? "High-severity QR findings · stale branch evidence"
        : findingsEvidenceState === "unscoped"
          ? "High-severity QR findings · unscoped evidence"
          : "High-severity QR findings · target unverified";
    const findingsDetail = findingsTargetVerified
      ? `${repository.quality.findings.high_severity_total} critical or high finding${repository.quality.findings.high_severity_total === 1 ? "" : "s"}`
      : findingsEvidenceState === "stale"
        ? "Stale branch evidence · scan predates the selected target head"
        : findingsEvidenceState === "unscoped"
          ? "Unscoped evidence · branch/head provenance is not recorded"
          : "Target evidence unavailable · raw findings are not a target result";
    items.push({
      kind: "findings",
      label: findingsLabel,
      detail: findingsDetail,
      staleOnly: false,
      targetVerified: findingsTargetVerified,
      targetEvidenceState: findingsEvidenceState,
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
          {item.gate && item.targetVerified && (
            <QualityGateStatusPill
              status={item.gate.status}
              freshness={item.gate.freshness}
            />
          )}
          {item.gate && !item.targetVerified && (
            <StatusPill tone="amber">
              {item.targetEvidenceState === "stale"
                ? "Stale branch evidence"
                : item.targetEvidenceState === "unscoped"
                  ? "Unscoped evidence"
                  : "Target unverified"}
            </StatusPill>
          )}
          {item.gate?.evidence[0] &&
            (item.targetVerified ||
              item.targetEvidenceState === "stale" ||
              item.targetEvidenceState === "unscoped") && (
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
