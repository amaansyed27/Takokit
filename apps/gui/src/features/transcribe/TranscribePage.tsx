import { Check, Copy, FileAudio, FileText, FolderOpen, Gauge, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { ProductSelect } from "../../components/ui/ProductSelect";
import { useTranscription } from "../../hooks/useTranscription";
import { pickAudioFile } from "../../lib/nativePicker";

export function TranscribePage({ runtime, onNavigate }: RouteComponentProps) {
  const sttModels = useMemo(
    () => runtime.models.filter((item) => item.capabilities.includes("stt")),
    [runtime.models]
  );
  const initialModel = sttModels.find((item) => item.id === "whisper-base" && item.executable)
    ?? sttModels.find((item) => item.executable)
    ?? sttModels[0];

  const [model, setModel] = useState(initialModel?.id ?? "");
  const [filePath, setFilePath] = useState("");
  const [pickerBusy, setPickerBusy] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const { clearResult, error, isTranscribing, result, transcribe } = useTranscription();

  const selectedModel = sttModels.find((item) => item.id === model) ?? sttModels[0];
  const serverOnline = runtime.server.status === "online";
  const canTranscribe = Boolean(
    serverOnline && selectedModel?.executable && filePath.trim() && !isTranscribing && !pickerBusy
  );
  const blocker = !serverOnline
    ? "Local runtime is offline."
    : selectedModel?.executable
      ? null
      : selectedModel?.missing.join("; ") || "This model needs attention before it can run.";
  const fileName = displayFileName(filePath);

  useEffect(() => {
    clearResult();
  }, [model, filePath]);

  async function browseAudio() {
    setPickerBusy(true);
    setPickerError(null);
    try {
      const selected = await pickAudioFile();
      if (selected) setFilePath(selected);
    } catch (caught) {
      setPickerError(caught instanceof Error ? caught.message : "The audio picker could not be opened.");
    } finally {
      setPickerBusy(false);
    }
  }

  async function submit() {
    if (!canTranscribe || !selectedModel) return;
    await transcribe({ model: selectedModel.id, filePath });
  }

  return (
    <section className="tk-page tk-transcribe-page">
      <ProductPageHeader
        eyebrow="Speech to text"
        title="Transcribe"
        description="Choose an audio file and a local speech-to-text model. Takokit writes the transcript into the active workspace session."
      />

      <div className="tk-transcribe-studio">
        <section className="tk-transcribe-source" aria-label="Audio input">
          <div className="tk-transcribe-source__header">
            <div>
              <span>Audio</span>
              <small>Local file</small>
            </div>
            {filePath ? (
              <button
                className="tk-subtle-icon-button"
                type="button"
                aria-label="Clear selected audio"
                title="Clear selected audio"
                onClick={() => setFilePath("")}
              >
                <X size={15} strokeWidth={1.9} />
              </button>
            ) : null}
          </div>

          <div className={filePath ? "tk-audio-source is-selected" : "tk-audio-source"}>
            <span className="tk-audio-source__icon">
              <FileAudio size={25} strokeWidth={1.6} />
            </span>
            <div className="tk-audio-source__copy">
              <strong>{filePath ? fileName : "Choose an audio file"}</strong>
              <span>
                {filePath
                  ? "Ready for local transcription."
                  : "WAV, MP3, FLAC, OGG, M4A, AAC and WMA are supported by the native picker."}
              </span>
            </div>
            <ProductButton
              type="button"
              tone={filePath ? "secondary" : "primary"}
              loading={pickerBusy}
              onClick={() => void browseAudio()}
            >
              <FolderOpen size={15} strokeWidth={1.8} />
              {filePath ? "Choose another" : "Browse audio"}
            </ProductButton>
          </div>

          <label className="tk-transcribe-path-field">
            <span>Or enter a local path</span>
            <input
              value={filePath}
              onChange={(event) => setFilePath(event.target.value)}
              placeholder="C:\\path\\to\\recording.wav"
              spellCheck={false}
            />
          </label>

          {pickerError ? <div className="tk-inline-error" role="alert">{pickerError}</div> : null}
          {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

          <div className="tk-transcribe-source__footer">
            <span>{selectedModel?.name ?? "No STT model installed"}</span>
            <ProductButton
              tone="primary"
              type="button"
              loading={isTranscribing}
              disabled={!canTranscribe}
              onClick={() => void submit()}
            >
              <FileText size={16} strokeWidth={1.9} />
              {isTranscribing ? "Transcribing" : "Transcribe audio"}
            </ProductButton>
          </div>
        </section>

        <aside className="tk-transcribe-controls" aria-label="Transcription settings">
          <div className="tk-control-section">
            <div className="tk-control-section__heading">
              <span>Transcription setup</span>
              <small>Local</small>
            </div>

            <ProductSelect
              label="Model"
              value={model}
              onChange={(event) => setModel(event.target.value)}
              options={sttModels.map((item) => ({ value: item.id, label: item.name }))}
              hint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : "Install an STT model first."}
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
                <div><dt>Language</dt><dd>{selectedModel.language}</dd></div>
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
              <span>No installed speech-to-text model is available.</span>
              <button type="button" onClick={() => onNavigate("models")}>Open model library →</button>
            </div>
          )}
        </aside>
      </div>

      <section className="tk-transcript-result" aria-live="polite">
        <div className="tk-section-heading">
          <div>
            <h2>Transcript</h2>
            <p>The latest transcription result from this workspace session.</p>
          </div>
        </div>

        {result ? (
          <div className="tk-transcript-card">
            <div className="tk-transcript-card__header">
              <div>
                <span className="tk-transcript-card__icon"><FileText size={17} strokeWidth={1.8} /></span>
                <div>
                  <strong>Transcript ready</strong>
                  <span>{result.model}</span>
                </div>
              </div>
              <button
                className="tk-copy-action"
                type="button"
                onClick={() => void navigator.clipboard.writeText(result.text)}
              >
                <Copy size={14} strokeWidth={1.8} />
                Copy text
              </button>
            </div>

            <div className="tk-transcript-text">{result.text}</div>

            {result.output_path ? (
              <div className="tk-output-path">
                <code title={result.output_path}>{result.output_path}</code>
                <button
                  type="button"
                  onClick={() => void navigator.clipboard.writeText(result.output_path ?? "")}
                  title="Copy output path"
                >
                  <Copy size={14} strokeWidth={1.8} />
                </button>
              </div>
            ) : null}
          </div>
        ) : (
          <div className="tk-result-empty">
            <FileText size={19} strokeWidth={1.7} />
            <div>
              <strong>No transcript yet</strong>
              <span>Choose an audio file and run a local STT model.</span>
            </div>
          </div>
        )}
      </section>
    </section>
  );
}

function displayFileName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "Selected audio";
}
