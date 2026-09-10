// quality-gate: allow static-ui-test: verifies draft and invalid producer states remain visible and legacy caches never imply available subsystem intent.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CompassFamilyProjection, RepositorySnapshot } from "../types";
import { CompassFamilyDetail } from "./CompassFamilyDetail";
import fixture from "../../../../fixtures/compass/family.json";

describe("Compass family detail", () => {
  it("shows draft, invalid child, and measured gaps without claiming proof", () => {
    const compass = {
      family: fixture as CompassFamilyProjection,
      projection_status: "partial",
    } as RepositorySnapshot["project_compass"];
    const html = renderToStaticMarkup(
      <CompassFamilyDetail compass={compass} />,
    );
    expect(html).toContain("playback · draft");
    expect(html).toContain("invalid · invalid");
    expect(html).toContain(
      "Declared progress does not establish current proof",
    );
    expect(html).toContain("Open source");
    expect(html).toContain("proposed");
    expect(html).toContain("Behavior coverage: unknown");
    expect(html).toContain("Intent documentation: incomplete");
  });
  it.each([
    { disposition: "worthwhile_now", expected: "Review: unknown" },
    {
      disposition: "insufficient_evidence",
      proposal_complete: true,
      review: { status: "missing" },
      expected: "Review: missing",
    },
    {
      disposition: "retain_current_design",
      proposal_complete: true,
      review: { status: "current" },
      expected: "Review: current",
    },
  ])(
    "keeps review and benefit uncertainty explicit for $disposition",
    ({ expected, ...assessment }) => {
      const html = renderToStaticMarkup(
        <CompassFamilyDetail
          compass={
            {
              family: fixture as CompassFamilyProjection,
              modernization_assessment: {
                ...assessment,
                reasons: [],
                execution_authority: false,
              },
            } as unknown as RepositorySnapshot["project_compass"]
          }
        />,
      );
      expect(html).toContain(expected);
      expect(html).toContain("Realized benefit: unknown");
      expect(html).toContain("Preservation proof is checked separately");
      expect(html).toContain("Declared review, not execution approval");
    },
  );
  it("keeps a legacy cached projection explicitly unavailable", () => {
    const html = renderToStaticMarkup(
      <CompassFamilyDetail
        compass={{} as RepositorySnapshot["project_compass"]}
      />,
    );
    expect(html).toContain("Subsystem projection unavailable");
  });
});
