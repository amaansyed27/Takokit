import { RouteLink } from "../../app/router";

const STEPS = [
  ["Pull", "Download a model and the runner it needs."],
  ["Run", "Use it through the CLI, TUI, GUI, or API."],
  ["Reuse", "Keep models, voices, and outputs ready locally."],
];

export function FinalCTA() {
  return (
    <section className="landing-closing" aria-labelledby="landing-closing-title">
      <div className="landing-shell">
        <ol className="landing-closing__steps">
          {STEPS.map(([title, description], index) => (
            <li key={title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <h3>{title}</h3>
              <p>{description}</p>
            </li>
          ))}
        </ol>

        <div className="landing-closing__cta">
          <img src="/brand/takokit-mark.svg" alt="" aria-hidden="true" />
          <div>
            <p className="landing-kicker">Takokit</p>
            <h2 id="landing-closing-title">Start running voice models locally.</h2>
            <div className="landing-actions">
              <RouteLink href="/download" className="landing-button landing-button--primary">Install Takokit</RouteLink>
              <RouteLink href="/docs" className="landing-button">Read documentation</RouteLink>
              <a href="https://github.com/amaansyed27/Takokit" className="landing-text-link">GitHub →</a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
