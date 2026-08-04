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
          <p>Real registry data, declared runners, hardware information, and evidence-based verification states.</p>
        </header>

        <div className="model-preview__list">
          {status === "loading" && <LoadingState />}
          {status === "error" && <ErrorState error={error} onRetry={retry} />}
          {status === "ready" && models.map((model, index) => (
            <RouteLink
              href={model.release.tag === model.default_tag
                ? `/models/${model.name}`
                : `/models/${model.name}/${model.release.tag}`}
              className="model-preview__row"
              key={model.ref}
            >
              <span className="model-preview__index">{String(index + 1).padStart(2, "0")}</span>
              <div className="model-preview__identity">
                <small>{model.tasks?.slice(0, 2).join(" / ") || "LOCAL VOICE"}</small>
                <h3>{model.release.target === "whisper-tiny" ? "Whisper Tiny" : model.display_name}</h3>
                <p>{model.shortSummary}</p>
              </div>
              <div className="model-preview__facts">
                <HardwareSummary hardware={model.hardware} />
                <VerificationBadge status={model.status} />
              </div>
              <code>tako pull {model.ref}</code>
              <span className="model-preview__arrow" aria-hidden="true">↗</span>
            </RouteLink>
          ))}
        </div>

        <RouteLink href="/models" className="landing-button landing-button--dark">Browse all models</RouteLink>
      </div>
    </section>
  );
}
