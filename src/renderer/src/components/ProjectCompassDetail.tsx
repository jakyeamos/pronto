import type { ReactElement } from "react";
import { Compass } from "lucide-react";
import type { RepositorySnapshot } from "../types";
import { StatusPill } from "./ConsolePrimitives";
import {
  projectCompassCoverageIsIncomplete,
  projectCompassCoverageLabel,
} from "../projectCompass";

function compassItemLabel(value: string): string {
  return value
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function compassDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat(undefined, {
        month: "short",
        day: "numeric",
        year: "numeric",
      }).format(date);
}

function compassContractStatus(status: string): {
  label: string;
  tone: "blue" | "coral" | "slate";
} {
  if (status === "Ready") return { label: "Contract valid", tone: "blue" };
  if (status === "Invalid") return { label: "Contract invalid", tone: "coral" };
  return { label: "Contract missing", tone: "slate" };
}

export function ProjectCompassDetail({
  repository,
}: {
  repository: RepositorySnapshot;
}): ReactElement {
  const compass = repository.project_compass;
  const contractStatus = compassContractStatus(compass.status);
  const blockerItems = compass.open_blocker_items ?? [];
  const driftItems = compass.open_drift_items ?? [];
  const missingBlockerDetails = Math.max(
    0,
    compass.open_blockers - blockerItems.length,
  );
  const missingDriftDetails = Math.max(
    0,
    compass.open_drift - driftItems.length,
  );
  return (
    <div className="drawer-section compass-detail-section">
      <div className="drawer-section-title">
        <div>
          <h3>
            <Compass size={15} /> Project Compass
          </h3>
          <small>
            Product-direction progress from {compass.contract_path}.
          </small>
        </div>
        <StatusPill tone={contractStatus.tone}>
          {contractStatus.label}
        </StatusPill>
      </div>
      {compass.status === "Ready" ? (
        <>
          <div className="compass-product-truth">
            <strong>{compass.project_name ?? repository.name}</strong>
            <p>{compass.identity}</p>
            <small>For {compass.audience}</small>
          </div>
          <div className="compass-progress-grid">
            <div>
              <span>MVP</span>
              <strong>
                {compass.mvp.progress_percent ?? "Unknown"}
                {compass.mvp.progress_percent === null ? "" : "%"}
              </strong>
              <small>{compass.mvp.confidence} confidence</small>
              <small
                className={
                  projectCompassCoverageIsIncomplete(compass.mvp)
                    ? "compass-coverage-warning"
                    : undefined
                }
              >
                {projectCompassCoverageLabel(compass.mvp)}
              </small>
            </div>
            <div>
              <span>Complete product</span>
              <strong>
                {compass.complete_product.progress_percent ?? "Unknown"}
                {compass.complete_product.progress_percent === null ? "" : "%"}
              </strong>
              <small>{compass.complete_product.confidence} confidence</small>
              <small
                className={
                  projectCompassCoverageIsIncomplete(compass.complete_product)
                    ? "compass-coverage-warning"
                    : undefined
                }
              >
                {projectCompassCoverageLabel(compass.complete_product)}
              </small>
            </div>
            <div>
              <span>Open blockers</span>
              <strong>{compass.open_blockers}</strong>
              <small>Across target outcomes</small>
            </div>
            <div>
              <span>Open drift</span>
              <strong>{compass.open_drift}</strong>
              <small>Revision {compass.revision ?? "unknown"}</small>
            </div>
          </div>
          {(compass.open_blockers > 0 || compass.open_drift > 0) && (
            <div className="compass-open-item-groups">
              {compass.open_blockers > 0 && (
                <section
                  className="compass-open-item-group"
                  aria-labelledby="compass-blockers-title"
                >
                  <div className="compass-open-item-heading">
                    <div>
                      <span>Blocking the finish line</span>
                      <h4 id="compass-blockers-title">Open blockers</h4>
                    </div>
                    <strong>{compass.open_blockers}</strong>
                  </div>
                  <div className="compass-open-item-list">
                    {blockerItems.map((blocker, index) => (
                      <article
                        className="compass-open-item"
                        key={`${blocker.outcome_id}-${blocker.kind}-${index}`}
                      >
                        <span>
                          {compassItemLabel(blocker.kind)} ·{" "}
                          {blocker.outcome_name}
                        </span>
                        <p>{blocker.summary}</p>
                      </article>
                    ))}
                    {missingBlockerDetails > 0 && (
                      <p className="quality-inline-empty">
                        {missingBlockerDetails} blocker description
                        {missingBlockerDetails === 1 ? " is" : "s are"}{" "}
                        unavailable in this snapshot. Refresh the repository to
                        load the
                        {missingBlockerDetails === 1 ? " detail" : " details"}.
                      </p>
                    )}
                  </div>
                </section>
              )}
              {compass.open_drift > 0 && (
                <section
                  className="compass-open-item-group compass-open-item-group-drift"
                  aria-labelledby="compass-drift-title"
                >
                  <div className="compass-open-item-heading">
                    <div>
                      <span>Product truth versus current evidence</span>
                      <h4 id="compass-drift-title">Open drift</h4>
                    </div>
                    <strong>{compass.open_drift}</strong>
                  </div>
                  <div className="compass-open-item-list">
                    {driftItems.map((drift, index) => (
                      <article
                        className="compass-open-item"
                        key={`${drift.kind}-${drift.observed_at}-${index}`}
                      >
                        <span>
                          {compassItemLabel(drift.kind)} · observed{" "}
                          {compassDate(drift.observed_at)}
                        </span>
                        <p>{drift.summary}</p>
                      </article>
                    ))}
                    {missingDriftDetails > 0 && (
                      <p className="quality-inline-empty">
                        {missingDriftDetails} drift description
                        {missingDriftDetails === 1 ? " is" : "s are"}{" "}
                        unavailable in this snapshot. Refresh the repository to
                        load the
                        {missingDriftDetails === 1 ? " detail" : " details"}.
                      </p>
                    )}
                  </div>
                </section>
              )}
            </div>
          )}
        </>
      ) : (
        <p className="quality-inline-empty">
          {compass.status === "Invalid"
            ? compass.error
            : "No Compass contract has been created for this repository yet."}
        </p>
      )}
    </div>
  );
}
