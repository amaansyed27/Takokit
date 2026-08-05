import { useEffect, useRef, useState } from "react";
import { RouteLink } from "../../app/router";

const FEATURES = [
  ["Models", "Versioned voice models resolved from one registry."],
  ["Runners", "The right execution backend is managed for each model family."],
  ["Every interface", "CLI, TUI, GUI, and API share the same models and state."],
  ["Local by default", "Models, voices, sessions, and outputs stay on your machine."],
];

export function RuntimeAssembly() {
  const [activeIndex, setActiveIndex] = useState(0);
  const stepRefs = useRef([]);

  useEffect(() => {
    const steps = stepRefs.current.filter(Boolean);
    if (!steps.length || typeof IntersectionObserver === "undefined") return undefined;

    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        setActiveIndex(Number(entry.target.dataset.index));
      });
    }, {
      rootMargin: "-38% 0px -38% 0px",
      threshold: 0.01,
    });

    steps.forEach((step) => observer.observe(step));
    return () => observer.disconnect();
  }, []);

  function selectStep(index) {
    setActiveIndex(index);
    stepRefs.current[index]?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  return (
    <section className="runtime-flow" id="features" aria-labelledby="runtime-flow-title">
      <div className="runtime-flow__layout landing-shell">
        <div className="runtime-flow__sticky">
          <header className="runtime-flow__header">
            <p className="landing-kicker">Inside Takokit</p>
            <h2 id="runtime-flow-title">
              The whole voice stack.
              <span>One local runtime.</span>
            </h2>
          </header>

          <div className="runtime-flow__wheel" aria-hidden="true">
            <div
              className="runtime-flow__rotor"
              style={{ transform: `rotate(${activeIndex * 90}deg)` }}
            >
              {FEATURES.map(([title], index) => (
                <span
                  className="runtime-flow__spoke"
                  key={title}
                  style={{ "--spoke-index": index }}
                >
                  <i />
                </span>
              ))}
              <span className="runtime-flow__hub">
                <img src="/brand/takokit-mark.svg" alt="" />
              </span>
            </div>
          </div>

          <div className="runtime-flow__progress" aria-label="Runtime feature navigation">
            {FEATURES.map(([title], index) => (
              <button
                type="button"
                className={activeIndex === index ? "is-active" : ""}
                aria-label={`Show ${title}`}
                aria-current={activeIndex === index ? "step" : undefined}
                onClick={() => selectStep(index)}
                key={title}
              >
                <span>{String(index + 1).padStart(2, "0")}</span>
                <i />
              </button>
            ))}
          </div>
        </div>

        <ol className="runtime-flow__steps">
          {FEATURES.map(([title, description], index) => (
            <li
              className={activeIndex === index ? "is-active" : ""}
              data-index={index}
              key={title}
              ref={(node) => { stepRefs.current[index] = node; }}
              onFocus={() => setActiveIndex(index)}
              tabIndex={0}
            >
              <span>{String(index + 1).padStart(2, "0")} / 04</span>
              <h3>{title}.</h3>
              <p>{description}</p>
            </li>
          ))}
        </ol>
      </div>

      <div className="runtime-flow__outro landing-shell">
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
