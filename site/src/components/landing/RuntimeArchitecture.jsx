const NODES = [
  ["Model", "Choose a versioned model reference."],
  ["Registry", "Resolve its manifest, runner, and requirements."],
  ["Runtime", "Execute through the shared local system."],
  ["Output", "Keep files, sessions, and reusable state locally."],
];

export function RuntimeArchitecture() {
  return (
    <section className="runtime-architecture" aria-labelledby="runtime-architecture-title">
      <div className="landing-shell runtime-architecture__inner">
        <header>
          <p className="landing-kicker">How it works</p>
          <h2 id="runtime-architecture-title">A direct path from model to local output.</h2>
        </header>

        <ol className="runtime-architecture__nodes">
          {NODES.map(([title, description], index) => (
            <li key={title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{title}</strong>
              <p>{description}</p>
              {index < NODES.length - 1 && <i aria-hidden="true">→</i>}
            </li>
          ))}
        </ol>

        <div className="runtime-architecture__signal" aria-hidden="true"><i /></div>
      </div>
    </section>
  );
}
