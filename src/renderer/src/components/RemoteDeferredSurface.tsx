import { GitBranch } from "lucide-react";
import type { ReactElement } from "react";
import { DeferredSurface } from "./WorkspaceSurfaces";

export function RemoteDeferredSurface(): ReactElement {
  return (
    <DeferredSurface
      eyebrow="Accepted provider decision"
      title="GitHub context is ready with one guarded action."
      body="Pronto can refresh GitHub and change repository visibility after fresh administration preflight, exact confirmation, and provider readback. Pull request mutation, merge, release publishing, and credential management remain deferred."
      icon={<GitBranch size={19} />}
      details={[
        { label: "Permission", value: "Approval-gated visibility" },
        { label: "Prerequisite", value: "Fresh GitHub admin permission" },
      ]}
    />
  );
}
