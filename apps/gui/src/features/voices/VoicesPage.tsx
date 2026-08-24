import {
  ArrowRight,
  Check,
  Copy,
  FolderOpen,
  Gauge,
  Mic2,
  ShieldCheck,
  Trash2,
  UserRoundPlus,
  X
} from "lucide-react";
import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { ProductSelect } from "../../components/ui/ProductSelect";
import { pickAudioFile } from "../../lib/nativePicker";
import type { VoiceSummary } from "../../lib/types";
import type { VoiceProfile } from "../../lib/voiceTypes";
import { createVoiceProfile, removeVoiceProfile } from "../../lib/voices";
import { setSpeakIntent } from "../../lib/workflowIntent";

export function VoicesPage({ runtime, onNavigate, onRefresh }: RouteComponentProps) {
  const cloningModels = useMemo(
    () => runtime.models.filter((item) => item.capabilities.includes("voice_cloning")),
    [runtime.models]
  );
  const initialModel = cloningModels.find((item) => item.id === "openvoice" && item.executable)
    ?? cloningModels.find((item) => item.executable)
    ?? cloningModels[0];

  const [name, setName] = useState("");
  const [samplePath, setSamplePath] = useState("");
  const [model, setModel] = useState(initialModel?.id ?? "");
  const [consent, setConsent] = useState(false);
  const [consentNote, setConsentNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [pickerBusy, setPickerBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createdProfile, setCreatedProfile] = useState<VoiceProfile | null>(null);
  const [removeTarget, setRemoveTarget] = useState<VoiceSummary | null>(null);
  const [removeBusy, setRemoveBusy] = useState(false);

  const selectedModel = cloningModels.find((item) => item.id === model) ?? cloningModels[0];
  const localVoices = runtime.voices.filter((voice) => voice.source === "local-profile");
  const builtInVoices = runtime.voices.filter((voice) => voice.source !== "local-profile");
  const serverOnline = runtime.server.status === "online";
  const canCreate = Boolean(
    serverOnline
      && selectedModel?.executable
      && name.trim()
      && samplePath.trim()
      && consent
      && !busy
      && !pickerBusy
  );
  const blocker = !serverOnline
    ? "Local runtime is offline."
    : selectedModel?.executable
      ? null
      : selectedModel?.missing.join("; ") || "This cloning model needs attention before it can run.";

  async function browseReference() {
    setPickerBusy(true);
    setError(null);
    try {
      const selected = await pickAudioFile();
      if (selected) setSamplePath(selected);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The audio picker could not be opened.");
    } finally {
      setPickerBusy(false);
    }
  }

  async function createProfile() {
    if (!canCreate || !selectedModel) return;
    setBusy(true);
    setError(null);
    setCreatedProfile(null);
    try {
      const profile = await createVoiceProfile({
        sample_path: samplePath.trim(),
        name: name.trim(),
        model: selectedModel.id,
        consent_affirmed: consent,
        consent_note: consentNote.trim() || undefined
      });
      await onRefresh();
      setCreatedProfile(profile);
      setName("");
      setSamplePath("");
      setConsent(false);
      setConsentNote("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Voice creation failed.");
    } finally {
      setBusy(false);
    }
  }

  function useInSpeak(voiceId: string, modelId: string) {
    setSpeakIntent({ voiceId, modelId });
    onNavigate("speak");
  }

  async function confirmRemove() {
    if (!removeTarget) return;
    setRemoveBusy(true);
    setError(null);
    try {
      await removeVoiceProfile(removeTarget.id);
      if (createdProfile?.id === removeTarget.id) setCreatedProfile(null);
      setRemoveTarget(null);
      await onRefresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Voice removal failed.");
    } finally {
      setRemoveBusy(false);
    }
  }

  return (
    <section className="tk-page tk-voices-page">
      <ProductPageHeader
        eyebrow="Voice cloning"
        title="Voices"
        description="Create a reusable local voice from one clean reference recording, then use it directly from Speak."
      />

      <div className="tk-voice-studio">
        <section className="tk-voice-builder" aria-label="Create a voice">
          <header className="tk-voice-builder__header">
            <div>
              <span className="tk-voice-builder__eyebrow">Instant clone</span>
              <h2>Create a voice</h2>
              <p>A short, clean recording is enough for supported cloning models.</p>
            </div>
            <span className="tk-voice-builder__icon"><UserRoundPlus size={20} strokeWidth={1.7} /></span>
          </header>

          <div className="tk-voice-builder__body">
            <label className="tk-field tk-voice-name-field">
              <span className="tk-field__label">Voice name</span>
              <input
                className="tk-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="For example, Studio narrator"
              />
              <span className="tk-field__hint">This is the name you will see later in Speak.</span>
            </label>

            <div className={samplePath ? "tk-voice-reference is-selected" : "tk-voice-reference"}>
              <span className="tk-voice-reference__icon"><Mic2 size={23} strokeWidth={1.7} /></span>
              <div className="tk-voice-reference__copy">
                <span>Reference audio</span>
                <strong>{samplePath ? displayFileName(samplePath) : "Choose a clean voice recording"}</strong>
                <p>{samplePath ? "Ready to create a reusable voice profile." : "Best results come from one speaker, clear speech, and little background noise."}</p>
              </div>
              <div className="tk-voice-reference__actions">
                <ProductButton
                  type="button"
                  tone={samplePath ? "secondary" : "primary"}
                  loading={pickerBusy}
                  onClick={() => void browseReference()}
                >
                  <FolderOpen size={15} strokeWidth={1.8} />
                  {samplePath ? "Choose another" : "Browse audio"}
                </ProductButton>
                {samplePath ? (
                  <button className="tk-subtle-icon-button" type="button" title="Clear reference" onClick={() => setSamplePath("")}>
                    <X size={15} strokeWidth={1.9} />
                  </button>
                ) : null}
              </div>
            </div>

            {samplePath ? <LocalAudioPlayer path={samplePath} compact label="Reference audio" /> : null}

            <details className="tk-voice-manual-path">
              <summary>Enter a local path instead</summary>
              <input
                value={samplePath}
                onChange={(event) => setSamplePath(event.target.value)}
                placeholder="C:\\path\\to\\reference.wav"
                spellCheck={false}
              />
            </details>

            <label className={consent ? "tk-voice-consent is-checked" : "tk-voice-consent"}>
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              <span className="tk-voice-consent__check">{consent ? <Check size={14} strokeWidth={2.4} /> : null}</span>
              <span className="tk-voice-consent__icon"><ShieldCheck size={18} strokeWidth={1.8} /></span>
              <span className="tk-voice-consent__copy">
                <strong>I own this voice or have explicit permission to use it.</strong>
                <small>Takokit requires this confirmation before creating a reusable voice profile.</small>
              </span>
            </label>

            <details className="tk-voice-consent-note">
              <summary>Add a consent note <span>Optional</span></summary>
              <input
                className="tk-input"
                value={consentNote}
                onChange={(event) => setConsentNote(event.target.value)}
                placeholder="For example: I recorded and own this voice."
              />
            </details>

            {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
          </div>

          <footer className="tk-voice-builder__footer">
            <div>
              <span>Clone with</span>
              <strong>{selectedModel?.name ?? "No compatible model installed"}</strong>
            </div>
            <ProductButton tone="primary" type="button" loading={busy} disabled={!canCreate} onClick={() => void createProfile()}>
              <UserRoundPlus size={16} strokeWidth={1.9} />
              {busy ? "Creating voice" : "Create voice"}
            </ProductButton>
          </footer>
        </section>

        <aside className="tk-voice-setup" aria-label="Voice cloning setup">
          <div className="tk-voice-setup__header">
            <span>Cloning setup</span>
            <small>Local</small>
          </div>

          <div className="tk-voice-setup__body">
            <ProductSelect
              label="Model"
              value={model}
              onChange={(event) => setModel(event.target.value)}
              options={cloningModels.map((item) => ({ value: item.id, label: item.name }))}
              hint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : "Install a cloning model first."}
            />

            <div className="tk-voice-flow" aria-label="Instant cloning flow">
              <div><span>1</span><p><strong>Reference</strong><small>Choose a clean voice recording.</small></p></div>
              <div><span>2</span><p><strong>Create</strong><small>Takokit stores a reusable local profile.</small></p></div>
              <div><span>3</span><p><strong>Speak</strong><small>Select the saved voice from Text to Speech.</small></p></div>
            </div>

            {selectedModel ? (
              <div className="tk-voice-model-summary">
                <div className="tk-voice-model-summary__title">
                  <span><Gauge size={16} strokeWidth={1.8} /></span>
                  <div><strong>{selectedModel.name}</strong><small>{selectedModel.family}</small></div>
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
                  <div className="tk-model-ready"><Check size={14} strokeWidth={2} /> Ready for instant cloning</div>
                )}
              </div>
            ) : (
              <div className="tk-model-blocker">
                <span>No installed voice-cloning model is available.</span>
                <button type="button" onClick={() => onNavigate("models")}>Open model library →</button>
              </div>
            )}
          </div>
        </aside>
      </div>

      {createdProfile ? (
        <section className="tk-created-voice" aria-live="polite">
          <span className="tk-created-voice__icon"><Check size={18} strokeWidth={2} /></span>
          <div>
            <strong>{createdProfile.name} is ready</strong>
            <span>Saved locally and ready to use in Speak.</span>
          </div>
          <div className="tk-created-voice__actions">
            <button type="button" onClick={() => void navigator.clipboard.writeText(createdProfile.id)}>
              <Copy size={14} strokeWidth={1.8} /> Copy ID
            </button>
            <ProductButton tone="primary" type="button" onClick={() => useInSpeak(createdProfile.id, createdProfile.model_id)}>
              Use in Speak <ArrowRight size={14} strokeWidth={1.9} />
            </ProductButton>
          </div>
        </section>
      ) : null}

      <section className="tk-voice-library">
        <div className="tk-section-heading tk-voice-library__heading">
          <div>
            <h2>Your voices</h2>
            <p>{localVoices.length > 0 ? `${localVoices.length} reusable ${localVoices.length === 1 ? "voice" : "voices"} saved on this device` : "Create your first reusable voice above"}</p>
          </div>
        </div>

        {localVoices.length > 0 ? (
          <div className="tk-voice-list">
            {localVoices.map((voice) => (
              <article className="tk-voice-row" key={`${voice.source}-${voice.id}`}>
                <span className="tk-voice-row__avatar"><Mic2 size={19} strokeWidth={1.7} /></span>
                <div className="tk-voice-row__identity">
                  <strong>{voice.name}</strong>
                  <span>Saved voice · {voice.model === "none" ? "model-defined" : voice.model}</span>
                </div>
                <div className="tk-voice-row__badges">
                  <span className="is-local">Local</span>
                  <span><ShieldCheck size={12} strokeWidth={1.8} /> Consent-backed</span>
                </div>
                <div className="tk-voice-row__actions">
                  {voice.model !== "none" ? (
                    <button className="tk-voice-use" type="button" onClick={() => useInSpeak(voice.id, voice.model)}>
                      Use in Speak <ArrowRight size={13} strokeWidth={1.9} />
                    </button>
                  ) : null}
                  <button className="tk-voice-icon-action" type="button" title="Copy voice ID" onClick={() => void navigator.clipboard.writeText(voice.id)}>
                    <Copy size={14} strokeWidth={1.8} />
                  </button>
                  <button className="tk-voice-icon-action is-danger" type="button" title={`Remove ${voice.name}`} onClick={() => setRemoveTarget(voice)}>
                    <Trash2 size={14} strokeWidth={1.8} />
                  </button>
                </div>
              </article>
            ))}
          </div>
        ) : (
          <div className="tk-result-empty">
            <Mic2 size={19} strokeWidth={1.7} />
            <div><strong>No saved voices yet</strong><span>Create an instant clone above, then use it from Speak.</span></div>
          </div>
        )}

        {builtInVoices.length > 0 ? (
          <div className="tk-built-in-voices">
            <div className="tk-built-in-voices__heading"><span>Built-in voices</span><small>Provided by installed models</small></div>
            {builtInVoices.map((voice) => (
              <div className="tk-built-in-voice" key={`${voice.source}-${voice.id}`}>
                <span><Mic2 size={16} strokeWidth={1.7} /></span>
                <div><strong>{voice.name}</strong><small>{voice.model === "none" ? "Model-defined voice" : voice.model}</small></div>
                <em>Built-in</em>
              </div>
            ))}
          </div>
        ) : null}
      </section>

      <ConfirmDialog
        open={Boolean(removeTarget)}
        title="Remove saved voice?"
        description={removeTarget ? (
          <>This permanently removes <strong>{removeTarget.name}</strong> and its Takokit-managed reference audio. Existing generated outputs are not deleted.</>
        ) : null}
        confirmLabel="Remove voice"
        destructive
        busy={removeBusy}
        onCancel={() => !removeBusy && setRemoveTarget(null)}
        onConfirm={() => void confirmRemove()}
      />
    </section>
  );
}

function displayFileName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "Selected audio";
}
