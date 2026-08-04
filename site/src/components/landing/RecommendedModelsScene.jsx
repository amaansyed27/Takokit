import { RouteLink } from "../../app/router";
import { HardwareSummary } from "../HardwareSummary";
import { ErrorState, LoadingState } from "../States";
import { VerificationBadge } from "../VerificationBadge";
import { useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

export function RecommendedModelsScene({ models, status, error, retry }) {
  const reducedMotion = useReducedMotion();
  const sectionRef = useScrollProgress((progress, section) => {
    section.style.setProperty("--tk-models-progress", progress.toFixed(4));
  }, reducedMotion);

  return (
    <section
      className={`tk-models-scene ${reducedMotion ? "is-static" : ""}`}
      ref={sectionRef}
      aria-labelledby="tk-models-title"
    >
      <div className="tk-models-scene__stage">
        <header className="tk-section-bar">
          <span>03 / START SOMEWHERE</span>
          <span>CURATED REFERENCES / HONEST SUPPORT LABELS</span>
        </header>

        <div className="tk-models-scene__intro">
          <p className="tk-kicker">THE MODEL LIBRARY</p>
          <h2 id="tk-models-title">PULL A MODEL.<br />KEEP MOVING.</h2>
          <p>
            Takokit gives each model a versioned reference, a declared runner, real hardware information, and an evidence-based support state.
          </p>
          <RouteLink href="/models" className="tk-action tk-action--dark">Browse the full library</RouteLink>
        </div>

        <div className="tk-models-scene__list">
          {status === "loading" && <LoadingState />}
          {status === "error" && <ErrorState error={error} onRetry={retry} />}
          {status === "ready" && models.map((model, index) => (
            <RouteLink
              href={model.release.tag === model.default_tag
                ? `/models/${model.name}`
                : `/models/${model.name}/${model.release.tag}`}
              key={model.ref}
              className="tk-model-card"
              style={{ "--model-index": index }}
            >
              <span className="tk-model-card__number">0{index + 1}</span>
              <div>
                <p>{model.tasks?.slice(0, 2).join(" / ") || "LOCAL VOICE"}</p>
                <h3>{model.release.target === "whisper-tiny" ? "WHISPER TINY" : model.display_name.toUpperCase()}</h3>
                <p>{model.shortSummary}</p>
              </div>
              <HardwareSummary hardware={model.hardware} />
              <VerificationBadge status={model.status} />
              <span aria-hidden="true">↗</span>
            </RouteLink>
          ))}
        </div>
      </div>
    </section>
  );
}
