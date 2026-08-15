// @vitest-environment happy-dom
// quality-gate: allow static-ui-test: verifies server rendering plus interactive corpus tabs.
import { renderToStaticMarkup } from "react-dom/server";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PapercutSurface } from "./PapercutSurface";
import type { PapercutBacklog } from "../types";

function backlogFixture(
  overrides: Partial<PapercutBacklog> = {},
): PapercutBacklog {
  return {
    schema_version: "pronto-papercuts/v2",
    family: "design-audit",
    generated_at: "2026-08-08T16:00:00+00:00",
    papercuts: [
      {
        id: "papercut-1",
        title: "The empty state hides the next action",
        detail: "A first-time user has to infer where to begin.",
        family: "design-audit",
        surface: "Pronto UI",
        source: "design-friction",
        evidence_refs: ["screen:portfolio-empty"],
        impact: "Adds avoidable orientation cost.",
        priority: "P1",
        status: "open",
        next_action: "Exercise the empty-state flow after the copy change.",
        created_at: "2026-08-08T15:00:00+00:00",
        updated_at: "2026-08-08T15:00:00+00:00",
        resolved_at: null,
      },
    ],
    counts: {
      total: 1,
      open: 1,
      in_progress: 0,
      deferred: 0,
      resolved: 0,
      observations: 1,
      local_patterns: 1,
      cross_scope_patterns: 0,
      draft_proposals: 0,
    },
    observations: [],
    patterns: [
      {
        id: "papercut-1",
        fingerprint: "v1|local|repo-a|artifact|empty-state|hidden-action",
        fingerprint_version: "v1",
        scope_kind: "local",
        scope_id: "repo-a",
        title: "The empty state hides the next action",
        detail: "A first-time user has to infer where to begin.",
        domain: "design",
        target_kind: "artifact",
        phenomenon_key: "empty-state",
        failure_mode: "hidden-action",
        surface: "Pronto UI",
        source: "design-friction",
        evidence_refs: ["screen:portfolio-empty"],
        impact: "Adds avoidable orientation cost.",
        priority: "P1",
        status: "open",
        next_action: "Exercise the empty-state flow after the copy change.",
        evidence_tier: "local_recurring",
        occurrence_count: 2,
        scope_count: 1,
        first_observed_at: "2026-08-08T14:00:00+00:00",
        last_observed_at: "2026-08-08T15:00:00+00:00",
        created_at: "2026-08-08T15:00:00+00:00",
        updated_at: "2026-08-08T15:00:00+00:00",
        resolved_at: null,
      },
    ],
    proposals: [],
    digests: [],
    health: {
      status: "healthy",
      database_writable: true,
      consecutive_failures: 0,
      spooled_events: 0,
      quarantined_events: 0,
      oldest_spool_at: null,
      last_success_at: "2026-08-08T15:00:00+00:00",
      warning: null,
      excerpt_retention_days: 90,
    },
    ...overrides,
  };
}

