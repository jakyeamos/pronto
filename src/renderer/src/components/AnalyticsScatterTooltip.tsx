import type { ReactElement } from "react";
import { formatAnalyticsNumber } from "./AnalyticsChartFormatting";

interface QualityScatterPoint {
  name: string;
  maturity: number;
  coverage: number;
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
      <span>
        Maturity evidence {formatAnalyticsNumber(point.coverage * 100)}%
      </span>
    </div>
  );
}
