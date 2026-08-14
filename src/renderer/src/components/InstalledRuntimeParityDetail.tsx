import type { ReactElement } from "react";
import type { InstalledRuntimeSnapshot } from "../types";
import { StatusPill } from "./ConsolePrimitives";

export function InstalledRuntimeParityDetail({
  runtime,
}: {
  runtime: InstalledRuntimeSnapshot;
}): ReactElement {
  return (
    <div className="drawer-section quality-detail-section">
      <div className="drawer-section-title">
        <div>
          <h3>Installed runtime parity</h3>
          <small>{runtime.summary}</small>
        </div>
        <StatusPill tone={runtime.status === "current" ? "mint" : "amber"}>
          {runtime.status === "current" ? "Current" : "Attention required"}
        </StatusPill>
      </div>
      <div className="repository-release-rule-list">
        {runtime.targets.map((target) => (
          <div className="repository-release-rule-row" key={target.id}>
            <span>
              <strong>{target.label}</strong>
              <small>
                {target.issues.length > 0
                  ? target.issues.map((item) => item.message).join(" ")
                  : "Source, build, installed artifact, and running process match."}
              </small>
            </span>
            <StatusPill tone={target.status === "current" ? "mint" : "amber"}>
              {target.status.replaceAll("_", " ")}
            </StatusPill>
          </div>
        ))}
      </div>
    </div>
  );
}
