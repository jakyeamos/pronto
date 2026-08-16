/*
THESIS: Quality Runner makes a repository's own definition of quality operational, then reconciles the resulting evidence without treating detector output as defects.
STORY: A recruiter sees the values that drove Tenure's 4,022-row baseline, follows a bounded historical burndown, and lands on the crucial distinction: 537 raw rows, 0 open actionable findings.
FIRST VIEWPORT: “4,022 findings, driven by what this codebase values” beside the actual eight-pack selection and coverage receipt.
*/
import type { ReactElement } from "react";
import {
  ArrowLeft,
  Ban,
  CheckCircle2,
  FileCheck2,
  GitBranch,
  Monitor,
  Search,
  ShieldCheck,
} from "lucide-react";
import burndown from "../../../../showcase-materials/quality-runner/burndown.json";
import caseStudy from "../../../../showcase-materials/quality-runner/case-study.json";
import claimLedger from "../../../../showcase-materials/quality-runner/claim-ledger.json";
import findingDrivers from "../../../../showcase-materials/quality-runner/finding-drivers.json";

function titleCase(value: string): string {
  return value
    .split("-")
    .map((word) => `${word.charAt(0).toUpperCase()}${word.slice(1)}`)
    .join(" ");
}

function ClaimIcon({ id }: { id: string }): ReactElement {
  switch (id) {
    case "candidate":
      return <Search size={17} aria-hidden="true" />;
    case "finding":
      return <FileCheck2 size={17} aria-hidden="true" />;
    case "local_gate":
      return <CheckCircle2 size={17} aria-hidden="true" />;
    case "branch_promotion":
      return <GitBranch size={17} aria-hidden="true" />;
    case "browser":
      return <Monitor size={17} aria-hidden="true" />;
    default:
      return <Ban size={17} aria-hidden="true" />;
  }
}

