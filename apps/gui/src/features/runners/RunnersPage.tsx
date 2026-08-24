import { Boxes, Check, ChevronRight, RefreshCw, Trash2, Wrench } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { getRunnerDoctor, installAdapter, installRunner, pullRunner, removeRunner } from "../../lib/api";
import type { RunnerSummary } from "../../lib/types";

export function RunnersPage({ runtime, onRefresh }: RouteComponentProps) {
  const [selectedId, setSelectedId] = useState(runtime.runners[0]?.id ?? "");
  const [doctor, setDoctor] = useState<Record<string, unknown> | null>(null);
  const [doctorLoading, setDoctorLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<RunnerSummary | null>(null);
  const selected = runtime.runners.find((runner) => runner.id === selectedId) ?? runtime.runners[0];
  const online = runtime.server.status === "online";
  const affectedModels = useMemo(
    () => selected ? runtime.models.filter((model) => model.runner === selected.id) : [],
    [runtime.models, selected?.id]
  );
  const adapterRecords = Array.isArray(doctor?.adapters)
    ? doctor.adapters.filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
    : [];

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected?.id, selectedId]);

  useEffect(() => {
    if (!selected || !online) {
      setDoctor(null);
      return;
    }
    void inspect(selected.id);
  }, [selected?.id, online]);

  async function inspect(id: string) {
    setDoctorLoading(true);
    setNotice(null);
    try {
      setDoctor(await getRunnerDoctor(id));
    } catch (error) {
      setDoctor(null);
      setNotice(error instanceof Error ? error.message : "Runner diagnostics failed.");
    } finally {
      setDoctorLoading(false);
    }
  }

  async function run(label: string, action: () => Promise<void>) {
    if (busyAction) return;
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
      if (selectedId) await inspect(selectedId);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Runner operation failed.");
    } finally {
      setBusyAction(null);
    }
  }

  async function installComplete(runner: RunnerSummary) {
    await run(`install-${runner.id}`, async () => {
      if (!runner.installed) await pullRunner(runner.id);
      await installRunner(runner.id);
    });
  }

  async function confirmRemoval() {
    if (!removeTarget) return;
    const target = removeTarget;
    await run(`remove-${target.id}`, async () => {
      await removeRunner(target.id);
      setRemoveTarget(null);
      setDoctor(null);
    });
  }

  return (
    <section className="tk-page tk-runners-page">
      <ProductPageHeader
        eyebrow="Execution layer"
        title="Runners"
        description="Install and inspect the shared local runtimes that execute Takokit models. Runners are reused across compatible model families."
      />

      {notice ? <div className="tk-inline-error" role="status">{notice}</div> : null}

      <div className="tk-runners-layout">
        <section className="tk-runner-list">
          <div className="tk-system-panel__header">
            <div><h2>Runtime families</h2><p>{runtime.runners.filter((item) => item.install_state === "ready").length} ready · {runtime.runners.length} available</p></div>
          </div>
          {runtime.runners.map((runner) => {
            const ready = runner.install_state === "ready";
            return (
              <article className={selected?.id === runner.id ? "tk-runner-row is-active" : "tk-runner-row"} key={runner.id}>
                <button className="tk-runner-row__main" type="button" onClick={() => setSelectedId(runner.id)}>
                  <span className="tk-runner-row__icon"><Boxes size={18} strokeWidth={1.7} /></span>
                  <span className="tk-runner-row__copy">
                    <strong>{runner.name}</strong>
                    <small>{runner.id}</small>
                    <span>{runner.supported_model_families?.slice(0, 4).join(" · ") || runner.kind}</span>
                  </span>
                  <span className={ready ? "tk-state-pill is-ready" : runner.installed ? "tk-state-pill is-warning" : "tk-state-pill"}>{ready ? "Ready" : runner.installed ? "Needs setup" : "Not installed"}</span>
                  <ChevronRight size={15} strokeWidth={1.7} />
                </button>
                <div className="tk-runner-row__actions">
                  {!ready ? (
                    <ProductButton tone="secondary" disabled={!online || Boolean(busyAction)} loading={busyAction === `install-${runner.id}`} onClick={() => void installComplete(runner)}>
                      {runner.installed ? <Wrench size={14} /> : <RefreshCw size={14} />}
                      {runner.installed ? "Repair" : "Install"}
                    </ProductButton>
                  ) : null}
                  {runner.installed ? (
                    <button className="tk-row-icon-action is-danger" type="button" title={`Remove ${runner.name}`} disabled={!online || Boolean(busyAction)} onClick={() => setRemoveTarget(runner)}><Trash2 size={15} /></button>
                  ) : null}
                </div>
              </article>
            );
          })}
        </section>

        <aside className="tk-runner-inspector">
          {selected ? (
            <>
              <div className="tk-runner-inspector__header">
                <span className="tk-runner-inspector__icon"><Boxes size={20} strokeWidth={1.7} /></span>
                <div><strong>{selected.name}</strong><span>{selected.id}</span></div>
                <button className="tk-row-icon-action" type="button" title="Refresh diagnostics" disabled={doctorLoading || !online} onClick={() => void inspect(selected.id)}><RefreshCw size={14} className={doctorLoading ? "is-spinning" : ""} /></button>
              </div>
              <div className="tk-runner-health">
                <div><span>Contract</span><strong>{selected.installed ? "Installed" : "Missing"}</strong></div>
                <div><span>Runtime</span><strong>{friendly(String(doctor?.runtime_state ?? selected.install_state ?? "missing"))}</strong></div>
                <div><span>Models using it</span><strong>{affectedModels.length}</strong></div>
              </div>
              <div className="tk-inspector-facts">
                <Fact label="Version" value={selected.version} />
                <Fact label="Kind" value={selected.kind} />
                <Fact label="Strategy" value={selected.dependency_strategy ?? "Managed by Takokit"} />
                <Fact label="Runtime path" value={String(doctor?.runtime_path ?? "Not installed")} />
                <Fact label="Logs" value={String(doctor?.logs_path ?? "Not available")} />
              </div>
              {String(doctor?.runtime_state ?? selected.install_state) === "ready" ? <div className="tk-model-ready"><Check size={14} /> Runtime is healthy</div> : <div className="tk-model-warning">Install or repair this runtime before dependent models can execute.</div>}
              {affectedModels.length > 0 ? <div className="tk-runner-models"><span>Executable models</span>{affectedModels.map((model) => <strong key={model.id}>{model.name}</strong>)}</div> : null}
              {adapterRecords.length > 0 ? (
                <div className="tk-adapter-list">
                  <div className="tk-adapter-list__heading"><strong>Python adapters</strong><span>Installed only when model families need them.</span></div>
                  {adapterRecords.map((adapter) => {
                    const id = String(adapter.id ?? "adapter");
                    const state = String(adapter.state ?? "unknown");
                    return (
                      <div className="tk-adapter-row" key={id}>
                        <div><strong>{id.replace(/_/g, " ")}</strong><span>{friendly(state)}</span></div>
                        {state !== "ready" && selected.install_state === "ready" ? <ProductButton tone="ghost" disabled={Boolean(busyAction)} loading={busyAction === `adapter-${id}`} onClick={() => void run(`adapter-${id}`, () => installAdapter(id))}>Install</ProductButton> : <span className={state === "ready" ? "tk-adapter-status is-ready" : "tk-adapter-status"}>{state === "ready" ? "Ready" : "Not installed"}</span>}
                      </div>
                    );
                  })}
                </div>
              ) : null}
            </>
          ) : <div className="tk-system-empty"><Boxes size={20} /><div><strong>No runner selected</strong><span>Select a runtime family to inspect it.</span></div></div>}
        </aside>
      </div>

      <ConfirmDialog
        open={Boolean(removeTarget)}
        title={removeTarget ? `Remove ${removeTarget.name}?` : "Remove runner?"}
        description={removeTarget ? <div className="tk-confirm-copy"><p>This removes the installed runner contract. Models that require it may stop being executable until the runner is installed again.</p>{runtime.models.filter((model) => model.runner === removeTarget.id).length > 0 ? <span><strong>{runtime.models.filter((model) => model.runner === removeTarget.id).length}</strong> installed model(s) currently depend on this runner.</span> : null}</div> : null}
        confirmLabel="Remove runner"
        destructive
        busy={busyAction?.startsWith("remove-") ?? false}
        onCancel={() => { if (!busyAction) setRemoveTarget(null); }}
        onConfirm={() => void confirmRemoval()}
      />
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong title={value}>{value}</strong></div>;
}

function friendly(value: string): string {
  return value.replace(/[-_]/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
