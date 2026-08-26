import { CircleAlert, Play, RotateCcw, Square, Wrench } from "lucide-react";
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
  const epoch = currentEpoch(logs);
  const totalEpochs = job?.config.epochs ?? config?.epochs ?? 0;

  async function startTraining() {
    if (!config || !detail.dataset.ready_for_preparation || running) return;
    setBusy(true);
    setError(null);
    try {
      const check = await preflightRvc(voice, config);
      setPreflight(check);
      if (check.class === "unsupported") {
        throw new Error(check.reasons.join(" ") || "This training configuration is not supported on this computer.");
      }
      const next = await trainRvc(voice, selected, selected === "custom" ? custom ?? undefined : undefined);
      setJob(next);
      setLogs("");
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Training could not start.");
    } finally {
      setBusy(false);
    }
  }

  async function prepareOnly() {
    if (!config || !detail.dataset.ready_for_preparation || running) return;
    setBusy(true);
    setError(null);
    try {
      const next = await prepareRvc(voice, selected, selected === "custom" ? custom ?? undefined : undefined);
      setJob(next);
      setLogs("");
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Dataset preparation could not start.");
    } finally {
      setBusy(false);
    }
  }

  async function cancel() {
    setBusy(true);
    setError(null);
    try {
      setJob(await cancelRvcJob(voice));
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Training cancellation failed.");
    } finally {
      setBusy(false);
    }
  }

  async function recover() {
    setBusy(true);
    setError(null);
    try {
      setJob(await recoverRvcJob(voice));
      setLogs("");
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Training recovery could not start.");
    } finally {
      setBusy(false);
    }
  }

  function updateCustom<K extends keyof RvcTrainingConfig>(key: K, value: RvcTrainingConfig[K]) {
    setCustom((current) => current ? { ...current, [key]: value } : current);
    setPreflight(null);
  }

  return (
    <div className="tk-rvc-simple-panel tk-rvc-simple-training">
      <header className="tk-rvc-simple-section-heading">
        <div><h3>{detail.managed ? "Retrain voice" : "Train voice"}</h3><p>Choose a quality level. Takokit checks your hardware, prepares the recordings, trains the model, builds the index and activates the finished voice automatically.</p></div>
      </header>

      <div className="tk-rvc-simple-presets" role="radiogroup" aria-label="Training quality">
        {presets.filter((preset) => preset.id !== "custom").map((preset) => (
          <button key={preset.id} type="button" className={selected === preset.id ? "is-active" : ""} disabled={running} onClick={() => { setSelected(preset.id); setPreflight(null); }}>
            <strong>{preset.label}</strong><span>{preset.description}</span>
          </button>
        ))}
      </div>

      {running && job ? (
        <section className="tk-rvc-simple-running" aria-live="polite">
          <div><span>{stageLabel(job.stage)}</span><strong>{epoch ? `Epoch ${epoch} / ${job.config.epochs}` : stageLabel(job.stage)}</strong></div>
          {epoch && job.config.epochs ? <progress value={Math.min(epoch, job.config.epochs)} max={job.config.epochs} /> : <div className="tk-rvc-simple-indeterminate" />}
          <p>The job continues if you leave this page or close the GUI.</p>
          <ProductButton tone="secondary" disabled={busy} onClick={() => void cancel()}><Square size={12} /> Cancel training</ProductButton>
        </section>
      ) : (
        <div className="tk-rvc-simple-train-action">
          <ProductButton tone="primary" loading={busy} disabled={busy || !config || !detail.dataset.ready_for_preparation} onClick={() => void startTraining()}>
            <Play size={15} /> {detail.managed ? "Retrain voice" : "Train voice"}
          </ProductButton>
          {!detail.dataset.ready_for_preparation ? <p>Add and check at least one usable recording first.</p> : null}
        </div>
      )}

      {preflight && !running ? (
        <p className={`tk-rvc-simple-preflight is-${preflight.class}`}>{preflight.gpu ?? preflight.cpu} · {preflight.resolved_device.toUpperCase()} · {preflight.resolved_precision.toUpperCase()}</p>
      ) : null}

      {job?.failure ? <div className="tk-rvc-job-failure"><CircleAlert size={15} /> {job.failure}</div> : null}
      {job && ["failed", "cancelled", "stale"].includes(job.status) ? (
        <ProductButton tone="secondary" disabled={busy} onClick={() => void recover()}><RotateCcw size={13} /> Recover training</ProductButton>
      ) : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      <details className="tk-rvc-simple-advanced">
        <summary>Advanced training settings</summary>
        <div className="tk-rvc-simple-advanced-body">
          <button type="button" className={selected === "custom" ? "tk-rvc-simple-custom-toggle is-active" : "tk-rvc-simple-custom-toggle"} disabled={running} onClick={() => setSelected("custom")}>Use custom settings</button>
          {selected === "custom" && custom ? (
            <div className="tk-rvc-custom-config">
              <NumberControl label="Epochs" value={custom.epochs} min={1} max={1200} onChange={(value) => updateCustom("epochs", value)} />
              <NumberControl label="Batch size" value={custom.batch_size} min={1} max={64} onChange={(value) => updateCustom("batch_size", value)} />
              <NumberControl label="Save every epochs" value={custom.save_every_epochs} min={1} max={custom.epochs} onChange={(value) => updateCustom("save_every_epochs", value)} />
              <label><span>Device</span><select value={custom.device} onChange={(event) => updateCustom("device", event.target.value as RvcTrainingConfig["device"])}><option value="auto">Auto</option><option value="cuda">CUDA</option><option value="cpu">CPU</option></select></label>
              <label><span>Precision</span><select value={custom.precision} onChange={(event) => updateCustom("precision", event.target.value as RvcTrainingConfig["precision"])}><option value="auto">Auto</option><option value="fp16">FP16</option><option value="fp32">FP32</option></select></label>
              <label className="tk-rvc-checkbox"><input type="checkbox" checked={custom.cache_dataset_on_gpu} onChange={(event) => updateCustom("cache_dataset_on_gpu", event.target.checked)} /><span>Cache dataset on GPU</span></label>
            </div>
          ) : null}
          <ProductButton tone="secondary" disabled={busy || running || !config || !detail.dataset.ready_for_preparation} onClick={() => void prepareOnly()}><Wrench size={14} /> Prepare only</ProductButton>
        </div>
      </details>

      {(logs || job) ? (
        <details className="tk-rvc-simple-advanced tk-rvc-simple-log">
          <summary>Technical log</summary>
          <pre>{logs || "No log output yet."}</pre>
        </details>
      ) : null}
    </div>
  );
}

function currentEpoch(logs: string): number | null {
  const matches = Array.from(logs.matchAll(/====>\s*Epoch:\s*(\d+)/g));
  if (matches.length === 0) return null;
  const value = Number(matches[matches.length - 1][1]);
  return Number.isFinite(value) ? value : null;
}

function stageLabel(stage: string): string {
  const key = stage.toLowerCase();
  if (key.includes("validate_samples")) return "Checking recordings";
  if (key.includes("preprocess")) return "Preparing audio";
  if (key.includes("f0")) return "Analyzing pitch";
  if (key.includes("feature")) return "Learning voice features";
  if (key.includes("train")) return "Training voice";
  if (key.includes("index")) return "Building voice index";
  if (key.includes("artifact")) return "Finalizing voice";
  if (key.includes("complete")) return "Voice ready";
  return stage.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function NumberControl({ label: title, value, min, max, onChange }: { label: string; value: number; min: number; max: number; onChange: (value: number) => void }) {
  return <label><span>{title}</span><input type="number" value={value} min={min} max={max} onChange={(event) => onChange(Math.max(min, Math.min(max, Number(event.target.value))))} /></label>;
}
