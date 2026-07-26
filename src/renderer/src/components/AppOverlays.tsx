import type { ReactElement } from "react";
import { DetailDrawer, EvidenceDrawer } from "./Drawers";
import { PreparationDrawer } from "./PreparationDrawer";
import type {
  Condition,
  ExternalTool,
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
  onLifecycleChange: (lifecycle: string) => Promise<void>;
  onCondition: (condition: Condition) => void;
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
          onCondition={onCondition}
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
        />
      )}
    </>
  );
}
