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
        <p>STT through the local Live Transcription API surface. Whisper Base runs when its model artifact and whisper.cpp runner are installed.</p>
      </header>

      <Section title="Audio input">
        <div className="upload-zone">
          <div className="upload-zone__icon">
            <Upload size={24} aria-hidden="true" />
          </div>
          <div>
            <strong>Browser upload not wired yet</strong>
            <p>Use the CLI or API transcription endpoint while the GUI upload flow is being built.</p>
          </div>
          <Tooltip content="Disabled because the browser upload workflow is not implemented yet.">
            <Button disabled>Select audio</Button>
          </Tooltip>
        </div>
      </Section>

      <Section title="Pipeline preview" description="Local steps stay visible.">
        <div className="pipeline-strip">
          <div className="pipeline-node">
            <strong>Audio file</strong>
            <span>WAV, MP3, samples.</span>
          </div>
          <div className="pipeline-node">
            <strong>STT runner</strong>
            <span>whisper.cpp when installed.</span>
          </div>
          <div className="pipeline-node">
            <strong>Live Transcription API</strong>
            <span>Text response.</span>
          </div>
          <div className="pipeline-node is-disabled">
            <strong>Dataset prep</strong>
            <span>Clips and metadata.</span>
          </div>
        </div>
      </Section>
    </section>
  );
}
