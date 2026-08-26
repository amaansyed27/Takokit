import { AudioWaveform, Check, CircleAlert, Copy, FileAudio, Gauge, ShieldCheck, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { ProductSelect } from "../../components/ui/ProductSelect";
import { useVoiceConversion } from "../../hooks/useVoiceConversion";
import { pickAudioFile, pickFolder } from "../../lib/nativePicker";
import type { RvcF0Method } from "../../lib/types";
import { consumeCloneIntent } from "../../lib/workflowIntent";
import { ConvertAdvancedSettings } from "./ConvertAdvancedSettings";
import { ConversionPathCard, ReviewItem, displayFileName, emptyReview, formatBytes } from "./ConvertComponents";

type ConversionMode = "reference" | "rvc";
type ReviewState = typeof emptyReview;

export function ConvertPage({ runtime, onNavigate, onRefresh }: RouteComponentProps) {
  const conversionModels = useMemo(() => runtime.models.filter((item) => item.capabilities.includes("voice_conversion")), [runtime.models]);
  const referenceModels = useMemo(() => conversionModels.filter((item) => item.id !== "rvc"), [conversionModels]);
  const rvcModels = useMemo(() => conversionModels.filter((item) => item.id === "rvc"), [conversionModels]);
  const cloneIntent = useMemo(() => consumeCloneIntent(), []);
  const managedRvcVoices = useMemo(
    () => runtime.voices.filter((voice) => voice.source === "managed-rvc" && voice.model === "rvc"),
    [runtime.voices]
  );

  const [mode, setMode] = useState<ConversionMode>(cloneIntent?.mode ?? (managedRvcVoices.length ? "rvc" : "reference"));
  const modeModels = mode === "rvc" ? rvcModels : referenceModels;
  const [model, setModel] = useState("");
  const [sourcePath, setSourcePath] = useState(cloneIntent?.sourcePath ?? "");
  const [targetPath, setTargetPath] = useState(cloneIntent?.targetPath ?? "");
  const [sourcePickerBusy, setSourcePickerBusy] = useState(false);
  const [targetPickerBusy, setTargetPickerBusy] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [consent, setConsent] = useState(false);
  const [f0Method, setF0Method] = useState<RvcF0Method>("rmvpe");
  const [pitchShift, setPitchShift] = useState(0);
  const [indexRate, setIndexRate] = useState(0.75);
  const [rmsMixRate, setRmsMixRate] = useState(0.25);
  const [protect, setProtect] = useState(0.33);
  const [filterRadius, setFilterRadius] = useState(3);
  const [review, setReview] = useState<ReviewState>(emptyReview);
  const { clearResult, convert, error, isConverting, result } = useVoiceConversion();

  const selectedModel = modeModels.find((item) => item.id === model) ?? modeModels[0];
  const selectedManagedRvcVoice = mode === "rvc" ? managedRvcVoices.find((voice) => voice.id === targetPath) : undefined;
  const serverOnline = runtime.server.status === "online";
  const blocker = !serverOnline
    ? "The local runtime is offline."
    : !selectedModel
      ? mode === "rvc" ? "RVC is not installed." : "No reference-cloning model is installed."
      : selectedModel.executable
        ? null
        : selectedModel.missing.join("; ") || "The selected model is not ready.";
  const canConvert = Boolean(serverOnline && selectedModel?.executable && sourcePath.trim() && targetPath.trim() && consent && !isConverting && !sourcePickerBusy && !targetPickerBusy);
  const qualityPassed = review.words && review.timbre && review.similarity && review.artifacts;
  const resultIsRvc = result?.model === "rvc";

  useEffect(() => {
    if (modeModels.some((item) => item.id === model)) return;
    setModel(modeModels.find((item) => item.executable)?.id ?? modeModels[0]?.id ?? "");
  }, [mode, modeModels, model]);

  useEffect(() => { setReview(emptyReview); }, [mode, model, sourcePath, targetPath]);

  function changeMode(nextMode: ConversionMode) {
    if (nextMode === mode) return;
    setMode(nextMode);
    setTargetPath("");
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

  async function browseTargetReference() {
    setTargetPickerBusy(true);
    setPickerError(null);
    try {
      const selected = await pickAudioFile();
      if (selected) setTargetPath(selected);
    } catch (caught) {
      setPickerError(caught instanceof Error ? caught.message : "The target audio picker could not be opened.");
    } finally {
      setTargetPickerBusy(false);
    }
  }

  async function browseLegacyTarget() {
    setTargetPickerBusy(true);
    setPickerError(null);
    try {
      const selected = await pickFolder();
      if (selected) setTargetPath(selected);
    } catch (caught) {
      setPickerError(caught instanceof Error ? caught.message : "The legacy RVC folder picker could not be opened.");
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
    <section className="tk-page tk-convert-page tk-clone-studio-page">
      <ProductPageHeader
        eyebrow="Voice-to-voice"
        title="Clone audio"
        description="Keep the original words and timing, but change the speaker to a reference or trained voice."
      />

      <div className="tk-convert-mode-switch tk-clone-mode-switch" role="tablist" aria-label="Voice source type">
        <button className={mode === "reference" ? "is-active" : ""} type="button" role="tab" aria-selected={mode === "reference"} onClick={() => changeMode("reference")}>
          <span className="tk-convert-mode-switch__icon"><FileAudio size={17} strokeWidth={1.8} /></span>
          <span><strong>Reference voice</strong><small>Clone toward a voice recording</small></span>
        </button>
        <button className={mode === "rvc" ? "is-active" : ""} type="button" role="tab" aria-selected={mode === "rvc"} onClick={() => changeMode("rvc")}>
          <span className="tk-convert-mode-switch__icon"><AudioWaveform size={17} strokeWidth={1.8} /></span>
          <span><strong>Trained voice</strong><small>Use one of your trained voices</small></span>
        </button>
      </div>

      {!serverOnline ? (
        <section className="tk-convert-offline" aria-live="polite">
          <span><CircleAlert size={19} strokeWidth={1.8} /></span>
          <div><strong>Local runtime is unavailable</strong><p>Reconnect the local runtime before starting a clone.</p></div>
          <ProductButton type="button" tone="secondary" onClick={() => void onRefresh()}>Retry runtime</ProductButton>
        </section>
      ) : (
        <div className="tk-convert-studio">
          <section className="tk-convert-workflow tk-clone-workflow" aria-label="Voice cloning input">
            <div className="tk-convert-workflow__header">
              <div><span>Voice conversion</span><small>{mode === "rvc" ? "Trained voice" : "Reference voice"}</small></div>
              {selectedModel?.executable ? <span className="tk-model-state is-ready"><span className="tk-status-dot is-online" /> Ready</span> : null}
            </div>

            <div className="tk-clone-input-stack">
              <ConversionPathCard
                label="Source speech"
                description="Choose the recording whose words and timing you want to keep."
                path={sourcePath}
                busy={sourcePickerBusy}
                actionLabel={sourcePath ? "Choose another" : "Choose audio"}
                onBrowse={() => void browseSource()}
                onClear={() => setSourcePath("")}
              />

              <div className="tk-convert-arrow" aria-hidden="true"><span /><AudioWaveform size={17} strokeWidth={1.7} /><span /></div>

              {mode === "reference" ? (
                <ConversionPathCard
                  label="Target voice"
                  description="Choose a clear recording of the voice you want the output to resemble."
                  path={targetPath}
                  busy={targetPickerBusy}
                  actionLabel={targetPath ? "Choose another" : "Choose voice"}
                  onBrowse={() => void browseTargetReference()}
                  onClear={() => setTargetPath("")}
                />
              ) : (
                <div className={selectedManagedRvcVoice ? "tk-managed-voice-target is-selected" : "tk-managed-voice-target"}>
                  <span className="tk-convert-path__icon"><AudioWaveform size={21} strokeWidth={1.7} /></span>
                  <div className="tk-managed-voice-target__copy">
                    <span>Target voice</span>
                    <strong>{selectedManagedRvcVoice?.name ?? "Choose a trained voice"}</strong>
                    <small>{selectedManagedRvcVoice ? "Takokit will use its active model and index automatically." : "Only ready trained voices appear here."}</small>
                  </div>
                  {managedRvcVoices.length ? (
                    <ProductSelect
                      label="Trained voice"
                      value={selectedManagedRvcVoice?.id ?? (targetPath && !targetPath.includes("\\") && !targetPath.includes("/") ? targetPath : "")}
                      onChange={(event) => setTargetPath(event.target.value)}
                      options={[{ value: "", label: "Choose a voice" }, ...managedRvcVoices.map((voice) => ({ value: voice.id, label: voice.name }))]}
                    />
                  ) : (
                    <div className="tk-managed-voice-target__empty"><span>No trained voice is ready.</span><ProductButton tone="secondary" onClick={() => onNavigate("voices")}>Train a voice</ProductButton></div>
                  )}
                </div>
              )}
            </div>

            <label className={consent ? "tk-consent-card is-checked" : "tk-consent-card"}>
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              <span className="tk-consent-card__check">{consent ? <Check size={14} strokeWidth={2.3} /> : null}</span>
              <span className="tk-consent-card__icon"><ShieldCheck size={18} strokeWidth={1.8} /></span>
              <span><strong>Voice permission</strong><small>I own these voices or have explicit permission to use them.</small></span>
            </label>

            <ConvertAdvancedSettings
              mode={mode}
              model={selectedModel?.id ?? ""}
              modelOptions={modeModels.map((item) => ({ value: item.id, label: item.name }))}
              modelHint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : undefined}
              onModelChange={setModel}
              sourcePath={sourcePath}
              targetPath={targetPath}
              onSourcePathChange={setSourcePath}
              onTargetPathChange={setTargetPath}
              onBrowseLegacyTarget={() => void browseLegacyTarget()}
              legacyBusy={targetPickerBusy}
              f0Method={f0Method}
              pitchShift={pitchShift}
              indexRate={indexRate}
              rmsMixRate={rmsMixRate}
              protect={protect}
              filterRadius={filterRadius}
              onF0MethodChange={setF0Method}
              onPitchShiftChange={setPitchShift}
              onIndexRateChange={setIndexRate}
              onRmsMixRateChange={setRmsMixRate}
              onProtectChange={setProtect}
              onFilterRadiusChange={setFilterRadius}
            />

            {blocker ? <div className="tk-inline-error">{blocker} <button type="button" className="tk-text-button" onClick={() => onNavigate("models")}>Open Models</button></div> : null}
            {pickerError ? <div className="tk-inline-error" role="alert">{pickerError}</div> : null}
            {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

            <div className="tk-convert-workflow__footer">
              <button className="tk-text-button" type="button" onClick={() => onNavigate("files")}>Choose from Files</button>
              <ProductButton tone="primary" loading={isConverting} disabled={!canConvert} onClick={() => void submit()}>
                <AudioWaveform size={16} strokeWidth={1.9} /> {isConverting ? "Cloning voice" : "Clone voice"}
              </ProductButton>
            </div>
          </section>

          <aside className="tk-convert-controls tk-clone-controls" aria-label="Cloning setup">
            <div className="tk-control-section">
              <div className="tk-control-section__heading"><span>Cloning setup</span><small>Local</small></div>
              {modeModels.length > 1 ? (
                <ProductSelect
                  label="Model"
                  value={selectedModel?.id ?? ""}
                  onChange={(event) => setModel(event.target.value)}
                  options={modeModels.map((item) => ({ value: item.id, label: item.name }))}
                />
              ) : null}
            </div>

            {selectedModel ? (
              <div className="tk-selected-model">
                <div className="tk-selected-model__title">
                  <span className="tk-selected-model__icon"><Gauge size={16} strokeWidth={1.8} /></span>
                  <div><strong>{selectedModel.name}</strong><span>{mode === "rvc" ? "Trained voice engine" : "Reference voice engine"}</span></div>
                </div>
                <dl>
                  <div><dt>Backend</dt><dd>{selectedModel.backend}</dd></div>
                  <div><dt>Runtime</dt><dd>{selectedModel.runtime}</dd></div>
                  {mode === "rvc" ? <div><dt>Ready voices</dt><dd>{managedRvcVoices.length}</dd></div> : null}
                </dl>
                {blocker ? <div className="tk-model-blocker"><span>{blocker}</span><button type="button" onClick={() => onNavigate("models")}>Manage model →</button></div> : <div className="tk-model-ready"><Check size={14} strokeWidth={2} /> Executable locally</div>}
              </div>
            ) : null}
          </aside>
        </div>
      )}

      <section className="tk-speak-result tk-clone-result" aria-live="polite">
        <div className="tk-section-heading">
          <div><h2>Output</h2><p>{result ? "Listen to the clone before using it." : "Your latest cloned audio will appear here."}</p></div>
          {result || error ? <ProductButton tone="ghost" disabled={isConverting} onClick={clearResult}><X size={14} /> Clear</ProductButton> : null}
        </div>

        {result ? (
          <div className="tk-result-card">
            <div className="tk-result-card__icon"><AudioWaveform size={18} strokeWidth={1.8} /></div>
            <div className="tk-result-card__body">
              <div className="tk-result-card__heading">
                <div><strong>Clone ready</strong><span>{result.model} · {formatBytes(result.bytes)}</span></div>
                <span className="tk-result-success"><Check size={13} strokeWidth={2.2} /> Saved</span>
              </div>
              <LocalAudioPlayer path={result.output_path} label="Cloned audio" />
              <div className="tk-output-path"><code title={result.output_path}>{result.output_path}</code><button type="button" onClick={() => void navigator.clipboard.writeText(result.output_path)} title="Copy output path"><Copy size={14} /></button></div>

              {result.quality_review_required ? (
                <details className="tk-clone-quality-review">
                  <summary>Listening check</summary>
                  <div className="tk-listening-review__items">
                    <ReviewItem checked={review.words} label="Words remain intelligible and unchanged" onChange={(checked) => setReview((current) => ({ ...current, words: checked }))} />
                    <ReviewItem checked={review.timbre} label="Voice changed materially from the source" onChange={(checked) => setReview((current) => ({ ...current, timbre: checked }))} />
                    <ReviewItem checked={review.similarity} label="Output resembles the target voice" onChange={(checked) => setReview((current) => ({ ...current, similarity: checked }))} />
                    <ReviewItem checked={review.artifacts} label="No severe robotic, tearing or dropout artifacts" onChange={(checked) => setReview((current) => ({ ...current, artifacts: checked }))} />
                  </div>
                  <div className={qualityPassed ? "tk-listening-review__status is-passed" : "tk-listening-review__status"}>{qualityPassed ? <Check size={14} /> : <CircleAlert size={14} />}{qualityPassed ? "Listening check passed" : "Listening check incomplete"}</div>
                </details>
              ) : null}

              {resultIsRvc ? (
                <details className="tk-clone-technical-details">
                  <summary>Technical details</summary>
                  <dl><div><dt>Model</dt><dd>{displayFileName(result.checkpoint.checkpoint_path)}</dd></div><div><dt>Index</dt><dd>{result.checkpoint.index_path ? displayFileName(result.checkpoint.index_path) : "None"}</dd></div></dl>
                </details>
              ) : null}
            </div>
          </div>
        ) : isConverting ? (
          <div className="tk-result-empty"><AudioWaveform size={19} strokeWidth={1.7} /><div><strong>Cloning voice…</strong><span>The conversion continues if you leave this page.</span></div></div>
        ) : (
          <div className="tk-result-empty"><AudioWaveform size={19} strokeWidth={1.7} /><div><strong>No clone yet</strong><span>Choose source speech and a target voice above.</span></div></div>
        )}
      </section>
    </section>
  );
}
