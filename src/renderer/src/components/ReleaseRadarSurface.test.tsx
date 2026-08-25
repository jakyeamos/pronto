// @vitest-environment happy-dom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  PortfolioSnapshot,
  ReleaseRadarRow,
  RemoteRepositorySnapshot,
  GitHubVisibility,
} from "../types";
import { ReleaseRadarSurface } from "./ReleaseRadarSurface";

afterEach(cleanup);

function row(overrides: Partial<ReleaseRadarRow> = {}): ReleaseRadarRow {
  return {
    id: "repo-aios",
    name: "AIOS",
    full_name: "acme/AIOS",
    locality: "Local and remote",
    github_url: "https://github.com/acme/AIOS",
    visibility: "public",
    visibility_freshness: "fresh",
    public_framing: "eligible",
    profile_link_sync: "in_sync",
    product_readiness: "not_applicable",
    demo_materials: "missing evidence",
    release_readiness: "unknown",
    next_action: "Review independent product, demo, and release evidence",
    blocker: undefined,
    repository_id: "repo-aios",
    remote_repository_id: "github-aios",
    ...overrides,
  };
}

function remote(
  id: string,
  fullName: string,
  visibility: RemoteRepositorySnapshot["visibility"],
): RemoteRepositorySnapshot {
  return {
    id,
    provider: "github",
    full_name: fullName,
    name: fullName.split("/")[1],
    owner: fullName.split("/")[0],
    html_url: `https://github.com/${fullName}`,
    default_branch: "main",
    archived: false,
    visibility,
    viewer_can_administer: true,
    viewer_permission: "admin",
    locality: "Local and remote",
    identity_id: "github:jakyeamos",
    last_refreshed_at: "2026-08-25T12:00:00Z",
    pull_requests: [],
    releases: [],
    ci_checks: [],
    ci_runs: [],
  };
}

function snapshot(): PortfolioSnapshot["release_radar"] {
  return {
    schema_version: "pronto-release-radar/v1",
    generated_at: "2026-08-25T12:00:00Z",
    provider_state: "Ready",
    provider_last_refresh_at: "2026-08-25T12:00:00Z",
    profile_state: "Available",
    counts: {
      total: 4,
      local_only: 0,
      github_only: 1,
      local_and_remote: 3,
      public_fresh: 1,
      private_or_internal_fresh: 2,
      unknown_or_blocked: 1,
      public_framing_eligible: 1,
      profile_needs_sync: 1,
    },
    rows: [
      row(),
      row({
        id: "repo-private",
        name: "Private App",
        full_name: "acme/private-app",
        visibility: "private",
        public_framing: "suppressed",
        profile_link_sync: "needs_sync",
        product_readiness: "assessed (4.0/5)",
        demo_materials: "assessed (3.0/5)",
        repository_id: "repo-private",
        remote_repository_id: "github-private",
      }),
      row({
        id: "repo-internal",
        name: "Internal App",
        full_name: "acme/internal-app",
        visibility: "internal",
        public_framing: "suppressed",
        profile_link_sync: "suppressed",
        repository_id: "repo-internal",
        remote_repository_id: "github-internal",
      }),
      row({
        id: "github-only",
        name: "Remote Only",
        full_name: "acme/remote-only",
        locality: "GitHub only",
        visibility: "unknown",
        visibility_freshness: "blocked",
        public_framing: "blocked",
        profile_link_sync: "blocked",
        next_action: "Resolve GitHub provider evidence",
        blocker: "Visibility is unknown.",
        repository_id: undefined,
        remote_repository_id: "github-unknown",
      }),
    ],
  };
}

type VisibilityChange = (
  repositoryId: string,
  targetVisibility: GitHubVisibility,
  confirmation: string,
) => Promise<void>;

