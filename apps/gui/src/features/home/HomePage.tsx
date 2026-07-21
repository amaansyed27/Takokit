import { Box, FileAudio, Mic, Server, Volume2 } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { StatTile } from "../../components/ui/StatTile";
import { Tooltip } from "../../components/ui/Tooltip";
import { useServerStatus } from "../../hooks/useServerStatus";

export function HomePage({ runtime, onNavigate }: RouteComponentProps) {
  const ready = runtime.models.filter((model) => model.executable).length;
  const status = useServerStatus(runtime);

  return (
    <section className="page">
      <header className="page__header">
        <h1>Local voice runtime</h1>
        <p>Use installed models, manage local voices, and run speech or transcription tasks.</p>
      </header>

      <div className="stats-grid" aria-label="Runtime summary">
        <StatTile label="Installed models" value={runtime.models.length} detail="Verified locally" />
        <StatTile label="Ready" value={ready} detail="Executable now" />
        <StatTile label="Voices" value={runtime.voices.length} detail="Profiles available" />
        <Tooltip content={`Local server at ${status.url}`}>
          <div>
            <StatTile label="Server" value={status.label} detail={status.uptime} />
          </div>
        </Tooltip>
      </div>

      <Section title="Quick actions" description="Start from the common paths.">
        <div className="quick-actions">
          <button className="quick-action" type="button" onClick={() => onNavigate("speak")}>
            <Volume2 size={18} />
            <strong>Generate speech</strong>
            <span>Installed TTS models</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("voices")}>
            <Mic size={18} />
            <strong>Add voice</strong>
            <span>Consent required</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("transcribe")}>
            <FileAudio size={18} />
            <strong>Transcribe audio</strong>
            <span>Installed STT models</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("models")}>
            <Box size={18} />
            <strong>Manage models</strong>
            <span>Installed only</span>
          </button>
        </div>
      </Section>

      <Section title="Available tasks" description="Capabilities exposed by the local runtime.">
        <div className="capability-strip">
          {runtime.capabilities.map((capability) => (
            <div className="capability-chip" key={capability.id}>
              <strong>{capability.label}</strong>
              <span>{capability.description}</span>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Runtime boundary">
        <div className="runtime-boundary">
          <div className="boundary-grid" aria-hidden="true">
            <span>cli</span>
            <span>server</span>
            <span>gui</span>
            <span>adapters</span>
            <span>runners</span>
          </div>
          <p>Rust CLI, daemon, browser GUI, storage, safety, and runners stay separated.</p>
          <Badge tone="success"><Server size={12} /> Local-first</Badge>
        </div>
      </Section>

      <Section title="Recent outputs">
        <div className="empty-state">
          <strong>No generated audio yet</strong>
          <p>{runtime.modeNote}</p>
        </div>
      </Section>
    </section>
  );
}
