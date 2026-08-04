import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const FEATURES = [
  ["Models", "Versioned references with declared variants and support states."],
  ["Runners", "Managed execution backends selected for each model family."],
  ["Adapters", "Model-specific integration behind one runtime contract."],
  ["Interfaces", "CLI, TUI, GUI, and API sharing the same local state."],
  ["Local state", "Models, voices, sessions, and outputs kept on your machine."],
  ["Consent", "Ownership and permission remain visible in sensitive workflows."],
];

export function RuntimeAssembly() {
  const reducedMotion = useReducedMotion();
  const compactLayout = useMediaQuery("(max-width: 900px)");
  const staticLayout = reducedMotion || compactLayout;
  const [activeIndex, setActiveIndex] = useState(0);
  const sectionRef = useScrollProgress((progress, section) => {
    const bounded = Math.min(0.9999, Math.max(0, progress));
    const nextIndex = Math.min(FEATURES.length - 1, Math.floor(bounded * FEATURES.length));
    setActiveIndex((current) => (current === nextIndex ? current : nextIndex));
    section.style.setProperty("--assembly-progress-width", `${(bounded * 100).toFixed(2)}%`);
    section.style.setProperty("--assembly-scale", (0.94 + bounded * 0.06).toFixed(4));
  }, staticLayout);

  function selectFeature(index) {
    if (staticLayout || !sectionRef.current) {
      setActiveIndex(index);
      return;
    }

    const section = sectionRef.current;
    const sectionTop = window.scrollY + section.getBoundingClientRect().top;
    const range = Math.max(section.offsetHeight - window.innerHeight, 1);
    window.scrollTo({
      top: sectionTop + range * ((index + 0.18) / FEATURES.length),
      behavior: "smooth",
    });
  }

  const [title, description] = FEATURES[activeIndex];

  return (
    <section
      className={`runtime-assembly ${staticLayout ? "is-static" : ""}`}
      id="features"
      ref={sectionRef}
      aria-labelledby="runtime-assembly-title"
    >
      <div className="runtime-assembly__stage landing-shell">
        <div className="runtime-assembly__copy">
          <p className="landing-kicker">Inside Takokit</p>
          <h2 id="runtime-assembly-title">The runtime is the shell.</h2>
          <p className="runtime-assembly__summary">
            Each scroll step adds one real part of the local voice stack and locks it into the same system.
          </p>

          <div className="runtime-assembly__active" aria-live="polite">
            <span>{String(activeIndex + 1).padStart(2, "0")} / 06</span>
            <h3>{title}</h3>
            <p>{description}</p>
          </div>

          <RouteLink href="/docs" className="landing-text-link">Read how the runtime works →</RouteLink>
        </div>

        <div className="runtime-assembly__visual" role="img" aria-label="Takokit features entering the abstract logo shell">
          <img src="/brand/takokit-mark.svg" alt="" />
          <div className="runtime-assembly__ingredient" aria-hidden="true">
            <span>{String(activeIndex + 1).padStart(2, "0")}</span>
            <strong>{title}</strong>
          </div>
          <div className="runtime-assembly__signal" aria-hidden="true"><i /></div>
        </div>

        <ol className="runtime-assembly__steps">
          {FEATURES.map(([feature], index) => (
            <li key={feature}>
              <button
                type="button"
                className={activeIndex === index ? "is-active" : ""}
                aria-current={activeIndex === index ? "step" : undefined}
                onClick={() => selectFeature(index)}
              >
                <span>{String(index + 1).padStart(2, "0")}</span>
                <strong>{feature}</strong>
              </button>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
