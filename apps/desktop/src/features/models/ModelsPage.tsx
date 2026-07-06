import { SectionHeader } from "../../components/SectionHeader";
import type { ModelSummary } from "../../lib/types";

type ModelsPageProps = {
  models: ModelSummary[];
};

export function ModelsPage({ models }: ModelsPageProps) {
  return (
    <section className="page-flow">
      <SectionHeader title="Model registry" description="Adapter metadata without direct UI coupling to runner internals." />
      <div className="data-table" role="table" aria-label="Models">
        <div className="table-row table-head" role="row">
          <span>Model</span>
          <span>Purpose</span>
          <span>Runtime</span>
          <span>Status</span>
        </div>
        {models.map((model) => (
          <div className="table-row" role="row" key={model.id}>
            <strong>{model.name}</strong>
            <span>{model.purpose}</span>
            <span>{model.runtime}</span>
            <span className={model.status === "installed" ? "status installed" : "status planned"}>{model.status}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

