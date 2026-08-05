import { useEffect, useRef } from "react";
import { RouteLink } from "../../app/router";

const CAPABILITIES = [
  {
    title: "Generate speech",
    label: "TTS",
    description: "Turn text into speech with local voice models.",
    task: "speech",
  },
  {
    title: "Transcribe audio",
    label: "STT",
    description: "Turn recordings into searchable text on your machine.",
    task: "transcription",
  },
  {
    title: "Clone a voice",
    label: "Consent required",
    description: "Create a reusable local voice profile from permitted audio.",
    task: "cloning",
  },
  {
    title: "Convert a voice",
    label: "Voice conversion",
    description: "Transform recordings with a compatible custom voice package.",
    task: "conversion",
  },
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
        <p className="landing-kicker">Choose a task</p>
        <h2 id="capability-story-title">Start with what you need to do.</h2>
      </header>

      <div className="capability-story__moments">
        {CAPABILITIES.map(({ title, label, description, task }, index) => (
          <article
            className={`capability-moment ${index % 2 ? "capability-moment--right" : "capability-moment--left"}`}
            key={task}
            ref={(node) => { itemRefs.current[index] = node; }}
          >
            <div className="landing-shell capability-moment__inner">
              <span className="capability-moment__number">{String(index + 1).padStart(2, "0")}</span>
              <RouteLink
                className="capability-moment__copy"
                href={`/models?task=${task}`}
                aria-label={`${title}: browse matching models`}
              >
                <span className="capability-moment__label">{label}</span>
                <h3>{title}.</h3>
                <p>{description}</p>
                <span className="capability-moment__action">Browse matching models →</span>
              </RouteLink>
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
