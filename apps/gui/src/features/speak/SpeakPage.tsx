import { useMemo, useState } from "react";
import { ChevronDown, Waves } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Select } from "../../components/ui/Select";
import { Table, TableRow } from "../../components/ui/Table";
import { TextArea } from "../../components/ui/TextArea";
import { Tooltip } from "../../components/ui/Tooltip";
import { useMockGeneration } from "../../hooks/useMockGeneration";
import { installRunner, pullModel, pullRunner } from "../../lib/api";

export function SpeakPage({ runtime, onNavigate }: RouteComponentProps) {
  const ttsModels = useMemo(() => runtime.models.filter((model) => model.capabilities.includes("tts")), [runtime.models]);
  const [text, setText] = useState("");
  const [model, setModel] = useState(ttsModels[0]?.id ?? "");
  const [voice, setVoice] = useState(runtime.voices[0]?.id ?? "");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const selectedModel = ttsModels.find((item) => item.id === model) ?? ttsModels[0];
  const selectedVoice = runtime.voices.find((item) => item.id === voice) ?? runtime.voices[0];
  const { error, generate, isGenerating, result } = useMockGeneration();
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const apiUnavailable = runtime.server.status !== "online";
  const requiredRunner = selectedModel ? runtime.runners.find((runner) => runner.id === selectedModel.runner) : undefined;
  const canGenerate = Boolean(selectedModel?.executable && !apiUnavailable);
  const blocker = apiUnavailable
    ? "Start takokit serve or takokit gui to use the local API."
    : selectedModel?.executable
      ? null
      : selectedModel?.missing.join("; ") || "This TTS model is not executable today.";

  async function runAction(label: string, action: () => Promise<void>) {
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      setNotice("Local runtime state updated. Refreshing the page data may change executable state.");
    } catch (caught) {
      setNotice(caught instanceof Error ? caught.message : "Takokit API action failed.");
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Speak</h1>
        <p>TTS through the local Live Audio API surface. Real models only run when their planner state is executable.</p>
      </header>

      <form
        className="section"
        onSubmit={(event) => {
          event.preventDefault();
          if (!canGenerate) return;
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
                hint={`${selectedModel?.family ?? "local"} - ${selectedModel?.lifecycleState ?? "unknown"} - ${selectedModel?.backend ?? "runtime"}`}
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
            <div className="generation-actions__meta">
              <strong>TTS + Live Audio API</strong>
              <span>{selectedModel?.executable ? "Selected model can run through the local API." : blocker}</span>
            </div>
            <span className="badge-list">
              <Badge tone={selectedModel?.executable ? "success" : "warning"}>{selectedModel?.executable ? "executable" : "blocked"}</Badge>
              <Badge tone={selectedModel?.id === "mock-tts" ? "neutral" : "warning"}>{selectedModel?.id === "mock-tts" ? "internal test path" : selectedModel?.runnerRuntimeState ?? "unknown"}</Badge>
            </span>
            <Button variant="primary" type="submit" loading={isGenerating} disabled={!canGenerate}>
              <Waves size={16} /> Generate Speech
            </Button>
            <span className="action-cluster">
              <Button
                type="button"
                variant="ghost"
                disabled={apiUnavailable || !selectedModel || selectedModel.status === "installed"}
                loading={busyAction === "pull-model"}
                onClick={() => selectedModel && runAction("pull-model", () => pullModel(selectedModel.id).then(() => undefined))}
              >
                Pull model
              </Button>
              <Button
                type="button"
                variant="ghost"
                disabled={apiUnavailable || !requiredRunner || requiredRunner.installed}
                loading={busyAction === "pull-runner"}
                onClick={() => requiredRunner && runAction("pull-runner", () => pullRunner(requiredRunner.id).then(() => undefined))}
              >
                Pull runner
              </Button>
              <Button
                type="button"
                variant="ghost"
                disabled={apiUnavailable || !requiredRunner?.installed || requiredRunner.install_state === "ready"}
                loading={busyAction === "install-runner"}
                onClick={() => requiredRunner && runAction("install-runner", () => installRunner(requiredRunner.id).then(() => undefined))}
              >
                Install runner
              </Button>
            </span>
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
          {result ? (
            <div className="detail-grid output-detail">
              <span><strong>Model</strong>{result.model}</span>
              <span><strong>Engine</strong>{result.engine}</span>
              <span><strong>Content type</strong>{result.content_type}</span>
              <span><strong>Bytes</strong>{result.bytes}</span>
              <span><strong>Sample rate</strong>{result.sample_rate ? `${result.sample_rate} Hz` : "not reported"}</span>
              <span><strong>Output path</strong>{result.output_path}</span>
            </div>
          ) : (
            <div className="empty-state">
              <strong>No speech output yet</strong>
              <p>Audio saved locally will appear here after a successful API response.</p>
            </div>
          )}
          <div className="audio-output__footer">
            <span className={result ? "reveal-note" : ""}>{result ? "Audio saved locally at the path above." : "No output path yet."}</span>
          </div>
        </div>
      </Section>

      <Section title="Installed models">
        <Table columns={["Model", "Capabilities", "Size", "Backend", "Status"]} ariaLabel="Installed text to speech models">
          {ttsModels.slice(0, 3).map((item) => (
            <TableRow key={item.id}>
              <strong>{item.name}</strong>
              <span className="badge-list">
                <Badge tone="neutral">TTS</Badge>
                {item.capabilities.includes("live_audio") ? <Badge tone="neutral">Live Audio</Badge> : null}
              </span>
              <span>{item.size ?? "-"}</span>
              <Tooltip content={`${item.runtime} runner, ${item.license} license label`}>
                <span>{item.backend}</span>
              </Tooltip>
              <Badge tone={item.executable ? "success" : "warning"}>{item.executable ? "executable" : item.lifecycleState}</Badge>
            </TableRow>
          ))}
        </Table>
        {notice && <p className="notice-line">{notice}</p>}
        {blocker && <p className="notice-line">Selected blocker: {blocker} Next: {selectedModel?.nextCommand}</p>}
        <Button className="align-start" variant="ghost" type="button" onClick={() => onNavigate("models")}>
          View all models
        </Button>
      </Section>
    </section>
  );
}
