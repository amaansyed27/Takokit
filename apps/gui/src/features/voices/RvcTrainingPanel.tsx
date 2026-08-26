import { CircleAlert, Cpu, Play, RotateCcw, Square, Wrench } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { ProductButton } from "../../components/ui/ProductButton";
import {
  cancelRvcJob,
  getRvcJob,
  getRvcLogs,
  getRvcPresets,
  preflightRvc,
  prepareRvc,
  recoverRvcJob,
  trainRvc,
  type RvcJob,
  type RvcPreflight,
  type RvcTrainingConfig,
  type RvcTrainingPreset,
  type RvcTrainingPresetId,
  type RvcVoiceDetail
} from "../../lib/rvcApi";

type Props = { detail: RvcVoiceDetail; onChanged: () => Promise<void> };

export function RvcTrainingPanel({ detail, onChanged }: Props) {
  const voice = detail.project.id;
  const [presets, setPresets] = useState<RvcTrainingPreset[]>([]);
  const [selected, setSelected] = useState<RvcTrainingPresetId>("balanced");
  const [custom, setCustom] = useState<RvcTrainingConfig | null>(null);
  const [preflight, setPreflight] = useState<RvcPreflight | null>(null);
  const [job, setJob] = useState<RvcJob | null>(detail.active_job ?? null);
  const [logs, setLogs] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pollInFlight = useRef(false);

  useEffect(() => {
    void getRvcPresets().then((items) => {
      setPresets(items);
      const balanced = items.find((item) => item.id === "balanced")?.config;
      if (balanced) setCustom({ ...balanced, preset: "custom" });
    }).catch((caught) => setError(caught instanceof Error ? caught.message : "Training presets could not be loaded."));
  }, []);

  const running = job?.status === "queued" || job?.status === "running";
  useEffect(() => {
    if (!running) return;
    void poll();
    const timer = window.setInterval(() => void poll(), 3000);
    return () => window.clearInterval(timer);
  }, [running, voice]);

  async function poll() {
    if (pollInFlight.current) return;
    pollInFlight.current = true;
    try {
      const [nextJob, nextLogs] = await Promise.all([getRvcJob(voice), getRvcLogs(voice)]);
      setJob(nextJob);
      setLogs(nextLogs);
      if (nextJob && !["queued", "running"].includes(nextJob.status)) await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Training state could not be refreshed.");
    } finally {
      pollInFlight.current = false;
    }
  }

  const selectedPreset = presets.find((preset) => preset.id === selected);
  const config = useMemo(() => selected === "custom" ? custom : selectedPreset?.config ?? null, [selected, custom, selectedPreset]);

  async function runPreflight() {
    if (!config) return;
    setBusy(true); setError(null);
    try { setPreflight(await preflightRvc(voice, config)); }
    catch (caught) { setError(caught instanceof Error ? caught.message : "Hardware preflight failed."); }
    finally { setBusy(false); }
  }

  async function prepare() {
    setBusy(true); setError(null);
    try {
      const next = await prepareRvc(voice, selected, selected === "custom" ? custom ?? undefined : undefined);
      setJob(next); setLogs(""); await onChanged();
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Dataset preparation could not start."); }
    finally { setBusy(false); }
  }

  async function startTraining() {
    if (!preflight || preflight.class === "unsupported") return;
    setBusy(true); setError(null);
    try {
      const next = await trainRvc(voice, selected, selected === "custom" ? custom ?? undefined : undefined);
      setJob(next); setLogs(""); await onChanged();
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Training could not start."); }
    finally { setBusy(false); }
  }

  async function cancel() {
    setBusy(true); setError(null);
    try { setJob(await cancelRvcJob(voice)); await poll(); await onChanged(); }
    catch (caught) { setError(caught instanceof Error ? caught.message : "Training cancellation failed."); }
    finally { setBusy(false); }
  }

  async function recover() {
    setBusy(true); setError(null);
    try { setJob(await recoverRvcJob(voice)); setLogs(""); await onChanged(); }
    catch (caught) { setError(caught instanceof Error ? caught.message : "Training recovery could not start."); }
    finally { setBusy(false); }
  }

  function updateCustom<K extends keyof RvcTrainingConfig>(key: K, value: RvcTrainingConfig[K]) {
    setCustom((current) => current ? { ...current, [key]: value } : current);
    setPreflight(null);
  }

  return (
    <div className="tk-rvc-panel tk-rvc-training-panel">
      <section className="tk-rvc-presets">
        <header><div><strong>Training preset</strong><p>Takokit owns the verified RVC v2 / 40 kHz / RMVPE envelope. Presets only change safe resource/training controls.</p></div></header>
        <div className="tk-rvc-preset-grid">
          {presets.map((preset) => (
            <button key={preset.id} type="button" className={selected === preset.id ? "is-active" : ""} disabled={running} onClick={() => { setSelected(preset.id); setPreflight(null); }}>
              <strong>{preset.label}</strong><span>{preset.description}</span>
            </button>
          ))}
        </div>
      </section>

      {selected === "custom" && custom ? (
        <section className="tk-rvc-custom-config">
          <NumberControl label="Epochs" value={custom.epochs} min={1} max={1200} onChange={(value) => updateCustom("epochs", value)} />
          <NumberControl label="Batch size" value={custom.batch_size} min={1} max={64} onChange={(value) => updateCustom("batch_size", value)} />
          <NumberControl label="Save every epochs" value={custom.save_every_epochs} min={1} max={custom.epochs} onChange={(value) => updateCustom("save_every_epochs", value)} />
          <label><span>Device</span><select value={custom.device} onChange={(event) => updateCustom("device", event.target.value as RvcTrainingConfig["device"])}><option value="auto">Auto</option><option value="cuda">CUDA</option><option value="cpu">CPU</option></select></label>
          <label><span>Precision</span><select value={custom.precision} onChange={(event) => updateCustom("precision", event.target.value as RvcTrainingConfig["precision"])}><option value="auto">Auto</option><option value="fp16">FP16</option><option value="fp32">FP32</option></select></label>
          <label className="tk-rvc-checkbox"><input type="checkbox" checked={custom.cache_dataset_on_gpu} onChange={(event) => updateCustom("cache_dataset_on_gpu", event.target.checked)} /><span>Cache dataset on GPU</span></label>
        </section>
      ) : null}

      <section className="tk-rvc-training-sequence">
        <article><span>1</span><div><strong>Prepare dataset</strong><p>Preprocess, extract RMVPE F0 and HuBERT features. Prepared data is reused when its input fingerprint is unchanged.</p></div><ProductButton tone="secondary" disabled={busy || running || !detail.dataset.ready_for_preparation || !config} onClick={() => void prepare()}><Wrench size={14} /> Prepare</ProductButton></article>
        <article><span>2</span><div><strong>Hardware preflight</strong><p>Resolve CPU/CUDA, precision, VRAM/RAM and disk feasibility before training.</p></div><ProductButton tone="secondary" disabled={busy || running || !config || !detail.dataset.ready_for_preparation} onClick={() => void runPreflight()}><Cpu size={14} /> Run preflight</ProductButton></article>
        <article><span>3</span><div><strong>Start training</strong><p>Launch the managed persistent job. Closing the GUI does not turn it into an in-memory fake job.</p></div><ProductButton tone="primary" disabled={busy || running || !preflight || preflight.class === "unsupported"} onClick={() => void startTraining()}><Play size={14} /> Start training</ProductButton></article>
      </section>

      {preflight ? <PreflightCard preflight={preflight} /> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      <section className="tk-rvc-job-card">
        <header><div><strong>Training job</strong><span>{job ? `${label(job.status)} · ${label(job.stage)}` : "No job started"}</span></div><div>{running ? <ProductButton tone="secondary" disabled={busy} onClick={() => void cancel()}><Square size={12} /> Cancel</ProductButton> : job && ["failed", "cancelled", "stale"].includes(job.status) ? <ProductButton tone="secondary" disabled={busy} onClick={() => void recover()}><RotateCcw size={13} /> Recover</ProductButton> : null}<button type="button" onClick={() => void poll()}>Refresh</button></div></header>
        {job?.failure ? <div className="tk-rvc-job-failure"><CircleAlert size={15} /> {job.failure}</div> : null}
        <pre>{logs || "Training and preparation logs will appear here. Takokit does not invent completion percentages."}</pre>
      </section>
    </div>
  );
}

function NumberControl({ label: title, value, min, max, onChange }: { label: string; value: number; min: number; max: number; onChange: (value: number) => void }) {
  return <label><span>{title}</span><input type="number" value={value} min={min} max={max} onChange={(event) => onChange(Math.max(min, Math.min(max, Number(event.target.value))))} /></label>;
}
function PreflightCard({ preflight }: { preflight: RvcPreflight }) {
  return <section className={`tk-rvc-preflight is-${preflight.class}`}><header><strong>{label(preflight.class)}</strong><span>{preflight.backend} · {preflight.resolved_device} · {preflight.resolved_precision}</span></header><div><span>GPU <strong>{preflight.gpu ?? "Not detected"}</strong></span><span>VRAM <strong>{formatBytes(preflight.vram_bytes)}</strong></span><span>RAM <strong>{formatBytes(preflight.system_ram_bytes)}</strong></span><span>Disk free <strong>{formatBytes(preflight.available_disk_bytes)}</strong></span></div>{preflight.reasons.map((reason) => <p key={reason}>{reason}</p>)}</section>;
}
function formatBytes(value?: number): string { if (value == null) return "Unknown"; const gib = value / 1024 / 1024 / 1024; return gib >= 1 ? `${gib.toFixed(1)} GiB` : `${(value / 1024 / 1024).toFixed(0)} MiB`; }
function label(value: string): string { return value.replace(/_/g, " ").replace(/-/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase()); }
