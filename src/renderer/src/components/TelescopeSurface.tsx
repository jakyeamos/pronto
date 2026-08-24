import { ReactFlowProvider } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { ReactElement } from "react";
import type { TelescopeWorkspaceProps } from "./telescopeSurfaceTypes";
import { TelescopeWorkspaceView } from "./TelescopeWorkspaceView";
import { useTelescopeWorkspaceModel } from "./useTelescopeWorkspaceModel";

export function TelescopeSurface(props: TelescopeWorkspaceProps): ReactElement {
  return (
    <ReactFlowProvider>
      <TelescopeWorkspace {...props} />
    </ReactFlowProvider>
  );
}

function TelescopeWorkspace(props: TelescopeWorkspaceProps): ReactElement {
  const model = useTelescopeWorkspaceModel(props);
  return <TelescopeWorkspaceView {...props} model={model} />;
}
