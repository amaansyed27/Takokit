import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";
import { getModelPlan, installRunner, pullModel, pullRunner, removeModel } from "../../lib/api";
import type { ModelCapability, ModelPlan } from "../../lib/types";

export function ModelsPage({ runtime, onRefresh }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(runtime.models[0]?.id ?? "");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [modelPlan, setModelPlan] = useState<ModelPlan | null>(null);
  const apiUnavailable = runtime.server.status !== "online";

  const models = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return runtime.models;
    return runtime.models.filter((model) =>
      [model.name, model.family, model.runner, model.status]
        .some((value) => value.toLowerCase().includes(needle))
    );
  }, [query, runtime.models]);

  const selectedModel = runtime.models.find((model) => model.id === selectedId) ?? models[0];
  const requiredRunner = selectedModel
    ? runtime.runners.find((runner) => runner.id === selectedModel.runner)
    : undefined;

  useEffect(() => {
    if (!runtime.models.some((model) => model.id === selectedId)) {
      setSelectedId(runtime.models[0]?.id ?? "");
    }
  }, [runtime.models, selectedId]);

  useEffect(() => {
    let cancelled = false;
    setModelPlan(null);
    if (!selectedModel || apiUnavailable) return;

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

  const readyCount = runtime.models.filter((model) => model.executable).length;
  const ttsCount = runtime.models.filter((model) => model.capabilities.includes("tts")).length;
  const sttCount = runtime.models.filter((model) => model.capabilities.includes("stt")).length;

  return (
    <section className="page">
      <header className="page__header">
        <h1>Installed models</h1>
        <p>Models installed and verified on this machine.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Installed</span><strong className="stat-tile__value">{runtime.models.length}</strong><small>Verified locally</small></div>
        <div className="stat-tile"><span>Ready</span><strong className="stat-tile__value">{readyCount}</strong><small>Executable now</small></div>
        <div className="stat-tile"><span>TTS</span><strong className="stat-tile__value">{ttsCount}</strong><small>Speech models</small></div>
        <div className="stat-tile"><span>STT</span><strong className="stat-tile__value">{sttCount}</strong><small>Transcription models</small></div>
      </div>

      <Section title="Models" description={runtime.modeNote}>
        {runtime.models.length > 0 && (
          <input
            className="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filter installed models..."
            aria-label="Filter installed models"
          />
        )}

        {runtime.models.length === 0 ? (
          <div className="empty-state">
            <strong>No models installed</strong>
            <p>Install a model through the Takokit CLI or companion library site, then refresh this page.</p>
          </div>
        ) : (
          <Table columns={["Model", "Type", "Runner", "State", "Actions"]} ariaLabel="Installed models">
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
                <Badge tone={model.executable ? "success" : "warning"}>
                  {model.executable ? "ready" : "needs repair"}
                </Badge>
                <span className="action-cluster">
                  <Button type="button" variant="ghost" onClick={() => setSelectedId(model.id)}>Show</Button>
                  {!model.executable && (
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={apiUnavailable}
                      loading={busyAction === `repair-model-${model.id}`}
                      onClick={() => runAction(`repair-model-${model.id}`, () => pullModel(model.id).then(() => undefined))}
                    >
                      Repair
                    </Button>
                  )}
                  <Button
                    type="button"
                    variant="ghost"
                    disabled={apiUnavailable}
                    loading={busyAction === `remove-model-${model.id}`}
                    onClick={() => runAction(`remove-model-${model.id}`, () => removeModel(model.id))}
                  >
                    Remove
                  </Button>
                </span>
              </TableRow>
            ))}
          </Table>
        )}
      </Section>

      {selectedModel && (
        <Section title="Details">
          <div className="details-panel">
            <div className="details-panel__main">
              <h3>{selectedModel.name}</h3>
              <p>{selectedModel.purpose}</p>
              <div className="detail-grid">
                <span><strong>ID</strong>{selectedModel.id}</span>
                <span><strong>Version</strong>{selectedModel.version}</span>
                <span><strong>Family</strong>{selectedModel.family}</span>
                <span><strong>Runner</strong>{selectedModel.runner}</span>
                <span><strong>Backend</strong>{selectedModel.backend}</span>
                <span><strong>License</strong>{selectedModel.license}</span>
                <span><strong>Hardware</strong>{selectedModel.hardwareNotes}</span>
                <span><strong>Lifecycle</strong>{stateLabel(modelPlan?.lifecycle_state ?? selectedModel.lifecycleState)}</span>
                <span><strong>Runner runtime</strong>{stateLabel(modelPlan?.runner_runtime_state ?? selectedModel.runnerRuntimeState)}</span>
              </div>
              {(modelPlan?.missing.length ?? selectedModel.missing.length) > 0 && (
                <p className="notice-line">Missing: {(modelPlan?.missing ?? selectedModel.missing).join("; ")}</p>
              )}
            </div>
            <div className="details-panel__side">
              <Badge tone={selectedModel.executable ? "success" : "warning"}>
                {selectedModel.executable ? "ready" : "needs repair"}
              </Badge>
              {requiredRunner && !requiredRunner.installed && (
                <Button
                  type="button"
                  disabled={apiUnavailable}
                  loading={busyAction === `pull-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`pull-runner-${requiredRunner.id}`, () => pullRunner(requiredRunner.id).then(() => undefined))}
                >
                  Repair runner
                </Button>
              )}
              {requiredRunner && requiredRunner.installed && requiredRunner.install_state !== "ready" && (
                <Button
                  type="button"
                  disabled={apiUnavailable}
                  loading={busyAction === `install-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`install-runner-${requiredRunner.id}`, () => installRunner(requiredRunner.id).then(() => undefined))}
                >
                  Repair runner runtime
                </Button>
              )}
            </div>
          </div>
          {notice && <p className="notice-line">{notice}</p>}
        </Section>
      )}
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
      return "Cloning";
    case "live_transcription":
      return "Live STT";
    case "live_audio":
      return "Live Audio";
  }
}
