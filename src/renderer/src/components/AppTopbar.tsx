import {
  ChevronRight,
  Command,
  LoaderCircle,
  RefreshCw,
  Search,
} from "lucide-react";
import type { ReactElement, RefObject } from "react";
import { IconButton } from "./ConsolePrimitives";

export function AppTopbar({
  activeNavLabel,
  isPortfolio,
  repositoryName,
  query,
  searchInputRef,
  isRefreshing,
  onQueryChange,
  onRefresh,
  onBackToPortfolio,
}: {
  activeNavLabel: string | undefined;
  isPortfolio: boolean;
  repositoryName?: string;
  query: string;
  searchInputRef: RefObject<HTMLInputElement | null>;
  isRefreshing: boolean;
  onQueryChange: (query: string) => void;
  onRefresh: () => void;
  onBackToPortfolio?: () => void;
}): ReactElement {
  return (
    <header className="topbar">
      <div className="breadcrumbs">
        <span>Workspace</span>
        <ChevronRight size={13} />
        {repositoryName && onBackToPortfolio ? (
          <>
            <button
              className="breadcrumb-button"
              type="button"
              onClick={onBackToPortfolio}
            >
              {activeNavLabel}
            </button>
            <ChevronRight size={13} />
            <strong>{repositoryName}</strong>
          </>
        ) : (
          <strong>{activeNavLabel}</strong>
        )}
      </div>
      <div className="topbar-actions">
        <label className="search-box">
          <Search size={15} />
          <input
            ref={searchInputRef}
            aria-label="Search repositories"
            placeholder={
              isPortfolio
                ? repositoryName
                  ? "Search this portfolio"
                  : "Search repos, branches, paths"
                : "Search local portfolio"
            }
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
          />
          <kbd>
            <Command size={11} />K
          </kbd>
        </label>
        <IconButton
          label="Refresh local evidence"
          onClick={onRefresh}
          disabled={isRefreshing}
        >
          {isRefreshing ? (
            <LoaderCircle className="spin" size={17} />
          ) : (
            <RefreshCw size={17} />
          )}
        </IconButton>
        <div className="avatar">JA</div>
      </div>
    </header>
  );
}
