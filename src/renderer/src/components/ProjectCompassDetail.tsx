import type { ReactElement } from "react";
import { Compass } from "lucide-react";
import type { RepositorySnapshot } from "../types";
import { StatusPill } from "./ConsolePrimitives";

export function ProjectCompassDetail({
  repository,
}: {
  repository: RepositorySnapshot;
}): ReactElement {
  const compass = repository.project_compass;
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
        <StatusPill
          tone={
            compass.status === "Ready"
              ? "mint"
              : compass.status === "Invalid"
                ? "coral"
                : "slate"
          }
        >
          {compass.status}
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
            </div>
            <div>
              <span>Complete product</span>
              <strong>
                {compass.complete_product.progress_percent ?? "Unknown"}
                {compass.complete_product.progress_percent === null ? "" : "%"}
              </strong>
              <small>{compass.complete_product.confidence} confidence</small>
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
