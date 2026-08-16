import { lazy, Suspense } from "react";
import type { ReactElement } from "react";
import type {
  AnalyticsSnapshot,
  GroupConfig,
  ProductConfig,
  RepositorySnapshot,
} from "../types";

const AnalyticsSurface = lazy(() =>
  import("./AnalyticsComponents").then((module) => ({
    default: module.AnalyticsSurface,
  })),
);

export function AnalyticsRoute({
  analytics,
  repositories,
  groups,
  products,
}: {
  analytics: AnalyticsSnapshot;
  repositories: RepositorySnapshot[];
  groups: GroupConfig[];
  products: ProductConfig[];
}): ReactElement {
  return (
    <Suspense
      fallback={
        <div className="surface-loading" role="status">
          Loading analytics workspace…
        </div>
      }
    >
      <AnalyticsSurface
        analytics={analytics}
        repositories={repositories}
        groups={groups}
        products={products}
      />
    </Suspense>
  );
}
