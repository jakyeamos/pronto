import { useState } from "react";
import type { ReactElement } from "react";
import { Pencil, Plus, Save, ShieldCheck, Trash2 } from "lucide-react";
import type { GroupConfig, ProductConfig, RepositorySnapshot } from "../types";

type CollectionKind = "product" | "group";
type CollectionItem = ProductConfig | GroupConfig;

export function PortfolioConfigSurface({
  kind,
  items,
  repositories,
  onSave,
  onDelete,
}: {
  kind: CollectionKind;
  items: CollectionItem[];
  repositories: RepositorySnapshot[];
  onSave: (
    id: string | null,
    name: string,
    repositoryIds: string[],
    releaseMode: string,
  ) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
}): ReactElement {
  const isProduct = kind === "product";
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [repositoryIds, setRepositoryIds] = useState<string[]>([]);
  const [releaseMode, setReleaseMode] = useState("Independent");
  const [isSaving, setIsSaving] = useState(false);

  const resetForm = (): void => {
    setEditingId(null);
    setName("");
    setRepositoryIds([]);
    setReleaseMode("Independent");
  };

  const editItem = (item: CollectionItem): void => {
    setEditingId(item.id);
    setName(item.name);
    setRepositoryIds(item.repository_ids);
    setReleaseMode("release_mode" in item ? item.release_mode : "Independent");
  };

  const toggleRepository = (repositoryId: string): void => {
    setRepositoryIds((current) =>
      current.includes(repositoryId)
        ? current.filter((id) => id !== repositoryId)
        : [...current, repositoryId],
    );
  };

  const submit = async (): Promise<void> => {
    if (!name.trim()) return;
    setIsSaving(true);
    try {
      await onSave(editingId, name, repositoryIds, releaseMode);
      resetForm();
    } finally {
      setIsSaving(false);
    }
  };

  const collectionLabel = isProduct ? "product" : "group";

  return (
    <div className="surface-config-layout">
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Manual configuration</p>
            <h2>{isProduct ? "Products" : "Groups"}</h2>
            <p>
              {isProduct
                ? "Name operational products and keep repository release modes explicit."
                : "Create intentional labels without inferring organization from repository names."}
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={resetForm}
          >
            <Plus size={15} />
            New {collectionLabel}
          </button>
        </div>
        <form
          className="config-form"
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <div className="config-form-heading">
            <div>
              <p className="eyebrow">{editingId ? "Edit" : "Create"}</p>
              <h3>
                {editingId
                  ? "Edit " + collectionLabel
                  : "New " + collectionLabel}
              </h3>
            </div>
            {editingId && (
              <button
                className="button button-quiet"
                type="button"
                onClick={resetForm}
              >
                Cancel
              </button>
            )}
          </div>
          <label className="field-label">
            Name
            <input
              className="text-input"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder={
                isProduct ? "e.g. Portfolio console" : "e.g. Experiments"
              }
              maxLength={80}
            />
          </label>
          {isProduct && (
            <label className="field-label">
              Release mode
              <select
                className="text-input"
                value={releaseMode}
                onChange={(event) => setReleaseMode(event.target.value)}
              >
                <option>Independent</option>
                <option>Coordinated independent versions</option>
                <option>Unified product version</option>
              </select>
            </label>
          )}
          <fieldset className="config-repository-picker">
            <legend>Repositories</legend>
            {repositories.length === 0 ? (
              <p className="field-help">
                Register and refresh a discovery root before attaching
                repositories.
              </p>
            ) : (
              <div className="repository-options">
                {repositories.map((repository) => (
                  <label className="repository-option" key={repository.id}>
                    <input
                      type="checkbox"
                      checked={repositoryIds.includes(repository.id)}
                      onChange={() => toggleRepository(repository.id)}
                    />
                    <span>
                      <strong>{repository.name}</strong>
                      <small>
                        {repository.branch} · {repository.path}
                      </small>
                    </span>
                  </label>
                ))}
              </div>
            )}
          </fieldset>
          <button
            className="button button-primary"
            type="submit"
            disabled={isSaving || !name.trim()}
          >
            <Save size={15} />
            {isSaving ? "Saving…" : "Save " + collectionLabel}
          </button>
        </form>
      </section>
      <section className="surface-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Configured now</p>
            <h2>
              {items.length} {collectionLabel}
              {items.length === 1 ? "" : "s"}
            </h2>
            <p>
              Membership is explicit and remains independent from repository Git
              state.
            </p>
          </div>
        </div>
        {items.length === 0 ? (
          <div className="surface-empty">
            <ShieldCheck size={18} />
            <span>No {collectionLabel}s configured yet.</span>
          </div>
        ) : (
          <div className="config-card-list">
            {items.map((item) => (
              <article className="config-card" key={item.id}>
                <div className="config-card-main">
                  <strong>{item.name}</strong>
                  <span>
                    {item.repository_ids.length} repositor
                    {item.repository_ids.length === 1 ? "y" : "ies"}
                    {"release_mode" in item ? " · " + item.release_mode : ""}
                  </span>
                  <small>
                    {item.repository_ids
                      .map(
                        (repositoryId) =>
                          repositories.find(
                            (repository) => repository.id === repositoryId,
                          )?.name ?? "Unknown repository",
                      )
                      .join(" · ") || "No repositories attached"}
                  </small>
                </div>
                <div className="config-card-actions">
                  <button
                    className="icon-button"
                    type="button"
                    aria-label={"Edit " + item.name}
                    onClick={() => editItem(item)}
                  >
                    <Pencil size={14} />
                  </button>
                  <button
                    className="icon-button"
                    type="button"
                    aria-label={"Delete " + item.name}
                    onClick={() => {
                      if (window.confirm("Delete " + item.name + "?"))
                        void onDelete(item.id);
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
