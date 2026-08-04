import { RouteLink } from "../../app/router";
import { useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

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
  const reducedMotion = useReducedMotion();
  const sectionRef = useScrollProgress((progress, section) => {
    section.style.setProperty("--tk-flow-progress", progress.toFixed(4));
  }, reducedMotion);

  return (
    <section
      className={`tk-flow ${reducedMotion ? "is-static" : ""}`}
      ref={sectionRef}
      aria-labelledby="tk-flow-title"
    >
      <div className="tk-flow__stage">
        <header className="tk-section-bar tk-section-bar--light">
          <span>04 / THE LOCAL LOOP</span>
          <span>MODEL → RUNNER → OUTPUT</span>
        </header>

        <div className="tk-flow__headline">
          <p className="tk-kicker">THE TAKOKIT CONTRACT</p>
          <h2 id="tk-flow-title">PULL.<br />RUN.<br />REUSE.</h2>
        </div>

        <ol className="tk-flow__steps">
          {STEPS.map((step, index) => (
            <li key={step.number} style={{ "--flow-index": index }}>
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
          <p>Open voice models should feel like tools—not fragile research demos.</p>
          <div>
            <RouteLink href="/download" className="tk-action tk-action--gold">Install Takokit</RouteLink>
            <RouteLink href="/docs" className="tk-action tk-action--light">Open documentation</RouteLink>
          </div>
        </div>

        <div className="tk-flow__signal" aria-hidden="true">
          <i />
          <i />
          <i />
        </div>
      </div>
    </section>
  );
}
