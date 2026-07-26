import { useCallback } from "react";
import type { Dispatch, SetStateAction } from "react";
import * as api from "../api";
import type {
  AiPayloadPreview,
  PortfolioSnapshot,
  ReleaseRuleConfig,
  RepositoryPreparation,
  RepositorySnapshot,
} from "../types";

type SelectedPreparation = {
  repository: RepositorySnapshot;
  preparation: RepositoryPreparation;
};

export function usePreparationActions({
  selectedRepository,
  selectedPreparation,
  loadSnapshot,
  setSnapshot,
  setSelectedRepository,
  setSelectedPreparation,
  setError,
}: {
  selectedRepository: RepositorySnapshot | null;
  selectedPreparation: SelectedPreparation | null;
  loadSnapshot: (operation: () => Promise<PortfolioSnapshot>) => Promise<void>;
  setSnapshot: Dispatch<SetStateAction<PortfolioSnapshot>>;
  setSelectedRepository: Dispatch<SetStateAction<RepositorySnapshot | null>>;
  setSelectedPreparation: Dispatch<SetStateAction<SelectedPreparation | null>>;
  setError: Dispatch<SetStateAction<string | null>>;
}): {
  handleSaveReleaseRule: (
    releaseRule: ReleaseRuleConfig | null,
  ) => Promise<void>;
  handleSaveAiPermission: (permission: string) => Promise<void>;
  handlePreviewAiSummary: () => Promise<AiPayloadPreview>;
} {
  const handleSaveReleaseRule = useCallback(
    async (releaseRule: ReleaseRuleConfig | null): Promise<void> => {
      if (!selectedRepository) return;
      await loadSnapshot(() =>
        api.setReleaseRule(selectedRepository.id, releaseRule),
      );
      setSelectedPreparation(null);
      setSelectedRepository(null);
    },
    [
      loadSnapshot,
      selectedRepository,
      setSelectedPreparation,
      setSelectedRepository,
    ],
  );

  const handleSaveAiPermission = useCallback(
    async (permission: string): Promise<void> => {
      if (!selectedRepository) return;
      try {
        const nextSnapshot = await api.setAiPermission(
          selectedRepository.id,
          permission,
        );
        setSnapshot(nextSnapshot);
        const nextRepository =
          nextSnapshot.repositories.find(
            (repository) => repository.id === selectedRepository.id,
          ) ?? null;
        setSelectedRepository(nextRepository);
        setSelectedPreparation((current) =>
          current && nextRepository
            ? { ...current, repository: nextRepository }
            : current,
        );
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not save the AI permission.",
        );
        throw caught;
      }
    },
    [
      selectedRepository,
      setError,
      setSelectedPreparation,
      setSelectedRepository,
      setSnapshot,
    ],
  );

  const handlePreviewAiSummary =
    useCallback(async (): Promise<AiPayloadPreview> => {
      if (!selectedRepository || !selectedPreparation) {
        throw new Error(
          "Select a repository preparation before previewing AI.",
        );
      }
      try {
        return await api.previewAiSummary(
          selectedRepository.id,
          selectedPreparation.preparation.pull_request.workspace_id,
        );
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not build the AI payload preview.",
        );
        throw caught;
      }
    }, [selectedPreparation, selectedRepository, setError]);

  return {
    handleSaveReleaseRule,
    handleSaveAiPermission,
    handlePreviewAiSummary,
  };
}
