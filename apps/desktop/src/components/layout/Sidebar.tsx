import { navItems, type PageId } from "../../app/navigation";
import type { RuntimeSnapshot } from "../../lib/types";
import { ServerStatusCard } from "./ServerStatusCard";

type SidebarProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  runtime: RuntimeSnapshot;
};

export function Sidebar({ activePage, onNavigate, runtime }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <strong>Takokit</strong>
        <span>Local Voice AI Runtime</span>
      </div>

      <nav className="nav-list" aria-label="Main navigation">
        {navItems.map((item) => {
          const Icon = item.icon;
          const active = item.id === activePage;

          return (
            <button
              key={item.id}
              className={active ? "nav-item active" : "nav-item"}
              aria-current={active ? "page" : undefined}
              onClick={() => onNavigate(item.id)}
              type="button"
            >
              <Icon size={16} aria-hidden="true" />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="sidebar__footer">
        <ServerStatusCard runtime={runtime} />
        <span className="version-label">Takokit 0.1.0</span>
      </div>
    </aside>
  );
}
