import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RepositorySnapshot } from "../types";

export function CompassFamilyDetail({
  compass,
}: {
  compass: RepositorySnapshot["project_compass"];
}) {
  const family = compass.family;
  const [sourceError, setSourceError] = useState<string | null>(null);
  if (!family || family.schema !== "compass-family/v1") {
    return (
      <p className="quality-inline-empty">
        Subsystem projection unavailable. Refresh with the current Compass
        helper.
      </p>
    );
  }
  const assessment = compass.modernization_assessment;
  return (
    <section className="compass-family" aria-label="Subsystem intent">
      <p>
        Subsystem intent · projection{" "}
        {compass.projection_status ?? family.status}
      </p>
      <small>
        Snapshot captured {family.generated_at}. Refresh to check source
        freshness. Declared progress does not establish current proof.
      </small>
      {assessment && (
        <aside aria-label="Modernization assessment">
          <strong>
            Recorded assessment: {assessment.disposition.replaceAll("_", " ")}
          </strong>
          <p>
            Proposal:{" "}
            {assessment.proposal_complete === undefined
              ? "unknown"
              : assessment.proposal_complete
                ? "complete"
                : "incomplete"}
            {" · "}Review: {assessment.review?.status ?? "unknown"}
          </p>
          <p>
            Realized benefit:{" "}
            {assessment.benefit_observation?.benefit_realized.replaceAll(
              "_",
              " ",
            ) ?? "unknown"}
            {" · "}Preservation proof is checked separately.
          </p>
          <p>
            Declared review, not execution approval ·{" "}
            {assessment.remediation_id ?? "No remediation linked"}
          </p>
          {assessment.reasons.map((reason) => (
            <p key={reason}>{reason}</p>
          ))}
        </aside>
      )}
      <aside aria-label="Intent documentation maturity">
        <strong>
          Intent documentation: {family.bootstrap?.status ?? "unknown"}
        </strong>
        <p>
          {family.bootstrap?.verification ??
            "Refresh with a helper that assesses behavior links and preservation proof."}
        </p>
        {family.bootstrap?.subsystems.map((item) => (
          <p key={item.compass_id}>
            {item.compass_id}: {item.status}
            {item.gaps.length > 0 ? ` · ${item.gaps.join(", ")}` : ""}
          </p>
        ))}
        <p>
          Behavior coverage: {family.coverage.behavior_status ?? "unknown"}.
          {family.coverage.behavior_status === "measured"
            ? ` ${family.coverage.unmapped_behaviors.length} unmapped behaviors.`
            : ` ${family.coverage.behavior_reason ?? "No measured behavior inventory in this snapshot."}`}
        </p>
      </aside>
      {sourceError && <p role="status">{sourceError}</p>}
      <nav aria-label="Compass subsystems">
        {family.nodes.map((node) => (
          <details key={node.id}>
            <summary>
              {node.id} · {node.valid ? node.status : "invalid"} · revision{" "}
              {node.revision ?? "unknown"}
            </summary>
            <p>{node.purpose ?? node.error}</p>
            <small>
              {node.authority} ·{" "}
              <button
                className="ghost-button"
                type="button"
                onClick={() => {
                  void invoke("open_compass_source", {
                    workspace: family.source.workspace,
                    sourceRef: node.source_ref,
                  })
                    .then(() => setSourceError(null))
                    .catch((error) => setSourceError(String(error)));
                }}
              >
                Open source
              </button>
            </small>
            <p id={`compass-source-${node.id}`}>
              <code>{node.source_ref}</code>
            </p>
            {node.constraints.length > 0 && (
              <ul>
                {node.constraints.map((constraint) => (
                  <li key={constraint}>{constraint}</li>
                ))}
              </ul>
            )}
            {node.error && <p role="status">{node.error}</p>}
          </details>
        ))}
      </nav>
      {family.relationships.length > 0 && (
        <div aria-label="Subsystem handoffs">
          {family.relationships.map((link) => (
            <p key={link.id}>
              {link.from} → {link.to} · {link.valid ? link.status : "dangling"}:{" "}
              {link.summary}
            </p>
          ))}
        </div>
      )}
      <p>
        Path coverage {family.coverage.status}:{" "}
        {family.coverage.covered_paths.length} accepted,{" "}
        {family.coverage.proposed_paths.length} proposed,{" "}
        {family.coverage.unmapped_paths.length} unmapped,{" "}
        {family.coverage.ambiguous_paths.length} ambiguous.
      </p>
      {family.errors.map((error, index) => (
        <p role="status" key={`${error.scope}-${index}`}>
          {error.scope}: {error.message}
        </p>
      ))}
    </section>
  );
}