export function QualityRunnerCaseStudy({
  onBack,
}: {
  onBack: () => void;
}): ReactElement {
  const maxFindings = burndown.baseline.raw_code_quality_findings;

  return (
    <article className="qr-case" aria-labelledby="qr-case-title">
      <header className="qr-case-nav">
        <button type="button" className="qr-case-back" onClick={onBack}>
          <ArrowLeft size={15} aria-hidden="true" />
          All showcase projects
        </button>
        <span>Historical case · Tenure · July 2026</span>
      </header>

      <section className="qr-case-opening" aria-labelledby="qr-case-title">
        <div className="qr-case-opening-copy">
          <p className="eyebrow">Quality Runner × Tenure</p>
          <h2 id="qr-case-title">
            4,022 findings, driven by what this codebase values.
          </h2>
          <p>
            Quality Runner selected the standards relevant to Tenure, kept each
            finding tied to its reason and scope, and turned an overwhelming
            baseline into bounded, reviewable engineering work.
          </p>
          <div className="qr-case-scope-note">
            <strong>8 of 12 packs selected</strong>
            <span>
              from repository signals—not one generic definition of quality.
            </span>
          </div>
        </div>

        <div className="qr-case-values" aria-label="Finding driver receipt">
          <div className="qr-case-values-heading">
            <span>Actual baseline configuration</span>
            <strong>
              {findingDrivers.scan_result.coverage_entries} coverage entries
            </strong>
          </div>
          <div className="qr-case-packs">
            {findingDrivers.selection.selected_packs.map((pack) => (
              <div key={pack.id}>
                <strong>{titleCase(pack.id)}</strong>
                <span>{pack.matched_terms.slice(0, 3).join(" · ")}</span>
              </div>
            ))}
          </div>
          <p>
            Owners can override selection, activate local skills, change rule
            scope and thresholds, and exclude surfaces that should not be
            judged.
          </p>
        </div>
      </section>

      <section
        className="qr-case-customer"
        aria-labelledby="qr-case-customer-title"
      >
        <div className="qr-case-customer-heading">
          <div>
            <p className="eyebrow">What this means for your team</p>
            <h3 id="qr-case-customer-title">
              The same standards that guide your agents can audit what they
              produce.
            </h3>
          </div>
          <p>
            Your Skills and agent rules already describe the output and
            execution you want. Their implied failure modes can become concrete
            checks, giving you a measurable debt ledger and a plan to improve
            the repository against your own definition of quality.
          </p>
        </div>

        <ol className="qr-case-customer-flow">
          {findingDrivers.value_execution_loop.stages.map((stage, index) => (
            <li key={stage.id}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <small>{stage.label}</small>
              <strong>{stage.title}</strong>
              <p>{stage.detail}</p>
            </li>
          ))}
        </ol>

        <div className="qr-case-customer-example">
          <span className="qr-case-customer-example-label">
            Tenure · one value traced through the system
          </span>
          <div>
            {findingDrivers.value_execution_loop.tenure_example.map((item) => (
              <article key={item.label}>
                <small>{item.label}</small>
                <strong>{item.value}</strong>
              </article>
            ))}
          </div>
        </div>

        <p className="qr-case-customer-boundary">
          <strong>Reviewed compilation, not arbitrary execution.</strong>{" "}
          {findingDrivers.value_execution_loop.boundary}
        </p>
      </section>

      <section
        className="qr-case-driver"
        aria-labelledby="qr-case-driver-title"
      >
        <div className="qr-case-section-heading">
          <p className="eyebrow">What drives a finding</p>
          <h3 id="qr-case-driver-title">Values become inspectable evidence</h3>
          <p>
            The interesting part is not the warning count. It is the chain from
            repository signal to selected standard, scoped rule, finding, and
            coverage receipt.
          </p>
        </div>
        <ol className="qr-case-mechanism">
          {findingDrivers.mechanism.map((step, index) => (
            <li key={step.id}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <div>
                <strong>{step.label}</strong>
                <small>{step.detail}</small>
              </div>
            </li>
          ))}
        </ol>
        <div className="qr-case-driver-example">
          <span>Tenure baseline output</span>
          <strong>192 UI-foundations + 179 UI-specificity findings</strong>
          <small>
            alongside structural categories such as hardening, simplification,
            integration, and deduplication.
          </small>
        </div>
      </section>

      <section
        className="qr-case-burndown"
        aria-labelledby="qr-case-burndown-title"
      >
        <div className="qr-case-section-heading">
          <p className="eyebrow">The burndown</p>
          <h3 id="qr-case-burndown-title">
            A large baseline, reconciled in bounded slices
          </h3>
          <p>
            Full scans re-established the baseline after roughly 100 net
            findings or any material scope change. Changed-surface scans kept
            each slice small.
          </p>
        </div>
        <div
          className="qr-case-chart"
          aria-label="Raw code-quality findings by checkpoint"
        >
          {burndown.checkpoints.map((checkpoint) => (
            <div
              key={`${checkpoint.label}-${checkpoint.raw_code_quality_findings}`}
            >
              <span>{checkpoint.label}</span>
              <i
                aria-hidden="true"
                style={{
                  width: `${Math.max(
                    3,
                    (checkpoint.raw_code_quality_findings / maxFindings) * 100,
                  )}%`,
                }}
              />
              <strong>
                {checkpoint.raw_code_quality_findings.toLocaleString()}
              </strong>
            </div>
          ))}
        </div>
        <div className="qr-case-resolution">
          <div>
            <strong>537</strong>
            <span>raw code-quality rows remained</span>
          </div>
          <span aria-hidden="true">≠</span>
          <div>
            <strong>0</strong>
            <span>open actionable findings</span>
          </div>
          <p>
            Exact source review preserved intentional test control flow,
            file-local fixture helpers, UI primitive wrappers, and documented
            environment accessors. Zero actionable never meant zero detector
            output.
          </p>
        </div>
      </section>

      <section className="qr-case-story" aria-labelledby="qr-case-story-title">
        <div className="qr-case-section-heading">
          <p className="eyebrow">The operating model</p>
          <h3 id="qr-case-story-title">From values to a defensible zero</h3>
          <p>
            The workflow prevented both one-shot cleanup and false victory from
            count suppression.
          </p>
        </div>
        <ol className="qr-case-timeline">
          {caseStudy.stages.map((stage, index) => (
            <li key={stage.id}>
              <span className="qr-case-step">
                {String(index + 1).padStart(2, "0")}
              </span>
              <div>
                <h4>{stage.label}</h4>
                <p>{stage.fact}</p>
                <small>{stage.interpretation}</small>
              </div>
            </li>
          ))}
        </ol>
      </section>

      <section
        className="qr-case-vignette"
        aria-labelledby="qr-case-vignette-title"
      >
        <div>
          <p className="eyebrow">Second proof · August branch reconciliation</p>
          <h3 id="qr-case-vignette-title">Why review remains essential</h3>
          <p>
            A later 684-candidate comparison rehearsed the apparent bulk route.
            Changing 127 files produced 15 integration or type errors, so the
            shortcut was rejected before merge.
          </p>
        </div>
        <div className="qr-case-vignette-numbers">
          <span>
            <strong>127</strong> files rehearsed
          </span>
          <span>
            <strong>15</strong> errors
          </span>
          <span>
            <strong>0</strong> unsafe bulk merges
          </span>
        </div>
      </section>

      <section className="qr-case-proof" aria-labelledby="qr-case-proof-title">
        <div className="qr-case-section-heading">
          <p className="eyebrow">The receipt</p>
          <h3 id="qr-case-proof-title">Historical evidence, explicit limits</h3>
          <p>
            The July baseline was dirty, the final run directory is not
            retained, and gate execution remained blocked without consent. The
            tracked final state at its exact evidence commit is the historical
            receipt.
          </p>
        </div>
        <div className="qr-case-proof-line">
          {claimLedger.levels.map((level) => (
            <div
              key={level.id}
              className={
                level.public_claim_allowed ? undefined : "is-unverified"
              }
            >
              <ClaimIcon id={level.id} />
              <span>{level.label}</span>
              <strong>{level.status_display}</strong>
              <small>{level.evidence_summary}</small>
            </div>
          ))}
        </div>
        <div className="qr-case-boundary">
          <ShieldCheck size={17} aria-hidden="true" />
          <strong>Critical boundary</strong>
          <p>
            This is a historical Tenure case with exact provenance. It is not
            proof of the current checkout, a fresh gate run, or production
            deployment.
          </p>
        </div>
      </section>

      <footer className="qr-case-appendix">
        <div>
          <p className="eyebrow">Reproducibility appendix</p>
          <h3>A small deterministic replay, kept in its proper role</h3>
          <p>
            The synthetic fixture can replay the evidence contract without
            replacing the real 4,022-finding Tenure case.
          </p>
        </div>
        <code>{caseStudy.reproducibility_appendix.path}</code>
      </footer>
    </article>
  );
}
