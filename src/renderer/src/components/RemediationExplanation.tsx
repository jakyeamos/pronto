import type { ReactElement } from "react";
import { CheckCircle2 } from "lucide-react";
import type { RemediationExplanation as RemediationExplanationModel } from "../types/remediation";
import { StatusPill } from "./ConsolePrimitives";
import {
  remediationStatusLabel,
  remediationStatusTone,
} from "./RemediationActionRow";

export function RemediationExplanation({
  explanation,
}: {
  explanation: RemediationExplanationModel;
}): ReactElement {
  return (
    <section
      className="remediation-explanation-block"
      aria-label="Remediation path"
    >
      <div className="remediation-explanation-heading">
        <div>
          <span>Remediation path</span>
          <strong>
            {explanation.phases.length} phase
            {explanation.phases.length === 1 ? "" : "s"} remaining
          </strong>
        </div>
        <small>{explanation.summary}</small>
      </div>
      <div className="remediation-phase-list">
        {explanation.phases.map((phase, index) => (
          <article className="remediation-phase" key={phase.id}>
            <div className="remediation-phase-heading">
              <div>
                <span>Phase {index + 1}</span>
                <h3>{phase.title}</h3>
              </div>
              <StatusPill tone={remediationStatusTone(phase.status)}>
                {remediationStatusLabel(phase.status)}
              </StatusPill>
            </div>
            <p>{phase.summary}</p>
            <ol className="remediation-phase-steps">
              {phase.steps.map((step) => (
                <li key={step.action_id}>
                  <div>
                    <strong>{step.title}</strong>
                    <span>{step.summary}</span>
                  </div>
                  <div className="remediation-phase-step-meta">
                    <span>{step.priority}</span>
                    <StatusPill tone={remediationStatusTone(step.status)}>
                      {remediationStatusLabel(step.status)}
                    </StatusPill>
                  </div>
                  {step.completion_criteria.length > 0 && (
                    <details>
                      <summary>What done means</summary>
                      <ul>
                        {step.completion_criteria.map((criterion) => (
                          <li key={criterion}>{criterion}</li>
                        ))}
                      </ul>
                    </details>
                  )}
                </li>
              ))}
            </ol>
            <div className="remediation-phase-exit">
              <CheckCircle2 size={13} />
              <span>{phase.completion_criterion}</span>
            </div>
          </article>
        ))}
      </div>
      <div className="remediation-explanation-footnotes">
        <details>
          <summary>
            Already healthy · {explanation.healthy_surfaces.length} surfaces
          </summary>
          <div className="remediation-healthy-list">
            {explanation.healthy_surfaces.map((surface) => (
              <div key={surface.surface}>
                <CheckCircle2 size={12} />
                <span>
                  <strong>{surface.label}</strong>
                  {surface.detail}
                </span>
              </div>
            ))}
          </div>
        </details>
        <details>
          <summary>What clears this queue for this refresh</summary>
          <ul>
            {explanation.closure_requirements.map((requirement) => (
              <li key={requirement}>{requirement}</li>
            ))}
          </ul>
        </details>
        <small>{explanation.authority}</small>
      </div>
    </section>
  );
}
