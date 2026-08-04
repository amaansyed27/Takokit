import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";
import { PlatformInstall } from "../PlatformInstall";

export function LandingHero() {
  const reducedMotion = useReducedMotion();
  const compactLayout = useMediaQuery("(max-width: 900px)");
  const staticLayout = reducedMotion || compactLayout;
  const sectionRef = useScrollProgress((progress, section) => {
    const bounded = Math.min(1, Math.max(0, progress));
    section.style.setProperty("--hero-mark-y", `${(-18 * bounded).toFixed(2)}px`);
    section.style.setProperty("--hero-mark-scale", (1 - 0.08 * bounded).toFixed(4));
    section.style.setProperty("--hero-progress-width", `${(100 * bounded).toFixed(2)}%`);
    section.style.setProperty("--hero-wave-opacity", (0.25 + 0.45 * bounded).toFixed(4));
  }, staticLayout);

  return (
    <section
      className={`landing-hero ${staticLayout ? "is-static" : ""}`}
      ref={sectionRef}
      aria-labelledby="landing-hero-title"
    >
      <div className="landing-hero__stage landing-shell">
        <div className="landing-hero__mark" aria-hidden="true">
          <span className="landing-hero__wave">
            {Array.from({ length: 15 }, (_, index) => <i key={index} />)}
          </span>
          <img src="/brand/takokit-mark.svg" alt="" />
        </div>

        <div className="landing-hero__copy">
          <p className="landing-kicker">Local voice runtime</p>
          <h1 id="landing-hero-title">Run open voice models locally.</h1>
          <p className="landing-hero__summary">
            One runtime for speech generation, transcription, voice cloning, and conversion across Windows, Linux, and macOS.
          </p>

          <PlatformInstall heading="Install Takokit" />

          <div className="landing-actions">
            <RouteLink href="/models" className="landing-button landing-button--primary">Explore models</RouteLink>
            <RouteLink href="/docs" className="landing-button">Read documentation</RouteLink>
          </div>
        </div>

        <div className="landing-hero__scroll" aria-hidden="true">
          <span>Scroll to open the runtime</span>
          <i />
        </div>
      </div>
    </section>
  );
}
