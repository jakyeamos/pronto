// @vitest-environment happy-dom
// quality-gate: allow static-ui-test: verifies read-only authority language and generic ledger projection
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  QrCommandRunLedgerSnapshot,
  TestPortfolioSnapshot,
} from "./QualityComponents.test-support";
import {
  QualityRunnerCommandRunsSummary,
  TestPortfolioSummary,
} from "./QualityComponents.test-support";

describe("Quality Runner command run summaries", () => {
  it("rolls audit and removal-proof runs up without granting deletion authority", () => {
    const testPortfolio: TestPortfolioSnapshot = {
      status: "Blocked",
      implementation_allowed: false,
      detail: "The latest matching removal proof is blocked.",
      audit: {
        schema: "quality-runner-test-portfolio-audit-v0.1",
        run_id: "portfolio-1",
        operation: "portfolio-audit",
        status: "review-required",
        freshness: "Fresh",
        observed_at: "2026-08-25T13:00:00Z",
        scanned_commit: "abc",
        scanned_branch: "main",
        report_path:
          "/tmp/repo/.quality-runner/runs/portfolio-1/test-portfolio-audit.json",
        artifact_hash: "audit-hash",
        removal_candidate_count: 1,
        insufficient_evidence_count: 2,
        removed_test_count: 0,
        blocker_ids: [],
        detail: "Reviewer judgments are advisory.",
      },
      removal_proof: {
        schema: "quality-runner-test-removal-proof-v0.1",
        run_id: "removal-1",
        operation: "removal-proof",
        status: "blocked",
        freshness: "Fresh",
        observed_at: "2026-08-25T13:01:00Z",
        scanned_commit: "abc",
        scanned_branch: "main",
        report_path:
          "/tmp/repo/.quality-runner/runs/removal-1/test-removal-proof.json",
        artifact_hash: "proof-hash",
        source_audit_hash: "audit-hash",
        removal_candidate_count: 0,
        insufficient_evidence_count: 0,
        removed_test_count: 1,
        blocker_ids: ["mutation-preservation"],
        detail: "Removal evidence is blocked.",
      },
    };
    const markup = renderToStaticMarkup(
      <TestPortfolioSummary
        testPortfolio={testPortfolio}
        onOpenReport={() => undefined}
      />,
    );
    expect(markup).toContain("Test portfolio evidence");
    expect(markup).toContain("portfolio-1");
    expect(markup).toContain("removal-1");
    expect(markup).toContain("1 candidates");
    expect(markup).toContain("2 insufficient");
    expect(markup).toContain("1 blockers");
    expect(markup).toContain(
      "never authorizes source mutation or test deletion",
    );
  });

  it("gives every Quality Runner command run a generic ledger destination", () => {
    const commandRuns: QrCommandRunLedgerSnapshot = {
      status: "Available",
      implementation_allowed: false,
      total_count: 3,
      visible_count: 3,
      invalid_count: 0,
      mode_counts: {
        inspect: 1,
        "test-portfolio-audit": 1,
        "test-removal-proof": 1,
      },
      detail: "Every Quality Runner command run is counted by mode.",
      runs: [
        {
          run_id: "later-general-run",
          mode: "inspect",
          status: "Imported",
          freshness: "Fresh",
          observed_at: "2026-08-25T13:02:00Z",
          scanned_commit: "abc",
          scanned_branch: "main",
          artifact_count: 4,
          manifest_path:
            "/tmp/repo/.quality-runner/runs/later-general-run/run-manifest.json",
          detail: "Run manifest is available for repository-level roll-up.",
        },
      ],
    };
    const markup = renderToStaticMarkup(
      <QualityRunnerCommandRunsSummary
        commandRuns={commandRuns}
        onOpenReport={() => undefined}
      />,
    );
    expect(markup).toContain("Quality Runner command runs");
    expect(markup).toContain("3 total runs");
    expect(markup).toContain("test-portfolio-audit: 1");
    expect(markup).toContain("later-general-run");
    expect(markup).toContain("Run manifest");
  });
});
