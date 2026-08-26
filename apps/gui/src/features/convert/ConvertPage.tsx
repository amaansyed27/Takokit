import { AudioWaveform, Check, CircleAlert, Copy, FileAudio, ShieldCheck, X } from "lucide-react";
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
    <section className="tk-page tk-convert-page tk-clone-simple-page">
      <ProductPageHeader
        eyebrow="Voice-to-voice"
        title="Clone audio"
        description="Choose speech, choose the voice it should sound like, then convert. The original words and timing stay intact."
      />

      <div className="tk-clone-simple-modes" role="tablist" aria-label="Voice source type">
        <button className={mode === "reference" ? "is-active" : ""} type="button" onClick={() => changeMode("reference")}>
          <FileAudio size={16} /><span><strong>Reference voice</strong><small>Use a voice recording as the target</small></span>
        </button>
        <button className={mode === "rvc" ? "is-active" : ""} type="button" onClick={() => changeMode("rvc")}>
          <AudioWaveform size={16} /><span><strong>Trained voice</strong><small>Use one of your trained RVC voices</small></span>
        </button>
      </div>

      {!serverOnline ? (
        <div className="tk-clone-simple-offline">
          <CircleAlert size={17} /><span>Local runtime is unavailable.</span>
          <ProductButton tone="secondary" onClick={() => void onRefresh()}>Retry</ProductButton>
        </div>
      ) : (
        <section className="tk-clone-simple-workflow">
          <ConversionPathCard
            label="Speech to convert"
            description="Choose the recording whose words and timing you want to keep."
            path={sourcePath}
            busy={sourcePickerBusy}
            actionLabel={sourcePath ? "Choose another" : "Choose audio"}
            onBrowse={() => void browseSource()}
            onClear={() => setSourcePath("")}
          />

          <div className="tk-clone-simple-arrow"><span>to</span></div>

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
            <div className="tk-clone-simple-target">
              <div><span>Target voice</span><strong>{selectedManagedRvcVoice?.name ?? "Choose a trained voice"}</strong><small>Takokit automatically uses that voice's active model and index.</small></div>
              {managedRvcVoices.length ? (
                <ProductSelect
                  label="Trained voice"
                  value={selectedManagedRvcVoice?.id ?? (targetPath && !targetPath.includes("\\") && !targetPath.includes("/") ? targetPath : "")}
                  onChange={(event) => setTargetPath(event.target.value)}
                  options={[{ value: "", label: "Choose a voice" }, ...managedRvcVoices.map((voice) => ({ value: voice.id, label: voice.name }))]}
                />
              ) : (
                <div className="tk-clone-simple-no-voice"><span>No trained voice is ready yet.</span><ProductButton tone="secondary" onClick={() => onNavigate("voices")}>Train a voice</ProductButton></div>
              )}
            </div>
          )}

          <label className="tk-clone-simple-consent">
            <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
            <ShieldCheck size={16} />
            <span><strong>I own these voices or have permission to use them.</strong><small>Required before voice cloning.</small></span>
          </label>

          {blocker ? <div className="tk-inline-error">{blocker} <button type="button" className="tk-text-button" onClick={() => onNavigate("models")}>Open Models</button></div> : null}
          {pickerError ? <div className="tk-inline-error" role="alert">{pickerError}</div> : null}
          {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

          <div className="tk-clone-simple-submit">
            <ProductButton tone="primary" loading={isConverting} disabled={!canConvert} onClick={() => void submit()}>
              <AudioWaveform size={16} /> {isConverting ? "Cloning voice" : "Clone voice"}
            </ProductButton>
            <button className="tk-text-button" type="button" onClick={() => onNavigate("files")}>Choose from Files</button>
          </div>

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
        </section>
      )}

      <section className="tk-clone-simple-result" aria-live="polite">
        <header className="tk-clone-simple-result__heading">
          <div><h2>Result</h2><p>{result ? "Listen before deciding whether the clone is good enough." : "Your latest cloned audio will appear here."}</p></div>
          {result || error ? <ProductButton tone="ghost" disabled={isConverting} onClick={clearResult}><X size={14} /> Clear</ProductButton> : null}
        </header>

        {result ? (
          <div className="tk-clone-simple-result__body">
            <div className="tk-clone-simple-result__status"><Check size={14} /><span><strong>Cloning complete</strong><small>{result.model} · {formatBytes(result.bytes)}</small></span></div>
            <LocalAudioPlayer path={result.output_path} label="Cloned audio" />
            <div className="tk-output-path"><code title={result.output_path}>{result.output_path}</code><button type="button" onClick={() => void navigator.clipboard.writeText(result.output_path)} title="Copy output path"><Copy size={14} /></button></div>

            {result.quality_review_required ? (
              <details className="tk-clone-simple-review">
                <summary>Quality check</summary>
                <p>Listen to the result and confirm these before treating this run as a quality pass.</p>
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
              <details className="tk-clone-simple-advanced">
                <summary>Technical RVC details</summary>
                <dl className="tk-clone-simple-evidence">
                  <div><dt>Model file</dt><dd>{displayFileName(result.checkpoint.checkpoint_path)}</dd></div>
                  <div><dt>Index</dt><dd>{result.checkpoint.index_path ? displayFileName(result.checkpoint.index_path) : "None"}</dd></div>
                  <div><dt>F0</dt><dd>{result.effective_settings.f0_method}</dd></div>
                  <div><dt>Pitch</dt><dd>{result.effective_settings.pitch_shift}</dd></div>
                </dl>
              </details>
            ) : null}
          </div>
        ) : isConverting ? (
          <div className="tk-result-empty"><AudioWaveform size={19} /><div><strong>Cloning voice…</strong><span>The conversion continues if you leave this page.</span></div></div>
        ) : (
          <div className="tk-result-empty"><AudioWaveform size={19} /><div><strong>No result yet</strong><span>Choose source audio and a target voice above.</span></div></div>
        )}
      </section>
    </section>
  );
}
