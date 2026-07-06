import { Upload } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Tooltip } from "../../components/ui/Tooltip";

export function TranscribePage(_props: RouteComponentProps) {
  return (
    <section className="page">
      <header className="page__header">
        <h1>Transcribe</h1>
        <p>Whisper and whisper.cpp adapters will connect through the speech-to-text trait.</p>
      </header>

      <Section title="Audio input">
        <div className="empty-state">
          <Upload size={24} aria-hidden="true" />
          <strong>Transcription runner not installed</strong>
          <p>Drop-zone wiring, transcript output, and dataset preparation belong here after the runner boundary is implemented.</p>
          <Tooltip content="Disabled because no real Whisper runner is connected yet.">
            <Button disabled>Select audio</Button>
          </Tooltip>
        </div>
      </Section>
    </section>
  );
}

