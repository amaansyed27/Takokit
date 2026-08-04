import { RouteLink } from "../../app/router";
import { PlatformInstall } from "../PlatformInstall";
import { useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

export function CinematicHero() {
  const reducedMotion = useReducedMotion();
  const sectionRef = useScrollProgress((progress, section) => {
    const bounded = Math.min(1, Math.max(0, progress));
    section.style.setProperty("--tk-hero-progress", bounded.toFixed(4));
  }, reducedMotion);

  return (
    <section className="tk-hero" ref={sectionRef} aria-labelledby="tk-hero-title">
      <div className="tk-hero__grid shell">
        <div className="tk-hero__copy">
          <p className="tk-kicker">TAKOKIT / LOCAL VOICE RUNTIME</p>
          <h1 id="tk-hero-title">
            <span>OPEN VOICE MODELS.</span>
            <span>ONE LOCAL RUNTIME.</span>
          </h1>
          <p className="tk-hero__summary">
            Pull, run, inspect, and reuse speech models through one Rust-first system for Windows, Linux, and macOS.
          </p>

          <PlatformInstall heading="Install for your machine" />

          <div className="tk-hero__actions">
            <RouteLink href="/models" className="tk-action tk-action--primary">Explore models</RouteLink>
            <RouteLink href="/docs" className="tk-action">Read the docs</RouteLink>
          </div>
        </div>

        <div className="tk-hero__visual" aria-hidden="true">
          <div className="tk-hero__mark-field">
            <div className="tk-hero__orbit" />
            <img src="/brand/takokit-mark.svg" alt="" />
            <div className="tk-hero__wave">
              {Array.from({ length: 18 }, (_, index) => <i key={index} />)}
            </div>
          </div>
          <p>CLI · TUI · GUI · API</p>
        </div>
      </div>

      <div className="tk-hero__scroll shell" aria-hidden="true">
        <span>SCROLL TO SEE THE SYSTEM</span>
        <i />
      </div>
    </section>
  );
}
