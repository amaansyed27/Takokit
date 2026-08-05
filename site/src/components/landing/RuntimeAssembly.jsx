import { RouteLink } from "../../app/router";

const FEATURES = [
  ["Models", "Versioned voice models resolved from one registry."],
  ["Runners", "The right execution backend is managed for each model family."],
  ["Every interface", "CLI, TUI, GUI, and API share the same models and state."],
  ["Local by default", "Models, voices, sessions, and outputs stay on your machine."],
];

export function RuntimeAssembly() {
  return (
    <section className="runtime-wheel" id="features" aria-labelledby="runtime-wheel-title">
      <div className="runtime-wheel__stage landing-shell">
        <header className="runtime-wheel__header">
          <p className="landing-kicker">Inside Takokit</p>
          <h2 id="runtime-wheel-title">
            The whole voice stack.
            <span>One local runtime.</span>
          </h2>
        </header>

        <div className="runtime-wheel__scene">
          <div className="runtime-wheel__pinwheel" aria-hidden="true">
            <div className="runtime-wheel__rotor">
              {FEATURES.map(([title], index) => (
                <span
                  className="runtime-wheel__spoke"
                  key={title}
                  style={{ "--spoke-index": index }}
                >
                  <i />
                </span>
              ))}
              <span className="runtime-wheel__hub">
                <img src="/brand/takokit-mark.svg" alt="" />
              </span>
            </div>
          </div>

          <div className="runtime-wheel__roller">
            <div className="runtime-wheel__track">
              {FEATURES.map(([title, description], index) => (
                <article className="runtime-wheel__panel" key={title}>
                  <span>{String(index + 1).padStart(2, "0")} / 04</span>
                  <h3>{title}.</h3>
                  <p>{description}</p>
                </article>
              ))}
            </div>
          </div>
        </div>

        <div className="runtime-wheel__footer" aria-hidden="true">
          <span>Scroll through the runtime</span>
          <i />
        </div>
      </div>

      <div className="runtime-wheel__outro landing-shell">
        <img src="/brand/takokit-mark.svg" alt="" aria-hidden="true" />
        <div>
          <p className="landing-kicker">Takokit runtime</p>
          <h3>Different workflows. One inspectable system.</h3>
          <RouteLink href="/docs" className="landing-text-link">Read how it works →</RouteLink>
        </div>
      </div>
    </section>
  );
}
