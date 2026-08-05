import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";
import {
  getModelPlan,
  installRunner,
  previewModelRemoval,
  pullModel,
  pullRunner,
  removeModel
} from "../../lib/api";
import type { ModelCapability, ModelPlan, ModelSummary } from "../../lib/types";

type ViewMode = "installed" | "library";

export function ModelsPage({ runtime, onRefresh }: RouteComponentProps) {
  const [view, setView] = useState<ViewMode>("installed");
  const [query, setQuery] = useState("");
  const sourceModels = view === "installed" ? runtime.models : runtime.catalogModels;
  const [selectedId, setSelectedId] = useState(sourceModels[0]?.id ?? "");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [modelPlan, setModelPlan] = useState<ModelPlan | null>(null);
  const apiUnavailable = runtime.server.status !== "online";

  const models = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return sourceModels;
    return sourceModels.filter((model) =>
      [model.id, model.name, model.family, model.runner, model.status, ...model.capabilities]
        .some((value) => value.toLowerCase().includes(needle))
    );
  }, [query, sourceModels]);

  const selectedModel = sourceModels.find((model) => model.id === selectedId) ?? models[0];
  const requiredRunner = selectedModel
    ? runtime.runners.find((runner) => runner.id === selectedModel.runner)
    : undefined;

  useEffect(() => {
    if (!sourceModels.some((model) => model.id === selectedId)) {
      setSelectedId(sourceModels[0]?.id ?? "");
    }
  }, [sourceModels, selectedId]);

  useEffect(() => {
    let cancelled = false;
    setModelPlan(null);
    if (!selectedModel || apiUnavailable) return;

    getModelPlan(selectedModel.id)
      .then((plan) => {
        if (!cancelled) setModelPlan(plan);
      })
      .catch((error) => {
        if (!cancelled) {
          setModelPlan(null);
          setNotice(error instanceof Error ? error.message : "Model planning failed.");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [apiUnavailable, selectedModel]);

  async function runAction(label: string, action: () => Promise<void>) {
    if (busyAction) return;
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
      setNotice("Backend operation completed and local state was refreshed.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Takokit API action failed.");
    } finally {
      setBusyAction(null);
    }
  }

  async function removeInstalledModel(model: ModelSummary) {
    const preview = await previewModelRemoval(model.id);
    if (!preview.installed) {
      throw new Error(`${model.id} is not installed; no files were deleted.`);
    }
    const reclaim = preview.reclaim_bytes ? ` Approximately ${formatBytes(preview.reclaim_bytes)} can be reclaimed.` : "";
    const retained = preview.retained_shared_paths?.length
      ? ` ${preview.retained_shared_paths.length} shared path(s) will be retained.`
      : "";
    if (!window.confirm(`Remove ${model.name}?${reclaim}${retained}`)) return;
    await removeModel(model.id);
  }

  const readyCount = runtime.models.filter((model) => model.executable).length;
  const ttsCount = runtime.models.filter((model) => model.capabilities.includes("tts")).length;
  const sttCount = runtime.models.filter((model) => model.capabilities.includes("stt")).length;

  return (
    <section className="page">
      <header className="page__header">
        <h1>Models</h1>
        <p>Browse the registry separately from models installed and verified on this machine.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Installed</span><strong className="stat-tile__value">{runtime.models.length}</strong><small>Verified locally</small></div>
        <div className="stat-tile"><span>Ready</span><strong className="stat-tile__value">{readyCount}</strong><small>Executable now</small></div>
        <div className="stat-tile"><span>TTS</span><strong className="stat-tile__value">{ttsCount}</strong><small>Installed speech models</small></div>
        <div className="stat-tile"><span>STT</span><strong className="stat-tile__value">{sttCount}</strong><small>Installed transcription models</small></div>
      </div>

      <Section title={view === "installed" ? "Installed models" : "Model library"} description={runtime.modeNote}>
        <div className="action-cluster">
          <Button type="button" variant={view === "installed" ? "primary" : "ghost"} onClick={() => setView("installed")}>Installed</Button>
          <Button type="button" variant={view === "library" ? "primary" : "ghost"} onClick={() => setView("library")}>Library</Button>
        </div>
        {sourceModels.length > 0 && (
          <input
            className="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={view === "installed" ? "Filter installed models..." : "Search model library..."}
            aria-label="Filter models"
          />
        )}

        {sourceModels.length === 0 ? (
          <div className="empty-state">
            <strong>{view === "installed" ? "No models installed." : "Model catalog unavailable."}</strong>
            <p>{view === "installed" ? "Open the Library tab to plan and pull a model." : "Retry after the Takokit registry is available."}</p>
          </div>
        ) : (
          <Table columns={["Model", "Capabilities", "Runner", "State", "Actions"]} ariaLabel="Takokit models">
            {models.map((model) => (
              <TableRow key={model.id}>
                <div>
                  <strong>{model.name}</strong>
                  <span className="table-note">{model.id} · {model.family}</span>
                </div>
                <span className="badge-list" aria-label={`${model.name} capabilities`}>
                  {model.capabilities.map((capability) => (
                    <Badge key={capability} tone="neutral">{capabilityLabel(capability)}</Badge>
                  ))}
                </span>
                <Tooltip content={`${model.backend} backend, ${model.version} manifest version`}>
                  <span>{model.runner}</span>
                </Tooltip>
                <Badge tone={model.executable ? "success" : model.status === "installed" ? "warning" : "neutral"}>
                  {model.executable ? "ready" : model.status === "installed" ? "needs repair" : "available"}
                </Badge>
                <span className="action-cluster">
                  <Button type="button" variant="ghost" onClick={() => setSelectedId(model.id)}>Show</Button>
                  {model.status !== "installed" && (
                    <Button
                      type="button"
                      disabled={apiUnavailable || busyAction !== null}
                      loading={busyAction === `pull-model-${model.id}`}
                      onClick={() => runAction(`pull-model-${model.id}`, () => pullModel(model.id).then(() => undefined))}
                    >Pull</Button>
                  )}
                  {model.status === "installed" && !model.executable && (
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={apiUnavailable || busyAction !== null}
                      loading={busyAction === `repair-model-${model.id}`}
                      onClick={() => runAction(`repair-model-${model.id}`, () => pullModel(model.id).then(() => undefined))}
                    >Repair</Button>
                  )}
                  {model.status === "installed" && (
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={apiUnavailable || busyAction !== null}
                      loading={busyAction === `remove-model-${model.id}`}
                      onClick={() => runAction(`remove-model-${model.id}`, () => removeInstalledModel(model))}
                    >Remove</Button>
                  )}
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
                <span><strong>Canonical ID</strong>{selectedModel.id}</span>
                <span><strong>Version</strong>{selectedModel.version}</span>
                <span><strong>Family</strong>{selectedModel.family}</span>
                <span><strong>Runner</strong>{selectedModel.runner}</span>
                <span><strong>Backend</strong>{selectedModel.backend}</span>
                <span><strong>License</strong>{selectedModel.license}</span>
                <span><strong>Size</strong>{selectedModel.size ?? "Not declared"}</span>
                <span><strong>Hardware</strong>{selectedModel.hardwareNotes}</span>
                <span><strong>Lifecycle</strong>{stateLabel(modelPlan?.lifecycle_state ?? selectedModel.lifecycleState)}</span>
                <span><strong>Runner runtime</strong>{stateLabel(modelPlan?.runner_runtime_state ?? selectedModel.runnerRuntimeState)}</span>
              </div>
              {(modelPlan?.missing.length ?? selectedModel.missing.length) > 0 && (
                <p className="notice-line">Missing: {(modelPlan?.missing ?? selectedModel.missing).join("; ")}</p>
              )}
              <p className="notice-line">Next: {modelPlan?.next_command ?? selectedModel.nextCommand}</p>
            </div>
            <div className="details-panel__side">
              <Badge tone={selectedModel.executable ? "success" : "warning"}>
                {selectedModel.executable ? "ready" : selectedModel.status}
              </Badge>
              {requiredRunner && !requiredRunner.installed && (
                <Button
                  type="button"
                  disabled={apiUnavailable || busyAction !== null}
                  loading={busyAction === `pull-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`pull-runner-${requiredRunner.id}`, () => pullRunner(requiredRunner.id).then(() => undefined))}
                >Prepare runner</Button>
              )}
              {requiredRunner && requiredRunner.installed && requiredRunner.install_state !== "ready" && (
                <Button
                  type="button"
                  disabled={apiUnavailable || busyAction !== null}
                  loading={busyAction === `install-runner-${requiredRunner.id}`}
                  onClick={() => runAction(`install-runner-${requiredRunner.id}`, () => installRunner(requiredRunner.id).then(() => undefined))}
                >Repair runner runtime</Button>
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

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}

function capabilityLabel(capability: ModelCapability): string {
  switch (capability) {
    case "tts": return "TTS";
    case "stt": return "STT";
    case "voice_cloning": return "Cloning";
    case "voice_conversion": return "Conversion";
    case "live_transcription": return "Live STT";
    case "live_audio": return "Live Audio";
  }
}
