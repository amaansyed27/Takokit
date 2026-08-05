import { useEffect, useRef } from "react";
import { RouteLink } from "../../app/router";

const FEATURES = [
  ["Models", "Versioned voice models from one registry."],
  ["Runners", "The required execution backend is selected and managed for each model family."],
  ["Every interface", "CLI, TUI, GUI, and API share the same models, sessions, and local state."],
  ["Local by default", "Models, voices, sessions, and outputs stay on your machine."],
];

export function RuntimeAssembly() {
  const sectionRef = useRef(null);

  useEffect(() => {
    const section = sectionRef.current;
    if (!section || typeof IntersectionObserver === "undefined") return undefined;

    const targets = section.querySelectorAll("[data-reveal]");
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      });
    }, {
      rootMargin: "-8% 0px -14% 0px",
      threshold: 0.16,
    });

    targets.forEach((target) => observer.observe(target));
    return () => observer.disconnect();
  }, []);

  return (
    <section
      className="runtime-story"
      id="features"
      aria-labelledby="runtime-story-title"
      ref={sectionRef}
    >
      <header className="landing-shell runtime-story__header" data-reveal>
        <p className="landing-kicker">Inside Takokit</p>
        <h2 id="runtime-story-title">The whole voice stack. One local runtime.</h2>
      </header>

      <div className="runtime-story__bands">
        {FEATURES.map(([title, description], index) => (
          <article
            className={`runtime-band ${index % 2 ? "runtime-band--right" : "runtime-band--left"}`}
            data-reveal
            key={title}
          >
            <div className="landing-shell runtime-band__inner">
              <span className="runtime-band__number">{String(index + 1).padStart(2, "0")}</span>
              <div className="runtime-band__copy">
                <h3>{title}.</h3>
                <p>{description}</p>
              </div>
              <i className="runtime-band__signal" aria-hidden="true" />
            </div>
          </article>
        ))}
      </div>

      <div className="landing-shell runtime-story__outro" data-reveal>
        <img src="/brand/takokit-mark.svg" alt="" aria-hidden="true" />
        <div>
          <p className="landing-kicker">Takokit runtime</p>
          <h3>Different workflows. One inspectable system.</h3>
          <RouteLink href="/docs" className="landing-text-link">Read how it works →</RouteLink>
        </div>
      </div>
    </section>
  );
}
