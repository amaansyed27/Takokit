import { SectionHeader } from "../../components/SectionHeader";
import type { VoiceSummary } from "../../lib/types";

type VoicesPageProps = {
  voices: VoiceSummary[];
};

export function VoicesPage({ voices }: VoicesPageProps) {
  return (
    <section className="page-flow">
      <SectionHeader title="Voice library" description="Saved voices and consent state will live here as cloning support lands." />
      <div className="data-table compact" role="table" aria-label="Voices">
        <div className="table-row table-head" role="row">
          <span>Voice</span>
          <span>Source</span>
          <span>Model</span>
          <span>Consent</span>
        </div>
        {voices.map((voice) => (
          <div className="table-row" role="row" key={voice.id}>
            <strong>{voice.name}</strong>
            <span>{voice.source}</span>
            <span>{voice.model}</span>
            <span>{voice.consent}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

