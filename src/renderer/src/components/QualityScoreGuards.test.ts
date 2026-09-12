import { describe, expect, it } from "vitest";
import type { QualityPortfolioSnapshot } from "../types";
import {
  hasAcceptedFleetMaturityAudit,
  hasAcceptedRepositoryMaturityEvidence,
  macControlScoreDisplay,
} from "./QualityScoreGuards";
import { makeMaturity, makePortfolio } from "./QualityComponents.test-support";

const acceptedFleet = makePortfolio([], {
  audit_status: "Ready",
  latest_audit_id: "fixture-audit",
  latest_audit_at: "2026-09-11T00:00:00Z",
  matched_repository_count: 1,
  maturity_score: 0,
  maturity_score_display: "0.000",
}).quality;

const measuredMac: NonNullable<
  QualityPortfolioSnapshot["mac_control_ideal_state"]
> = {
  status: "Failed",
  freshness: "Fresh",
  ideal_state: false,
  applicable_repository_count: 1,
  not_applicable_repository_count: 0,
  evaluated_repository_count: 1,
  implementation_status: "Failed",
  implementation_score: 0,
  implementation_score_display: "0.0",
  implementation_criteria_passed_count: 0,
  implementation_criteria_total: 8,
  live_status: "Failed",
  live_score: 0,
  live_score_display: "0.0",
  live_task_count: 1,
  measured_task_count: 1,
};

describe("quality score evidence guards", () => {
  it.each(["Ready", "Ready with blockers"])(
    "retains an imported numeric zero for %s without treating it as passing",
    (audit_status) => {
      expect(
        hasAcceptedFleetMaturityAudit({ ...acceptedFleet, audit_status }),
      ).toBe(true);
    },
  );

  it.each([
    { audit_status: "Stale" },
    { audit_status: "Blocked" },
    { latest_audit_id: undefined },
    { latest_audit_at: " " },
    { matched_repository_count: 0 },
    { maturity_score: Number.NaN },
    { maturity_score_display: "" },
  ])(
    "withholds cached fleet numbers with incomplete evidence %j",
    (missing) => {
      expect(
        hasAcceptedFleetMaturityAudit({ ...acceptedFleet, ...missing }),
      ).toBe(false);
    },
  );

  it("requires fresh repository identity and leaves target matching to its caller", () => {
    const fresh = makeMaturity({
      freshness: "Fresh",
      audit_id: "fixture-audit",
      observed_at: "2026-09-11T00:00:00Z",
      score: 0,
      score_display: "0.000",
    });
    expect(hasAcceptedRepositoryMaturityEvidence(fresh)).toBe(true);
    expect(
      hasAcceptedRepositoryMaturityEvidence({ ...fresh, freshness: "Stale" }),
    ).toBe(false);
    expect(
      hasAcceptedRepositoryMaturityEvidence({ ...fresh, audit_id: undefined }),
    ).toBe(false);
    expect(
      hasAcceptedRepositoryMaturityEvidence({
        ...fresh,
        observed_at: undefined,
      }),
    ).toBe(false);
  });

  it("retains genuinely measured failed zero scores in each Mac Control lane", () => {
    expect(macControlScoreDisplay(measuredMac, "implementation")).toBe("0.0");
    expect(macControlScoreDisplay(measuredMac, "live")).toBe("0.0");
  });

  it("distinguishes zero measured tasks from a measured zero score", () => {
    expect(
      macControlScoreDisplay(
        { ...measuredMac, measured_task_count: 0 },
        "live",
      ),
    ).toBeUndefined();
    expect(
      macControlScoreDisplay({ ...measuredMac, live_task_count: 0 }, "live"),
    ).toBeUndefined();
    expect(
      macControlScoreDisplay(
        { ...measuredMac, implementation_criteria_total: undefined },
        "implementation",
      ),
    ).toBeUndefined();
  });

  it.each(["implementation", "live"] as const)(
    "withholds %s scores for stale or blocked evidence",
    (lane) => {
      expect(
        macControlScoreDisplay({ ...measuredMac, freshness: "Stale" }, lane),
      ).toBeUndefined();
      expect(
        macControlScoreDisplay({ ...measuredMac, status: "Blocked" }, lane),
      ).toBeUndefined();
      expect(
        macControlScoreDisplay(
          { ...measuredMac, [`${lane}_status`]: "Not measured" },
          lane,
        ),
      ).toBeUndefined();
    },
  );

  it.each([undefined, "", " ", "future-status"])(
    "withholds a Mac lane with absent or unrecognized status %s",
    (status) => {
      for (const lane of ["implementation", "live"] as const) {
        expect(
          macControlScoreDisplay(
            { ...measuredMac, [`${lane}_status`]: status },
            lane,
          ),
        ).toBeUndefined();
        expect(
          macControlScoreDisplay(
            { ...measuredMac, status: status ?? "" },
            lane,
          ),
        ).toBeUndefined();
      }
    },
  );
});
