import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const FEATURES = [
  {
    key: "models",
    title: "Models",
    short: "Versioned model references",
    description: "Discover and pull speech models from one registry with explicit variants, requirements, and support states.",
  },
  {
    key: "runners",
    title: "Runners",
    short: "Managed execution backends",
    description: "Takokit resolves the native or managed runner required by each model instead of exposing installation glue to every project.",
  },
  {
    key: "adapters",
    title: "Adapters",
    short: "Model-specific integration",
    description: "Adapters translate a model family into the same runtime contract while preserving its real capabilities and limits.",
  },
  {
    key: "interfaces",
    title: "Interfaces",
    short: "CLI, TUI, GUI, and API",
    description: "Every interface uses the same registry, local state, runners, sessions, and outputs.",
  },
  {
    key: "storage",
    title: "Local state",
    short: "Visible files on your machine",
    description: "Models, runners, voices, sessions, and outputs remain inspectable under one local Takokit structure.",
  },
  {
    key: "consent",
    title: "Consent",
    short: "Safety inside the workflow",
    description: "Voice cloning, conversion, and training keep ownership and permission visible instead of hiding them behind a generic action.",
  },
];

export function RuntimeAssembly() {
  const reducedMotion = useReducedMotion();
  const compactLayout = useMediaQuery("(max-width: 920px)");
  const staticLayout = reducedMotion || compactLayout;
  const [activeIndex, setActiveIndex] = useState(0);
  const sectionRef = useScrollProgress((progress, section) => {
    const bounded = Math.min(0.9999, Math.max(0, progress));
    const nextIndex = Math.min(FEATURES.length - 1, Math.floor(bounded * FEATURES.length));
    setActiveIndex((current) => (current === nextIndex ? current : nextIndex));
    section.style.setProperty("--assembly-progress", bounded.toFixed(4));
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

  const active = FEATURES[activeIndex];

  return (
    <section
      className={`runtime-assembly ${staticLayout ? "is-static" : ""}`}
      id="features"
      ref={sectionRef}
      aria-labelledby="runtime-assembly-title"
    >
      <div className="runtime-assembly__stage landing-shell">
        <div className="runtime-assembly__copy">
          <p className="landing-kicker">What Takokit contains</p>
          <h2 id="runtime-assembly-title">One system. Not six disconnected tools.</h2>
          <p className="runtime-assembly__summary">
            The abstract mark becomes the shell. Each layer adds a real part of the local runtime.
          </p>

          <div className="runtime-assembly__active" aria-live="polite">
            <span>{String(activeIndex + 1).padStart(2, "0")} / 06</span>
            <h3>{active.title}</h3>
            <strong>{active.short}</strong>
            <p>{active.description}</p>
          </div>

          <RouteLink href="/docs" className="landing-text-link">See the runtime architecture →</RouteLink>
        </div>

        <div className="runtime-assembly__visual" aria-label="Takokit runtime layers assembling inside the abstract logo">
          <img className="runtime-assembly__outline" src="/brand/takokit-mark.svg" alt="" />
          <div className="runtime-assembly__mask" aria-hidden="true">
            {FEATURES.map((feature, index) => (
              <span
                className={`runtime-layer ${index <= activeIndex || staticLayout ? "is-visible" : ""} ${index === activeIndex ? "is-active" : ""}`}
                key={feature.key}
                style={{ "--layer-index": index }}
              >
                <i />
                <b>{feature.title}</b>
              </span>
            ))}
          </div>
          <div className="runtime-assembly__connector" aria-hidden="true"><i /></div>
        </div>

        <ol className="runtime-assembly__steps">
          {FEATURES.map((feature, index) => (
            <li key={feature.key}>
              <button
                type="button"
                className={activeIndex === index ? "is-active" : ""}
                aria-current={activeIndex === index ? "step" : undefined}
                onClick={() => selectFeature(index)}
              >
                <span>{String(index + 1).padStart(2, "0")}</span>
                <strong>{feature.title}</strong>
              </button>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
