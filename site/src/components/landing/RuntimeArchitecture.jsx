const NODES = [
  ["01", "Model reference", "A versioned name such as whisper:tiny"],
  ["02", "Registry resolver", "Manifest, artifacts, hardware, and support state"],
  ["03", "Runner + adapter", "The execution path required by that model"],
  ["04", "Shared interfaces", "CLI, TUI, GUI, and local API"],
  ["05", "Local output", "Files, sessions, and reusable state on your machine"],
];

export function RuntimeArchitecture() {
  return (
    <section className="runtime-architecture" aria-labelledby="runtime-architecture-title">
      <div className="landing-shell runtime-architecture__inner">
        <header>
          <p className="landing-kicker">How it moves</p>
          <h2 id="runtime-architecture-title">One path from model reference to local output.</h2>
        </header>

        <ol className="runtime-architecture__nodes">
          {NODES.map(([number, title, description], index) => (
            <li key={title}>
              <span>{number}</span>
              <div>
                <strong>{title}</strong>
                <p>{description}</p>
              </div>
              {index < NODES.length - 1 && <i aria-hidden="true" />}
            </li>
          ))}
        </ol>

        <div className="runtime-architecture__signal" aria-hidden="true"><i /></div>
      </div>
    </section>
  );
}
