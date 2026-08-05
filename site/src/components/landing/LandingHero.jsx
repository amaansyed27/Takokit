import { RouteLink } from "../../app/router";
import { CommandBar } from "../CommandBar";

export function LandingHero() {
  return (
    <section className="landing-hero" aria-labelledby="landing-hero-title">
      <div className="landing-hero__stage landing-shell">
        <div className="landing-hero__mark" aria-hidden="true">
          <span className="landing-hero__wave">
            {Array.from({ length: 9 }, (_, index) => <i key={index} />)}
          </span>
          <img src="/brand/takokit-mark.svg" alt="" />
        </div>

        <div className="landing-hero__copy">
          <p className="landing-kicker">Local voice runtime</p>
          <h1 id="landing-hero-title">Run open voice models locally.</h1>
          <p className="landing-hero__summary">
            One runtime for speech generation, transcription, voice cloning, and conversion across Windows, Linux, and macOS.
          </p>

          <div className="landing-hero__quickstart">
            <span>Start with a model</span>
            <CommandBar label="Pull Kokoro">tako pull kokoro</CommandBar>
          </div>

          <div className="landing-actions">
            <RouteLink href="/download" className="landing-button landing-button--primary">Download for Windows</RouteLink>
            <RouteLink href="/models" className="landing-button">Browse models</RouteLink>
          </div>
        </div>

        <div className="landing-hero__scroll" aria-hidden="true">
          <span>Scroll to explore</span>
          <i />
        </div>
      </div>
    </section>
  );
}
