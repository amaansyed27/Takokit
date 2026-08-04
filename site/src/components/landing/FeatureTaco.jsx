import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const FEATURES = [
  {
    key: "catalog",
    eyebrow: "THE FILLING",
    title: "CURATED MODEL CATALOG",
    description: "Speech-to-text, text-to-speech, voice cloning, conversion, training, and audio-language models in one versioned registry.",
  },
  {
    key: "runners",
    eyebrow: "THE HEAT",
    title: "MANAGED RUNNERS",
    description: "Takokit resolves the right runtime, adapter, and dependencies instead of leaving every model to invent its own setup.",
  },
  {
    key: "surfaces",
    eyebrow: "THE LAYERS",
    title: "CLI · TUI · GUI · API",
    description: "One shared system exposed through the interface that fits the job, without splitting state across separate tools.",
  },
  {
    key: "storage",
    eyebrow: "THE SHELL",
    title: "LOCAL STATE",
    description: "Models, runners, voices, sessions, and outputs stay under a visible local structure on the machine you control.",
  },
  {
    key: "consent",
    eyebrow: "THE BOUNDARY",
    title: "CONSENT CONTROLS",
    description: "Cloning, conversion, and training keep voice ownership and permission inside the workflow instead of hiding it.",
  },
  {
    key: "platforms",
    eyebrow: "THE PLATE",
    title: "WINDOWS · LINUX · macOS",
    description: "A Rust-first core designed for the desktop platforms where serious local voice work actually happens.",
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
    section.style.setProperty("--tk-taco-turn", `${(-8 + bounded * 16).toFixed(2)}deg`);
  }, staticLayout);

  function handlePointerMove(event) {
    if (staticLayout) return;
    const visual = sectionRef.current?.querySelector(".tk-taco__visual");
    if (!visual) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = (event.clientX - bounds.left) / bounds.width - 0.5;
    const y = (event.clientY - bounds.top) / bounds.height - 0.5;
    visual.style.setProperty("--tk-taco-tilt-x", `${(-y * 8).toFixed(2)}deg`);
    visual.style.setProperty("--tk-taco-tilt-y", `${(x * 10).toFixed(2)}deg`);
  }

  function resetPointer() {
    const visual = sectionRef.current?.querySelector(".tk-taco__visual");
    visual?.style.setProperty("--tk-taco-tilt-x", "0deg");
    visual?.style.setProperty("--tk-taco-tilt-y", "0deg");
  }

  return (
    <section
      className={`tk-taco ${staticLayout ? "is-static" : ""}`}
      id="features"
      ref={sectionRef}
      onPointerMove={handlePointerMove}
      onPointerLeave={resetPointer}
      aria-labelledby="tk-taco-title"
    >
      <div className="tk-taco__stage">
        <header className="tk-section-bar tk-section-bar--light">
          <span>01 / WHAT IS INSIDE</span>
          <span>THE RUNTIME IS THE SHELL. THE CAPABILITIES ARE THE INGREDIENTS.</span>
        </header>

        <div className="tk-taco__intro">
          <p className="tk-kicker">THE COMPLETE LOCAL VOICE STACK</p>
          <h2 id="tk-taco-title">BUILT AS<br />ONE SYSTEM.</h2>
          <p>
            Takokit wraps the difficult parts of running open voice models into one inspectable local runtime.
          </p>
          <RouteLink href="/docs" className="tk-text-link">See how it works →</RouteLink>
        </div>

        <div className="tk-taco__visual" aria-hidden="true">
          <div className="tk-taco__aura" />
          <div className="tk-taco__shell tk-taco__shell--back" />
          <div className="tk-taco__ingredients">
            {FEATURES.map((feature, index) => (
              <div
                className={`tk-ingredient tk-ingredient--${feature.key} ${activeIndex >= index ? "is-added" : ""} ${activeIndex === index ? "is-active" : ""}`}
                key={feature.key}
                style={{ "--ingredient-index": index }}
              >
                <i />
                <i />
                <i />
              </div>
            ))}
          </div>
          <div className="tk-taco__shell tk-taco__shell--front">
            <i />
            <i />
            <i />
            <i />
            <i />
          </div>
          <div className="tk-taco__shadow" />
        </div>

        <ol className="tk-taco__feature-list" aria-label="Takokit features">
          {FEATURES.map((feature, index) => (
            <li className={activeIndex === index ? "is-active" : ""} key={feature.key}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <div>
                <p>{feature.eyebrow}</p>
                <h3>{feature.title}</h3>
                <p>{feature.description}</p>
              </div>
            </li>
          ))}
        </ol>

        <div className="tk-taco__progress" aria-hidden="true">
          <span style={{ transform: `scaleX(${staticLayout ? 1 : (activeIndex + 1) / FEATURES.length})` }} />
        </div>
      </div>
    </section>
  );
}
