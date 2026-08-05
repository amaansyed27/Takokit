import { RouteLink } from "../app/router";
import { CommandBar } from "../components/CommandBar";
import { ExampleTabs } from "../components/ExampleTabs";
import { HardwareSummary } from "../components/HardwareSummary";
import { ErrorState, LoadingState } from "../components/States";
import { TechnicalDetails } from "../components/TechnicalDetails";
import { VariantTable } from "../components/VariantTable";
import { VerificationBadge } from "../components/VerificationBadge";
import { useRegistry } from "../hooks/useRegistry";
import { integrationExamples, pullCommand, taskCommand } from "../models/examples";
import {
  defaultRelease,
  formatBytes,
  resolveModel,
  resolveRelease,
} from "../models/registry";
import {
  canonicalRef,
  knownLimitations,
  taskLabels,
  verificationStatus,
} from "../models/presentation";

function RvcGuidance() {
  return (
    <section className="content-section notice-section">
      <h2>Using RVC safely and correctly</h2>
      <ul>
        <li>RVC is a voice-conversion runtime, not a bundled target voice.</li>
        <li>A compatible custom <code>.pth</code> checkpoint is required.</li>
        <li>A matching <code>.index</code> file is recommended where available.</li>
        <li>Takokit does not ship celebrity or public-figure impersonation models.</li>
        <li>Successful WAV generation is not proof of perceptual similarity.</li>
        <li>Custom RVC creation and training is planned separately under Issue #68.</li>
      </ul>
    </section>
  );
}

export function ModelDetailPage({ model: modelName, tag }) {
  const { status, registry, error, retry } = useRegistry();
  if (status === "loading") return <main className="shell page"><LoadingState label="Loading model…" /></main>;
  if (status === "error") return <main className="shell page"><ErrorState error={error} onRetry={retry} /></main>;

  const model = resolveModel(registry, modelName);
  const release = resolveRelease(model, tag);
  if (!model || !release) {
    return (
      <main className="shell page not-found">
        <p className="eyebrow">Model library</p>
        <h1>Model not found</h1>
        <p>The requested model or variant is not present in the current registry.</p>
        <RouteLink href="/models" className="button button-primary">Browse models</RouteLink>
      </main>
    );
  }

  const selectedRef = canonicalRef(model.name, release.tag, model.default_tag);
  const statusValue = verificationStatus(model, release);
  const hardware = {
    cpu: Boolean(release.hardware?.cpu),
    gpu: Boolean(release.hardware?.gpu),
    gpuRequired: release.hardware?.cpu === false && release.hardware?.gpu === true,
    minRam: release.hardware?.min_ram || null,
    minVram: release.hardware?.min_vram || null,
  };
  const limitations = knownLimitations(model);
  const defaultVariant = defaultRelease(model);
  const recommendedText = release.tag === model.default_tag
    ? "This is the default variant Takokit resolves when no tag is supplied."
    : `Takokit's default is ${model.name}:${defaultVariant.tag}. This page shows the explicitly selected ${release.tag} variant.`;

  return (
    <main className="shell page model-detail">
      <nav className="breadcrumbs" aria-label="Breadcrumb">
        <RouteLink href="/models">Models</RouteLink><span>/</span><span>{model.display_name}</span>
      </nav>
      <header className="model-detail-head">
        <div>
          <h1>{release.target === "whisper-tiny" ? "Whisper Tiny" : model.display_name}</h1>
          <p className="model-summary">{model.summary}</p>
          <div className="model-badges">
            <span>{taskLabels(model).join(" · ")}</span>
            <HardwareSummary hardware={hardware} />
            <span>{release.license || "Not declared"}</span>
            <VerificationBadge status={statusValue} />
          </div>
          <CommandBar>{pullCommand(model, release)}</CommandBar>
        </div>
        <div className="model-quick-facts">
          <dl>
            <div><dt>Download size</dt><dd>{formatBytes(release.size_bytes)}</dd></div>
            <div><dt>Minimum RAM</dt><dd>{hardware.minRam || "Not declared"}</dd></div>
            <div><dt>Minimum VRAM</dt><dd>{hardware.minVram || "Not declared"}</dd></div>
          </dl>
        </div>
      </header>

      <section className="content-section recommended-variant">
        <p className="eyebrow">Recommended variant</p>
        <h2>{model.name}:{model.default_tag}</h2>
        <p>{recommendedText}</p>
      </section>

      <section className="content-section">
        <h2>Run this model</h2>
        <p>The command below follows Takokit's current CLI parser and capability contract.</p>
        <CommandBar>{taskCommand(model, release)}</CommandBar>
      </section>

      <section className="content-section">
        <h2>Integration examples</h2>
        <ExampleTabs examples={integrationExamples(model, release)} />
      </section>

      <section className="content-section">
        <h2>Hardware guidance</h2>
        <HardwareSummary hardware={hardware} detailed />
        <p><strong>Approximate download size:</strong> {formatBytes(release.size_bytes)}</p>
        {limitations.length ? (
          <div><h3>Important limitations</h3><ul>{limitations.map((item) => <li key={item}>{item}</li>)}</ul></div>
        ) : <p>Additional limitations are not declared in the current presentation metadata.</p>}
      </section>

      {model.name === "rvc" && <RvcGuidance />}

      <section className="content-section">
        <h2>Variants</h2>
        <VariantTable model={model} selected={release} />
      </section>

      <TechnicalDetails release={release} />
      <p className="canonical-ref">Canonical reference: <code>{selectedRef}</code></p>
    </main>
  );
}
