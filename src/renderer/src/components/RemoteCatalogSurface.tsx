import type { ReactElement } from "react";
import {
  ExternalLink,
  GitBranch as Github,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import type {
  ProviderIdentity,
  ProviderStatus,
  RemoteRepositorySnapshot,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

export function RemoteCatalogSurface({
  status,
  identities,
  repositories,
  isRefreshing,
  onRefresh,
}: {
  status: ProviderStatus;
  identities: ProviderIdentity[];
  repositories: RemoteRepositorySnapshot[];
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
}): ReactElement {
  return (
    <div className="remote-catalog-layout">
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Read-only provider boundary</p>
            <h2>GitHub remote catalog</h2>
            <p>
              Only GitHub repositories connected to an in-scope local checkout
              are shown here.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh GitHub"}
          </button>
        </div>
        <div className="provider-status-card">
          <div className="provider-status-icon">
            <Github size={19} />
          </div>
          <div>
            <div className="provider-status-heading">
              <strong>{status.provider}</strong>
              <StatusPill tone={status.state === "Ready" ? "mint" : "amber"}>
                {status.state}
              </StatusPill>
            </div>
            <p>{status.message}</p>
            <small>
              {status.last_refresh_at
                ? "Last refresh " + formatTime(status.last_refresh_at)
                : "No successful provider refresh yet"}
            </small>
          </div>
        </div>
        {identities.length > 0 && (
          <div className="provider-identity-list">
            {identities.map((identity) => (
              <div className="provider-identity" key={identity.id}>
                <ShieldCheck size={15} />
                <span>
                  <strong>{identity.login}</strong>
                  <small>
                    {identity.display_name || "Authenticated GitHub identity"} ·{" "}
                    {identity.credential_state}
                  </small>
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Provider snapshot</p>
            <h2>{repositories.length} repositories</h2>
            <p>
              Remote-only GitHub repositories are omitted until a matching local
              checkout is discovered.
            </p>
          </div>
        </div>
        {repositories.length === 0 ? (
          <div className="surface-empty">
            <Github size={18} />
            <span>
              Refresh GitHub after authenticating and connecting local
              repositories to their remotes.
            </span>
          </div>
        ) : (
          <div className="remote-repository-list">
            {repositories.map((repository) => (
              <article className="remote-repository-card" key={repository.id}>
                <div>
                  <div className="remote-repository-heading">
                    <strong>{repository.full_name}</strong>
                    <StatusPill tone="mint">{repository.locality}</StatusPill>
                  </div>
                  <small>
                    {repository.archived ? "Archived" : "Active"} · default{" "}
                    {repository.default_branch || "unknown"} · refreshed{" "}
                    {formatTime(repository.last_refreshed_at)}
                  </small>
                </div>
                <a
                  className="icon-button"
                  href={repository.html_url}
                  target="_blank"
                  rel="noreferrer"
                  aria-label={"Open " + repository.full_name + " on GitHub"}
                >
                  <ExternalLink size={14} />
                </a>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
