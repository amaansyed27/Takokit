import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";
import { getModelPlan, installRunner, pullModel, pullRunner, removeModel } from "../../lib/api";
import type { ModelCapability, ModelInstallResponse, ModelPlan } from "../../lib/types";

export function ModelsPage({ runtime, onRefresh }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(runtime.models.find((model) => model.id !== "mock-tts")?.id ?? runtime.models[0]?.id ?? "");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [modelPlan, setModelPlan] = useState<ModelPlan | null>(null);
  const [installReport, setInstallReport] = useState<ModelInstallResponse | null>(null);
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

  useEffect(() => {
    let cancelled = false;
    setModelPlan(null);
    if (!selectedModel || selectedModel.id === "mock-tts" || apiUnavailable) return;

    getModelPlan(selectedModel.id)
      .then((plan) => {
        if (!cancelled) setModelPlan(plan);
      })
      .catch(() => {
        if (!cancelled) setModelPlan(null);
      });

    return () => {
      cancelled = true;
    };
  }, [apiUnavailable, selectedModel]);

  async function runAction(label: string, action: () => Promise<void>) {
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
      setNotice("Local runtime state updated.");
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
        <p>Runtime manifests, installed artifacts, shared runners, and executable state from the canonical planner.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Installed</span><strong className="stat-tile__value">{runtime.models.filter((model) => model.status === "installed").length}</strong><small>Artifacts present</small></div>
        <div className="stat-tile"><span>Executable</span><strong className="stat-tile__value">{runtime.models.filter((model) => model.executable).length}</strong><small>Can run today</small></div>
        <div className="stat-tile"><span>Blocked</span><strong className="stat-tile__value">{runtime.models.filter((model) => !model.executable).length}</strong><small>Missing pieces shown</small></div>
        <div className="stat-tile"><span>Runners</span><strong className="stat-tile__value">{runtime.runners.length}</strong><small>Shared runtime families</small></div>
      </div>

      <Section title="Runtime honesty" description={runtime.modeNote}>
        <div className="capability-strip">
          <div className="capability-chip">
            <strong>TTS</strong>
            <span>Piper artifacts are verified; ONNX phonemizer and session execution remain tracked blockers.</span>
          </div>
          <div className="capability-chip">
            <strong>STT</strong>
            <span>Whisper Base can execute through the whisper.cpp runner when model artifacts and runtime are installed.</span>
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
        <Table columns={["Model", "Capabilities", "Runner", "Lifecycle", "Actions"]} ariaLabel="Models">
          {models.map((model) => (
            <TableRow key={model.id}>
              <div>
                <strong>{model.name}</strong>
                <span className="table-note">{model.family}</span>
              </div>
              <span className="badge-list" aria-label={`${model.name} capabilities`}>
                {model.capabilities.map((capability) => (
                  <Badge key={capability} tone="neutral">{capabilityLabel(capability)}</Badge>
                ))}
              </span>
              <Tooltip content={`${model.backend} backend, ${model.version} manifest version`}>
                <span>{model.runner}</span>
              </Tooltip>
              <span className="badge-list">
                <Badge tone={model.executable ? "success" : model.lifecycleState === "failed" ? "warning" : "neutral"}>
                  {stateLabel(model.lifecycleState)}
                </Badge>
                <Badge tone={model.runnerRuntimeState === "ready" ? "success" : "warning"}>
                  runner {stateLabel(model.runnerRuntimeState)}
                </Badge>
              </span>
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
                      onClick={() => runAction(`pull-model-${model.id}`, () => pullModel(model.id).then((report) => { setInstallReport(report); }))}
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
                <span><strong>Family</strong>{selectedModel.family}</span>
                <span><strong>Backend</strong>{selectedModel.backend}</span>
                <span><strong>License</strong>{selectedModel.license}</span>
                <span><strong>Artifacts</strong>{selectedModel.artifactCount}</span>
                <span><strong>Hardware</strong>{selectedModel.hardwareNotes}</span>
                <span><strong>Execution</strong>{selectedModel.executionStatus}</span>
                <span><strong>Lifecycle</strong>{stateLabel(modelPlan?.lifecycle_state ?? selectedModel.lifecycleState)}</span>
                <span><strong>Artifact state</strong>{stateLabel(modelPlan?.artifact_state ?? selectedModel.lifecycleState)}</span>
                <span><strong>Runner runtime</strong>{stateLabel(modelPlan?.runner_runtime_state ?? selectedModel.runnerRuntimeState)}</span>
                <span><strong>Executable today</strong>{(modelPlan?.executable ?? selectedModel.executable) ? "yes" : "no"}</span>
                <span><strong>Next command</strong>{modelPlan?.next_command ?? selectedModel.nextCommand}</span>
                {selectedModel.licenseWarning && <span><strong>License warning</strong>{selectedModel.licenseWarning}</span>}
              </div>
              {(modelPlan?.missing.length ?? selectedModel.missing.length) > 0 && (
                <p className="notice-line">Missing: {(modelPlan?.missing ?? selectedModel.missing).join("; ")}</p>
              )}
              {installReport?.model_id === selectedModel.id && (
                <p className="notice-line">Install: artifacts {installReport.artifacts.state}; runner {installReport.runner_runtime.state}; {installReport.executable ? "executable" : installReport.missing.join("; ")}</p>
              )}
            </div>
            <div className="details-panel__side">
              <Badge tone={selectedModel.executable ? "success" : "warning"}>{selectedModel.executable ? "executable" : "not executable"}</Badge>
              <Badge tone={selectedModel.status === "installed" ? "success" : "neutral"}>{selectedModel.status}</Badge>
              <Badge tone={selectedModel.runnerRuntimeState === "ready" ? "success" : "warning"}>
                runner {stateLabel(selectedModel.runnerRuntimeState)}
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
              {requiredRunner && requiredRunner.installed && requiredRunner.install_state !== "ready" && selectedModel.id !== "mock-tts" && (
                <Button
                  type="button"
                  disabled={apiUnavailable}
                  loading={busyAction === `install-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`install-runner-${requiredRunner.id}`, () => installRunner(requiredRunner.id).then(() => undefined))}
                >
                  Install Runner Runtime
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
              <Badge tone={runner.install_state === "ready" ? "success" : runner.installed ? "neutral" : "warning"}>
                {runner.install_state ? stateLabel(runner.install_state) : runner.installed ? "contract installed" : "available"}
              </Badge>
              <span>{runner.description}</span>
            </TableRow>
          ))}
        </Table>
      </Section>
    </section>
  );
}

function stateLabel(value: string): string {
  return value.replace(/-/g, " ");
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
