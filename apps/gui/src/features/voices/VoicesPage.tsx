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
        description="Create a reusable voice from reference audio, then use it directly from Speak. Local profiles stay in Takokit-managed storage on this device."
      />

      <div className="tk-voice-create-studio">
        <section className="tk-voice-create" aria-label="Create a voice">
          <div className="tk-voice-create__header">
            <div>
              <span>Create a voice</span>
              <small>Reference audio → reusable voice</small>
            </div>
            <UserRoundPlus size={17} strokeWidth={1.8} />
          </div>

          <div className="tk-voice-create__body">
            <label className="tk-field">
              <span className="tk-field__label">Voice name</span>
              <input
                className="tk-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="My narration voice"
              />
              <span className="tk-field__hint">This becomes the friendly name shown in Speak.</span>
            </label>

            <div className={samplePath ? "tk-reference-audio is-selected" : "tk-reference-audio"}>
              <span className="tk-reference-audio__icon"><Mic2 size={22} strokeWidth={1.7} /></span>
              <div className="tk-reference-audio__copy">
                <strong>{samplePath ? displayFileName(samplePath) : "Choose reference audio"}</strong>
                <span>{samplePath ? "Reference ready for cloning." : "Use a clean, single-speaker recording with minimal background noise."}</span>
              </div>
              <div className="tk-reference-audio__actions">
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

            <label className="tk-voice-path-field">
              <span>Or enter a local path</span>
              <input
                value={samplePath}
                onChange={(event) => setSamplePath(event.target.value)}
                placeholder="C:\\path\\to\\reference.wav"
                spellCheck={false}
              />
            </label>

            <label className={consent ? "tk-consent-card is-checked" : "tk-consent-card"}>
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              <span className="tk-consent-card__icon"><ShieldCheck size={18} strokeWidth={1.8} /></span>
              <span className="tk-consent-card__copy">
                <strong>I own this voice or have explicit permission to use it.</strong>
                <small>Takokit requires this confirmation before creating a reusable voice profile.</small>
              </span>
            </label>

            <label className="tk-field">
              <span className="tk-field__label">Consent note <em>optional</em></span>
              <input
                className="tk-input"
                value={consentNote}
                onChange={(event) => setConsentNote(event.target.value)}
                placeholder="For example: I recorded and own this voice."
              />
            </label>

            {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
          </div>

          <div className="tk-voice-create__footer">
            <span>{selectedModel?.name ?? "No cloning model installed"}</span>
            <ProductButton tone="primary" type="button" loading={busy} disabled={!canCreate} onClick={() => void createProfile()}>
              <UserRoundPlus size={16} strokeWidth={1.9} />
              {busy ? "Creating" : "Create voice"}
            </ProductButton>
          </div>
        </section>

        <aside className="tk-voice-model-panel" aria-label="Voice cloning model">
          <div className="tk-control-section">
            <div className="tk-control-section__heading">
              <span>Cloning setup</span>
              <small>Local</small>
            </div>
            <ProductSelect
              label="Model"
              value={model}
              onChange={(event) => setModel(event.target.value)}
              options={cloningModels.map((item) => ({ value: item.id, label: item.name }))}
              hint={selectedModel ? `${selectedModel.runtime} · ${selectedModel.runner}` : "Install a cloning model first."}
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
                <div className="tk-model-ready"><Check size={14} strokeWidth={2} /> Ready for voice creation</div>
              )}
            </div>
          ) : (
            <div className="tk-model-blocker">
              <span>No installed voice-cloning model is available.</span>
              <button type="button" onClick={() => onNavigate("models")}>Open model library →</button>
            </div>
          )}
        </aside>
      </div>

      {createdProfile ? (
        <section className="tk-created-voice" aria-live="polite">
          <span className="tk-created-voice__icon"><Check size={18} strokeWidth={2} /></span>
          <div>
            <strong>{createdProfile.name} is ready</strong>
            <span>Saved as <code>{createdProfile.id}</code> for {createdProfile.model_id}.</span>
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
        <div className="tk-section-heading">
          <div>
            <h2>Voice library</h2>
            <p>{localVoices.length} saved locally{builtInVoices.length > 0 ? ` · ${builtInVoices.length} built-in` : ""}</p>
          </div>
        </div>

        {runtime.voices.length > 0 ? (
          <div className="tk-voice-grid">
            {runtime.voices.map((voice) => {
              const local = voice.source === "local-profile";
              return (
                <article className="tk-voice-card" key={`${voice.source}-${voice.id}`}>
                  <div className="tk-voice-card__main">
                    <span className={local ? "tk-voice-avatar is-local" : "tk-voice-avatar"}><Mic2 size={18} strokeWidth={1.7} /></span>
                    <div>
                      <strong>{voice.name}</strong>
                      <span>{local ? "Saved voice" : "Built-in voice"} · {voice.model === "none" ? "model-defined" : voice.model}</span>
                    </div>
                    <span className={local ? "tk-voice-kind is-local" : "tk-voice-kind"}>{local ? "Local" : "Built-in"}</span>
                  </div>

                  <div className="tk-voice-card__id">
                    <code>{voice.id}</code>
                    <button type="button" title="Copy voice ID" onClick={() => void navigator.clipboard.writeText(voice.id)}>
                      <Copy size={14} strokeWidth={1.8} />
                    </button>
                  </div>

                  <div className="tk-voice-card__footer">
                    <span>{local ? <><ShieldCheck size={13} strokeWidth={1.8} /> Consent-backed</> : "Available locally"}</span>
                    <div>
                      {local && voice.model !== "none" ? (
                        <button className="tk-voice-use" type="button" onClick={() => useInSpeak(voice.id, voice.model)}>
                          Use in Speak <ArrowRight size={13} strokeWidth={1.9} />
                        </button>
                      ) : null}
                      {local ? (
                        <button className="tk-voice-remove" type="button" title={`Remove ${voice.name}`} onClick={() => setRemoveTarget(voice)}>
                          <Trash2 size={14} strokeWidth={1.8} />
                        </button>
                      ) : null}
                    </div>
                  </div>
                </article>
              );
            })}
          </div>
        ) : (
          <div className="tk-result-empty">
            <Mic2 size={19} strokeWidth={1.7} />
            <div>
              <strong>No voices yet</strong>
              <span>Create a consent-backed voice above to use it in Speak.</span>
            </div>
          </div>
        )}
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
