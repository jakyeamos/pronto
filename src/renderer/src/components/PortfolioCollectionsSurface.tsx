import { ChevronDown } from "lucide-react";
import type { ReactElement } from "react";
import type { ProductConfig, GroupConfig, RepositorySnapshot } from "../types";
import { PortfolioConfigSurface } from "./PortfolioConfigSurface";

export function PortfolioCollectionsSurface({
  groups,
  products,
  repositories,
  onSaveGroup,
  onDeleteGroup,
  onSaveProduct,
  onDeleteProduct,
}: {
  groups: GroupConfig[];
  products: ProductConfig[];
  repositories: RepositorySnapshot[];
  onSaveGroup: (
    id: string | null,
    name: string,
    repositoryIds: string[],
    releaseMode: string,
  ) => Promise<void>;
  onDeleteGroup: (id: string) => Promise<void>;
  onSaveProduct: (
    id: string | null,
    name: string,
    repositoryIds: string[],
    releaseMode: string,
  ) => Promise<void>;
  onDeleteProduct: (id: string) => Promise<void>;
}): ReactElement {
  return (
    <div className="collection-surface">
      <PortfolioConfigSurface
        kind="group"
        items={groups}
        repositories={repositories}
        onSave={onSaveGroup}
        onDelete={onDeleteGroup}
      />
      <details className="surface-panel collection-subsection" open>
        <summary className="collection-subsection-summary">
          <div>
            <p className="eyebrow">Groups · release sublabel</p>
            <strong>Release products</strong>
            <span>
              Keep release-oriented repository sets here without adding a second
              top-level destination.
            </span>
          </div>
          <ChevronDown size={17} aria-hidden="true" />
        </summary>
        <div className="collection-subsection-content">
          <PortfolioConfigSurface
            kind="product"
            items={products}
            repositories={repositories}
            onSave={onSaveProduct}
            onDelete={onDeleteProduct}
          />
        </div>
      </details>
    </div>
  );
}
