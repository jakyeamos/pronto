import { describe, expect, it } from "vitest";
import { measurementConfidenceSummary } from "./QualityGatesSurface";

describe("measurementConfidenceSummary", () => {
  it("reports the attested fleet population", () => {
    expect(
      measurementConfidenceSummary({
        level: "high",
        basis: ["population_complete", "dynamic_verification_conclusive"],
        limitations: [],
        population_status: "complete",
        expected_repository_count: 66,
        observed_repository_count: 66,
        excluded_repository_count: 2,
        unresolved_measurement_gap_count: 0,
        deterministic_replay: true,
      }),
    ).toBe("High measurement confidence · 66/66 repositories measured");
  });

  it("does not imply confidence when the feed omits its attestation", () => {
    expect(measurementConfidenceSummary(undefined)).toBe(
      "Measurement confidence unavailable",
    );
  });
});
