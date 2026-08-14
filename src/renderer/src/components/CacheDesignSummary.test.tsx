// @vitest-environment happy-dom
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  CacheDesignAssessment,
  QualityMaturity,
  QualityReadiness,
} from "../types";
import { QualityMaturityWithCacheSummary } from "./CacheDesignSummary";

const readiness: QualityReadiness = {
  applicable_gate_ids: [],
  configured_gate_ids: [],
  unconfigured_gate_ids: [],
  covered_gate_ids: [],
  fresh_passing_gate_ids: [],
  missing_gate_ids: [],
  stale_gate_ids: [],
  failed_gate_ids: [],
  blocked_gate_ids: [],
};

function assessment(status: string, score?: number): CacheDesignAssessment {
  return {
    schema: "quality-runner-cache-design-assessment-v1",
    status,
    score,
    measurement_complete: status !== "unknown",
    totals: {
      logical_bytes: 2097152,
      allocated_bytes: 1048576,
      exclusive_allocated_bytes: 1048576,
      shared_allocated_bytes: 0,
      file_count: 12,
      shared_file_count: 0,
    },
    categories: {},
    risk_flags: status === "failed" ? ["bound_exceeded"] : [],
    growth: {},
  };
}

function markup(status: string, score?: number, freshness = "Fresh"): string {
  const maturity: QualityMaturity = {
    freshness: freshness as QualityMaturity["freshness"],
    cache_design: assessment(status, score),
  };
  return renderToStaticMarkup(
    <QualityMaturityWithCacheSummary
      maturity={maturity}
      readiness={readiness}
    />,
  );
}

// quality-gate: allow static-ui-test: verifies cache evidence states and remediation copy
describe("CacheDesignSummary", () => {
  it.each([
    ["maintained", 4, "continue automated enforcement"],
    ["failed", 1, "Restore complete traversal or feed evidence"],
    ["unknown", undefined, "Restore complete traversal or feed evidence"],
    ["not_applicable", undefined, "No derived-storage surface was detected"],
    ["missing", undefined, "Restore complete traversal or feed evidence"],
  ] as const)("preserves the %s state", (status, score, expected) => {
    const rendered = markup(status, score);
    expect(rendered).toContain('aria-label="Cache design maturity"');
    expect(rendered).toContain(expected);
    expect(rendered).toContain(
      status === "not_applicable" ? "N/A" : "1.0 MiB allocated",
    );
    if (status === "failed") expect(rendered).toContain("bound exceeded");
  });

  it("marks otherwise maintained evidence stale with the fleet feed", () => {
    const rendered = markup("maintained", 4, "Stale");
    expect(rendered).toContain("Rerun the complete QR fleet audit");
    expect(rendered).toContain(">stale<");
  });
});
