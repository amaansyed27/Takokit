const CAPABILITIES = [
  ["Speak", "Generate speech from text with local TTS models."],
  ["Transcribe", "Turn recordings into text with local STT models."],
  ["Clone", "Create reusable voices with explicit consent controls."],
  ["Convert", "Transform recordings through compatible voice models."],
];

export function ProductCapabilities() {
  return (
    <section className="capability-strip" aria-labelledby="capability-strip-title">
      <div className="landing-shell capability-strip__inner">
        <header>
          <p className="landing-kicker">One runtime</p>
          <h2 id="capability-strip-title">Every local voice workflow.</h2>
        </header>

        <div className="capability-strip__signal" aria-hidden="true"><i /></div>

        <ul>
          {CAPABILITIES.map(([title, description], index) => (
            <li key={title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{title}</strong>
              <p>{description}</p>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
