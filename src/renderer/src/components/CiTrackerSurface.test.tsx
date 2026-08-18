// @vitest-environment happy-dom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  CiRunSnapshot,
  ProviderStatus,
  RemoteRepositorySnapshot,
} from "../types";
import { CiTrackerSurface } from "./CiTrackerSurface";

const providerStatus: ProviderStatus = {
  provider: "GitHub",
  state: "Ready",
  message: "Read-only GitHub context refreshed.",
  last_refresh_at: "2026-08-14T12:10:00Z",
  identity_count: 1,
  repository_count: 1,
};

function makeRun(overrides: Partial<CiRunSnapshot> = {}): CiRunSnapshot {
  return {
    id: 7001,
    workflow_name: "Quality",
    workflow_path: ".github/workflows/quality.yml",
    display_title: "Add tracker",
    run_number: 42,
    run_attempt: 1,
    event: "pull_request",
    status: "completed",
    conclusion: "failure",
    head_branch: "feature/tracker",
    head_sha: "abc123",
    html_url: "https://github.com/acme/project/actions/runs/7001",
    created_at: "2026-08-14T12:00:00Z",
    updated_at: "2026-08-14T12:05:00Z",
    pull_request_number: 17,
    is_fork: false,
    jobs: [
      {
        id: 9001,
        name: "macOS",
        status: "completed",
        conclusion: "failure",
        html_url: "https://github.com/acme/project/actions/runs/7001/job/9001",
        failed_steps: ["Run tests"],
      },
    ],
    failure_summary: "macOS: Run tests",
    failure_signature: "ci-abc",
    prompt_artifact: { id: 55, name: "codex-ci-prompt-7001-1", expired: false },
    last_refreshed_at: "2026-08-14T12:10:00Z",
    ...overrides,
  };
}

function makeRepository(
  overrides: Partial<RemoteRepositorySnapshot> = {},
): RemoteRepositorySnapshot {
  return {
    id: "github:42",
    provider: "github",
    full_name: "acme/project",
    name: "project",
    owner: "acme",
    html_url: "https://github.com/acme/project",
    default_branch: "main",
    archived: false,
    locality: "Local and remote",
    identity_id: "github:jakyeamos",
    last_refreshed_at: "2026-08-14T12:10:00Z",
    pull_requests: [],
    releases: [],
    ci_checks: [],
    ci_branch: "main",
    ci_commit: "abc123",
    ci_runs: [makeRun()],
    ...overrides,
  };
}

afterEach(() => cleanup());

describe("CI tracker surface", () => {
  it("explains a failed run and enables the handoff only with local evidence", async () => {
    const onStartCodex = vi.fn(async () => undefined);
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[makeRepository()]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onStartCodex={onStartCodex}
      />,
    );

    expect(screen.getByText("macOS: Run tests")).toBeTruthy();
    const button = screen.getByRole("button", { name: "Diagnose with Codex" });
    expect((button as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(button);
    await waitFor(() => expect(onStartCodex).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Started" })).toBeTruthy();
  });

  it("keeps successful runs out of the actionable list and disables missing-artifact handoffs", () => {
    const markup = renderToStaticMarkup(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[
          makeRepository({
            ci_runs: [
              makeRun({
                id: 7002,
                workflow_name: "Passing",
                conclusion: "success",
                failure_summary: undefined,
                prompt_artifact: undefined,
              }),
            ],
          }),
          makeRepository({
            id: "github:43",
            full_name: "acme/remote-only",
            locality: "GitHub only",
            ci_runs: [makeRun({ id: 7003 })],
          }),
          makeRepository({
            id: "github:44",
            full_name: "acme/missing-artifact",
            ci_runs: [makeRun({ id: 7004, prompt_artifact: undefined })],
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onStartCodex={async () => undefined}
      />,
    );

    expect(markup).not.toContain("Passing");
    expect(markup).toContain("Prompt artifact unavailable");
    expect(markup).toContain(
      "Remote-only repository · diagnosis stays in GitHub",
    );
    expect(markup).toContain("disabled");
  });
});
