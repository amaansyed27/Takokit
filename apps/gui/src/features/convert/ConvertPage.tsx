import { AudioWaveform, CheckCircle2, CircleAlert } from "lucide-react";
import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Select } from "../../components/ui/Select";
import { convertSessionVoice } from "../../lib/sessionInference";
import type {
  RvcF0Method,
  VoiceConversionApiResponse
} from "../../lib/types";

const f0Options = [
  { value: "rmvpe", label: "RMVPE" },
  { value: "harvest", label: "Harvest" },
  { value: "crepe", label: "CREPE" },
  { value: "pm", label: "Parselmouth" }
];

export function ConvertPage({ runtime }: RouteComponentProps) {
  const models = useMemo(
    () => runtime.models.filter((model) => model.capabilities.includes("voice_conversion")),
    [runtime.models]
  );
  const [model, setModel] = useState(models.find((item) => item.id === "rvc")?.id ?? models[0]?.id ?? "rvc");
  const [sourcePath, setSourcePath] = useState("");
  const [targetVoice, setTargetVoice] = useState("");
  const [f0Method, setF0Method] = useState<RvcF0Method>("rmvpe");
  const [pitchShift, setPitchShift] = useState(0);
  const [indexRate, setIndexRate] = useState(0.75);
  const [rmsMixRate, setRmsMixRate] = useState(0.25);
  const [protect, setProtect] = useState(0.33);
  const [filterRadius, setFilterRadius] = useState(3);
  const [consent, setConsent] = useState(false);
  const [running, setRunning] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [result, setResult] = useState<VoiceConversionApiResponse | null>(null);
  const [review, setReview] = useState({
    words: false,
    timbre: false,
    similarity: false,
    artifacts: false
  });

  const selectedModel = models.find((item) => item.id === model) ?? models[0];
  const apiUnavailable = runtime.server.status !== "online";
  const canRun = Boolean(
    selectedModel?.executable &&
      sourcePath.trim() &&
      targetVoice.trim() &&
      consent &&
      !apiUnavailable
  );
  const qualityPassed =
    result?.checkpoint.quality_baseline_ready === true &&
    review.words &&
    review.timbre &&
    review.similarity &&
    review.artifacts;

  async function runConversion() {
    if (!canRun) return;
    setRunning(true);
    setNotice(null);
    setResult(null);
    setReview({ words: false, timbre: false, similarity: false, artifacts: false });
    try {
      const response = await convertSessionVoice({
        model: selectedModel?.id ?? model,
        source_path: sourcePath,
        target_voice: targetVoice,
        f0_method: f0Method,
        pitch_shift: pitchShift,
        index_rate: indexRate,
        rms_mix_rate: rmsMixRate,
        protect,
        filter_radius: filterRadius,
        consent_affirmed: consent
      });
      setResult(response);
      setNotice("Execution passed. Perceptual quality is not evaluated until the listening checklist is completed.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Voice conversion failed.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Convert voice</h1>
        <p>Run RVC locally, inspect the exact checkpoint and settings, then judge quality separately.</p>
      </header>

      <Section title="Conversion input" description="Use a consent-backed source recording and target RVC package.">
        <div className="form-grid">
          <Select
            label="Model"
            value={model}
            onChange={(event) => setModel(event.target.value)}
            hint={selectedModel ? `${selectedModel.lifecycleState} · ${selectedModel.runner}` : "RVC is not installed"}
            options={models.map((item) => ({ value: item.id, label: item.name }))}
          />
          <Select
            label="F0 method"
            value={f0Method}
            onChange={(event) => setF0Method(event.target.value as RvcF0Method)}
            hint="RMVPE is the default. Other methods are exposed for checkpoint-specific tuning."
            options={f0Options}
          />
        </div>

        <div className="form-grid">
          <PathField
            id="rvc-source"
            label="Source audio"
            value={sourcePath}
            onChange={setSourcePath}
            placeholder="C:\\path\\to\\source.wav"
          />
          <PathField
            id="rvc-target"
            label="Target package or checkpoint"
            value={targetVoice}
            onChange={setTargetVoice}
            placeholder="C:\\path\\to\\rvc-package"
          />
        </div>

        <div className="form-grid">
          <NumberField label="Pitch shift" value={pitchShift} min={-24} max={24} step={1} onChange={setPitchShift} />
          <NumberField label="Index rate" value={indexRate} min={0} max={1} step={0.05} onChange={setIndexRate} />
          <NumberField label="RMS mix rate" value={rmsMixRate} min={0} max={1} step={0.05} onChange={setRmsMixRate} />
          <NumberField label="Protect" value={protect} min={0} max={0.5} step={0.01} onChange={setProtect} />
          <NumberField label="Filter radius" value={filterRadius} min={0} max={7} step={1} onChange={setFilterRadius} />
        </div>

        <label className="settings-row">
          <span>I own these voices or have explicit permission to perform this conversion.</span>
          <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
        </label>

        <div className="generation-actions">
          <div className="generation-actions__meta">
            <strong>{selectedModel?.name ?? "RVC unavailable"}</strong>
            <span>
              {apiUnavailable
                ? "Start the Takokit daemon."
                : selectedModel?.executable
                  ? "Ready to execute. A valid WAV is not automatically a quality pass."
                  : selectedModel?.missing.join("; ") || "Install and repair RVC first."}
            </span>
          </div>
          <Button type="button" variant="primary" disabled={!canRun} loading={running} onClick={() => void runConversion()}>
            <AudioWaveform size={16} /> Convert voice
          </Button>
        </div>
        {notice && <p className="notice-line">{notice}</p>}
      </Section>

      <Section title="Execution evidence">
        {result ? (
          <div className="detail-grid output-detail">
            <span><strong>Execution</strong>{result.execution_status}</span>
            <span><strong>Quality</strong>{result.quality_status.replace("_", " ")}</span>
            <span><strong>Output</strong>{result.output_path}</span>
            <span><strong>Bytes</strong>{result.bytes}</span>
            <span><strong>Checkpoint</strong>{result.checkpoint.checkpoint_path}</span>
            <span><strong>Checkpoint SHA-256</strong>{result.checkpoint.checkpoint_sha256}</span>
            <span><strong>Index</strong>{result.checkpoint.index_path ?? "none"}</span>
            <span><strong>Pairing</strong>{result.checkpoint.pairing_status.replaceAll("_", " ")}</span>
            <span><strong>Target reference</strong>{result.checkpoint.target_reference_path ?? "not supplied"}</span>
            <span><strong>Quality baseline</strong>{result.checkpoint.quality_baseline_ready ? "ready" : "not established"}</span>
            <span><strong>Effective F0</strong>{result.effective_settings.f0_method}</span>
            <span><strong>Effective index rate</strong>{result.effective_settings.index_rate}</span>
          </div>
        ) : (
          <div className="empty-state">
            <strong>No conversion evidence yet</strong>
            <p>Takokit will report execution and quality as separate states.</p>
          </div>
        )}
      </Section>

      <Section title="Human listening gate" description="Compare the source, target reference, and output. Do not infer quality from file creation.">
        <div className="settings-list">
          <ReviewItem
            checked={review.words}
            label="The same words remain intelligible and unchanged."
            onChange={(checked) => setReview((current) => ({ ...current, words: checked }))}
          />
          <ReviewItem
            checked={review.timbre}
            label="The vocal timbre changed materially from the source."
            onChange={(checked) => setReview((current) => ({ ...current, timbre: checked }))}
          />
          <ReviewItem
            checked={review.similarity}
            label="The result resembles the supplied target reference."
            onChange={(checked) => setReview((current) => ({ ...current, similarity: checked }))}
          />
          <ReviewItem
            checked={review.artifacts}
            label="There are no severe robotic, metallic, tearing, octave-jump, dropout, or timing artefacts."
            onChange={(checked) => setReview((current) => ({ ...current, artifacts: checked }))}
          />
        </div>
        <p className="notice-line">
          {qualityPassed ? (
            <><CheckCircle2 size={14} /> Human quality gate passed for this run.</>
          ) : (
            <><CircleAlert size={14} /> Quality remains failed or not evaluated. A package without a licensed target reference cannot be promoted.</>
          )}
        </p>
      </Section>
    </section>
  );
}

type PathFieldProps = {
  id: string;
  label: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
};

function PathField({ id, label, value, placeholder, onChange }: PathFieldProps) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input id={id} className="search-input" value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

type NumberFieldProps = {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
};

function NumberField({ label, value, min, max, step, onChange }: NumberFieldProps) {
  return (
    <label className="field">
      <span>{label}</span>
      <input
        className="search-input"
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <small>Allowed: {min} to {max}</small>
    </label>
  );
}

type ReviewItemProps = {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
};

function ReviewItem({ checked, label, onChange }: ReviewItemProps) {
  return (
    <label className="settings-row">
      <span>{label}</span>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
    </label>
  );
}
