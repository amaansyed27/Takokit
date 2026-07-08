import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";
import type { ModelCapability } from "../../lib/types";

export function ModelsPage({ runtime }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const models = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return runtime.models;
    return runtime.models.filter((model) =>
      [model.name, model.purpose, model.runtime, model.status, model.license].some((value) => value.toLowerCase().includes(needle))
    );
  }, [query, runtime.models]);

  return (
    <section className="page">
      <header className="page__header">
        <h1>Models</h1>
        <p>Available packages, installed manifests, runners, and license labels.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Installed</span><strong className="stat-tile__value">{runtime.models.filter((model) => model.status === "installed").length}</strong><small>Ready entries</small></div>
        <div className="stat-tile"><span>Available</span><strong className="stat-tile__value">{runtime.models.filter((model) => model.status === "available").length}</strong><small>Can be wired next</small></div>
        <div className="stat-tile"><span>Planned</span><strong className="stat-tile__value">{runtime.models.filter((model) => model.status === "planned").length}</strong><small>Runner backlog</small></div>
        <div className="stat-tile"><span>Native path</span><strong className="stat-tile__value">ONNX</strong><small>Preferred where practical</small></div>
      </div>

      <Section title="Runtime honesty" description={runtime.modeNote}>
        <div className="capability-strip">
          <div className="capability-chip">
            <strong>TTS</strong>
            <span>Mock TTS can generate test WAV files. Real TTS runners are not wired yet.</span>
          </div>
          <div className="capability-chip">
            <strong>STT</strong>
            <span>Whisper manifests exist; runner execution is not wired.</span>
          </div>
          <div className="capability-chip">
            <strong>Voice Cloning</strong>
            <span>Voice profile models are tracked as capability metadata only.</span>
          </div>
          <div className="capability-chip">
            <strong>Live APIs</strong>
            <span>Live transcription and live audio are local API surfaces, not cloud calls.</span>
          </div>
        </div>
      </Section>

      <Section title="Registry">
        <input
          className="search-input"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Filter by model, runtime, license, or status..."
          aria-label="Filter models"
        />
        <Table columns={["Model", "Capabilities", "Runtime", "Status", "License"]} ariaLabel="Models">
          {models.map((model) => (
            <TableRow key={model.id}>
              <strong>{model.name}</strong>
              <span className="badge-list" aria-label={`${model.name} capabilities`}>
                {model.capabilities.map((capability) => (
                  <Badge key={capability} tone="neutral">{capabilityLabel(capability)}</Badge>
                ))}
              </span>
              <Tooltip content={`${model.backend} backend, ${model.version} manifest version`}>
                <span>{model.runtime}</span>
              </Tooltip>
              <Badge tone={model.status === "installed" ? "success" : model.status === "available" ? "neutral" : "warning"}>
                {model.status}
              </Badge>
              <Tooltip content="Verify model license before commercial use.">
                <span>{model.license}</span>
              </Tooltip>
            </TableRow>
          ))}
        </Table>
      </Section>

      <Section title="Runners">
        <Table columns={["Runner", "Kind", "Platforms", "Status", "Notes"]} ariaLabel="Runners">
          {runtime.runners.map((runner) => (
            <TableRow key={runner.id}>
              <strong>{runner.name}</strong>
              <span>{runner.kind}</span>
              <span>{runner.platforms.join(", ")}</span>
              <Badge tone={runner.installed ? "success" : "warning"}>{runner.installed ? "installed" : "contract"}</Badge>
              <span>{runner.description}</span>
            </TableRow>
          ))}
        </Table>
      </Section>
    </section>
  );
}

function capabilityLabel(capability: ModelCapability): string {
  switch (capability) {
    case "tts":
      return "TTS";
    case "stt":
      return "STT";
    case "voice_cloning":
      return "Voice Cloning";
    case "live_transcription":
      return "Live Transcription";
    case "live_audio":
      return "Live Audio";
  }
}
