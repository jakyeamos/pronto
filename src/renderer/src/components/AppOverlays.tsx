import type { ReactElement } from "react";
import { DetailDrawer, EvidenceDrawer } from "./Drawers";
import { PreparationDrawer } from "./PreparationDrawer";
import type {
  AiPayloadPreview,
  Condition,
  ExternalTool,
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
  selectedRepository,
  selectedEvidence,
  selectedPreparation,
  onCloseRepository,
  onOpenWorkspace,
  onPrepareRepository,
  onSaveReleaseRule,
  onSaveReleaseRecipe,
  onConfirmReleaseVersion,
  onSaveAiPermission,
  onPreviewAiSummary,
  onLifecycleChange,
  onCondition,
  onCloseEvidence,
  onExpected,
  onClosePreparation,
}: {
  selectedRepository: RepositorySnapshot | null;
  selectedEvidence: SelectedEvidence | null;
  selectedPreparation: SelectedPreparation | null;
  onCloseRepository: () => void;
  onOpenWorkspace: (workspaceId: string, tool: ExternalTool) => Promise<void>;
  onPrepareRepository: (workspaceId?: string) => Promise<void>;
  onSaveReleaseRule: (rule: ReleaseRuleConfig | null) => Promise<void>;
  onSaveReleaseRecipe: (recipe: ReleaseRecipeConfig | null) => Promise<void>;
  onConfirmReleaseVersion: (version: string | null) => Promise<void>;
  onSaveAiPermission: (permission: string) => Promise<void>;
  onPreviewAiSummary: () => Promise<AiPayloadPreview>;
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (repository: RepositorySnapshot, condition: Condition) => void;
  onCloseEvidence: () => void;
  onExpected: () => void;
  onClosePreparation: () => void;
}): ReactElement {
  return (
    <>
      {selectedRepository && (
        <DetailDrawer
          repository={selectedRepository}
          onClose={onCloseRepository}
          onOpenWorkspace={onOpenWorkspace}
          onPrepareRepository={onPrepareRepository}
          onLifecycleChange={onLifecycleChange}
          onCondition={(condition) => {
            if (!selectedRepository) return;
            onCondition(selectedRepository, condition);
          }}
        />
      )}
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
