import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { InstalledRuntimeParityDetail } from "./InstalledRuntimeParityDetail";

describe("InstalledRuntimeParityDetail", () => {
  // quality-gate: allow static-ui-test: verifies typed parity status and issue projection at the repository detail boundary
  it("renders installed runtime drift as a distinct repository state", () => {
    const markup = renderToStaticMarkup(
      <InstalledRuntimeParityDetail
        runtime={{
          schema_version: "pronto-installed-runtime-parity-snapshot/v1",
          applicability: "applicable",
          status: "attention_required",
          summary: "1 installed-runtime parity issue requires attention.",
          config_path: ".pronto/installed-runtime-parity.json",
          targets: [
            {
              id: "daemon",
              label: "Mac Control daemon",
              status: "restart_required",
              process_id: 42,
              issues: [
                {
                  stage: "runtime",
                  status: "restart_required",
                  message:
                    "The running process does not match the installed artifact.",
                },
              ],
            },
          ],
        }}
      />,
    );

    expect(markup).toContain("Installed runtime parity");
    expect(markup).toContain("Mac Control daemon");
    expect(markup).toContain("restart required");
    expect(markup).toContain(
      "The running process does not match the installed artifact.",
    );
  });
});
