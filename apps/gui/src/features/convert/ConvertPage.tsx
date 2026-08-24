import {
  AudioWaveform,
  Check,
  CircleAlert,
  Copy,
  FileAudio,
  FolderOpen,
  Gauge,
  Settings2,
  ShieldCheck,
  X
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { ProductSelect } from "../../components/ui/ProductSelect";
import { useVoiceConversion } from "../../hooks/useVoiceConversion";
import { pickAudioFile, pickFolder } from "../../lib/nativePicker";
import type { RvcF0Method } from "../../lib/types";

type ConversionMode = "reference" | "rvc";

type ReviewState = {
  words: boolean;
  timbre: boolean;
  similarity: boolean;
  artifacts: boolean;
};

const f0Options = [
  { value: "rmvpe", label: "RMVPE" },
  { value: "harvest", label: "Harvest" },
  { value: "crepe", label: "CREPE" },
  { value: "pm", label: "Parselmouth" }
];

const emptyReview: ReviewState = {
  words: false,
  timbre: false,
  similarity: false,
  artifacts: false
};

export function ConvertPage({ runtime, onNavigate, onRefresh }: RouteComponentProps) {
  const conversionModels = useMemo(
    () => runtime.models.filter((item) => item.capabilities.includes("voice_conversion")),
    [runtime.models]
  );
  const referenceModels = useMemo(
    () => conversionModels.filter((item) => item.id !== "rvc"),
    [conversionModels]
  );
  const rvcModels = useMemo(
    () => conversionModels.filter((item) => item.id === "rvc"),
    [conversionModels]
  );

  const [mode, setMode] = useState<ConversionMode>("reference");
  const modeModels = mode === "rvc" ? rvcModels : referenceModels;
  const [model, setModel] = useState("");
  const [sourcePath, setSourcePath] = useState("");
  const [targetPath, setTargetPath] = useState("");
  const [sourcePickerBusy, setSourcePickerBusy] = useState(false);
  const [targetPickerBusy, setTargetPickerBusy] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [consent, setConsent] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [f0Method, setF0Method] = useState<RvcF0Method>("rmvpe");
  const [pitchShift, setPitchShift] = useState(0);
  const [indexRate, setIndexRate] = useState(0.75);
  const [rmsMixRate, setRmsMixRate] = useState(0.25);
  const [protect, setProtect] = useState(0.33);
  const [filterRadius, setFilterRadius] = useState(3);
  const [review, setReview] = useState<ReviewState>(emptyReview);
  const { clearResult, convert, error, isConverting, result } = useVoiceConversion();

  const selectedModel = modeModels.find((item) => item.id === model) ?? modeModels[0];
  const serverOnline = runtime.server.status === "online";
  const blocker = !serverOnline
    ? "The local runtime is not connected."
    : !selectedModel
      ? mode === "rvc"
        ? "RVC is not installed in this runtime."
        : "No installed reference-voice conversion model is available."
      : selectedModel.executable
        ? null
        : selectedModel.missing.join("; ") || "This model needs attention before it can run.";
  const canConvert = Boolean(
    serverOnline &&
      selectedModel?.executable &&
      sourcePath.trim() &&
      targetPath.trim() &&
      consent &&
      !isConverting &&
      !sourcePickerBusy &&
      !targetPickerBusy
  );
  const qualityPassed = review.words && review.timbre && review.similarity && review.artifacts;

  useEffect(() => {
    if (modeModels.some((item) => item.id === model)) return;
    setModel(modeModels.find((item) => item.executable)?.id ?? modeModels[0]?.id ?? "");
  }, [mode, modeModels, model]);

  useEffect(() => {
    clearResult();
    setReview(emptyReview);
  }, [mode, model, sourcePath, targetPath]);

  function changeMode(nextMode: ConversionMode) {
    if (nextMode === mode) return;
    setMode(nextMode);
    setTargetPath("");
    setAdvancedOpen(false);
    setPickerError(null);
  }

  async function browseSource() {
    setSourcePickerBusy(true);
    setPickerError(null);
    try {
      const selected = await pickAudioFile();
      if (selected) setSourcePath(selected);
    } catch (caught) {
      setPickerError(caught instanceof Error ? caught.message : "The source audio picker could not be opened.");
    } finally {
      setSourcePickerBusy(false);
    }
  }

  async function browseTarget() {
    setTargetPickerBusy(true);
    setPickerError(null);
    try {
      const selected = mode === "rvc" ? await pickFolder() : await pickAudioFile();
      if (selected) setTargetPath(selected);
    } catch (caught) {
      setPickerError(caught instanceof Error ? caught.message : "The target picker could not be opened.");
    } finally {
      setTargetPickerBusy(false);
    }
  }

  async function submit() {
    if (!canConvert || !selectedModel) return;
    setReview(emptyReview);
    await convert({
      model: selectedModel.id,
      source_path: sourcePath.trim(),
      target_voice: targetPath.trim(),
      f0_method: f0Method,
      pitch_shift: pitchShift,
      index_rate: indexRate,
      rms_mix_rate: rmsMixRate,
      protect,
      filter_radius: filterRadius,
      consent_affirmed: consent
    });
  }

  return (
    <section className="tk-page tk-convert-page">
      <ProductPageHeader
        eyebrow="Voice conversion"
        title="Convert voice"
        description="Keep the spoken words while changing the voice. Use a reference recording for OpenVoice, or an RVC package when you need checkpoint-based conversion."
      />

      <div className="tk-convert-mode-switch" role="tablist" aria-label="Conversion type">
        <button
          className={mode === "reference" ? "is-active" : ""}
          type="button"
          role="tab"
          aria-selected={mode === "reference"}
          onClick={() => changeMode("reference")}
        >
          <span className="tk-convert-mode-switch__icon"><FileAudio size={17} strokeWidth={1.8} /></span>
          <span>
            <strong>Reference voice</strong>
            <small>Audio → same words in a target reference voice</small>
          </span>
        </button>
        <button
          className={mode === "rvc" ? "is-active" : ""}
          type="button"
          role="tab"
          aria-selected={mode === "rvc"}
          onClick={() => changeMode("rvc")}
        >
          <span className="tk-convert-mode-switch__icon"><AudioWaveform size={17} strokeWidth={1.8} /></span>
          <span>
            <strong>RVC</strong>
            <small>Audio → voice using a local checkpoint package</small>
          </span>
        </button>
      </div>

      {!serverOnline ? (
        <section className="tk-convert-offline" aria-live="polite">
          <span><CircleAlert size={19} strokeWidth={1.8} /></span>
          <div>
            <strong>Local runtime is unavailable</strong>
            <p>Takokit could not load the current models and runners. Retry the local runtime before starting a conversion.</p>
          </div>
          <ProductButton type="button" tone="secondary" onClick={() => void onRefresh()}>
            Retry runtime
          </ProductButton>
        </section>
      ) : (
        <div className="tk-convert-studio">
          <section className="tk-convert-workflow" aria-label="Voice conversion input">
            <div className="tk-convert-workflow__header">
              <div>
                <span>{mode === "rvc" ? "RVC conversion" : "Reference conversion"}</span>
                <small>{mode === "rvc" ? "Checkpoint based" : "Reference audio"}</small>
              </div>
              {selectedModel?.executable ? (
                <span className="tk-model-state is-ready"><span className="tk-status-dot is-online" /> Ready</span>
              ) : null}
            </div>

            <ConversionPathCard
              label="Source audio"
              description="The words and timing come from this recording."
              path={sourcePath}
              busy={sourcePickerBusy}
              actionLabel={sourcePath ? "Choose another" : "Browse audio"}
              onBrowse={() => void browseSource()}
              onClear={() => setSourcePath("")}
            />

            <div className="tk-convert-arrow" aria-hidden="true">
              <span />
              <AudioWaveform size={17} strokeWidth={1.7} />
              <span />
            </div>

            <ConversionPathCard
              label={mode === "rvc" ? "Target RVC package" : "Target reference"}
              description={
                mode === "rvc"
                  ? "Choose the folder containing the target checkpoint and optional index."
                  : "Choose a clean recording of the voice you want the output to resemble."
              }
              path={targetPath}
              busy={targetPickerBusy}
              actionLabel={targetPath ? "Choose another" : mode === "rvc" ? "Browse package" : "Browse reference"}
              onBrowse={() => void browseTarget()}
              onClear={() => setTargetPath("")}
              folder={mode === "rvc"}
            />

            <details className="tk-convert-manual-paths">
              <summary>Enter paths manually</summary>
              <div>
                <label className="tk-field">
                  <span className="tk-field__label">Source audio path</span>
                  <input
                    className="tk-input"
                    value={sourcePath}
                    onChange={(event) => setSourcePath(event.target.value)}
                    placeholder="C:\\path\\to\\source.wav"
                    spellCheck={false}
                  />
                </label>
                <label className="tk-field">
                  <span className="tk-field__label">{mode === "rvc" ? "RVC package or checkpoint path" : "Reference audio path"}</span>
                  <input
                    className="tk-input"
                    value={targetPath}
                    onChange={(event) => setTargetPath(event.target.value)}
                    placeholder={mode === "rvc" ? "C:\\path\\to\\rvc-package" : "C:\\path\\to\\reference.wav"}
                    spellCheck={false}
                  />
                </label>
              </div>
            </details>

            {mode === "rvc" ? (
              <div className="tk-convert-advanced">
                <button type="button" onClick={() => setAdvancedOpen((value) => !value)}>
                  <span><Settings2 size={15} strokeWidth={1.8} /> RVC controls</span>
                  <span>{advancedOpen ? "Hide" : "Show"}</span>
                </button>
                {advancedOpen ? (
                  <div className="tk-convert-advanced__body">
                    <ProductSelect
                      label="F0 method"
                      value={f0Method}
                      onChange={(event) => setF0Method(event.target.value as RvcF0Method)}
                      options={f0Options}
                      hint="RMVPE is the recommended default for most voices."
                    />
                    <div className="tk-convert-number-grid">
                      <NumberField label="Pitch" value={pitchShift} min={-24} max={24} step={1} suffix="semitones" onChange={setPitchShift} />
                      <NumberField label="Index rate" value={indexRate} min={0} max={1} step={0.05} onChange={setIndexRate} />
                      <NumberField label="RMS mix" value={rmsMixRate} min={0} max={1} step={0.05} onChange={setRmsMixRate} />
                      <NumberField label="Protect" value={protect} min={0} max={0.5} step={0.01} onChange={setProtect} />
                      <NumberField label="Filter radius" value={filterRadius} min={0} max={7} step={1} onChange={setFilterRadius} />
                    </div>
                  </div>
                ) : null}
              </div>
            ) : null}

            <label className={consent ? "tk-consent-card is-checked" : "tk-consent-card"}>
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              <span className="tk-consent-card__check">{consent ? <Check size={14} strokeWidth={2.3} /> : null}</span>
              <span className="tk-consent-card__icon"><ShieldCheck size={18} strokeWidth={1.8} /></span>
              <span>
                <strong>Voice permission</strong>
                <small>I own these voices or have explicit permission to perform this conversion.</small>
              </span>
            </label>

            {pickerError ? <div className="tk-inline-error" role="alert">{pickerError}</div> : null}
            {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

            <div className="tk-convert-workflow__footer">
              <span>{selectedModel?.name ?? (mode === "rvc" ? "RVC not installed" : "No reference conversion model")}</span>
              <ProductButton
                tone="primary"
                type="button"
                loading={isConverting}
                disabled={!canConvert}
                onClick={() => void submit()}
              >
                <AudioWaveform size={16} strokeWidth={1.9} />
                {isConverting ? "Converting" : "Convert voice"}
              </ProductButton>
            </div>
          </section>

          <aside className="tk-convert-controls" aria-label="Conversion model">
            <div className="tk-control-section">
              <div className="tk-control-section__heading">
                <span>Conversion setup</span>
                <small>Local</small>
              </div>
              {modeModels.length > 0 ? (
                <ProductSelect
                  label="Model"
                  value={selectedModel?.id ?? ""}
                  onChange={(event) => setModel(event.target.value)}
                  options={modeModels.map((item) => ({ value: item.id, label: item.name }))}
                  hint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : undefined}
                />
              ) : (
                <div className="tk-convert-model-empty">
                  <strong>{mode === "rvc" ? "RVC is not installed" : "No compatible model installed"}</strong>
                  <span>{mode === "rvc" ? "Install RVC from Models to use checkpoint conversion." : "Install a model with voice-conversion support."}</span>
                  <button type="button" onClick={() => onNavigate("models")}>Open Models →</button>
                </div>
              )}
            </div>

            {selectedModel ? (
              <div className="tk-selected-model">
                <div className="tk-selected-model__title">
                  <span className="tk-selected-model__icon"><Gauge size={16} strokeWidth={1.8} /></span>
                  <div>
                    <strong>{selectedModel.name}</strong>
                    <span>{selectedModel.family}</span>
                  </div>
                </div>
                <dl>
                  <div><dt>Backend</dt><dd>{selectedModel.backend}</dd></div>
                  <div><dt>Runtime</dt><dd>{selectedModel.runtime}</dd></div>
                  <div><dt>Runner</dt><dd>{selectedModel.runner}</dd></div>
                  <div><dt>License</dt><dd>{selectedModel.license}</dd></div>
                </dl>
                {blocker ? (
                  <div className="tk-model-blocker">
                    <span>{blocker}</span>
                    <button type="button" onClick={() => onNavigate("models")}>Manage model →</button>
                  </div>
                ) : (
                  <div className="tk-model-ready"><Check size={14} strokeWidth={2} /> Executable locally</div>
                )}
              </div>
            ) : null}

            <div className="tk-convert-explainer">
              <strong>{mode === "rvc" ? "What RVC changes" : "What stays the same"}</strong>
              <p>
                {mode === "rvc"
                  ? "Takokit preserves the source speech content while applying the selected RVC target and tuning settings."
                  : "The source words stay unchanged. OpenVoice changes the vocal identity toward the target reference recording."}
              </p>
            </div>
          </aside>
        </div>
      )}

      <section className="tk-convert-result" aria-live="polite">
        <div className="tk-section-heading">
          <div>
            <h2>Output</h2>
            <p>Execution success and listening quality are tracked separately.</p>
          </div>
        </div>

        {result ? (
          <div className="tk-convert-result-card">
            <div className="tk-convert-result-card__header">
              <div>
                <span className="tk-convert-result-card__icon"><AudioWaveform size={18} strokeWidth={1.8} /></span>
                <div>
                  <strong>Conversion complete</strong>
                  <span>{result.model} · {formatBytes(result.bytes)}</span>
                </div>
              </div>
              <span className="tk-result-success"><Check size={13} strokeWidth={2.2} /> Execution passed</span>
            </div>

            <div className="tk-convert-result-card__summary">
              <div><span>Quality</span><strong>{result.quality_status.replace(/_/g, " ")}</strong></div>
              <div><span>Target</span><strong>{displayFileName(mode === "rvc" ? result.checkpoint.checkpoint_path : result.checkpoint.target_reference_path ?? result.target_voice)}</strong></div>
              <div><span>Review</span><strong>{result.quality_review_required ? "Listening required" : "Not required"}</strong></div>
            </div>

            <LocalAudioPlayer path={result.output_path} label="Converted audio" />

            <div className="tk-output-path">
              <code title={result.output_path}>{result.output_path}</code>
              <button type="button" onClick={() => void navigator.clipboard.writeText(result.output_path)} title="Copy output path">
                <Copy size={14} strokeWidth={1.8} />
              </button>
            </div>

            {result.quality_review_required ? (
              <div className="tk-listening-review">
                <div>
                  <strong>Listening review</strong>
                  <span>Compare the source, target and output before calling this a quality pass.</span>
                </div>
                <div className="tk-listening-review__items">
                  <ReviewItem checked={review.words} label="Words remain intelligible and unchanged" onChange={(checked) => setReview((current) => ({ ...current, words: checked }))} />
                  <ReviewItem checked={review.timbre} label="Voice changed materially from the source" onChange={(checked) => setReview((current) => ({ ...current, timbre: checked }))} />
                  <ReviewItem checked={review.similarity} label="Output resembles the target voice" onChange={(checked) => setReview((current) => ({ ...current, similarity: checked }))} />
                  <ReviewItem checked={review.artifacts} label="No severe robotic, tearing or dropout artifacts" onChange={(checked) => setReview((current) => ({ ...current, artifacts: checked }))} />
                </div>
                <div className={qualityPassed ? "tk-listening-review__status is-passed" : "tk-listening-review__status"}>
                  {qualityPassed ? <Check size={14} strokeWidth={2.1} /> : <CircleAlert size={14} strokeWidth={1.9} />}
                  {qualityPassed ? "Human listening gate passed for this run" : "Quality is not evaluated until every listening check passes"}
                </div>
              </div>
            ) : null}

            {mode === "rvc" ? (
              <details className="tk-convert-evidence">
                <summary>RVC execution details</summary>
                <dl>
                  <div><dt>Checkpoint</dt><dd>{result.checkpoint.checkpoint_path}</dd></div>
                  <div><dt>Index</dt><dd>{result.checkpoint.index_path ?? "None"}</dd></div>
                  <div><dt>Pairing</dt><dd>{result.checkpoint.pairing_status.replace(/_/g, " ")}</dd></div>
                  <div><dt>F0</dt><dd>{result.effective_settings.f0_method}</dd></div>
                  <div><dt>Pitch</dt><dd>{result.effective_settings.pitch_shift}</dd></div>
                  <div><dt>Index rate</dt><dd>{result.effective_settings.index_rate}</dd></div>
                </dl>
              </details>
            ) : null}
          </div>
        ) : (
          <div className="tk-result-empty">
            <AudioWaveform size={19} strokeWidth={1.7} />
            <div>
              <strong>No conversion yet</strong>
              <span>Choose a source, target and executable conversion model.</span>
            </div>
          </div>
        )}
      </section>
    </section>
  );
}

type ConversionPathCardProps = {
  label: string;
  description: string;
  path: string;
  busy: boolean;
  actionLabel: string;
  onBrowse: () => void;
  onClear: () => void;
  folder?: boolean;
};

function ConversionPathCard({ label, description, path, busy, actionLabel, onBrowse, onClear, folder = false }: ConversionPathCardProps) {
  return (
    <div className={path ? "tk-convert-path is-selected" : "tk-convert-path"}>
      <span className="tk-convert-path__icon">{folder ? <FolderOpen size={21} strokeWidth={1.7} /> : <FileAudio size={21} strokeWidth={1.7} />}</span>
      <div>
        <span>{label}</span>
        <strong title={path || label}>{path ? displayFileName(path) : label}</strong>
        <small>{path ? path : description}</small>
      </div>
      <ProductButton type="button" tone={path ? "secondary" : "primary"} loading={busy} onClick={onBrowse}>
        <FolderOpen size={14} strokeWidth={1.8} />
        {actionLabel}
      </ProductButton>
      {path ? (
        <button className="tk-subtle-icon-button" type="button" onClick={onClear} title={`Clear ${label.toLowerCase()}`} aria-label={`Clear ${label.toLowerCase()}`}>
          <X size={14} strokeWidth={1.9} />
        </button>
      ) : null}
      {path && !folder ? <LocalAudioPlayer path={path} compact label={`${label} preview`} /> : null}
    </div>
  );
}

type NumberFieldProps = {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix?: string;
  onChange: (value: number) => void;
};

function NumberField({ label, value, min, max, step, suffix, onChange }: NumberFieldProps) {
  return (
    <label className="tk-field tk-number-field">
      <span className="tk-field__label">{label}</span>
      <input
        className="tk-input"
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <span className="tk-field__hint">{min}–{max}{suffix ? ` ${suffix}` : ""}</span>
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
    <label className={checked ? "tk-review-item is-checked" : "tk-review-item"}>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{checked ? <Check size={12} strokeWidth={2.2} /> : null}</span>
      <strong>{label}</strong>
    </label>
  );
}

function displayFileName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "Selected target";
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}
