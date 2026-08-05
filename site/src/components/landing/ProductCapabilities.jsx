import { useEffect, useRef } from "react";

const CAPABILITIES = [
  ["Speak", "Generate speech from text with local TTS models."],
  ["Transcribe", "Turn recordings into text with local STT models."],
  ["Clone", "Create reusable voices with explicit consent controls."],
  ["Convert", "Transform recordings through compatible voice models."],
];

export function ProductCapabilities() {
  const itemRefs = useRef([]);

  useEffect(() => {
    const items = itemRefs.current.filter(Boolean);
    if (!items.length || typeof IntersectionObserver === "undefined") {
      items.forEach((item) => item.classList.add("is-visible"));
      return undefined;
    }

    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      });
    }, {
      rootMargin: "-8% 0px -12% 0px",
      threshold: 0.18,
    });

    items.forEach((item) => observer.observe(item));
    return () => observer.disconnect();
  }, []);

  return (
    <section className="capability-story" aria-labelledby="capability-story-title">
      <header className="landing-shell capability-story__header">
        <p className="landing-kicker">Voice workflows</p>
        <h2 id="capability-story-title">Four ways in. One runtime underneath.</h2>
      </header>

      <div className="capability-story__moments">
        {CAPABILITIES.map(([title, description], index) => (
          <article
            className={`capability-moment ${index % 2 ? "capability-moment--right" : "capability-moment--left"}`}
            key={title}
            ref={(node) => { itemRefs.current[index] = node; }}
          >
            <div className="landing-shell capability-moment__inner">
              <span className="capability-moment__number">{String(index + 1).padStart(2, "0")}</span>
              <div className="capability-moment__copy">
                <h3>{title}.</h3>
                <p>{description}</p>
              </div>
              <div className="capability-moment__signal" aria-hidden="true">
                {Array.from({ length: 18 }, (_, barIndex) => <i key={barIndex} />)}
              </div>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
