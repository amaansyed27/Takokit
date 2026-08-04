const CAPABILITIES = [
  ["Speak", "Text to speech"],
  ["Transcribe", "Speech to text"],
  ["Clone", "Consented voices"],
  ["Convert", "Voice conversion"],
];

export function ProductCapabilities() {
  return (
    <section className="capability-strip" aria-labelledby="capability-strip-title">
      <div className="landing-shell capability-strip__inner">
        <div className="capability-strip__intro">
          <p className="landing-kicker">One runtime</p>
          <h2 id="capability-strip-title">Every local voice workflow.</h2>
        </div>
        <div className="capability-strip__line" aria-hidden="true"><i /></div>
        <ul>
          {CAPABILITIES.map(([title, label], index) => (
            <li key={title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{title}</strong>
              <small>{label}</small>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
