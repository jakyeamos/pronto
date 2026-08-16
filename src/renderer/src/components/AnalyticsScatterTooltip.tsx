import type { ReactElement } from "react";
import { formatAnalyticsNumber } from "./AnalyticsChartFormatting";

interface QualityScatterPoint {
  name: string;
  maturity: number;
  evidence: number;
}

export interface QualityScatterTooltipPayload {
  payload?: QualityScatterPoint;
}

export function QualityScatterTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: ReadonlyArray<QualityScatterTooltipPayload>;
}): ReactElement | null {
  const point = payload?.[0]?.payload;
  if (!active || !point) return null;
  return (
    <div className="analytics-tooltip" role="status">
      <strong>{point.name}</strong>
      <span>Maturity {formatAnalyticsNumber(point.maturity)}</span>
      <span>Evidence {formatAnalyticsNumber(point.evidence)}</span>
    </div>
  );
}
