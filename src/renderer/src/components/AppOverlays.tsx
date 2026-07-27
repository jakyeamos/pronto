import type { ReactElement } from "react";
import { EvidenceDrawer } from "./Drawers";
import { PreparationDrawer } from "./PreparationDrawer";
import type {
  AiPayloadPreview,
  Condition,
  ReleaseRecipeConfig,
  ReleaseRuleConfig,
  RepositoryPreparation,
  RepositorySnapshot,
} from "../types";

type SelectedEvidence = {
  repository: RepositorySnapshot;
  condition: Condition;
};

type SelectedPreparation = {
  repository: RepositorySnapshot;
  preparation: RepositoryPreparation;
};

export function AppOverlays({
  selectedEvidence,
  selectedPreparation,
  onSaveReleaseRule,
  onSaveReleaseRecipe,
  onConfirmReleaseVersion,
  onSaveAiPermission,
  onPreviewAiSummary,
  onCloseEvidence,
  onExpected,
  onClosePreparation,
}: {
  selectedEvidence: SelectedEvidence | null;
  selectedPreparation: SelectedPreparation | null;
  onSaveReleaseRule: (rule: ReleaseRuleConfig | null) => Promise<void>;
  onSaveReleaseRecipe: (recipe: ReleaseRecipeConfig | null) => Promise<void>;
  onConfirmReleaseVersion: (version: string | null) => Promise<void>;
  onSaveAiPermission: (permission: string) => Promise<void>;
  onPreviewAiSummary: () => Promise<AiPayloadPreview>;
  onCloseEvidence: () => void;
  onExpected: () => void;
  onClosePreparation: () => void;
}): ReactElement {
  return (
    <>
      {selectedEvidence && (
        <EvidenceDrawer
          repository={selectedEvidence.repository}
          condition={selectedEvidence.condition}
          onClose={onCloseEvidence}
          onExpected={onExpected}
        />
      )}
      {selectedPreparation && (
        <PreparationDrawer
          repository={selectedPreparation.repository}
          preparation={selectedPreparation.preparation}
          onClose={onClosePreparation}
          onSaveReleaseRule={onSaveReleaseRule}
          onSaveReleaseRecipe={onSaveReleaseRecipe}
          onConfirmReleaseVersion={onConfirmReleaseVersion}
          onSaveAiPermission={onSaveAiPermission}
          onPreviewAiSummary={onPreviewAiSummary}
        />
      )}
    </>
  );
}
