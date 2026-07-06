import { Upload } from "lucide-react";
import { SectionHeader } from "../../components/SectionHeader";

export function TranscribePage() {
  return (
    <section className="page-flow">
      <SectionHeader title="Transcribe" description="Whisper and whisper.cpp adapters will connect through the speech-to-text trait." />
      <div className="plain-panel empty-state">
        <Upload size={24} aria-hidden="true" />
        <h3>Transcription runner not installed</h3>
        <p>Drop-zone wiring, transcript output, and dataset preparation belong here after the runner boundary is implemented.</p>
      </div>
    </section>
  );
}

