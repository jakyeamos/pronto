// quality-gate: allow static-ui-test: verifies the review-only promotion contract, provenance copy, and explicit unavailable states.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { PromotionSurface } from "./PromotionSurface";
import type { PromotionInbox } from "../types";

function inboxFixture(overrides: Partial<PromotionInbox> = {}): PromotionInbox {
  return {
    schema_version: "leverage-promotion-inbox/v1",
    visibility: "private",
    generated_at: "2026-08-08T16:00:00+00:00",
    source_root: "/Users/example/projects/ai-workflow-leverage",
    candidates: [
      {
        candidate_id: "candidate-workflow-port",
        title: "Workflow port candidate",
        asset_kind: "workflow",
        improvement_key: "workflow-port",
        source_refs: ["source:private:workflow-port"],
        evidence_refs: ["evidence:run:1"],
        quantification: { baseline: 30, observed: 10, unit: "minutes" },
        portability: "portable",
        status: "candidate",
        review_status: "pending",
        maturity: "experimental",
        package_status: "ready_for_projection",
        candidate_kind: "complete",
        candidate_source: "promotion-candidates",
        candidate_artifact: "artifacts/promotion-candidates/candidate.json",
        candidate_provenance_hash: "abc123456789",
        decision: null,
        decision_at: null,
        decision_reason: null,
        decision_reviewer: null,
        decision_artifact: null,
        next_action: "review_candidate",
      },
    ],
    counts: {
      total: 1,
      pending: 1,
      deferred: 0,
      rejected: 0,
      accepted: 0,
      complete: 1,
      drafts: 0,
    },
    coverage: {
      schema_version: "leverage-source-coverage/v1",
      visibility: "private",
      generated_at: "2026-08-08T16:00:00+00:00",
      source_owner: "local-owner",
      status: "pass",
      coverage_status: "partial",
      source_manifest: [
        {
          source_id: "source-skills",
          category: "skills",
          path: "/Users/example/.agents/skills",
          status: "assessed",
          scan_mode: "content_metadata",
          match_policy: "supported text files",
          files_seen: 12,
          bytes_seen: 1024,
          file_kinds: { ".md": 12 },
          repository_count: 0,
          unknown_reason: null,
          exclusion_reason: null,
          notes: "file metadata only; source contents were not copied",
        },
      ],
      assessed_sources: 1,
      partial_sources: 0,
      unassessed_sources: 2,
      excluded_sources: 0,
      blocked_sources: 0,
      unknown_sources: ["prompts", "sessions"],
      excluded_source_labels: [],
      files_seen: 12,
      bytes_seen: 1024,
      errors: [],
      raw_source_bytes_copied: false,
      jas_mutation: false,
    },
    discovery: {
      schema_version: "leverage-improvement-discovery-summary/v1",
      generated_at: "2026-08-08T16:00:00+00:00",
      status: "pass",
      observations_seen: 352,
      observations_inserted: 191,
      duplicates: 159,
      asset_observation_documents: 314,
      asset_observations_inserted: 191,
      asset_observation_duplicates: 123,
      candidate_drafts: 30,
      asset_roots: ["prompt=/Users/example/prompts"],
      raw_source_bytes_copied: false,
      jas_mutation: false,
    },
    errors: [],
    manual_review_required: true,
    jas_mutation: false,
    status: "pass",
    provenance_hash: "inbox-hash",
    message: null,
    ...overrides,
  };
}

describe("PromotionSurface", () => {
  it("renders the AWL decision boundary and candidate actions", () => {
    const markup = renderToStaticMarkup(
      <PromotionSurface
        inbox={inboxFixture()}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onDecide={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("AWL finds it. You decide.");
    expect(markup).toContain(
      "review-only inbox for backend-produced candidate packets",
    );
    expect(markup).toContain(
      "AWL owns discovery, testing, quantification, and packet generation",
    );
    expect(markup).toContain("Discovery coverage");
    expect(markup).toContain("Asset observations");
    expect(markup).toContain(
      "Asset observations are review inputs, not candidates.",
    );
    expect(markup).toContain("prompts");
    expect(markup).toContain("Workflow port candidate");
    expect(markup).toContain("Promote public");
    expect(markup).toContain("Keep private");
    expect(markup).toContain(
      "Accepted complete packets trigger the validated JAS admission/install path",
    );
    expect(markup).toContain("JAS unchanged");
  });

  it("renders the JAS apply result without exposing a second approval control", () => {
    const markup = renderToStaticMarkup(
      <PromotionSurface
        inbox={inboxFixture({
          jas_admission: {
            status: "JAS_APPLIED",
            candidate_id: "candidate-workflow-port",
            decision: "public",
            mutated: true,
            target: "catalog/manifest.json",
          },
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onDecide={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("JAS admission/install completed");
    expect(markup).toContain("JAS applied");
    expect(markup).not.toContain("Apply to JAS");
  });

  it("keeps a candidate-level admission result visible after inbox refresh", () => {
    const base = inboxFixture({ jas_admission: null });
    const candidate = {
      ...base.candidates[0],
      decision: "public" as const,
      jas_admission: {
        status: "JAS_APPLIED" as const,
        candidate_id: "candidate-workflow-port",
        decision: "public" as const,
        mutated: true,
        target: "catalog/manifest.json",
      },
    };
    const markup = renderToStaticMarkup(
      <PromotionSurface
        inbox={{ ...base, candidates: [candidate] }}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onDecide={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("JAS admission/install completed");
    expect(markup).toContain("JAS applied");
  });

  it("keeps an accepted AWL decision visible when JAS is blocked", () => {
    const markup = renderToStaticMarkup(
      <PromotionSurface
        inbox={inboxFixture({
          jas_admission: {
            status: "blocked",
            candidate_id: "candidate-workflow-port",
            decision: "public",
            mutated: false,
            message: "candidate has no embedded sanitized promotion projection",
          },
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onDecide={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("JAS admission/install is blocked");
    expect(markup).toContain(
      "candidate has no embedded sanitized promotion projection",
    );
  });

  it("makes an unavailable AWL state explicit", () => {
    const markup = renderToStaticMarkup(
      <PromotionSurface
        inbox={inboxFixture({
          candidates: [],
          counts: {
            total: 0,
            pending: 0,
            deferred: 0,
            rejected: 0,
            accepted: 0,
            complete: 0,
            drafts: 0,
          },
          status: "unavailable",
          message: "The AWL checkout is not available.",
        })}
        isRefreshing={false}
        onRefresh={vi.fn(async () => undefined)}
        onDecide={vi.fn(async () => undefined)}
      />,
    );

    expect(markup).toContain("AWL review is unavailable");
    expect(markup).toContain("The AWL checkout is not available.");
    expect(markup).toContain("No evaluated candidates are waiting");
  });
});
