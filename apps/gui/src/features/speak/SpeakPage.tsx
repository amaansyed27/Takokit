import { Check, Copy, Gauge, Sparkles, Volume2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { ProductSelect } from "../../components/ui/ProductSelect";
import { useSpeechGeneration } from "../../hooks/useSpeechGeneration";
import { consumeSpeakIntent } from "../../lib/workflowIntent";

export function SpeakPage({ runtime, onNavigate }: RouteComponentProps) {
  const ttsModels = useMemo(
    () => runtime.models.filter((model) => model.capabilities.includes("tts")),
    [runtime.models]
  );
  const speakIntent = useMemo(() => consumeSpeakIntent(), []);
  const initialModel = ttsModels.find((item) =>
    speakIntent?.modelId && (item.id === speakIntent.modelId || item.family === speakIntent.modelId) && item.executable
  ) ?? ttsModels.find((item) => item.id === "kokoro" && item.executable)
    ?? ttsModels.find((item) => item.executable)
    ?? ttsModels[0];
  const [text, setText] = useState(speakIntent?.text ?? "");
  const [model, setModel] = useState(initialModel?.id ?? "");
  const [voice, setVoice] = useState(speakIntent?.voiceId ?? "default");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [language, setLanguage] = useState("");
  const [instruction, setInstruction] = useState("");
  const [referenceText, setReferenceText] = useState("");
  const { error, generate, isGenerating, result, clearResult } = useSpeechGeneration();

  const selectedModel = ttsModels.find((item) => item.id === model) ?? ttsModels[0];
  const compatibleVoices = useMemo(() => {
    if (!selectedModel) return [];
    return runtime.voices.filter((item) =>
      item.model === selectedModel.id || item.model === selectedModel.family
    );
  }, [runtime.voices, selectedModel]);
  const supportsSavedVoices = Boolean(selectedModel?.capabilities.includes("voice_cloning"));
  const supportsGuidance = Boolean(selectedModel && (selectedModel.id.includes("qwen3") || selectedModel.family.includes("qwen3")));
  const serverOnline = runtime.server.status === "online";
  const canGenerate = Boolean(serverOnline && selectedModel?.executable && text.trim() && !isGenerating);
  const blocker = !serverOnline
    ? "Local runtime is offline."
    : selectedModel?.executable
      ? null
      : selectedModel?.missing.join("; ") || "This model needs attention before it can run.";

  useEffect(() => {
    if (!selectedModel) return;
    if (supportsSavedVoices && compatibleVoices.length > 0) {
      const currentStillValid = compatibleVoices.some((item) => item.id === voice);
      if (!currentStillValid) setVoice(compatibleVoices[0].id);
      return;
    }
    if (voice !== "default") setVoice("default");
  }, [selectedModel, supportsSavedVoices, compatibleVoices, voice]);

  const voiceOptions = [
    ...(!supportsSavedVoices || compatibleVoices.length === 0 ? [{ value: "default", label: "Default voice" }] : []),
    ...compatibleVoices.map((item) => ({ value: item.id, label: item.name }))
  ];

  async function submit() {
    if (!canGenerate || !selectedModel) return;
    await generate({
      model: selectedModel.id,
      voice,
      input: text,
      language: language.trim() || undefined,
      instruction: instruction.trim() || undefined,
      reference_text: referenceText.trim() || undefined,
      response_format: "wav"
    });
  }

  return (
    <section className="tk-page tk-speak-page">
      <ProductPageHeader
        eyebrow="Text to speech"
        title="Speak"
        description="Write your script, choose a local model and voice, then generate a WAV directly into the active workspace."
      />

      <div className="tk-speak-studio">
        <form
          className="tk-speak-editor"
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <div className="tk-speak-editor__header">
            <div>
              <span className="tk-editor-label">Script</span>
              <small>{text.length.toLocaleString()} / 5,000</small>
            </div>
            {selectedModel ? (
              <span className={selectedModel.executable ? "tk-model-state is-ready" : "tk-model-state"}>
                <span className="tk-status-dot is-online" />
                {selectedModel.executable ? "Ready" : "Needs attention"}
              </span>
            ) : null}
          </div>

          <textarea
            className="tk-script-input"
            value={text}
            maxLength={5000}
            onChange={(event) => setText(event.target.value)}
            placeholder="Type or paste the text you want Takokit to speak…"
            aria-label="Speech text"
          />

          {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

          <div className="tk-speak-editor__footer">
            <span>{selectedModel?.name ?? "No TTS model installed"}</span>
            <ProductButton tone="primary" type="submit" loading={isGenerating} disabled={!canGenerate}>
              <Volume2 size={16} strokeWidth={1.9} />
              {isGenerating ? "Generating" : "Generate speech"}
            </ProductButton>
          </div>
        </form>

        <aside className="tk-speak-controls" aria-label="Speech settings">
          <div className="tk-control-section">
            <div className="tk-control-section__heading">
              <span>Voice setup</span>
              <small>Local</small>
            </div>

            <ProductSelect
              label="Model"
              value={model}
              onChange={(event) => setModel(event.target.value)}
              options={ttsModels.map((item) => ({ value: item.id, label: item.name }))}
              hint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : "Install a TTS model first."}
            />

            <ProductSelect
              label="Voice"
              value={voice}
              onChange={(event) => setVoice(event.target.value)}
              options={voiceOptions}
              hint={supportsSavedVoices
                ? compatibleVoices.length > 0
                  ? `${compatibleVoices.length} compatible saved ${compatibleVoices.length === 1 ? "voice" : "voices"}`
                  : "Create a compatible voice profile first."
                : "This model uses its default local voice."}
              disabled={voiceOptions.length === 0}
            />
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
          ) : (
            <div className="tk-model-blocker">
              <span>No installed text-to-speech model is available.</span>
              <button type="button" onClick={() => onNavigate("models")}>Open model library →</button>
            </div>
          )}

          {supportsGuidance ? (
            <div className="tk-advanced-wrap">
              <button className="tk-advanced-toggle" type="button" onClick={() => setAdvancedOpen((value) => !value)}>
                <span><Sparkles size={14} strokeWidth={1.8} /> Model controls</span>
                <span>{advancedOpen ? "Hide" : "Show"}</span>
              </button>
              {advancedOpen ? (
                <div className="tk-advanced-fields">
                  <label className="tk-field">
                    <span className="tk-field__label">Language</span>
                    <input className="tk-input" value={language} onChange={(event) => setLanguage(event.target.value)} placeholder="Optional" />
                  </label>
                  <label className="tk-field">
                    <span className="tk-field__label">Instruction</span>
                    <textarea className="tk-input tk-input--textarea" value={instruction} onChange={(event) => setInstruction(event.target.value)} placeholder="Optional delivery or style guidance" />
                  </label>
                  <label className="tk-field">
                    <span className="tk-field__label">Reference text</span>
                    <textarea className="tk-input tk-input--textarea" value={referenceText} onChange={(event) => setReferenceText(event.target.value)} placeholder="Only when the selected model requires it" />
                  </label>
                </div>
              ) : null}
            </div>
          ) : null}
        </aside>
      </div>

      <section className="tk-speak-result" aria-live="polite">
        <div className="tk-section-heading">
          <div>
            <h2>Output</h2>
            <p>The latest result stays here while you move around Takokit. Clear it when you are finished.</p>
          </div>
          {result || error ? (
            <ProductButton tone="ghost" type="button" disabled={isGenerating} onClick={clearResult}>
              <X size={14} /> Clear
            </ProductButton>
          ) : null}
        </div>

        {result ? (
          <div className="tk-result-card">
            <div className="tk-result-card__icon"><Volume2 size={18} strokeWidth={1.8} /></div>
            <div className="tk-result-card__body">
              <div className="tk-result-card__heading">
                <div>
                  <strong>Speech ready</strong>
                  <span>{result.model} · {result.engine}</span>
                </div>
                <span className="tk-result-success"><Check size={13} strokeWidth={2.2} /> Saved</span>
              </div>
              <div className="tk-result-meta">
                <span>{formatBytes(result.bytes)}</span>
                {result.sample_rate ? <span>{result.sample_rate.toLocaleString()} Hz</span> : null}
                <span>{result.content_type}</span>
              </div>
              <LocalAudioPlayer path={result.output_path} label="Generated speech" />
              <div className="tk-output-path">
                <code title={result.output_path}>{result.output_path}</code>
                <button type="button" onClick={() => void navigator.clipboard.writeText(result.output_path)} title="Copy output path">
                  <Copy size={14} strokeWidth={1.8} />
                </button>
              </div>
            </div>
          </div>
        ) : isGenerating ? (
          <div className="tk-result-empty">
            <Volume2 size={19} strokeWidth={1.7} />
            <div>
              <strong>Generating speech…</strong>
              <span>You can switch pages. This process will stay active here.</span>
            </div>
          </div>
        ) : (
          <div className="tk-result-empty">
            <Volume2 size={19} strokeWidth={1.7} />
            <div>
              <strong>No speech generated yet</strong>
              <span>Your latest local result will appear here.</span>
            </div>
          </div>
        )}
      </section>
    </section>
  );
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
