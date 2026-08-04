import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";
import { PlatformInstall } from "../PlatformInstall";

export function LandingHero() {
  const reducedMotion = useReducedMotion();
  const compactLayout = useMediaQuery("(max-width: 960px)");
  const staticLayout = reducedMotion || compactLayout;
  const sectionRef = useScrollProgress((progress, section) => {
    const unsettled = 1 - progress;
    section.style.setProperty("--hero-top-shift", `${(unsettled * -18).toFixed(2)}px`);
    section.style.setProperty("--hero-left-shift", `${(unsettled * -18).toFixed(2)}px`);
    section.style.setProperty("--hero-right-shift", `${(unsettled * 18).toFixed(2)}px`);
    section.style.setProperty("--hero-lower-shift", `${(unsettled * 13).toFixed(2)}px`);
    section.style.setProperty("--hero-axis-opacity", Math.max(0.15, 0.6 - progress * 0.45).toFixed(3));
    section.style.setProperty("--hero-progress-width", `${(progress * 100).toFixed(2)}%`);
  }, staticLayout);

  function handlePointerMove(event) {
    if (staticLayout) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - bounds.left) / bounds.width - 0.5) * 2;
    const y = ((event.clientY - bounds.top) / bounds.height - 0.5) * 2;
    event.currentTarget.style.setProperty("--pointer-x-positive", `${(x * 5).toFixed(2)}px`);
    event.currentTarget.style.setProperty("--pointer-x-negative", `${(x * -5).toFixed(2)}px`);
    event.currentTarget.style.setProperty("--pointer-y-positive", `${(y * 4).toFixed(2)}px`);
    event.currentTarget.style.setProperty("--pointer-y-negative", `${(y * -4).toFixed(2)}px`);
  }

  function resetPointer(event) {
    event.currentTarget.style.setProperty("--pointer-x-positive", "0px");
    event.currentTarget.style.setProperty("--pointer-x-negative", "0px");
    event.currentTarget.style.setProperty("--pointer-y-positive", "0px");
    event.currentTarget.style.setProperty("--pointer-y-negative", "0px");
  }

  return (
    <section
      className={`landing-hero ${staticLayout ? "is-static" : ""}`}
      ref={sectionRef}
      onPointerMove={handlePointerMove}
      onPointerLeave={resetPointer}
      aria-labelledby="landing-hero-title"
    >
      <div className="landing-hero__stage landing-shell">
        <div className="landing-hero__copy">
          <p className="landing-kicker">Rust-first local voice runtime</p>
          <h1 id="landing-hero-title">Run open voice models locally.</h1>
          <p className="landing-hero__summary">
            Speech generation, transcription, voice cloning, and conversion through one runtime for Windows, Linux, and macOS.
          </p>

          <PlatformInstall heading="Install Takokit" />

          <div className="landing-actions">
            <RouteLink href="/models" className="landing-button landing-button--primary">Explore models</RouteLink>
            <RouteLink href="/docs" className="landing-button">Read the docs</RouteLink>
          </div>
        </div>

        <div className="landing-hero__visual" aria-hidden="true">
          <div className="landing-hero__signal">
            {Array.from({ length: 18 }, (_, index) => <i key={index} />)}
          </div>
          <div className="landing-hero__mark">
            <img className="landing-hero__piece landing-hero__piece--top" src="/brand/takokit-mark.svg" alt="" />
            <img className="landing-hero__piece landing-hero__piece--left" src="/brand/takokit-mark.svg" alt="" />
            <img className="landing-hero__piece landing-hero__piece--right" src="/brand/takokit-mark.svg" alt="" />
            <span className="landing-hero__axis landing-hero__axis--horizontal" />
            <span className="landing-hero__axis landing-hero__axis--vertical" />
          </div>
          <p>MODEL → RUNNER → LOCAL OUTPUT</p>
        </div>

        <div className="landing-hero__scroll" aria-hidden="true">
          <span>Scroll to see the runtime assemble</span>
          <i />
        </div>
      </div>
    </section>
  );
}
