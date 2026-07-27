import { useCallback } from "react";
import type { Dispatch, SetStateAction } from "react";
import * as api from "../api";
import type {
  Condition,
  PortfolioSnapshot,
  RepositorySnapshot,
} from "../types";

type SelectedEvidence = {
  repository: RepositorySnapshot;
  condition: Condition;
};

export function useEvidenceActions({
  selectedEvidence,
  loadSnapshot,
  setSelectedRepositoryId,
  setSelectedEvidence,
}: {
  selectedEvidence: SelectedEvidence | null;
  loadSnapshot: (operation: () => Promise<PortfolioSnapshot>) => Promise<void>;
  setSelectedRepositoryId: Dispatch<SetStateAction<string | null>>;
  setSelectedEvidence: Dispatch<SetStateAction<SelectedEvidence | null>>;
}): {
  handleCondition: (
    repository: RepositorySnapshot,
    condition: Condition,
  ) => void;
  handleExpected: () => Promise<void>;
} {
  const handleCondition = useCallback(
    (repository: RepositorySnapshot, condition: Condition): void => {
      setSelectedRepositoryId(repository.id);
      setSelectedEvidence({ repository, condition });
    },
    [setSelectedEvidence, setSelectedRepositoryId],
  );

  const handleExpected = useCallback(async (): Promise<void> => {
    if (!selectedEvidence) return;
    const { repository, condition } = selectedEvidence;
    await loadSnapshot(() =>
      condition.status === "Expected"
        ? api.clearExpected(repository.id, condition.id)
        : api.markExpected(repository.id, condition.id),
    );
    setSelectedEvidence(null);
  }, [loadSnapshot, selectedEvidence, setSelectedEvidence]);

  return { handleCondition, handleExpected };
}
