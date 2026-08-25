import type { ReactElement } from "react";
import type { RepositorySnapshot } from "../types";
import { QualityMaturityWithCacheSummary } from "./CacheDesignSummary";
import {
  QualityFindingsSummary,
  QualityRunnerCommandRunsSummary,
  TestPortfolioSummary,
  WebReadinessSummary,
  projectQualityReadinessForTarget,
} from "./QualityComponents";

export function RepositoryQualityOverview({
  repository,
  targetBranch,
  targetCommit,
  onOpenReport,
}: {
  repository: RepositorySnapshot;
  targetBranch?: string;
  targetCommit?: string;
  onOpenReport?: (reportPath: string) => void;
}): ReactElement {
  const targetReadiness = projectQualityReadinessForTarget(
    repository.quality.ci_readiness,
    repository.quality.gates,
    targetBranch,
    targetCommit,
  );
  return (
    <>
      <QualityMaturityWithCacheSummary
        maturity={repository.quality.maturity}
        readiness={targetReadiness.readiness}
        targetBranch={targetBranch}
        targetCommit={targetCommit}
        targetReadinessState={targetReadiness.state}
        onOpenReport={onOpenReport}
      />
      <QualityFindingsSummary
        findings={repository.quality.findings}
        targetBranch={targetBranch}
        targetCommit={targetCommit}
        onOpenReport={onOpenReport}
      />
      <WebReadinessSummary
        webReadiness={repository.quality.web_readiness}
        onOpenReport={onOpenReport}
      />
      <TestPortfolioSummary
        testPortfolio={repository.quality.test_portfolio}
        onOpenReport={onOpenReport}
      />
      <QualityRunnerCommandRunsSummary
        commandRuns={repository.quality.qr_command_runs}
        onOpenReport={onOpenReport}
      />
    </>
  );
}
