// @vitest-environment happy-dom
import { cleanup } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type {
  Condition,
  ProductConfig,
  RemediationRun,
} from "./QualityComponents.test-support";
import {
  workspace,
  makeGate,
  makeQuality,
  makeRepository,
  makePortfolio,
  noop,
  noopRepository,
  AppSidebar,
  AttentionQueue,
  PortfolioCollectionsSurface,
} from "./QualityComponents.test-support";
afterEach(cleanup);
// quality-gate: allow static-ui-test: verifies the read-only evidence contract and release-source copy
describe("portfolio navigation surfaces", () => {
  it("nests release products under the Groups destination", () => {
    const repository = makeRepository();
    const product: ProductConfig = {
      id: "product-1",
      name: "Pronto",
      repository_ids: [repository.id],
      release_mode: "Independent",
      created_at: "2026-07-26T11:00:00Z",
      updated_at: "2026-07-26T11:00:00Z",
    };
    const markup = renderToStaticMarkup(
      <PortfolioCollectionsSurface
        groups={[]}
        products={[product]}
        repositories={[repository]}
        onSaveGroup={noop}
        onDeleteGroup={noop}
        onSaveProduct={noop}
        onDeleteProduct={noop}
      />,
    );
    expect(markup).toContain("Groups");
    expect(markup).toContain("Release products");
    expect(markup).toContain('class="surface-panel collection-subsection"');
  });

  it("starts the Attention queue and repository groups collapsed", () => {
    const repository = makeRepository({
      quality: makeQuality({
        gates: [makeGate("build", "Build", "Failed", "Fresh")],
      }),
    });
    const markup = renderToStaticMarkup(
      <AttentionQueue
        repositories={[repository]}
      />,
    );
    expect(markup).toContain("Attention queue");
    expect(markup).toContain('class="rail-section attention-queue"');
    expect(markup).not.toContain('class="rail-section attention-queue" open');
    expect(markup).not.toContain(
      'class="attention-group quality-attention-group" open',
    );
  });

  it("keeps the sidebar repository index searchable and status-only", () => {
    const repository = makeRepository({ name: "Local project" });
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={[repository]}
        remediation={makePortfolio([repository]).remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );
    expect(markup).toContain("Repositories");
    expect(markup).toContain("Find a repository");
    expect(markup).toContain("Local project");
    expect(markup).toContain(
      'aria-label="Open repository Local project, item 1"',
    );
    expect(markup).not.toContain("/tmp/pronto");
    expect(markup).not.toContain("main");
    expect(markup).not.toContain("Quality gates");
    expect(markup).not.toContain('class="brand"');
    expect(markup).not.toContain("Portfolio command center");
    expect(markup).not.toContain("sidebar-rule");
    expect(markup).not.toContain("Local evidence only");
    expect(markup).not.toContain("Private by default");
  });

  it("keeps remediation-excluded repositories out of the sidebar", () => {
    const eligible = makeRepository({
      id: "eligible-repository",
      name: "Eligible project",
      path: "/tmp/eligible-project",
    });
    const excluded = makeRepository({
      id: "excluded-repository",
      name: "Excluded project",
      path: "/tmp/excluded-project",
    });
    const remediation = makePortfolio([eligible, excluded]).remediation;
    remediation.excluded_repositories = [
      {
        repository_id: excluded.id,
        repository_name: excluded.name,
        repository_path: excluded.path,
        reason: "Currently in progress; excluded from this refresh.",
      },
    ];

    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={[eligible, excluded]}
        remediation={remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup).toContain("1 eligible local");
    expect(markup).toContain("Eligible project");
    expect(markup).not.toContain("Excluded project");
  });

  it("shows stale-only repositories in blue and preserves red precedence", () => {
    const staleQuality = makeQuality({
      gates: [makeGate("build", "Build", "Passed", "Stale")],
    });
    const staleCondition: Condition = {
      id: "condition-stale",
      kind: "remote-stale",
      title: "Remote state stale",
      summary: "Pronto has not recorded a successful fetch.",
      priority: 8,
      status: "Active",
      fingerprint: "remote-stale",
      rule: "Remote comparisons require a recorded fetch.",
      evidence: [],
      missing: [],
    };
    const repositories = [
      makeRepository({
        id: "quality-stale",
        name: "Quality stale only",
        quality: staleQuality,
      }),
      makeRepository({
        id: "remote-stale",
        name: "Remote stale only",
        conditions: [staleCondition],
      }),
      makeRepository({
        id: "stale-and-failed",
        name: "Stale and failed",
        quality: makeQuality({
          gates: [makeGate("build", "Build", "Failed", "Stale")],
        }),
      }),
      makeRepository({
        id: "stale-and-dirty",
        name: "Stale and dirty",
        quality: staleQuality,
        workspace: { ...workspace, dirty: true },
      }),
    ];
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={repositories}
        remediation={makePortfolio(repositories).remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup.match(/sidebar-repository-status-stale/g)).toHaveLength(2);
    expect(markup.match(/title="Stale evidence only"/g)).toHaveLength(2);
    expect(markup.match(/sidebar-repository-status-attention/g)).toHaveLength(
      2,
    );
    expect(markup.match(/title="Needs attention"/g)).toHaveLength(2);
  });

  it("shows integration eligibility in violet only when it is the sole signal", () => {
    const integrationCondition: Condition = {
      id: "condition-integration",
      kind: "integration-eligible",
      title: "Branch is ready to integrate",
      summary: "The branch is ahead of its target and the workspace is clean.",
      priority: 5,
      status: "Active",
      fingerprint: "integration-eligible",
      rule: "Clean branches ahead of their target are integration eligible.",
      evidence: [],
      missing: [],
    };
    const staleCondition: Condition = {
      id: "condition-stale",
      kind: "remote-stale",
      title: "Remote state stale",
      summary: "Pronto has not recorded a successful fetch.",
      priority: 8,
      status: "Active",
      fingerprint: "remote-stale",
      rule: "Remote comparisons require a recorded fetch.",
      evidence: [],
      missing: [],
    };
    const repositories = [
      makeRepository({
        id: "integration-only",
        name: "Integration only",
        conditions: [integrationCondition],
      }),
      makeRepository({
        id: "integration-and-stale",
        name: "Integration and stale",
        conditions: [integrationCondition, staleCondition],
      }),
      makeRepository({
        id: "integration-and-dirty",
        name: "Integration and dirty",
        conditions: [integrationCondition],
        workspace: { ...workspace, dirty: true },
      }),
      makeRepository({
        id: "integration-and-blocked",
        name: "Integration condition with broader blockers",
        conditions: [integrationCondition],
      }),
    ];
    const remediation = makePortfolio(repositories).remediation;
    remediation.plans = [
      {
        repository_id: "integration-only",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-stale",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-dirty",
        status: "open",
        integration_only_remaining: true,
      } as RemediationRun["plans"][number],
      {
        repository_id: "integration-and-blocked",
        status: "blocked",
        integration_only_remaining: false,
      } as RemediationRun["plans"][number],
    ];
    const markup = renderToStaticMarkup(
      <AppSidebar
        activeNav="portfolio"
        activeConditionCount={0}
        repositories={repositories}
        remediation={remediation}
        selectedRepositoryId={null}
        onNavigate={() => undefined}
        onOpenRepository={noopRepository}
      />,
    );

    expect(markup.match(/sidebar-repository-status-opportunity/g)).toHaveLength(
      1,
    );
    expect(
      markup.match(/title="Integration is the only remaining remediation"/g),
    ).toHaveLength(1);
    expect(markup.match(/sidebar-repository-status-attention/g)).toHaveLength(
      3,
    );
    expect(markup.match(/title="Needs attention"/g)).toHaveLength(3);
  });
});
