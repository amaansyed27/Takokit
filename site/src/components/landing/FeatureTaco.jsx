import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const FEATURES = [
  {
    key: "catalog",
    short: "MODELS",
    title: "CURATED MODEL CATALOG",
    description: "Speech-to-text, text-to-speech, cloning, conversion, training, and audio-language models in one versioned registry.",
  },
  {
    key: "runners",
    short: "RUNNERS",
    title: "MANAGED RUNNERS",
    description: "Takokit resolves the runtime, adapter, and dependencies instead of leaving every model to invent its own setup.",
  },
  {
    key: "surfaces",
    short: "4 SURFACES",
    title: "CLI · TUI · GUI · API",
    description: "One shared system exposed through the interface that fits the job, without splitting state across separate tools.",
  },
  {
    key: "storage",
    short: "LOCAL",
    title: "LOCAL STATE",
    description: "Models, runners, voices, sessions, and outputs stay inside a visible structure on the machine you control.",
  },
  {
    key: "consent",
    short: "CONSENT",
    title: "CONSENT CONTROLS",
    description: "Cloning, conversion, and training keep voice ownership and permission inside the workflow.",
  },
  {
    key: "platforms",
    short: "3 OS",
    title: "WINDOWS · LINUX · macOS",
    description: "A Rust-first core designed for the desktop platforms where serious local voice work happens.",
  },
];

export function FeatureTaco() {
  const reducedMotion = useReducedMotion();
  const narrowLayout = useMediaQuery("(max-width: 900px)");
  const staticLayout = reducedMotion || narrowLayout;
  const [activeIndex, setActiveIndex] = useState(0);
  const sectionRef = useScrollProgress((progress, section) => {
    const bounded = Math.min(0.9999, Math.max(0, progress));
    const nextIndex = Math.min(FEATURES.length - 1, Math.floor(bounded * FEATURES.length));
    setActiveIndex((current) => (current === nextIndex ? current : nextIndex));
    section.style.setProperty("--tk-taco-progress", bounded.toFixed(4));
  }, staticLayout);

  function jumpToFeature(index) {
    if (staticLayout || !sectionRef.current) {
      setActiveIndex(index);
      return;
    }
    const section = sectionRef.current;
    const top = window.scrollY + section.getBoundingClientRect().top;
    const range = Math.max(section.offsetHeight - window.innerHeight, 1);
    window.scrollTo({ top: top + range * ((index + 0.15) / FEATURES.length), behavior: "smooth" });
  }

  const active = FEATURES[activeIndex];

  return (
    <section
      className={`tk-taco ${staticLayout ? "is-static" : ""}`}
      id="features"
      ref={sectionRef}
      aria-labelledby="tk-taco-title"
    >
      <div className="tk-taco__stage shell">
        <header className="tk-section-bar">
          <span>01 / WHAT IS INSIDE</span>
          <span>THE MARK IS THE SHELL. THE FEATURES ARE THE INGREDIENTS.</span>
        </header>

        <div className="tk-taco__copy">
          <p className="tk-kicker">THE COMPLETE LOCAL VOICE STACK</p>
          <h2 id="tk-taco-title">ONE RUNTIME.<br />SIX LAYERS.</h2>
          <p>{active.description}</p>
          <RouteLink href="/docs" className="tk-text-link">See how it works →</RouteLink>
        </div>

        <div className="tk-taco__assembly" aria-hidden="true">
          <div className="tk-taco__ingredients">
            {FEATURES.map((feature, index) => (
              <div
                className={`tk-ingredient ${index <= activeIndex ? "is-added" : ""} ${index === activeIndex ? "is-active" : ""}`}
                key={feature.key}
                style={{ "--ingredient-index": index }}
              >
                <span>{feature.short}</span>
              </div>
            ))}
          </div>
          <img className="tk-taco__mark" src="/brand/takokit-mark.svg" alt="" />
        </div>

        <div className="tk-taco__details">
          <span>{String(activeIndex + 1).padStart(2, "0")} / 06</span>
          <h3>{active.title}</h3>
          <div className="tk-taco__selector" role="tablist" aria-label="Choose a Takokit feature">
            {FEATURES.map((feature, index) => (
              <button
                aria-selected={activeIndex === index}
                className={activeIndex === index ? "is-active" : ""}
                key={feature.key}
                onClick={() => jumpToFeature(index)}
                role="tab"
                type="button"
              >
                {String(index + 1).padStart(2, "0")}
              </button>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
