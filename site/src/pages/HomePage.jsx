import { CapabilityPinwheel } from "../components/landing/CapabilityPinwheel";
import { CinematicHero } from "../components/landing/CinematicHero";
import { FeatureTaco } from "../components/landing/FeatureTaco";
import { RecommendedModelsScene } from "../components/landing/RecommendedModelsScene";
import { RuntimeFlow } from "../components/landing/RuntimeFlow";
import { useRegistry } from "../hooks/useRegistry";
import { RECOMMENDED_REFS } from "../models/presentation";
import { resolveModel, resolveRelease } from "../models/registry";

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
  const models = status === "ready" ? recommendedModels(registry) : [];

  return (
    <main className="takokit-cinematic">
      <CinematicHero />
      <FeatureTaco />
      <CapabilityPinwheel />
      <RecommendedModelsScene
        error={error}
        models={models}
        retry={retry}
        status={status}
      />
      <RuntimeFlow />
    </main>
  );
}
