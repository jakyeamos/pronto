import { ChevronDown, ChevronRight, Search } from "lucide-react";
import type { ReactElement } from "react";
import type { TelescopeProjection } from "../types/telescope";
import type { Selection } from "./telescopeSurfaceTypes";

export function TelescopeNavigator({
  projection,
  query,
  collapsedGroups,
  selection,
  onQuery,
  onToggleGroup,
  onSelect,
}: {
  projection: TelescopeProjection;
  query: string;
  collapsedGroups: Set<string>;
  selection: Selection;
  onQuery: (query: string) => void;
  onToggleGroup: (groupId: string) => void;
  onSelect: (selection: Selection) => void;
}): ReactElement {
  const normalized = query.trim().toLowerCase();
  return (
    <aside className="telescope-navigator" aria-label="Architecture navigator">
      <label>
        <Search size={13} />
        <input
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder="Find an entity"
          aria-label="Find a Telescope entity"
        />
      </label>
      <div>
        {projection.groups.map((group) => {
          const groupNodes = projection.nodes.filter(
            (node) =>
              node.group_id === group.id &&
              (!normalized ||
                `${node.label} ${node.kind} ${node.technology}`
                  .toLowerCase()
                  .includes(normalized)),
          );
          if (normalized && groupNodes.length === 0) return null;
          const collapsed = collapsedGroups.has(group.id);
          return (
            <section key={group.id}>
              <button
                className={selection?.id === group.id ? "active" : ""}
                type="button"
                onClick={() => {
                  onSelect({ kind: "group", id: group.id });
                  onToggleGroup(group.id);
                }}
              >
                {collapsed ? (
                  <ChevronRight size={12} />
                ) : (
                  <ChevronDown size={12} />
                )}
                <strong>{group.label}</strong>
                <span>{groupNodes.length}</span>
              </button>
              {!collapsed && (
                <ul>
                  {groupNodes.map((node) => (
                    <li key={node.id}>
                      <button
                        className={selection?.id === node.id ? "active" : ""}
                        type="button"
                        onClick={() => onSelect({ kind: "node", id: node.id })}
                      >
                        <i className={`kind-${node.kind}`} />
                        <span>
                          {node.label}
                          <small>{node.kind}</small>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </aside>
  );
}
