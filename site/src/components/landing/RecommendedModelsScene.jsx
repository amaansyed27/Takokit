import { RouteLink } from "../../app/router";
import { HardwareSummary } from "../HardwareSummary";
import { ErrorState, LoadingState } from "../States";
import { VerificationBadge } from "../VerificationBadge";

export function RecommendedModelsScene({ models, status, error, retry }) {
  return (
    <section className="tk-models-scene" aria-labelledby="tk-models-title">
      <div className="shell">
        <header className="tk-section-bar">
          <span>03 / START SOMEWHERE</span>
          <span>CURATED REFERENCES / HONEST SUPPORT LABELS</span>
        </header>

        <div className="tk-models-scene__heading">
          <div>
            <p className="tk-kicker">THE MODEL LIBRARY</p>
            <h2 id="tk-models-title">PULL A MODEL.<br />KEEP MOVING.</h2>
          </div>
          <div>
            <p>
              Each model has a versioned reference, declared runner, hardware information, and evidence-based support state.
            </p>
            <RouteLink href="/models" className="tk-action tk-action--dark">Browse the full library</RouteLink>
          </div>
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
            >
              <span className="tk-model-card__number">0{index + 1}</span>
              <div className="tk-model-card__copy">
                <p>{model.tasks?.slice(0, 2).join(" / ") || "LOCAL VOICE"}</p>
                <h3>{model.release.target === "whisper-tiny" ? "WHISPER TINY" : model.display_name.toUpperCase()}</h3>
                <p>{model.shortSummary}</p>
              </div>
              <div className="tk-model-card__meta">
                <HardwareSummary hardware={model.hardware} />
                <VerificationBadge status={model.status} />
              </div>
              <span aria-hidden="true">↗</span>
            </RouteLink>
          ))}
        </div>
      </div>
    </section>
  );
}
