// @vitest-environment happy-dom

import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  cancelRepositoryTelescopeRefresh,
  createPapercut,
  emptySnapshot,
  getRepositoryTelescope,
  refresh,
  refreshRepositoryTelescope,
  refreshRepositoryTargetEvidence,
  setRepositoryTargetBranch,
  setPapercutStatus,
  setMultiplierProposalStatus,
} from "./api";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  vi.clearAllMocks();
});

describe("refresh", () => {
  it("uses the bounded parallel refresh and returns its merged snapshot", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue({ snapshot: emptySnapshot });

    await expect(refresh()).resolves.toBe(emptySnapshot);

    expect(invoke).toHaveBeenCalledExactlyOnceWith("refresh_batch", {
      target: null,
      parallelism: null,
    });
  });
});

describe("Telescope commands", () => {
  it("uses repository-scoped get and refresh command contracts", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue({});

    await getRepositoryTelescope("repo-1");
    await refreshRepositoryTelescope("repo-1");
    await cancelRepositoryTelescopeRefresh("repo-1");

    expect(invoke).toHaveBeenNthCalledWith(1, "get_repository_telescope", {
      repositoryId: "repo-1",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "refresh_repository_telescope", {
      repositoryId: "repo-1",
    });
    expect(invoke).toHaveBeenNthCalledWith(
      3,
      "cancel_repository_telescope_refresh",
      { repositoryId: "repo-1" },
    );
  });
});

describe("refreshRepositoryTargetEvidence", () => {
  it("uses the target evidence command and camelCase arguments", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue(emptySnapshot);

    await refreshRepositoryTargetEvidence("repo-1", "dev");

    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "refresh_repository_target_evidence",
      {
        repositoryId: "repo-1",
        targetBranch: "dev",
      },
    );
  });
});

describe("setRepositoryTargetBranch", () => {
  it("uses Tauri's camelCase command argument contract", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue(emptySnapshot);

    await setRepositoryTargetBranch("repo-1", "develop");

    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "set_repository_target_branch",
      {
        repositoryId: "repo-1",
        targetBranch: "develop",
      },
    );
  });
});

describe("papercut commands", () => {
  it("keeps explicit capture in the papercut tab's local command boundary", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue(emptySnapshot);

    await createPapercut({
      title: "  Empty state  ",
      detail: "  The next action is hard to find.  ",
      surface: " Pronto UI ",
      source: "design-friction",
      priority: "P1",
      evidenceRefs: [" screen:portfolio-empty ", ""],
      impact: "  Adds orientation cost. ",
      nextAction: "  Exercise the empty state. ",
    });

    expect(invoke).toHaveBeenCalledExactlyOnceWith("create_papercut", {
      title: "Empty state",
      detail: "The next action is hard to find.",
      surface: "Pronto UI",
      source: "design-friction",
      priority: "P1",
      evidenceRefs: ["screen:portfolio-empty"],
      impact: "Adds orientation cost.",
      nextAction: "Exercise the empty state.",
    });
  });

  it("updates papercut status through the separate status command", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue(emptySnapshot);

    await setPapercutStatus("papercut-1", "resolved");

    expect(invoke).toHaveBeenCalledExactlyOnceWith("set_papercut_status", {
      papercutId: "papercut-1",
      status: "resolved",
    });
  });

  it("records proposal review without invoking implementation", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    vi.mocked(invoke).mockResolvedValue({
      id: "proposal-1",
      status: "accepted",
    });

    await setMultiplierProposalStatus("proposal-1", "accepted");

    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "set_multiplier_proposal_status",
      { proposalId: "proposal-1", status: "accepted" },
    );
  });
});
