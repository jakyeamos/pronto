// @vitest-environment happy-dom
// quality-gate: allow static-ui-test: verifies the manifest's accessibility identifiers and labels are rendered by the shared surfaces.
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import manifest from "../../../.mac-control/ideal-state.json";
import { AppSidebar } from "./components/AppSidebar";
import { RemediationRunOverview } from "./components/RemediationRunOverview";
import { MAC_CONTROL_TARGET_IDS } from "./macControlTargets";
import type { RemediationRun } from "./types";

afterEach(cleanup);

const remediationRun: RemediationRun = {
  schema_version: "pronto-remediation/v3",
  id: "mac-control-target-test",
  generated_at: "2026-08-09T00:00:00Z",
  source_refresh_id: null,
  status: "completed",
  message: null,
  eligible_repository_ids: [],
  eligible_repository_paths: [],
  refresh_steps: [],
  excluded_repositories: [],
  closures: [],
  plans: [],
};

describe("Mac Control ideal-state targets", () => {
  it("keeps every declared manifest identifier unique and rendered with its label", () => {
    const tasks = manifest.tasks;

    expect(tasks).toHaveLength(4);
    expect(tasks.map((task) => task.accessibility.identifier)).toEqual([
      MAC_CONTROL_TARGET_IDS.portfolio,
      MAC_CONTROL_TARGET_IDS.remediation,
      MAC_CONTROL_TARGET_IDS.refresh,
      MAC_CONTROL_TARGET_IDS.settings,
    ]);
    expect(
      new Set(tasks.map((task) => task.accessibility.identifier)).size,
    ).toBe(tasks.length);

    render(
      <>
        <AppSidebar
          activeNav="portfolio"
          activeConditionCount={0}
          repositories={[]}
          remediation={remediationRun}
          selectedRepositoryId={null}
          onNavigate={() => undefined}
          onOpenRepository={() => undefined}
        />
        <RemediationRunOverview
          run={remediationRun}
          isRefreshing={false}
          onRefresh={async () => undefined}
          onExport={async () => undefined}
        />
      </>,
    );

    for (const task of tasks) {
      const target = document.getElementById(task.accessibility.identifier);
      expect(target).not.toBeNull();
      expect(target?.tagName).toBe("BUTTON");
      expect(target?.textContent).toContain(task.accessibility.label);
      expect(
        screen.getByRole("button", { name: task.accessibility.label }),
      ).toBe(target);
    }
  });
});
