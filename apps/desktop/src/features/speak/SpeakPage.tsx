import { useMemo, useState } from "react";
import { ChevronDown, Download, MoreVertical, Play, Volume2, Waves } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Select } from "../../components/ui/Select";
import { Table, TableRow } from "../../components/ui/Table";
import { TextArea } from "../../components/ui/TextArea";
import { Tooltip } from "../../components/ui/Tooltip";
import { useMockGeneration } from "../../hooks/useMockGeneration";

export function SpeakPage({ runtime, onNavigate }: RouteComponentProps) {
  const ttsModels = useMemo(() => runtime.models.filter((model) => model.capabilities.includes("tts")), [runtime.models]);
  const [text, setText] = useState("");
  const [model, setModel] = useState(ttsModels[0]?.id ?? "");
  const [voice, setVoice] = useState(runtime.voices[0]?.id ?? "");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const selectedModel = ttsModels.find((item) => item.id === model) ?? ttsModels[0];
  const selectedVoice = runtime.voices.find((item) => item.id === voice) ?? runtime.voices[0];
  const { error, generate, isGenerating, result } = useMockGeneration();

  return (
    <section className="page">
      <header className="page__header">
        <h1>Speak</h1>
        <p>Generate natural speech from text using local models.</p>
      </header>

      <form
        className="section"
        onSubmit={(event) => {
          event.preventDefault();
          void generate({ model, voice, input: text });
        }}
      >
        <div className="form-grid">
          <Tooltip content={`License: ${selectedModel?.license ?? "unknown"}. Backend: ${selectedModel?.backend ?? "mock"}.`}>
            <div>
              <Select
                label="Model"
                value={model}
                onChange={(event) => setModel(event.target.value)}
                hint={`${selectedModel?.language ?? "Local"} - ${selectedModel?.params ?? "-"} params - ${selectedModel?.backend ?? "Mock"}`}
                options={ttsModels.map((item) => ({ value: item.id, label: item.name }))}
              />
            </div>
          </Tooltip>

          <Select
            label="Voice"
            value={voice}
            onChange={(event) => setVoice(event.target.value)}
            hint={selectedVoice?.label}
            options={runtime.voices.map((item) => ({ value: item.id, label: item.name }))}
          />
        </div>

        <div className="speak-input-grid">
          <TextArea
            label="Text input"
            value={text}
            onChange={(event) => setText(event.target.value)}
            maxLength={5000}
            placeholder="Enter text to speak..."
            error={error ?? undefined}
            count={`${text.length} / 5000`}
          />

          <aside className="generation-actions">
            <Button variant="primary" type="submit" loading={isGenerating}>
              <Waves size={16} /> Generate Speech
            </Button>
            <Tooltip content="Preview is disabled until real audio playback is wired.">
              <Button disabled type="button">
                <Play size={16} /> Preview (5s)
              </Button>
            </Tooltip>
            <button className="button button--secondary" type="button" onClick={() => setAdvancedOpen((open) => !open)}>
              <span>Advanced Options</span>
              <ChevronDown size={16} aria-hidden="true" />
            </button>
          </aside>
        </div>

        <div className={advancedOpen ? "advanced-panel open surface" : "advanced-panel surface"}>
          <div className="settings-row">
            <span>Temperature</span>
            <strong>Default</strong>
          </div>
          <div className="settings-row">
            <span>Output format</span>
            <strong>WAV</strong>
          </div>
        </div>
      </form>

      <Section title="Output">
        <div className={result ? "audio-output surface revealed" : "audio-output surface"}>
          <div className="audio-player">
            <Play size={18} fill="currentColor" />
            <span>00:00 / 00:00</span>
            <div className="audio-track"><span /></div>
            <Volume2 size={18} />
            <MoreVertical size={18} />
          </div>
          <div className="audio-output__footer">
            <span className={result ? "reveal-note" : ""}>{result ?? "Audio will appear here after generation."}</span>
            <Button type="button" disabled={!result}>
              <Download size={15} /> Download Audio
            </Button>
          </div>
        </div>
      </Section>

      <Section title="Installed models">
        <Table columns={["Model", "Params", "Size", "Backend", "Status"]} ariaLabel="Installed text to speech models">
          {ttsModels.slice(0, 3).map((item, index) => (
            <TableRow key={item.id}>
              <strong>{item.name}</strong>
              <span>{item.params}</span>
              <span>{item.size}</span>
              <Tooltip content={`${item.runtime} runner, ${item.license} license label`}>
                <span>{item.backend}</span>
              </Tooltip>
              <Badge tone={index === 0 ? "success" : "neutral"}>{index === 0 ? "active" : item.status}</Badge>
            </TableRow>
          ))}
        </Table>
        <Button className="align-start" variant="ghost" type="button" onClick={() => onNavigate("models")}>
          View all models
        </Button>
      </Section>
    </section>
  );
}

