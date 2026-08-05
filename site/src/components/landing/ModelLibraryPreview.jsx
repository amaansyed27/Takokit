import { RouteLink } from "../../app/router";
import { HardwareSummary } from "../HardwareSummary";
import { ErrorState, LoadingState } from "../States";
import { VerificationBadge } from "../VerificationBadge";

export function ModelLibraryPreview({ models, status, error, retry }) {
  return (
    <section className="model-preview" aria-labelledby="model-preview-title">
      <div className="landing-shell model-preview__inner">
        <header className="model-preview__header">
          <div>
            <p className="landing-kicker">Model library</p>
            <h2 id="model-preview-title">Start with a model.</h2>
          </div>
          <RouteLink href="/models" className="landing-text-link">Browse all models →</RouteLink>
        </header>

        <div className="model-preview__list">
          {status === "loading" && <LoadingState />}
          {status === "error" && <ErrorState error={error} onRetry={retry} />}
          {status === "ready" && models.map((model) => (
            <RouteLink
              href={model.release.tag === model.default_tag
                ? `/models/${model.name}`
                : `/models/${model.name}/${model.release.tag}`}
              className="model-preview__row"
              key={model.ref}
            >
              <div className="model-preview__identity">
                <h3>{model.release.target === "whisper-tiny" ? "Whisper Tiny" : model.display_name}</h3>
                <p>{model.tasks?.slice(0, 2).join(" · ") || "Local voice model"}</p>
              </div>
              <code>tako pull {model.ref}</code>
              <div className="model-preview__facts">
                <HardwareSummary hardware={model.hardware} />
                <VerificationBadge status={model.status} />
              </div>
              <span aria-hidden="true">→</span>
            </RouteLink>
          ))}
        </div>
      </div>
    </section>
  );
}
