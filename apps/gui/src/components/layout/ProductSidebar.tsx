import { ChevronRight } from "lucide-react";
import { navSections, type PageId } from "../../app/navigation";
import takokitMark from "../../assets/takokit-mark.svg";
import type { RuntimeSnapshot } from "../../lib/types";

type ProductSidebarProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  runtime: RuntimeSnapshot;
};

export function ProductSidebar({ activePage, onNavigate, runtime }: ProductSidebarProps) {
  const isOnline = runtime.server.status === "online";

  return (
    <aside className="tk-sidebar">
      <button className="tk-brand" type="button" onClick={() => onNavigate("home")}>
        <span className="tk-brand__mark" aria-hidden="true">
          <img src={takokitMark} alt="" />
        </span>
        <span className="tk-brand__copy">
          <strong>Takokit</strong>
          <small>Local voice runtime</small>
        </span>
      </button>

      <nav className="tk-nav" aria-label="Main navigation">
        {navSections.map((section, sectionIndex) => (
          <div className="tk-nav__section" key={section.label ?? `primary-${sectionIndex}`}>
            {section.label ? <span className="tk-nav__label">{section.label}</span> : null}
            <div className="tk-nav__items">
              {section.items.map((item) => {
                const Icon = item.icon;
                const active = item.id === activePage;
                return (
                  <button
                    key={item.id}
                    className={active ? "tk-nav-item is-active" : "tk-nav-item"}
                    type="button"
                    aria-current={active ? "page" : undefined}
                    onClick={() => onNavigate(item.id)}
                  >
                    <Icon size={16} strokeWidth={1.8} aria-hidden="true" />
                    <span>{item.label}</span>
                    {active ? <ChevronRight className="tk-nav-item__arrow" size={13} aria-hidden="true" /> : null}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      <div className="tk-sidebar__footer">
        <div className="tk-runtime-state">
          <span className={isOnline ? "tk-status-dot is-online" : "tk-status-dot"} aria-hidden="true" />
          <span>
            <strong>{isOnline ? "Runtime ready" : "Runtime offline"}</strong>
            <small>{runtime.models.filter((model) => model.executable).length} models ready</small>
          </span>
        </div>
        <span className="tk-build-label" title={runtime.buildId}>Build {runtime.buildId.slice(0, 8)}</span>
      </div>
    </aside>
  );
}