function renderRadar(
  onChangeVisibility: VisibilityChange = async () => undefined,
) {
  render(
    <ReleaseRadarSurface
      snapshot={snapshot()}
      remoteRepositories={[
        remote("github-aios", "acme/AIOS", "public"),
        remote("github-private", "acme/private-app", "private"),
        remote("github-internal", "acme/internal-app", "internal"),
        remote("github-unknown", "acme/remote-only", undefined),
      ]}
      isRefreshing={false}
      onRefresh={async () => undefined}
      onChangeVisibility={onChangeVisibility}
    />,
  );
  return onChangeVisibility;
}

describe("ReleaseRadarSurface", () => {
  it("renders the full-fleet columns and keeps Showcase evidence independent", () => {
    renderRadar();

    expect(screen.getByRole("heading", { name: "Release Radar" })).toBeTruthy();
    expect(screen.getByText("Full-fleet distribution control")).toBeTruthy();
    for (const column of [
      "Visibility",
      "Public framing",
      "Profile README",
      "Product",
      "Demo materials",
      "Release",
      "Next action / blocker",
    ]) {
      expect(screen.getByRole("columnheader", { name: column })).toBeTruthy();
    }
    expect(screen.getByText("AIOS")).toBeTruthy();
    expect(screen.getAllByText("missing evidence").length).toBeGreaterThan(0);
    expect(screen.getAllByText("not_applicable").length).toBeGreaterThan(0);
    expect(screen.queryByText("demo materials created")).toBeNull();
    expect(screen.getByText("Remote Only")).toBeTruthy();
    expect(
      screen.getAllByRole("button", { name: "Change visibility" }),
    ).toHaveLength(3);
  });

  it("filters public, private/internal, unknown, and GitHub-only rows", () => {
    renderRadar();

    fireEvent.click(screen.getByRole("button", { name: "Public" }));
    expect(screen.getByText("AIOS")).toBeTruthy();
    expect(screen.queryByText("Private App")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Private/internal" }));
    expect(screen.getByText("Private App")).toBeTruthy();
    expect(screen.getByText("Internal App")).toBeTruthy();
    expect(screen.queryByText("AIOS")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Unknown/blocked" }));
    expect(screen.getByText("Remote Only")).toBeTruthy();
    expect(screen.queryByText("Private App")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "GitHub-only" }));
    expect(screen.getByText("Remote Only")).toBeTruthy();
    expect(screen.queryByText("AIOS")).toBeNull();
  });

  it("requires exact repository confirmation before routing a visibility write", () => {
    const onChangeVisibility = vi.fn<VisibilityChange>(async () => undefined);
    renderRadar(onChangeVisibility);
    fireEvent.click(
      screen.getAllByRole("button", { name: "Change visibility" })[0],
    );

    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText("acme/AIOS")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("New visibility"), {
      target: { value: "private" },
    });
    const input = screen.getByPlaceholderText("acme/AIOS");
    const submit = screen
      .getAllByRole("button", { name: "Change visibility" })
      .at(-1) as HTMLButtonElement;
    expect((submit as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(input, { target: { value: "acme/AIOS" } });
    expect((submit as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(submit);
    expect(onChangeVisibility).toHaveBeenCalledWith(
      "github-aios",
      "private",
      "acme/AIOS",
    );
  });

  it("keeps the visibility dialog open when the provider write fails", async () => {
    const onChangeVisibility = vi.fn<VisibilityChange>(async () => {
      throw new Error("provider blocked");
    });
    renderRadar(onChangeVisibility);
    fireEvent.click(
      screen.getAllByRole("button", { name: "Change visibility" })[0],
    );
    fireEvent.change(screen.getByLabelText("New visibility"), {
      target: { value: "private" },
    });
    fireEvent.change(screen.getByPlaceholderText("acme/AIOS"), {
      target: { value: "acme/AIOS" },
    });
    fireEvent.click(
      screen
        .getAllByRole("button", { name: "Change visibility" })
        .at(-1) as HTMLButtonElement,
    );

    await waitFor(() =>
      expect(screen.getByRole("alert").textContent).toContain(
        "provider blocked",
      ),
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(onChangeVisibility).toHaveBeenCalledTimes(1);
  });
});
