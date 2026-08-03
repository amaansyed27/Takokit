import { RouteLink } from "../app/router";

export function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="shell footer-grid">
        <div className="footer-intro">
          <img src="/brand/takokit-mark.svg" alt="" />
          <div>
            <strong>Takokit</strong>
            <p>Run open voice models locally.</p>
          </div>
        </div>
        <div>
          <span>Product</span>
          <RouteLink href="/models">Models</RouteLink>
          <RouteLink href="/download">Download</RouteLink>
        </div>
        <div>
          <span>Developers</span>
          <RouteLink href="/docs">Documentation</RouteLink>
          <a href="/v1/registry.json">Registry API</a>
        </div>
        <div>
          <span>Source</span>
          <a href="https://github.com/amaansyed27/Takokit">GitHub</a>
          <a href="https://github.com/amaansyed27/Takokit/releases">Releases</a>
        </div>
      </div>
    </footer>
  );
}
