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
  });
  it("keeps a legacy cached projection explicitly unavailable", () => {
    const html = renderToStaticMarkup(
      <CompassFamilyDetail
        compass={{} as RepositorySnapshot["project_compass"]}
      />,
    );
    expect(html).toContain("Subsystem projection unavailable");
  });
});
