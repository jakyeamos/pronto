// @vitest-environment happy-dom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import { App } from "./App";

type ApiModule = typeof api;

vi.mock("./api", async (importOriginal) => {
  const actual = await importOriginal<ApiModule>();
  return {
    ...actual,
    getSnapshot: vi.fn(),
    getAnalytics: vi.fn(),
    getSkills: vi.fn(),
    pickRoot: vi.fn(),
    registerRoot: vi.fn(),
    refresh: vi.fn(),
    refreshGithub: vi.fn(),
    refreshRemediation: vi.fn(),
  };
});

type AddRootEntryPoint = {
  name: string;
  findButton: () => HTMLElement;
};

function openSettings(): void {
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));
}

const addRootEntryPoints: AddRootEntryPoint[] = [
  {
    name: "portfolio page action",
    findButton: () => screen.getByRole("button", { name: "Add root" }),
  },
  {
    name: "empty portfolio action",
    findButton: () =>
      screen.getByRole("button", { name: "Add discovery root" }),
  },
  {
    name: "settings page action",
    findButton: () => {
      openSettings();
      return screen.getAllByRole("button", { name: "Add root" })[0];
    },
  },
  {
    name: "discovery roots settings action",
    findButton: () => {
      openSettings();
      return screen.getAllByRole("button", { name: "Add root" })[1];
    },
  },
];

async function renderApp(): Promise<void> {
  render(<App />);
  await waitFor(() => {
    expect(api.getSnapshot).toHaveBeenCalledOnce();
    expect(api.getAnalytics).toHaveBeenCalledOnce();
    expect(api.getSkills).toHaveBeenCalledOnce();
  });
  vi.clearAllMocks();
}

beforeEach(() => {
  vi.mocked(api.getSnapshot).mockResolvedValue(api.emptySnapshot);
  vi.mocked(api.getAnalytics).mockResolvedValue(api.emptyAnalytics);
  vi.mocked(api.getSkills).mockResolvedValue(api.emptySkills);
});

afterEach(cleanup);

describe.each(addRootEntryPoints)("Add root from $name", ({ findButton }) => {
  it("treats picker cancellation as a no-op", async () => {
    vi.mocked(api.pickRoot).mockResolvedValue(null);
    await renderApp();

    fireEvent.click(findButton());

    await waitFor(() => expect(api.pickRoot).toHaveBeenCalledOnce());
    expect(api.registerRoot).not.toHaveBeenCalled();
    expect(api.getAnalytics).not.toHaveBeenCalled();
    expect(api.refresh).not.toHaveBeenCalled();
    expect(api.refreshGithub).not.toHaveBeenCalled();
    expect(api.refreshRemediation).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("surfaces registration failure without crossing refresh boundaries", async () => {
    vi.mocked(api.pickRoot).mockResolvedValue("/tmp/not-a-repository-root");
    vi.mocked(api.registerRoot).mockRejectedValue(
      new Error("The selected repository root could not be registered."),
    );
    await renderApp();

    fireEvent.click(findButton());

    expect((await screen.findByRole("alert")).textContent).toContain(
      "The selected repository root could not be registered.",
    );
    expect(api.pickRoot).toHaveBeenCalledOnce();
    expect(api.registerRoot).toHaveBeenCalledExactlyOnceWith(
      "/tmp/not-a-repository-root",
    );
    expect(api.getAnalytics).not.toHaveBeenCalled();
    expect(api.refresh).not.toHaveBeenCalled();
    expect(api.refreshGithub).not.toHaveBeenCalled();
    expect(api.refreshRemediation).not.toHaveBeenCalled();
  });
});
