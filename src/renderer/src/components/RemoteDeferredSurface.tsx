import { GitBranch } from "lucide-react";
import type { ReactElement } from "react";
import { DeferredSurface } from "./WorkspaceSurfaces";

export function RemoteDeferredSurface(): ReactElement {
  return (
    <DeferredSurface
      eyebrow="Accepted provider decision"
      title="Read-only GitHub comes later."
      body="Remote context will be additive and read-only at first. No credentials, network refresh, pull request mutation, or release publishing is active in this local slice."
      icon={<GitBranch size={19} />}
      details={[
        { label: "Permission", value: "Read-only" },
        { label: "Prerequisite", value: "Read-only provider contract" },
      ]}
    />
  );
}
