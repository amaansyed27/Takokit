import { RouteLink } from "../app/router";
import { CommandBar } from "../components/CommandBar";
import { HardwareSummary } from "../components/HardwareSummary";
import { PlatformInstall } from "../components/PlatformInstall";
import { ErrorState, LoadingState } from "../components/States";
import { VerificationBadge } from "../components/VerificationBadge";
import { useRegistry } from "../hooks/useRegistry";
import { RECOMMENDED_REFS, TASKS } from "../models/presentation";
import { resolveModel, resolveRelease } from "../models/registry";

const TASK_LINKS = [
  ["speech", "Generate speech", "Text to speech"],
  ["transcription", "Transcribe audio", "Speech to text"],
  ["cloning", "Clone a voice", "Consent-backed voice cloning"],
  ["conversion", "Convert a voice", "Voice conversion"],
];

function recommendedModels(registry) {
  return RECOMMENDED_REFS.map((ref) => {
    const split = ref.indexOf(":");
    const name = split === -1 ? ref : ref.slice(0, split);
    const tag = split === -1 ? undefined : ref.slice(split + 1);
    const model = resolveModel(registry, name);
    if (!model) return null;
    if (!tag) return model;
    const release = resolveRelease(model, tag);
    if (!release) return null;
    return {
      ...model,
      release,
      ref,
      status: ref === "whisper:tiny" ? "verified" : model.status,
      sizeBytes: release.size_bytes > 0 ? release.size_bytes : null,
      hardware: {
        cpu: Boolean(release.hardware?.cpu),
        gpu: Boolean(release.hardware?.gpu),
        gpuRequired: release.hardware?.cpu === false && release.hardware?.gpu === true,
        minRam: release.hardware?.min_ram || null,
        minVram: release.hardware?.min_vram || null,
      },
    };
  }).filter(Boolean);
}

export function HomePage() {
  const { status, registry, error, retry } = useRegistry();
  return (
    <main>
      <section className="hero shell">
        <div className="hero-copy">
          <p className="eyebrow">Local voice runtime</p>
          <h1>Run open voice models locally.</h1>
          <p className="hero-summary">
            Generate speech, transcribe audio, clone consented voices, and convert recordings
            through one local Rust-first runtime.
          </p>
          <PlatformInstall heading="Install for your machine" />
          <div className="hero-quickstart">
            <span>Already installed?</span>
            <CommandBar compact>tako pull kokoro</CommandBar>
          </div>
          <div className="hero-actions">
            <RouteLink href="/download" className="button button-primary">Installation details</RouteLink>
            <RouteLink href="/models" className="button button-secondary">Browse models</RouteLink>
          </div>
        </div>
        <div className="hero-mark" aria-hidden="true">
          <img src="/brand/takokit-mark.svg" alt="" />
          <p>CLI · TUI · GUI · API</p>
        </div>
      </section>

      <section className="section shell">
        <div className="section-heading">
          <div><p className="eyebrow">Choose a task</p><h2>What do you need to do?</h2></div>
        </div>
        <div className="task-shortcuts">
          {TASK_LINKS.map(([task, title, detail]) => (
            <RouteLink href={`/models?task=${task}`} key={task}>
              <strong>{title}</strong>
              <span>{detail}</span>
              <small>{TASKS[task].label} →</small>
            </RouteLink>
          ))}
        </div>
      </section>

      <section className="section shell">
        <div className="section-heading">
          <div><p className="eyebrow">Recommended models</p><h2>Useful places to start.</h2></div>
          <RouteLink href="/models">View all models →</RouteLink>
        </div>
        {status === "loading" && <LoadingState />}
        {status === "error" && <ErrorState error={error} onRetry={retry} />}
        {status === "ready" && (
          <div className="recommended-list">
            {recommendedModels(registry).map((model) => (
              <RouteLink
                href={model.release.tag === model.default_tag
                  ? `/models/${model.name}`
                  : `/models/${model.name}/${model.release.tag}`}
                key={model.ref}
                className="recommended-row"
              >
                <div><strong>{model.release.target === "whisper-tiny" ? "Whisper Tiny" : model.display_name}</strong><p>{model.shortSummary}</p></div>
                <HardwareSummary hardware={model.hardware} />
                <VerificationBadge status={model.status} />
              </RouteLink>
            ))}
          </div>
        )}
      </section>

      <section className="section section-ink">
        <div className="shell">
          <div className="section-heading">
            <div><p className="eyebrow">How Takokit works</p><h2>One local workflow.</h2></div>
          </div>
          <ol className="how-list">
            <li><span>1</span><div><strong>Pull a model</strong><p>Choose a model reference from the curated registry.</p></div></li>
            <li><span>2</span><div><strong>Run it locally</strong><p>Takokit prepares the required runtime and writes outputs to your project.</p></div></li>
            <li><span>3</span><div><strong>Use any surface</strong><p>Work from the CLI, GUI, TUI, or local HTTP API.</p></div></li>
          </ol>
        </div>
      </section>
    </main>
  );
}
