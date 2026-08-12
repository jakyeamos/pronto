import type { RepositorySnapshot } from "./types";

export type BranchEvidenceStatus =
  | "verified"
  | "branch_mismatch"
  | "commit_mismatch"
  | "incomplete"
  | "unscoped";

export type TargetEvidenceState =
  "verified" | "stale" | "unscoped" | "unavailable";

export interface BranchEvidenceScope {
  targetBranch?: string | null;
  targetCommit?: string | null;
  scannedBranch?: string | null;
  scannedCommit?: string | null;
}

export interface BranchEvidenceRecord {
  scanned_branch?: string | null;
  scanned_commit?: string | null;
  scannedBranch?: string | null;
  scannedCommit?: string | null;
}

export interface TargetEvidenceProjection<T> {
  evidence: T[];
  state: TargetEvidenceState;
}

function normalized(value?: string | null): string | undefined {
  const trimmed = value?.trim();
  return trimmed || undefined;
}

export function hasTargetScope(
  targetBranch?: string | null,
  targetCommit?: string | null,
): boolean {
  return Boolean(normalized(targetBranch) || normalized(targetCommit));
}

export function branchEvidenceStatus({
  targetBranch,
  targetCommit,
  scannedBranch,
  scannedCommit,
}: BranchEvidenceScope): BranchEvidenceStatus {
  const normalizedTargetBranch = normalized(targetBranch);
  const normalizedTargetCommit = normalized(targetCommit);
  const normalizedScannedBranch = normalized(scannedBranch);
  const normalizedScannedCommit = normalized(scannedCommit);

  if (!normalizedTargetBranch && !normalizedTargetCommit) return "unscoped";
  if (
    !normalizedTargetBranch ||
    !normalizedTargetCommit ||
    !normalizedScannedBranch ||
    !normalizedScannedCommit
  ) {
    return "incomplete";
  }
  if (normalizedScannedBranch !== normalizedTargetBranch) {
    return "branch_mismatch";
  }
  if (normalizedScannedCommit !== normalizedTargetCommit) {
    return "commit_mismatch";
  }
  return "verified";
}

export function targetEvidenceState(
  scope: BranchEvidenceScope,
): TargetEvidenceState {
  const status = branchEvidenceStatus(scope);
  if (status === "verified") return "verified";
  if (status === "commit_mismatch") return "stale";
  if (status === "unscoped") return "unscoped";
  if (
    status === "incomplete" &&
    !normalized(scope.scannedBranch) &&
    !normalized(scope.scannedCommit)
  ) {
    return "unscoped";
  }
  return "unavailable";
}

export function combineTargetEvidenceStates(
  states: readonly TargetEvidenceState[],
): TargetEvidenceState {
  if (states.includes("unavailable")) return "unavailable";
  if (states.includes("stale")) return "stale";
  if (states.includes("unscoped")) return "unscoped";
  return "verified";
}

function isTargetEvidence(scope: BranchEvidenceScope): boolean {
  return (
    branchEvidenceStatus(scope) === "verified" ||
    !hasTargetScope(scope.targetBranch, scope.targetCommit)
  );
}

function filterTargetEvidence<T extends BranchEvidenceRecord>(
  evidence: readonly T[],
  targetBranch?: string | null,
  targetCommit?: string | null,
): T[] {
  if (!hasTargetScope(targetBranch, targetCommit)) return [...evidence];
  return evidence.filter((item) =>
    isTargetEvidence({
      targetBranch,
      targetCommit,
      scannedBranch: item.scannedBranch ?? item.scanned_branch,
      scannedCommit: item.scannedCommit ?? item.scanned_commit,
    }),
  );
}

