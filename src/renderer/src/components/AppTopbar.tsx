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
  isCommandCenter,
  query,
  searchInputRef,
  isRefreshing,
  onQueryChange,
  onRefresh,
}: {
  activeNavLabel: string | undefined;
  isCommandCenter: boolean;
  query: string;
  searchInputRef: RefObject<HTMLInputElement | null>;
  isRefreshing: boolean;
  onQueryChange: (query: string) => void;
  onRefresh: () => void;
}): ReactElement {
  return (
    <header className="topbar">
      <div className="breadcrumbs">
        <span>Workspace</span>
        <ChevronRight size={13} />
        <strong>{activeNavLabel}</strong>
      </div>
      <div className="topbar-actions">
        <label className="search-box">
          <Search size={15} />
          <input
            ref={searchInputRef}
            aria-label="Search repositories"
            placeholder={
              isCommandCenter
                ? "Search repos, branches, paths"
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
