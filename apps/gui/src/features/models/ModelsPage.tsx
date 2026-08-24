import { Check, ChevronRight, Cpu, Download, PackageOpen, Search, Trash2, Wrench } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { getModelPlan, previewModelRemoval, pullModel, removeModel } from "../../lib/api";
import type { ModelCapability, ModelPlan, ModelRemovalReport, ModelSummary } from "../../lib/types";

type ViewMode = "installed" | "library";

export function ModelsPage({ runtime, onNavigate, onRefresh }: RouteComponentProps) {
  const [view, setView] = useState<ViewMode>("installed");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState("");
  const [plan, setPlan] = useState<ModelPlan | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<ModelSummary | null>(null);
  const [removePreview, setRemovePreview] = useState<ModelRemovalReport | null>(null);

  const source = view === "installed" ? runtime.models : runtime.catalogModels;
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return source;
    return source.filter((item) => [
      item.name,
      item.id,
      item.family,
      item.runner,
      item.runtime,
      item.backend,
      ...item.capabilities
    ].some((value) => value.toLowerCase().includes(needle)));
  }, [query, source]);

  const selected = source.find((item) => item.id === selectedId) ?? filtered[0] ?? source[0];
  const online = runtime.server.status === "online";

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected?.id, selectedId]);

  useEffect(() => {
    let cancelled = false;
    setPlan(null);
    if (!selected || !online) return;
    void getModelPlan(selected.id)
      .then((next) => { if (!cancelled) setPlan(next); })
      .catch((error) => { if (!cancelled) setNotice(error instanceof Error ? error.message : "Model plan could not be loaded."); });
    return () => { cancelled = true; };
  }, [selected?.id, online]);

  async function run(label: string, action: () => Promise<void>) {
    if (busyAction) return;
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Model operation failed.");
    } finally {
      setBusyAction(null);
    }
  }

  async function prepareRemoval(model: ModelSummary) {
    setBusyAction(`preview-${model.id}`);
    setNotice(null);
    try {
      const preview = await previewModelRemoval(model.id);
      setRemovePreview(preview);
      setRemoveTarget(model);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Removal preview failed.");
    } finally {
      setBusyAction(null);
    }
  }

  async function confirmRemoval() {
    if (!removeTarget) return;
    const target = removeTarget;
    await run(`remove-${target.id}`, async () => {
      await removeModel(target.id);
      setRemoveTarget(null);
      setRemovePreview(null);
      setView("installed");
    });
  }

  return (
    <section className="tk-page tk-models-page">
      <ProductPageHeader
        eyebrow="Local model library"
        title="Models"
        description="Discover, install, repair, inspect, and remove local audio models without leaving Takokit."
      />

      <div className="tk-library-toolbar">
        <div className="tk-segmented" role="tablist" aria-label="Model view">
          <button className={view === "installed" ? "is-active" : ""} type="button" onClick={() => setView("installed")}>Installed <span>{runtime.models.length}</span></button>
          <button className={view === "library" ? "is-active" : ""} type="button" onClick={() => setView("library")}>Library <span>{runtime.catalogModels.length}</span></button>
        </div>
        <label className="tk-search-field">
          <Search size={15} strokeWidth={1.8} />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={view === "installed" ? "Filter installed models" : "Search model library"} />
        </label>
      </div>

      {notice ? <div className="tk-inline-error" role="status">{notice}</div> : null}

      <div className="tk-library-layout">
        <section className="tk-model-list" aria-label={view === "installed" ? "Installed models" : "Model library"}>
          {filtered.map((model) => {
            const active = selected?.id === model.id;
            return (
              <article className={active ? "tk-model-row is-active" : "tk-model-row"} key={model.id}>
                <button className="tk-model-row__main" type="button" onClick={() => setSelectedId(model.id)}>
                  <span className="tk-model-row__icon"><PackageOpen size={18} strokeWidth={1.7} /></span>
                  <span className="tk-model-row__copy">
                    <strong>{model.name}</strong>
                    <small>{model.family} · {model.runtime} · {model.runner}</small>
                    <span className="tk-capability-line">{model.capabilities.map(capabilityLabel).join(" · ")}</span>
                  </span>
                  <span className={model.executable ? "tk-state-pill is-ready" : model.status === "installed" ? "tk-state-pill is-warning" : "tk-state-pill"}>
                    {model.executable ? "Ready" : model.status === "installed" ? "Needs repair" : "Available"}
                  </span>
                  <ChevronRight size={15} strokeWidth={1.7} />
                </button>
                <div className="tk-model-row__actions">
                  {model.status !== "installed" ? (
                    <ProductButton tone="secondary" loading={busyAction === `pull-${model.id}`} disabled={!online || Boolean(busyAction)} onClick={() => void run(`pull-${model.id}`, () => pullModel(model.id).then(() => undefined))}>
                      <Download size={14} strokeWidth={1.8} /> Pull
                    </ProductButton>
                  ) : !model.executable ? (
                    <ProductButton tone="secondary" loading={busyAction === `repair-${model.id}`} disabled={!online || Boolean(busyAction)} onClick={() => void run(`repair-${model.id}`, () => pullModel(model.id).then(() => undefined))}>
                      <Wrench size={14} strokeWidth={1.8} /> Repair
                    </ProductButton>
                  ) : null}
                  {model.status === "installed" ? (
                    <button className="tk-row-icon-action is-danger" type="button" title={`Remove ${model.name}`} disabled={!online || Boolean(busyAction)} onClick={() => void prepareRemoval(model)}>
                      <Trash2 size={15} strokeWidth={1.8} />
                    </button>
                  ) : null}
                </div>
              </article>
            );
          })}
          {filtered.length === 0 ? (
            <div className="tk-system-empty"><PackageOpen size={22} /><div><strong>{source.length === 0 ? "Nothing here yet" : "No matching models"}</strong><span>{view === "installed" ? "Open Library to install your first model." : "Try a different search."}</span></div></div>
          ) : null}
        </section>

        <aside className="tk-model-inspector">
          {selected ? (
            <>
              <div className="tk-model-inspector__hero">
                <span className="tk-model-inspector__icon"><Cpu size={20} strokeWidth={1.7} /></span>
                <div><strong>{selected.name}</strong><span>{selected.id}</span></div>
                <span className={selected.executable ? "tk-state-dot is-ready" : "tk-state-dot"} />
              </div>
              <p className="tk-model-inspector__summary">{selected.purpose}</p>
              <div className="tk-inspector-facts">
                <Fact label="Runtime" value={selected.runtime} />
                <Fact label="Backend" value={selected.backend} />
                <Fact label="Runner" value={selected.runner} />
                <Fact label="License" value={selected.license} />
                <Fact label="Version" value={selected.version} />
                <Fact label="Hardware" value={selected.hardwareNotes || "Model-defined"} />
              </div>
              <div className="tk-inspector-capabilities">
                {selected.capabilities.map((capability) => <span key={capability}>{capabilityLabel(capability)}</span>)}
              </div>
              <div className="tk-plan-state">
                <div><span>Lifecycle</span><strong>{friendly(plan?.lifecycle_state ?? selected.lifecycleState)}</strong></div>
                <div><span>Runner</span><strong>{friendly(plan?.runner_runtime_state ?? selected.runnerRuntimeState)}</strong></div>
                <div><span>Executable</span><strong>{(plan?.executable ?? selected.executable) ? "Yes" : "No"}</strong></div>
              </div>
              {(plan?.missing ?? selected.missing).length > 0 ? <div className="tk-model-warning">{(plan?.missing ?? selected.missing).join(" · ")}</div> : <div className="tk-model-ready"><Check size={14} /> Ready for local execution</div>}
              <button className="tk-inspector-link" type="button" onClick={() => onNavigate("runners")}>Manage {selected.runner} →</button>
            </>
          ) : <div className="tk-system-empty"><PackageOpen size={20} /><div><strong>Select a model</strong><span>Model details and lifecycle state will appear here.</span></div></div>}
        </aside>
      </div>

      <ConfirmDialog
        open={Boolean(removeTarget)}
        title={removeTarget ? `Remove ${removeTarget.name}?` : "Remove model?"}
        description={removeTarget ? (
          <div className="tk-confirm-copy">
            <p>Takokit will remove this model while retaining files still shared by other installed models or runners.</p>
            <span>Estimated reclaim: <strong>{formatBytes(removePreview?.reclaim_bytes ?? 0)}</strong></span>
            {(removePreview?.retained_shared_paths?.length ?? 0) > 0 ? <span>{removePreview?.retained_shared_paths?.length} shared path(s) retained.</span> : null}
          </div>
        ) : null}
        confirmLabel="Remove model"
        destructive
        busy={busyAction?.startsWith("remove-") ?? false}
        onCancel={() => { if (!busyAction) { setRemoveTarget(null); setRemovePreview(null); } }}
        onConfirm={() => void confirmRemoval()}
      />
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong title={value}>{value}</strong></div>;
}

function capabilityLabel(capability: ModelCapability): string {
  switch (capability) {
    case "tts": return "Text to speech";
    case "stt": return "Speech to text";
    case "voice_cloning": return "Voice cloning";
    case "voice_conversion": return "Voice conversion";
    case "live_transcription": return "Live transcription";
    case "live_audio": return "Live audio";
  }
}

function friendly(value: string): string {
  return value.replace(/[-_]/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) { value /= 1024; index += 1; }
  return `${value.toFixed(index === 0 ? 0 : value >= 10 ? 1 : 2)} ${units[index]}`;
}
