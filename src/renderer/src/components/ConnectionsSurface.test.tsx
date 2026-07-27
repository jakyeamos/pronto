// quality-gate: allow static-ui-test: verifies workflow ordering, explicit empty states, and redacted command evidence that build/typecheck cannot prove.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ConnectionsSurface, WorkflowInspector } from "./ConnectionsSurface";
import type {
  Connection,
  ConnectionNode,
  ConnectionsSnapshot,
  Workflow,
} from "../types";

const evidence = {
  adapter: "static-discovery",
  source_path: "package.json",
  detail: "package.json scripts.deploy",
  observed_at: "2026-07-26T12:00:00Z",
  freshness: "Fresh",
  command: "pnpm deploy --token=[REDACTED]",
};

const nodes: ConnectionNode[] = [
  {
    id: "connection-node:repository:one",
    kind: "repository",
    label: "one",
    identity: "repository:one",
    repository_id: "repo-one",
    origin: "Discovered",
    confidence: "High",
    status: "Active",
    evidence: [evidence],
  },
  {
    id: "connection-node:service:api",
    kind: "service",
    label: "API service",
    identity: "service:api",
    origin: "Manual",
    confidence: "Confirmed",
    status: "Active",
    evidence: [evidence],
  },
];

const connection: Connection = {
  id: "connection:handoff",
  fingerprint:
    "handoff:connection-node:repository:one->connection-node:service:api",
  source_node_id: nodes[0].id,
  target_node_id: nodes[1].id,
  relationship_type: "handoff",
  label: "publishes to API",
  origin: "Discovered",
  review_state: "Suggested",
  confidence: "High",
  status: "Active",
  evidence: [evidence],
};

const workflow: Workflow = {
  id: "workflow:deploy",
  name: "Deploy API",
  scope: "cross-repository",
  origin: "Discovered",
  status: "Active",
  review_state: "Suggested",
  participating_repositories: ["repo-one"],
  evidence: [evidence],
  steps: [
    {
      id: "step:0",
      order: 0,
      node_id: nodes[0].id,
      action_label: "Build package",
      command: "pnpm build",
      connection_id: connection.id,
      evidence: [evidence],
    },
    {
      id: "step:1",
      order: 1,
      node_id: nodes[1].id,
      action_label: "Publish API",
      command: "pnpm deploy --token=[REDACTED]",
      connection_id: connection.id,
      evidence: [evidence],
    },
  ],
};

const snapshot: ConnectionsSnapshot = {
  nodes,
  connections: [connection],
  workflows: [workflow],
  adapters: [
    {
      id: "static-discovery",
      enabled: true,
      freshness: "Fresh",
      permission_state: "Local read-only",
    },
  ],
  generated_at: "2026-07-26T12:00:00Z",
};

function renderSurface(connections: ConnectionsSnapshot): string {
  return renderToStaticMarkup(
    <ConnectionsSurface
      connections={connections}
      repositories={[]}
      isRefreshing={false}
      onRefresh={async () => undefined}
      onSaveNode={async () => undefined}
      onDeleteNode={async () => undefined}
      onSaveConnection={async () => undefined}
      onDeleteConnection={async () => undefined}
      onSaveWorkflow={async () => undefined}
      onDeleteWorkflow={async () => undefined}
      onReview={async () => undefined}
      onToggleAdapter={async () => undefined}
      onOpenRepository={() => undefined}
    />,
  );
}

describe("ConnectionsSurface", () => {
  it("renders the map, relationship legend, ordered workflow, and redacted command evidence", () => {
    const markup = renderSurface(snapshot);

    expect(markup).toContain("Evidence-backed connections map");
    expect(markup).toContain("Handoff");
    expect(markup).toContain("Deploy API");
    expect(markup.indexOf("Build package")).toBeLessThan(
      markup.indexOf("Publish API"),
    );
    expect(markup).toContain("2 ordered steps");
    expect(markup).toContain("Manual");

    const inspectorMarkup = renderToStaticMarkup(
      <WorkflowInspector
        workflow={workflow}
        nodeById={new Map(nodes.map((node) => [node.id, node]))}
        onReview={async () => undefined}
        onDelete={async () => undefined}
        renameValue={workflow.name}
        setRenameValue={() => undefined}
        onRename={async () => undefined}
      />,
    );
    expect(inspectorMarkup).toContain("pnpm deploy --token=[REDACTED]");
    expect(inspectorMarkup).toContain("Displayed only · never executed");
  });

  it("keeps the empty discovery state explicit", () => {
    const markup = renderSurface({
      ...snapshot,
      nodes: [],
      connections: [],
      workflows: [],
    });

    expect(markup).toContain("Refresh to discover relationships.");
    expect(markup).toContain(
      "Static discovery reads local repository configuration",
    );
  });
});
