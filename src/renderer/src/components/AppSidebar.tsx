import type { ReactElement } from "react";
import { ShieldCheck } from "lucide-react";
import { navItems, type NavItem } from "../navigation";

export function AppSidebar({
  activeNav,
  activeConditionCount,
  rootCount,
  onNavigate,
}: {
  activeNav: NavItem;
  activeConditionCount: number;
  rootCount: number;
  onNavigate: (nav: NavItem) => void;
}): ReactElement {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">P</div>
        <div>
          <strong>Pronto</strong>
          <span>Portfolio command center</span>
        </div>
      </div>
      <div className="sidebar-rule" />
      <nav className="primary-nav" aria-label="Primary navigation">
        {navItems.map(({ id, label, icon: Icon }) => (
          <button
            className={`nav-item ${activeNav === id ? "nav-item-active" : ""}`}
            type="button"
            key={id}
            aria-current={activeNav === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
          >
            <Icon size={17} />
            <span>{label}</span>
            {id === "command" && activeConditionCount > 0 && (
              <span className="nav-count">{activeConditionCount}</span>
            )}
          </button>
        ))}
      </nav>
      <div className="sidebar-bottom">
        <div className="local-status">
          <span className="status-beacon" />
          <div>
            <strong>Local evidence only</strong>
            <span>
              {rootCount} discovery root{rootCount === 1 ? "" : "s"}
            </span>
          </div>
        </div>
        <div className="privacy-card">
          <ShieldCheck size={16} />
          <p>
            <strong>Private by default</strong>
            <span>Source and uncommitted diff content stay local.</span>
          </p>
        </div>
      </div>
    </aside>
  );
}
