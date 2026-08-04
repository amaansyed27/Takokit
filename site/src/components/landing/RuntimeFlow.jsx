import { RouteLink } from "../../app/router";

const STEPS = [
  {
    number: "01",
    label: "PULL",
    title: "RESOLVE THE MODEL.",
    description: "Choose a versioned reference. Takokit resolves the manifest, runner, adapter, hardware requirements, and pinned artifacts.",
  },
  {
    number: "02",
    label: "RUN",
    title: "EXECUTE LOCALLY.",
    description: "Use the same model through the CLI, TUI, GUI, or local API while the runtime keeps state and outputs consistent.",
  },
  {
    number: "03",
    label: "REUSE",
    title: "KEEP THE SYSTEM WARM.",
    description: "Reuse installed models, isolated runners, local voices, and project sessions instead of rebuilding the stack every time.",
  },
];

export function RuntimeFlow() {
  return (
    <section className="tk-flow" aria-labelledby="tk-flow-title">
      <div className="shell">
        <header className="tk-section-bar tk-section-bar--light">
          <span>04 / THE LOCAL LOOP</span>
          <span>MODEL → RUNNER → OUTPUT</span>
        </header>

        <div className="tk-flow__heading">
          <div>
            <p className="tk-kicker">THE TAKOKIT CONTRACT</p>
            <h2 id="tk-flow-title">PULL.<br />RUN.<br />REUSE.</h2>
          </div>
          <p>Open voice models should feel like tools—not fragile research demos.</p>
        </div>

        <ol className="tk-flow__steps">
          {STEPS.map((step) => (
            <li key={step.number}>
              <span>{step.number}</span>
              <article>
                <p>{step.label}</p>
                <h3>{step.title}</h3>
                <p>{step.description}</p>
              </article>
            </li>
          ))}
        </ol>

        <div className="tk-flow__closing">
          <RouteLink href="/download" className="tk-action tk-action--gold">Install Takokit</RouteLink>
          <RouteLink href="/docs" className="tk-action tk-action--light">Open documentation</RouteLink>
        </div>
      </div>
    </section>
  );
}