export function projectEvidenceForTarget<T extends BranchEvidenceRecord>(
  evidence: readonly T[],
  targetBranch?: string | null,
  targetCommit?: string | null,
): TargetEvidenceProjection<T> {
  if (!hasTargetScope(targetBranch, targetCommit)) {
    return { evidence: [...evidence], state: "unscoped" };
  }

  const exactEvidence = filterTargetEvidence(
    evidence,
    targetBranch,
    targetCommit,
  );
  if (exactEvidence.length > 0) {
    return { evidence: exactEvidence, state: "verified" };
  }

  const staleEvidence = evidence.filter(
    (item) =>
      targetEvidenceState({
        targetBranch,
        targetCommit,
        scannedBranch: item.scannedBranch ?? item.scanned_branch,
        scannedCommit: item.scannedCommit ?? item.scanned_commit,
      }) === "stale",
  );
  if (staleEvidence.length > 0) {
    return { evidence: staleEvidence, state: "stale" };
  }

  const unscopedEvidence = evidence.filter(
    (item) =>
      targetEvidenceState({
        targetBranch,
        targetCommit,
        scannedBranch: item.scannedBranch ?? item.scanned_branch,
        scannedCommit: item.scannedCommit ?? item.scanned_commit,
      }) === "unscoped",
  );
  if (unscopedEvidence.length > 0) {
    return { evidence: unscopedEvidence, state: "unscoped" };
  }

  return { evidence: [], state: "unavailable" };
}

export function targetEvidenceMessage({
  targetBranch,
  targetCommit,
  scannedBranch,
  scannedCommit,
}: BranchEvidenceScope): string {
  const normalizedTargetBranch = normalized(targetBranch);
  const normalizedTargetCommit = normalized(targetCommit);
  const normalizedScannedBranch = normalized(scannedBranch);
  const normalizedScannedCommit = normalized(scannedCommit);
  const shortTargetCommit = normalizedTargetCommit?.slice(0, 8);
  const shortScannedCommit = normalizedScannedCommit?.slice(0, 8);

  if (
    branchEvidenceStatus({
      targetBranch,
      targetCommit,
      scannedBranch,
      scannedCommit,
    }) === "verified"
  ) {
    return `Verified for target ${normalizedTargetBranch} @ ${shortTargetCommit}`;
  }
  if (
    normalizedTargetBranch &&
    normalizedScannedBranch &&
    normalizedScannedBranch !== normalizedTargetBranch
  ) {
    return `Target ${normalizedTargetBranch} is not verified; this scan is from ${normalizedScannedBranch}.`;
  }
  if (
    normalizedTargetBranch &&
    normalizedTargetCommit &&
    normalizedScannedCommit &&
    normalizedScannedBranch === normalizedTargetBranch &&
    normalizedScannedCommit !== normalizedTargetCommit
  ) {
    return `Target ${normalizedTargetBranch} is not verified; this scan is ${shortScannedCommit}, target is ${shortTargetCommit}.`;
  }
  return normalizedTargetBranch
    ? `Target ${normalizedTargetBranch} is not verified; branch/commit provenance is incomplete.`
    : "Target branch is unavailable; evidence scope is not verified.";
}

export function targetEvidenceDescriptor({
  scannedBranch,
  scannedCommit,
}: Pick<BranchEvidenceScope, "scannedBranch" | "scannedCommit">): string {
  const branch = normalized(scannedBranch);
  const commit = normalized(scannedCommit)?.slice(0, 8);
  return [
    branch ? `branch ${branch}` : undefined,
    commit ? `commit ${commit}` : undefined,
  ]
    .filter((value): value is string => Boolean(value))
    .join(" · ");
}

export function targetScopeForRepository(repository: RepositorySnapshot): {
  branch?: string;
  commit?: string;
} {
  const branch = normalized(
    repository.target_branch ?? repository.default_branch,
  );
  if (!branch) return {};
  const branchSummary = repository.branches.find(
    (candidate) => candidate.name === branch,
  );
  const commit = normalized(
    branchSummary?.last_commit ??
      (repository.branch === branch
        ? repository.workspace.last_commit
        : undefined),
  );
  return { branch, commit };
}
