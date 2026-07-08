import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";
import { pullModel, pullRunner, removeModel } from "../../lib/api";
import type { ModelCapability } from "../../lib/types";

export function ModelsPage({ runtime, onRefresh }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(runtime.models.find((model) => model.id !== "mock-tts")?.id ?? runtime.models[0]?.id ?? "");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const models = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return runtime.models;
    return runtime.models.filter((model) =>
      [model.name, model.purpose, model.runtime, model.status, model.license].some((value) => value.toLowerCase().includes(needle))
    );
  }, [query, runtime.models]);
  const selectedModel = runtime.models.find((model) => model.id === selectedId) ?? models[0] ?? runtime.models[0];
  const requiredRunner = selectedModel ? runtime.runners.find((runner) => runner.id === selectedModel.runner) : undefined;
  const apiUnavailable = runtime.server.status !== "online";

  async function runAction(label: string, action: () => Promise<void>) {
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
      setNotice("Lifecycle metadata updated. Real inference remains unimplemented except mock-tts.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Takokit API action failed.");
    } finally {
      setBusyAction(null);
    }
  }

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
        <Table columns={["Model", "Capabilities", "Runner", "Status", "Actions"]} ariaLabel="Models">
          {models.map((model) => (
            <TableRow key={model.id}>
              <strong>{model.name}</strong>
              <span className="badge-list" aria-label={`${model.name} capabilities`}>
                {model.capabilities.map((capability) => (
                  <Badge key={capability} tone="neutral">{capabilityLabel(capability)}</Badge>
                ))}
              </span>
              <Tooltip content={`${model.backend} backend, ${model.version} manifest version`}>
                <span>{model.runner}</span>
              </Tooltip>
              <Badge tone={model.status === "installed" ? "success" : model.status === "available" ? "neutral" : "warning"}>
                {model.status}
              </Badge>
              <span className="action-cluster">
                <Button type="button" variant="ghost" onClick={() => setSelectedId(model.id)}>Show</Button>
                {model.id !== "mock-tts" && (
                  model.status === "installed" ? (
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={apiUnavailable}
                      loading={busyAction === `remove-model-${model.id}`}
                      onClick={() => runAction(`remove-model-${model.id}`, () => removeModel(model.id))}
                    >
                      Remove
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={apiUnavailable}
                      loading={busyAction === `pull-model-${model.id}`}
                      onClick={() => runAction(`pull-model-${model.id}`, () => pullModel(model.id).then(() => undefined))}
                    >
                      Pull
                    </Button>
                  )
                )}
              </span>
            </TableRow>
          ))}
        </Table>
      </Section>

      {selectedModel && (
        <Section title="Details" description="Manifest metadata and lifecycle state.">
          <div className="details-panel">
            <div className="details-panel__main">
              <h3>{selectedModel.name}</h3>
              <p>{selectedModel.purpose}</p>
              <div className="detail-grid">
                <span><strong>ID</strong>{selectedModel.id}</span>
                <span><strong>Version</strong>{selectedModel.version}</span>
                <span><strong>Backend</strong>{selectedModel.backend}</span>
                <span><strong>License</strong>{selectedModel.license}</span>
                <span><strong>Artifacts</strong>{selectedModel.artifactCount}</span>
                <span><strong>Hardware</strong>{selectedModel.hardwareNotes}</span>
                <span><strong>Execution</strong>{selectedModel.executionStatus}</span>
              </div>
            </div>
            <div className="details-panel__side">
              <Badge tone={selectedModel.status === "installed" ? "success" : "neutral"}>{selectedModel.status}</Badge>
              <Badge tone={selectedModel.runnerInstalled ? "success" : "warning"}>
                {selectedModel.runnerInstalled ? "runner installed" : "runner missing"}
              </Badge>
              <span className="details-panel__runner">Required runner: {selectedModel.runner}</span>
              {requiredRunner && !requiredRunner.installed && selectedModel.id !== "mock-tts" && (
                <Button
                  type="button"
                  disabled={apiUnavailable}
                  loading={busyAction === `pull-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`pull-runner-${requiredRunner.id}`, () => pullRunner(requiredRunner.id).then(() => undefined))}
                >
                  Pull Required Runner
                </Button>
              )}
            </div>
          </div>
          {notice && <p className="notice-line">{notice}</p>}
        </Section>
      )}

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
