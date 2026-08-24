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
  ProviderIdentity,
  ProviderStatus,
  RemoteRepositorySnapshot,
} from "../types";
import { RemoteCatalogSurface } from "./RemoteCatalogSurface";

afterEach(cleanup);

const status: ProviderStatus = {
  provider: "GitHub",
  state: "Ready",
  message: "GitHub context is current.",
  last_refresh_at: "2026-08-23T12:00:00Z",
  identity_count: 1,
  repository_count: 1,
};

const identity: ProviderIdentity = {
  id: "github:jakyeamos",
  provider: "github",
  login: "jakyeamos",
  display_name: "Jakye Amos",
  organizations: [],
  credential_state: "Authenticated",
  updated_at: "2026-08-23T12:00:00Z",
};

function repository(
  overrides: Partial<RemoteRepositorySnapshot> = {},
): RemoteRepositorySnapshot {
  return {
    id: "github:42",
    provider: "github",
    full_name: "Acme/Portfolio",
    name: "Portfolio",
    owner: "Acme",
    html_url: "https://github.com/Acme/Portfolio",
    default_branch: "main",
    archived: false,
    locality: "Remote only",
    identity_id: identity.id,
    last_refreshed_at: "2026-08-23T12:00:00Z",
    pull_requests: [],
    releases: [],
    ci_checks: [],
    visibility: "private",
    viewer_can_administer: true,
    viewer_permission: "admin",
    ...overrides,
  };
}

describe("GitHub remote catalog", () => {
  it("shows visibility evidence and requires exact confirmation before submitting", async () => {
    const onChangeVisibility = vi.fn(async () => undefined);
    render(
      <RemoteCatalogSurface
        status={status}
        identities={[identity]}
        repositories={[repository()]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onChangeVisibility={onChangeVisibility}
      />,
    );

    expect(screen.getByText("private")).toBeTruthy();
    expect(screen.getByText("admin permission")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Change visibility" }));
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(
      screen.getByText(/Private repositories remove public access/),
    ).toBeTruthy();

    const submit = screen.getAllByRole("button", {
      name: "Change visibility",
    })[1];
    expect((submit as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: "public" },
    });
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Acme/Portfolio" },
    });
    expect((submit as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(submit);

    await waitFor(() =>
      expect(onChangeVisibility).toHaveBeenCalledWith(
        "github:42",
        "public",
        "Acme/Portfolio",
      ),
    );
  });

  it("keeps the provider write unavailable without confirmed administration permission", () => {
    render(
      <RemoteCatalogSurface
        status={status}
        identities={[identity]}
        repositories={[
          repository({
            viewer_can_administer: false,
            viewer_permission: "push",
          }),
        ]}
        isRefreshing={false}
        onRefresh={async () => undefined}
        onChangeVisibility={async () => undefined}
      />,
    );

    expect(screen.getByText("push permission")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Change visibility" }),
    ).toBeNull();
  });
});
