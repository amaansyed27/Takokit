import { useEffect, useRef, useState } from "react";

const CAPABILITIES = [
  ["Speak", "Generate speech from text with local TTS models."],
  ["Transcribe", "Turn recordings into text with local STT models."],
  ["Clone", "Create reusable voices with explicit consent controls."],
  ["Convert", "Transform recordings through compatible voice models."],
];

export function ProductCapabilities() {
  const sectionRef = useRef(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const section = sectionRef.current;
    if (!section || typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return undefined;
    }

    const observer = new IntersectionObserver(([entry]) => {
      if (!entry.isIntersecting) return;
      setVisible(true);
      observer.disconnect();
    }, {
      rootMargin: "-10% 0px -18% 0px",
      threshold: 0.2,
    });

    observer.observe(section);
    return () => observer.disconnect();
  }, []);

  return (
    <section
      className={`capability-strip ${visible ? "is-visible" : ""}`}
      aria-labelledby="capability-strip-title"
      ref={sectionRef}
    >
      <div className="landing-shell capability-strip__inner">
        <header>
          <p className="landing-kicker">One runtime</p>
          <h2 id="capability-strip-title">Every local voice workflow.</h2>
        </header>

        <div className="capability-strip__signal" aria-hidden="true"><i /></div>

        <ul>
          {CAPABILITIES.map(([title, description], index) => (
            <li key={title} style={{ "--capability-delay": `${120 + index * 90}ms` }}>
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
