import { useState } from "react";
import { RouteLink } from "../app/router";

const links = [
  ["/models", "Models"],
  ["/docs", "Docs"],
  ["https://github.com/amaansyed27/Takokit", "GitHub"],
  ["/download", "Download"],
];

export function SiteHeader({ pathname }) {
  const [open, setOpen] = useState(false);
  return (
    <header className="site-header">
      <div className="shell header-inner">
        <RouteLink href="/" className="brand" aria-label="Takokit home">
          <img src="/brand/takokit-lockup.svg" alt="Takokit" />
        </RouteLink>
        <button
          className="menu-button"
          type="button"
          aria-expanded={open}
          aria-controls="site-navigation"
          onClick={() => setOpen((value) => !value)}
        >
          Menu
        </button>
        <nav id="site-navigation" className={open ? "site-nav is-open" : "site-nav"}>
          {links.map(([href, label]) => {
            const external = href.startsWith("http");
            const active = !external && pathname.startsWith(href);
            const Link = external ? "a" : RouteLink;
            return (
              <Link
                key={href}
                href={href}
                className={active ? "is-active" : ""}
                aria-current={active ? "page" : undefined}
                onClick={() => setOpen(false)}
              >
                {label}
              </Link>
            );
          })}
        </nav>
      </div>
    </header>
  );
}
