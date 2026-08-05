import { useMemo, useState } from "react";
import { RouteLink } from "../../app/router";

export function DocsSidebar({ groups, slug }) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const filteredGroups = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return groups;
    return groups
      .map((group) => ({
        ...group,
        pages: group.pages.filter(([, title]) => title.toLowerCase().includes(normalized)),
      }))
      .filter((group) => group.pages.length > 0);
  }, [groups, query]);

  return (
    <div className="docs-sidebar">
      <button
        className="docs-sidebar__toggle"
        type="button"
        aria-expanded={open}
        aria-controls="docs-navigation"
        onClick={() => setOpen((value) => !value)}
      >
        Documentation navigation
        <span aria-hidden="true">{open ? "−" : "+"}</span>
      </button>

      <aside
        id="docs-navigation"
        className={open ? "docs-sidebar__panel is-open" : "docs-sidebar__panel"}
        aria-label="Documentation"
      >
        <label className="docs-search">
          <span>Search documentation</span>
          <input
            type="search"
            value={query}
            placeholder="Search pages"
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>

        <nav className="docs-navigation">
          {filteredGroups.map((group) => (
            <section key={group.title}>
              <h2>{group.title}</h2>
              {group.pages.map(([id, title]) => (
                <RouteLink
                  key={id}
                  href={`/docs/${id}`}
                  className={id === slug ? "is-active" : ""}
                  aria-current={id === slug ? "page" : undefined}
                  onClick={() => setOpen(false)}
                >
                  {title}
                </RouteLink>
              ))}
            </section>
          ))}
          {!filteredGroups.length && <p className="docs-navigation__empty">No pages match.</p>}
        </nav>
      </aside>
    </div>
  );
}
