import { RouteLink } from "../../app/router";
import { PlatformInstall } from "../PlatformInstall";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

export function CinematicHero() {
  const reducedMotion = useReducedMotion();
  const narrowLayout = useMediaQuery("(max-width: 980px)");
  const staticLayout = reducedMotion || narrowLayout;
  const sectionRef = useScrollProgress((progress, section) => {
    const light = Math.min(1, progress / 0.62);
    const copyExit = Math.max(0, Math.min(1, (progress - 0.58) / 0.28));
    section.style.setProperty("--tk-hero-progress", progress.toFixed(4));
    section.style.setProperty("--tk-hero-light", light.toFixed(4));
    section.style.setProperty("--tk-hero-copy", (1 - copyExit).toFixed(4));
    section.style.setProperty("--tk-hero-lift", `${(-46 * copyExit).toFixed(2)}px`);
  }, staticLayout);

  return (
    <section
      className={`tk-hero ${staticLayout ? "is-static" : ""}`}
      ref={sectionRef}
      aria-labelledby="tk-hero-title"
    >
      <div className="tk-hero__stage">
        <div className="tk-hero__ambient" aria-hidden="true">
          <i />
          <i />
          <i />
        </div>

        <div className="tk-hero__copy">
          <p className="tk-kicker">TAKOKIT / LOCAL VOICE RUNTIME</p>
          <h1 id="tk-hero-title">
            <span>VOICE MODELS.</span>
            <span>ONE LOCAL</span>
            <span>RUNTIME.</span>
          </h1>
          <p className="tk-hero__summary">
            Pull, run, inspect, and reuse open speech models through one Rust-first system—without rebuilding a fragile stack for every project.
          </p>

          <PlatformInstall heading="Install for your machine" />

          <div className="tk-hero__actions">
            <RouteLink href="/models" className="tk-action tk-action--primary">Explore models</RouteLink>
            <RouteLink href="/docs" className="tk-action">Read the docs</RouteLink>
          </div>
        </div>

        <div className="tk-hero__visual" aria-hidden="true">
          <div className="tk-hero__disc">
            <div className="tk-hero__rings">
              <i />
              <i />
              <i />
            </div>
            <img src="/brand/takokit-mark.svg" alt="" />
            <div className="tk-hero__wave">
              {Array.from({ length: 22 }, (_, index) => <i key={index} />)}
            </div>
          </div>
          <div className="tk-hero__platforms">CLI · TUI · GUI · API</div>
        </div>

        <div className="tk-hero__scroll" aria-hidden="true">
          <span>SCROLL TO BUILD THE RUNTIME</span>
          <i />
        </div>
      </div>
    </section>
  );
}
