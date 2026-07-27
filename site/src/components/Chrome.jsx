import { useState } from "react";
import { RouteLink } from "../router";

export function Header() {
  const [open, setOpen] = useState(false);
  return <header className="site-header">
    <div className="shell header-inner">
      <RouteLink href="/" className="brand" aria-label="Takokit home">
        <img className="lockup" src="/brand/takokit-lockup.svg" alt="Takokit" />
      </RouteLink>
      <button className="menu-button" onClick={() => setOpen(!open)} aria-expanded={open}>Menu</button>
      <nav className={open ? "nav open" : "nav"} onClick={() => setOpen(false)}>
        <RouteLink href="/library">Models</RouteLink>
        <RouteLink href="/docs">Docs</RouteLink>
        <RouteLink href="/download">Download</RouteLink>
        <a href="https://github.com/amaansyed27/Takokit">GitHub</a>
      </nav>
    </div>
  </header>;
}

export function Footer() {
  return <footer className="footer"><div className="shell footer-grid">
    <div><strong>Takokit</strong><p>Run open speech models locally through one CLI, desktop app, and API.</p></div>
    <div><span>Product</span><RouteLink href="/library">Models</RouteLink><RouteLink href="/download">Download</RouteLink></div>
    <div><span>Developers</span><RouteLink href="/docs">Documentation</RouteLink><a href="/v1/registry.json">Registry JSON</a></div>
    <div><span>Source</span><a href="https://github.com/amaansyed27/Takokit">GitHub</a><a href="https://github.com/amaansyed27/Takokit/releases">Releases</a></div>
  </div></footer>;
}

export function CopyCommand({ children }) {
  const copy = async () => navigator.clipboard.writeText(children);
  return <div className="command"><code>{children}</code><button onClick={copy}>Copy</button></div>;
}
