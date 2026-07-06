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
        <h1>Local voice runtime</h1>
        <p>Run, organize, and test local voice models without hidden cloud calls or fake inference claims.</p>
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

      <Section title="Quick actions" description="Common local workflows. Unwired actions are marked honestly.">
        <div className="quick-actions">
          <button className="quick-action" type="button" onClick={() => onNavigate("speak")}>
            <Volume2 size={18} />
            <strong>Generate speech</strong>
            <span>Use the mock/local speech shape</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("voices")}>
            <Mic size={18} />
            <strong>Add voice</strong>
            <span>Consent flow planned</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("transcribe")}>
            <FileAudio size={18} />
            <strong>Transcribe audio</strong>
            <span>Whisper runner not installed</span>
          </button>
          <button className="quick-action" type="button" onClick={() => onNavigate("models")}>
            <Library size={18} />
            <strong>Manage models</strong>
            <span>Review registry metadata</span>
          </button>
        </div>
      </Section>

      <Section title="Runtime boundary">
        <div className="runtime-boundary">
          <p>
            Takokit keeps the desktop UI, CLI, Axum server, storage, safety checks, and model runners separated.
            Python is reserved for isolated runner processes where PyTorch models require it.
          </p>
          <Badge tone="success"><Server size={12} /> Local-first</Badge>
        </div>
      </Section>

      <Section title="Recent outputs">
        <div className="empty-state">
          <strong>No generated audio yet</strong>
          <p>Generated files will appear here after the mock speech flow writes output metadata.</p>
        </div>
      </Section>
    </section>
  );
}

