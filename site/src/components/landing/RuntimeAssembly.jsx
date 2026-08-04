import { useEffect, useRef, useState } from "react";
import { RouteLink } from "../../app/router";

const FEATURES = [
  ["Models", "Versioned references with declared variants and support states."],
  ["Runners", "Managed execution backends selected for each model family."],
  ["Adapters", "Model-specific integration behind one runtime contract."],
  ["Interfaces", "CLI, TUI, GUI, and API sharing the same local state."],
  ["Local state", "Models, voices, sessions, and outputs kept on your machine."],
  ["Consent", "Ownership and permission remain visible in sensitive workflows."],
];

export function RuntimeAssembly() {
  const [activeIndex, setActiveIndex] = useState(0);
  const cardRefs = useRef([]);

  useEffect(() => {
    const cards = cardRefs.current.filter(Boolean);
    if (!cards.length || typeof IntersectionObserver === "undefined") return undefined;

    const observer = new IntersectionObserver((entries) => {
      const visible = entries
        .filter((entry) => entry.isIntersecting)
        .sort((left, right) => right.intersectionRatio - left.intersectionRatio)[0];

      if (visible) setActiveIndex(Number(visible.target.dataset.index));
    }, {
      rootMargin: "-30% 0px -40% 0px",
      threshold: [0.2, 0.45, 0.7],
    });

    cards.forEach((card) => observer.observe(card));
    return () => observer.disconnect();
  }, []);

  const [activeTitle] = FEATURES[activeIndex];

  return (
    <section className="runtime-assembly" id="features" aria-labelledby="runtime-assembly-title">
      <div className="runtime-assembly__layout landing-shell">
        <div className="runtime-assembly__intro">
          <p className="landing-kicker">Inside Takokit</p>
          <h2 id="runtime-assembly-title">One shell for the whole voice stack.</h2>
          <p className="runtime-assembly__summary">
            Models, runners, interfaces, and local state come together as one inspectable system.
          </p>

          <div className="runtime-assembly__visual" aria-label="Takokit runtime features entering the abstract logo shell">
            <div className="runtime-assembly__ingredient" key={activeTitle} aria-hidden="true">
              <span>{String(activeIndex + 1).padStart(2, "0")}</span>
              <strong>{activeTitle}</strong>
            </div>

            <img src="/brand/takokit-mark.svg" alt="" />

            <div className="runtime-assembly__progress" aria-hidden="true">
              {FEATURES.map(([title], index) => (
                <i
                  className={`${index <= activeIndex ? "is-complete" : ""} ${index === activeIndex ? "is-active" : ""}`}
                  key={title}
                />
              ))}
            </div>
          </div>

          <RouteLink href="/docs" className="landing-text-link">Read how the runtime works →</RouteLink>
        </div>

        <ol className="runtime-assembly__cards">
          {FEATURES.map(([title, description], index) => (
            <li
              className={activeIndex === index ? "is-active" : ""}
              data-index={index}
              key={title}
              ref={(node) => { cardRefs.current[index] = node; }}
              onFocus={() => setActiveIndex(index)}
              onMouseEnter={() => setActiveIndex(index)}
              tabIndex={0}
            >
              <span>{String(index + 1).padStart(2, "0")}</span>
              <div>
                <h3>{title}</h3>
                <p>{description}</p>
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
