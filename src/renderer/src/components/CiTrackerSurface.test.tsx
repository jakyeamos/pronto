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
    const onOpenExternalUrl = vi.fn(async () => undefined);
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[makeRepository()]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={onOpenExternalUrl}
        onStartCodex={onStartCodex}
      />,
    );

    expect(screen.getByText("macOS: Run tests")).toBeTruthy();
    const button = screen.getByRole("button", { name: "Diagnose with Codex" });
    expect((button as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(button);
    await waitFor(() => expect(onStartCodex).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Started" })).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "View run" }));
    await waitFor(() =>
      expect(onOpenExternalUrl).toHaveBeenCalledWith(
        "https://github.com/acme/project/actions/runs/7001",
      ),
    );
  });

  it("keeps successful runs out of the actionable list and reserves handoffs without local evidence", () => {
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
        onOpenExternalUrl={async () => undefined}
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

  it("still delegates a local diagnosis when the prompt artifact is missing", async () => {
    const onStartCodex = vi.fn(async () => undefined);
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[
          makeRepository({
            ci_runs: [makeRun({ prompt_artifact: undefined })],
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={async () => undefined}
        onStartCodex={onStartCodex}
      />,
    );

    const button = screen.getByRole("button", { name: "Diagnose with Codex" });
    expect((button as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(button);
    await waitFor(() => expect(onStartCodex).toHaveBeenCalledOnce());
  });

  it("keeps remote-only diagnosis disabled when its prompt artifact is missing", () => {
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[
          makeRepository({
            locality: "GitHub only",
            ci_runs: [makeRun({ prompt_artifact: undefined })],
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={async () => undefined}
        onStartCodex={async () => undefined}
      />,
    );

    expect(
      (
        screen.getByRole("button", {
          name: "Diagnose with Codex",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(
      screen.getByText("Remote-only repository · diagnosis stays in GitHub"),
    ).toBeTruthy();
    expect(
      screen.queryByText(
        "Prompt artifact unavailable · diagnosis will use fresh GitHub evidence",
      ),
    ).toBeNull();
  });

  it("collapses repositories while keeping their disclosure target mounted", () => {
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[makeRepository()]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={async () => undefined}
        onStartCodex={async () => undefined}
      />,
    );

    const toggle = screen.getByRole("button", {
      name: "Collapse acme/project",
    });
    const runList = document.getElementById("ci-tracker-runs-github-42");
    expect(runList).toBeTruthy();
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(runList?.hasAttribute("hidden")).toBe(false);

    fireEvent.click(toggle);

    expect(
      screen.getByRole("button", { name: "Expand acme/project" }),
    ).toBeTruthy();
    expect(runList?.hasAttribute("hidden")).toBe(true);
  });

  it("shows only the latest consecutive run for a repeated failure signature", () => {
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[
          makeRepository({
            ci_runs: [
              makeRun({
                id: 7001,
                run_number: 42,
                updated_at: "2026-08-14T12:05:00Z",
              }),
              makeRun({
                id: 7002,
                run_number: 43,
                updated_at: "2026-08-15T12:05:00Z",
              }),
            ],
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={async () => undefined}
        onStartCodex={async () => undefined}
      />,
    );

    expect(screen.getByText(/Run #43/)).toBeTruthy();
    expect(screen.queryByText(/Run #42/)).toBeNull();
    expect(screen.getByText(/Same issue as 1 earlier run/)).toBeTruthy();
    expect(screen.getByText("1 failure issue need review")).toBeTruthy();
  });

  it("collapses and expands every repository from the tracker heading", () => {
    render(
      <CiTrackerSurface
        status={providerStatus}
        repositories={[
          makeRepository(),
          makeRepository({
            id: "github:43",
            full_name: "acme/other-project",
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onOpenExternalUrl={async () => undefined}
        onStartCodex={async () => undefined}
      />,
    );

    const collapseAll = screen.getByRole("button", {
      name: "Collapse all repositories",
    });
    fireEvent.click(collapseAll);

    expect(
      document
        .getElementById("ci-tracker-runs-github-42")
        ?.hasAttribute("hidden"),
    ).toBe(true);
    expect(
      document
        .getElementById("ci-tracker-runs-github-43")
        ?.hasAttribute("hidden"),
    ).toBe(true);
    expect(
      screen.getByRole("button", { name: "Expand all repositories" }),
    ).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", { name: "Expand all repositories" }),
    );
    expect(
      document
        .getElementById("ci-tracker-runs-github-42")
        ?.hasAttribute("hidden"),
    ).toBe(false);
    expect(
      document
        .getElementById("ci-tracker-runs-github-43")
        ?.hasAttribute("hidden"),
    ).toBe(false);
  });
});
