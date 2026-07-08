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
        <p>STT through the local Live Transcription API surface. Whisper runner execution is not wired yet.</p>
      </header>

      <Section title="Audio input">
        <div className="upload-zone">
          <div className="upload-zone__icon">
            <Upload size={24} aria-hidden="true" />
          </div>
          <div>
            <strong>STT runner not installed</strong>
            <p>Drop audio here once the local Live Transcription API has a real runner.</p>
          </div>
          <Tooltip content="Disabled because no real Whisper runner is connected yet.">
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
          <div className="pipeline-node is-disabled">
            <strong>STT runner</strong>
            <span>Not installed.</span>
          </div>
          <div className="pipeline-node is-disabled">
            <strong>Live Transcription API</strong>
            <span>Text and timestamps.</span>
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
