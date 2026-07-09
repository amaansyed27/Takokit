import { FileAudio, Library, Mic, Server, Volume2 } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { StatTile } from "../../components/ui/StatTile";
import { Tooltip } from "../../components/ui/Tooltip";
import { useServerStatus } from "../../hooks/useServerStatus";

export function HomePage({ runtime, onNavigate }: RouteComponentProps) {
  const installed = runtime.models.filter((model) => model.status === "installed").length;
  const status = useServerStatus(runtime);

  return (
    <section className="page">
      <header className="page__header">
        <h1>Local web GUI</h1>
        <p>Run the Rust daemon, inspect model plans, install shared runners, and execute models only when they are truly ready.</p>
      </header>

      <div className="stats-grid" aria-label="Runtime summary">
        <StatTile label="Models tracked" value={runtime.models.length} detail="Registry entries" />
        <StatTile label="Installed" value={installed} detail="Ready locally" />
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
            <span>Executable TTS only</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("voices")}>
            <Mic size={18} />
            <strong>Add voice</strong>
            <span>Consent required</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("transcribe")}>
            <FileAudio size={18} />
            <strong>Transcribe audio</strong>
            <span>Whisper runner path</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("models")}>
            <Library size={18} />
            <strong>Manage models</strong>
            <span>Registry</span>
          </button>
        </div>
      </Section>

      <Section title="Product surfaces" description="The runtime treats each surface as an explicit model capability.">
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
          <p>
            Rust CLI, daemon, browser GUI, storage, safety, and runners stay separated.
          </p>
          <Badge tone="success"><Server size={12} /> Local-first</Badge>
        </div>
      </Section>

      <Section title="Runtime lanes" description="Clean boundaries for future runners.">
        <div className="runtime-lanes">
          <div className="runtime-lane">
            <strong>Control plane</strong>
            <span>CLI, local web GUI, and API coordinate jobs.</span>
          </div>
          <div className="runtime-lane">
            <strong>Model adapters</strong>
            <span>TTS, STT, voice cloning, live transcription, and live audio.</span>
          </div>
          <div className="runtime-lane">
            <strong>Runner boundary</strong>
            <span>Python, ONNX, and whisper.cpp stay isolated.</span>
          </div>
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
