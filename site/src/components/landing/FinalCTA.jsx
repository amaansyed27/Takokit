import { RouteLink } from "../../app/router";

const STEPS = [
  ["01", "Pull", "Resolve a model, version, runner, and pinned artifacts."],
  ["02", "Run", "Use the same local system through every Takokit interface."],
  ["03", "Reuse", "Keep models, voices, sessions, and outputs ready on your machine."],
];

export function FinalCTA() {
  return (
    <section className="landing-closing" aria-labelledby="landing-closing-title">
      <div className="landing-shell">
        <ol className="landing-closing__steps">
          {STEPS.map(([number, title, description]) => (
            <li key={title}>
              <span>{number}</span>
              <h3>{title}</h3>
              <p>{description}</p>
            </li>
          ))}
        </ol>

        <div className="landing-closing__cta">
          <img src="/brand/takokit-mark.svg" alt="" aria-hidden="true" />
          <div>
            <p className="landing-kicker">Takokit</p>
            <h2 id="landing-closing-title">Your voice stack. On your machine.</h2>
            <div className="landing-actions">
              <RouteLink href="/download" className="landing-button landing-button--primary">Install Takokit</RouteLink>
              <RouteLink href="/docs" className="landing-button">Read the docs</RouteLink>
              <a href="https://github.com/amaansyed27/Takokit" className="landing-text-link">View on GitHub →</a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
