// @vitest-environment happy-dom
// quality-gate: allow static-ui-test: verifies incomplete persisted payloads normalize safely and provider parity is never overstated in rendered evidence.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { normalizeSkillsSnapshot } from "../skillsSnapshot";
import type { PapercutBacklog, SkillRecord, SkillsSnapshot } from "../types";
import { SkillsSurface } from "./SkillsComponents";

function makeSkill(overrides: Partial<SkillRecord> = {}): SkillRecord {
  return {
    id: "example",
    name: "example",
    description: "A provider-neutral example skill",
    category: "UI & Design",
    family: "Browser",
    lifecycle: "canonical",
    hosted_in_jakye_agent_setup: true,
    sources: [
      {
        path: "/Users/jakyeamos/projects/jakyeamos-agent-skills/skills/example/SKILL.md",
        root: "jakye-agent-setup",
        provenance: "Hosted in jakye-agent-setup",
        sha256: "abc",
        hosted_in_jakye_agent_setup: true,
      },
    ],
    providers: {
      codex: { state: "projected", reason: "Matches canonical source" },
      claude: { state: "divergent", reason: "Differs from canonical source" },
      gemini: { state: "unsupported", reason: "No evidence" },
      cursor: { state: "blocked", reason: "Runtime unverified" },
    },
    parity_evidence: ["Behavioral fixtures unavailable"],
    usage: {
      recent_count: 3,
      all_time_count: 7,
      by_provider: { codex: 3 },
      telemetry_source: "Local session records",
    },
    ...overrides,
  };
}

function makeSnapshot(skills: SkillRecord[] = [makeSkill()]): SkillsSnapshot {
  return {
    schema_version: "pronto-skills/v2",
    generated_at: "2026-07-27T12:00:00Z",
    refreshed_at: "2026-07-27T12:00:00Z",
    freshness: "Observed through 2026-07-27T12:00:00Z",
    source: "Local skill roots",
    recent_days: 30,
    roots: ["canonical"],
    skills,
    telemetry_gap: "Best-effort local telemetry",
  };
}

const papercutProps = {
  papercutBacklog: {
    schema_version: "pronto-papercuts/v2",
    family: "design-audit",
    generated_at: "2026-07-27T12:00:00Z",
    papercuts: [],
    counts: {
      total: 0,
      open: 0,
      in_progress: 0,
      deferred: 0,
      resolved: 0,
      observations: 0,
      local_patterns: 0,
      cross_scope_patterns: 0,
      draft_proposals: 0,
    },
    observations: [],
    patterns: [],
    proposals: [],
    digests: [],
    health: {
      status: "healthy",
      database_writable: true,
      consecutive_failures: 0,
      spooled_events: 0,
      quarantined_events: 0,
      oldest_spool_at: null,
      last_success_at: null,
      warning: null,
      excerpt_retention_days: 90,
    },
  } satisfies PapercutBacklog,
  onRefreshPapercutBacklog: async () => undefined,
  onCreatePapercut: async () => undefined,
  onPapercutStatusChange: async () => undefined,
  onMultiplierProposalStatusChange: async () => undefined,
};

afterEach(() => {
  cleanup();
});

describe("skills surface", () => {
  it("renders provider states, hosted badge, usage, and unknown parity honestly", () => {
    const markup = renderToStaticMarkup(
      <SkillsSurface
        snapshot={makeSnapshot()}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );
    expect(markup).toContain("jakye-agent-setup");
    expect(markup).toContain("codex: projected");
    expect(markup).toContain("claude: divergent");
    expect(markup).toContain("Unknown");
    expect(markup).toContain("7 all-time");
    expect(markup).toContain("UI &amp; Design");
    expect(markup).not.toContain("Browser");
    expect(markup).not.toContain("Standalone");
    expect(markup.match(/<details/g)?.length).toBe(1);
  });

  it("only shows a family disclosure when the family has multiple skills", () => {
    const markup = renderToStaticMarkup(
      <SkillsSurface
        snapshot={makeSnapshot([
          makeSkill(),
          makeSkill({ id: "example-two", name: "example-two" }),
        ])}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );
    expect(markup).toContain("Browser");
    expect(markup.match(/<details/g)?.length).toBe(2);
  });

  it("renders an explicit empty state", () => {
    const markup = renderToStaticMarkup(
      <SkillsSurface
        snapshot={makeSnapshot([])}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );
    expect(markup).toContain("No skills indexed yet");
    expect(markup).toContain("Refresh to scan");
  });

  it("normalizes incomplete persisted records without throwing", () => {
    const snapshot = normalizeSkillsSnapshot({
      schema_version: "pronto-skills/v1",
      skills: [{ name: "legacy-skill", description: "Old record" }],
    });
    const markup = renderToStaticMarkup(
      <SkillsSurface
        snapshot={snapshot}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );
    expect(markup).toContain("legacy-skill");
    expect(markup).toContain("Other");
    expect(markup).not.toContain("Standalone");
    expect(markup).toContain("Parity evidence is unavailable.");
  });

  it("keeps a missing top-level bridge payload in a usable empty state", () => {
    const markup = renderToStaticMarkup(
      <SkillsSurface
        snapshot={null as unknown as SkillsSnapshot}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );
    expect(markup).toContain("No skills indexed yet");
  });

  it("opens Papercuts as a skill detail route from the Skills inventory", () => {
    render(
      <SkillsSurface
        snapshot={makeSnapshot([
          makeSkill({
            id: "papercuts",
            name: "Papercuts",
            description:
              "Capture and triage durable small hurts from the design-audit family.",
            family: "Design Audit",
            sources: [],
            providers: {
              pronto: {
                state: "native",
                reason: "Native Pronto design-audit backlog surface",
              },
            },
          }),
        ])}
        isRefreshing={false}
        onRefresh={() => undefined}
        onOpenSource={() => undefined}
        {...papercutProps}
      />,
    );

    fireEvent.click(screen.getByText("Papercuts"));

    expect(
      screen.getByRole("heading", { name: "Papercuts", level: 2 }),
    ).toBeTruthy();
    expect(screen.getByText("Skill detail · Design audit")).toBeTruthy();
    expect(
      screen.getByText(
        "Turn friction into evidence, patterns, and multipliers.",
      ),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "All skills" })).toBeTruthy();
  });
});
