const STEPS = [
  ["Pull a model", "Choose a model reference and let Takokit resolve its files and runtime."],
  ["Run it locally", "Generate, transcribe, clone, or convert without sending the workflow to a hosted service."],
  ["Use any interface", "CLI, GUI, TUI, and the local API share the same models and local state."],
];

export function RuntimeArchitecture() {
  return (
    <section className="runtime-architecture" aria-labelledby="runtime-architecture-title">
      <div className="landing-shell runtime-architecture__inner">
        <header>
          <p className="landing-kicker">How Takokit works</p>
          <h2 id="runtime-architecture-title">Pull. Run. Use it your way.</h2>
        </header>

        <ol className="runtime-architecture__nodes runtime-architecture__nodes--three">
          {STEPS.map(([title, description], index) => (
            <li key={title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{title}</strong>
              <p>{description}</p>
              {index < STEPS.length - 1 && <i aria-hidden="true">→</i>}
            </li>
          ))}
        </ol>

        <div className="runtime-architecture__signal" aria-hidden="true"><i /></div>
      </div>
    </section>
  );
}