describe("PapercutSurface", () => {
  afterEach(cleanup);
  it("keeps the per-turn audit boundary separate from durable capture", () => {
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture()}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
        onProposalStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("Universal signal corpus");
    expect(markup).toContain("Explicit corrections");
    expect(markup).toContain("recurrence");
    expect(markup).toContain("Capture a papercut");
    expect(markup).toContain("Design-friction audit");
    expect(markup).toContain("The empty state hides the next action");
    expect(markup).toContain("Next validation step");
  });

  it("shows an actionable capture failure without requiring an error-code lookup", () => {
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture({
          health: {
            status: "failing",
            database_writable: false,
            consecutive_failures: 3,
            spooled_events: 7,
            quarantined_events: 0,
            oldest_spool_at: "2026-08-08T14:00:00+00:00",
            last_success_at: null,
            warning: "Papercuts drain failed on attempt 3.",
            excerpt_retention_days: 90,
            last_error: {
              error_code: "PAPERCUTS-E4001",
              failure_kind: "child_process_timeout",
              stage: "pronto_process",
              message: "the Pronto capture process timed out",
              operation: "drain",
              observed_at: "2026-08-08T16:00:00+00:00",
              retryable: true,
              recovery_command: "pronto-papercuts papercuts health --json",
              attempt: 3,
              timeout_seconds: 3,
              exit_code: null,
            },
          },
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
        onProposalStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("the Pronto capture process timed out");
    expect(markup).toContain("PAPERCUTS-E4001");
    expect(markup).toContain("Attempt 3 during drain");
    expect(markup).toContain("timed out after 3s");
    expect(markup).toContain("pronto-papercuts papercuts health --json");
  });

  it("surfaces isolated incompatible observations without treating the queue as blocked", () => {
    const fixture = backlogFixture();
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture({
          health: {
            ...fixture.health,
            status: "degraded",
            database_writable: true,
            quarantined_events: 2,
          },
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
        onProposalStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("2 incompatible signals isolated");
    expect(markup).not.toContain("signals awaiting flush");
  });

  it("makes an empty top-level backlog actionable", () => {
    const markup = renderToStaticMarkup(
      <PapercutSurface
        backlog={backlogFixture({
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
          patterns: [],
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
        onProposalStatusChange={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("No patterns in this scope.");
    expect(markup).toContain(
      "Two matching observations create a local pattern",
    );
    expect(markup).toContain("Capture papercut");
  });

  it("exposes expiring observations and human-only proposal review", () => {
    const onProposalStatusChange = vi.fn(async () => undefined);
    render(
      <PapercutSurface
        backlog={backlogFixture({
          observations: [
            {
              id: "observation-1",
              event_key: "v1:codex:opaque",
              scope_id: "repository:v1:opaque",
              scope_kind: "repository",
              domain: "software",
              signal_kind: "correction",
              target_kind: "agent_answer",
              summary: "The user corrected a premature success claim.",
              excerpt: null,
              excerpt_hash: "hash",
              excerpt_expires_at: "2026-11-06T00:00:00Z",
              source: "codex_passive_hook",
              evidence_refs: [],
              phenomenon_key: "premature-success",
              failure_mode: "missing-verification",
              priority: "P1",
              urgent: false,
              verified: true,
              observed_at: "2026-08-08T15:00:00Z",
            },
          ],
          digests: [
            {
              id: "digest-1",
              week_start: "2026-08-03T00:00:00Z",
              week_end: "2026-08-10T00:00:00Z",
              generated_at: "2026-08-09T22:00:00Z",
              observation_count: 1,
              local_pattern_count: 1,
              cross_scope_pattern_count: 0,
              draft_proposal_count: 1,
              top_patterns: backlogFixture().patterns,
            },
          ],
          proposals: [
            {
              id: "proposal-1",
              pattern_ids: ["papercut-1"],
              title: "Require forward-surface verification",
              hypothesis:
                "Source evidence is being mistaken for live behavior.",
              root_cause: "Evidence states are collapsed.",
              multiplier: "Add a direct verification gate.",
              evidence_tier: "local_recurring",
              status: "draft",
              created_at: "2026-08-09T22:00:00Z",
              updated_at: "2026-08-09T22:00:00Z",
              reviewed_at: null,
            },
          ],
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onCreate={vi.fn(async () => undefined)}
        onStatusChange={vi.fn(async () => undefined)}
        onProposalStatusChange={onProposalStatusChange}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Observations" }));
    expect(
      screen.getByText("The user corrected a premature success claim."),
    ).toBeTruthy();
    expect(
      screen.getByText("Excerpt expired; structured evidence retained."),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Weekly digest" }));
    expect(
      screen.getByText("Require forward-surface verification"),
    ).toBeTruthy();
    expect(
      screen.getByText(
        "Accepting records your judgment; it never starts implementation.",
      ),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "accepted" }));
    expect(onProposalStatusChange).toHaveBeenCalledWith(
      "proposal-1",
      "accepted",
    );
  });
});
