import type { ReactNode } from "react";
import { navItems, type PageId } from "../app/navigation";
import type { RuntimeSnapshot } from "../lib/types";

type AppShellProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  runtime: RuntimeSnapshot;
  children: ReactNode;
};

export function AppShell({ activePage, onNavigate, runtime, children }: AppShellProps) {
  return (
    <div className="app-window">
      <aside className="sidebar">
        <div className="window-controls" aria-hidden="true">
          <span className="window-dot red" />
          <span className="window-dot yellow" />
          <span className="window-dot green" />
        </div>

        <div className="brand">
          <div>
            <strong>Takokit</strong>
            <span>Local Voice AI Runtime</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Main navigation">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={item.id === activePage ? "nav-item active" : "nav-item"}
                onClick={() => onNavigate(item.id)}
                type="button"
              >
                <Icon size={17} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-footer">
          <div className="server-card">
            <div className="server-card-title">
              <span>Server Status</span>
              <span className="running-dot" />
              <strong>{runtime.server.status === "online" ? "Running" : "Offline"}</strong>
            </div>
            <code>{runtime.server.url}</code>
            <span>Uptime: {runtime.server.uptime}</span>
          </div>
          <span className="version-label">Takokit 0.1.0</span>
        </div>
      </aside>

      <main className="main-surface">
        {children}
      </main>
    </div>
  );
}
