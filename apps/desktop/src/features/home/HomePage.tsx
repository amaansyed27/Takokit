import { SectionHeader } from "../../components/SectionHeader";
import type { RuntimeSnapshot } from "../../lib/types";

type HomePageProps = {
  runtime: RuntimeSnapshot;
};

export function HomePage({ runtime }: HomePageProps) {
  return (
    <section className="page-flow">
      <SectionHeader
        title="Local voice runtime"
        description="Manage local speech models, voices, outputs, and server state from one desktop shell."
      />
      <div className="overview-grid">
        <div className="metric-panel">
          <span>Models tracked</span>
          <strong>{runtime.models.length}</strong>
        </div>
        <div className="metric-panel">
          <span>Installed</span>
          <strong>{runtime.models.filter((model) => model.status === "installed").length}</strong>
        </div>
        <div className="metric-panel">
          <span>Voices</span>
          <strong>{runtime.voices.length}</strong>
        </div>
      </div>
      <div className="plain-panel">
        <h3>Runtime boundary</h3>
        <p>
          Takokit keeps the desktop UI, CLI, local server, model registry, storage, and runner layers separate.
          Python runners are reserved for model families that require PyTorch.
        </p>
      </div>
    </section>
  );
}

