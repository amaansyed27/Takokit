import { Copy, ShieldCheck, UserRoundPlus } from "lucide-react";
import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Select } from "../../components/ui/Select";
import { Table, TableRow } from "../../components/ui/Table";
import { createVoiceProfile } from "../../lib/voices";

export function VoicesPage({ runtime, onRefresh }: RouteComponentProps) {
  const cloningModels = useMemo(
    () => runtime.models.filter((model) => model.capabilities.includes("voice_cloning")),
    [runtime.models]
  );
  const [name, setName] = useState("");
  const [samplePath, setSamplePath] = useState("");
  const [model, setModel] = useState(
    cloningModels.find((item) => item.id === "chatterbox")?.id ?? cloningModels[0]?.id ?? ""
  );
  const [consent, setConsent] = useState(false);
  const [consentNote, setConsentNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const selectedModel = cloningModels.find((item) => item.id === model);
  const ready = Boolean(
    selectedModel?.executable &&
      name.trim() &&
      samplePath.trim() &&
      consent &&
      runtime.server.status === "online"
  );

  async function createProfile() {
    if (!ready) return;
    setBusy(true);
    setNotice(null);
    try {
      const profile = await createVoiceProfile({
        sample_path: samplePath.trim(),
        name: name.trim(),
        model,
        consent_affirmed: consent,
        consent_note: consentNote.trim() || undefined
      });
      await onRefresh();
      setName("");
      setSamplePath("");
      setConsent(false);
      setConsentNote("");
      setNotice(
        `Created ${profile.name}. Use voice ID “${profile.id}” on the Speak page with ${profile.model_id}.`
      );
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Voice profile creation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Voices</h1>
        <p>Create reusable local voice profiles and use them across CLI, TUI, and GUI sessions.</p>
      </header>

      <Section
        title="Create a voice profile"
        description="Takokit stores the reference audio globally, while this action and its profile artifact are recorded in the active project session."
      >
        <div className="form-grid">
          <div className="field">
            <label htmlFor="voice-name">Profile name</label>
            <input
              id="voice-name"
              className="search-input"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="My narration voice"
            />
          </div>
          <Select
            label="Cloning model"
            value={model}
            onChange={(event) => setModel(event.target.value)}
            hint={
              selectedModel?.executable
                ? "Ready for local zero-shot profile creation."
                : selectedModel?.missing.join("; ") || "Install a supported cloning model first."
            }
            options={cloningModels.map((item) => ({ value: item.id, label: item.name }))}
          />
          <div className="field">
            <label htmlFor="voice-sample">Reference audio path</label>
            <input
              id="voice-sample"
              className="search-input"
              value={samplePath}
              onChange={(event) => setSamplePath(event.target.value)}
              placeholder="C:\\path\\to\\clean-reference.wav"
            />
            <small>Use a clean, single-speaker recording that the local daemon can read.</small>
          </div>
          <div className="field">
            <label htmlFor="consent-note">Consent note</label>
            <input
              id="consent-note"
              className="search-input"
              value={consentNote}
              onChange={(event) => setConsentNote(event.target.value)}
              placeholder="I recorded and own this voice."
            />
          </div>
        </div>

        <label className="consent-check">
          <input
            type="checkbox"
            checked={consent}
            onChange={(event) => setConsent(event.target.checked)}
          />
          <span>
            <ShieldCheck size={17} aria-hidden="true" />
            I own this voice or have explicit permission to create and use this profile.
          </span>
        </label>

        <div className="generation-actions">
          <div className="generation-actions__meta">
            <strong>{selectedModel?.name ?? "No cloning model available"}</strong>
            <span>
              {selectedModel?.executable
                ? "The profile will be usable by ID from every Takokit interface."
                : "Prepare the selected model and adapter before creating a profile."}
            </span>
          </div>
          <span className="badge-list">
            <Badge tone={selectedModel?.executable ? "success" : "warning"}>
              {selectedModel?.executable ? "ready" : "blocked"}
            </Badge>
            <Badge tone={consent ? "success" : "warning"}>
              {consent ? "consent affirmed" : "consent required"}
            </Badge>
          </span>
          <Button
            type="button"
            variant="primary"
            disabled={!ready}
            loading={busy}
            onClick={() => void createProfile()}
          >
            <UserRoundPlus size={16} /> Create profile
          </Button>
        </div>
        {notice ? <p className="notice-line">{notice}</p> : null}
      </Section>

      <Section title="Voice library" description="Preset and locally created profiles returned by the shared daemon.">
        <Table columns={["Voice", "ID", "Model", "Source", "Consent"]} ariaLabel="Takokit voices">
          {runtime.voices.map((voice) => (
            <TableRow key={`${voice.source}-${voice.id}`}>
              <strong>{voice.label}</strong>
              <span className="voice-id">
                {voice.id}
                <button
                  type="button"
                  className="icon-button"
                  aria-label={`Copy ${voice.id}`}
                  onClick={() => void navigator.clipboard.writeText(voice.id)}
                >
                  <Copy size={14} />
                </button>
              </span>
              <span>{voice.model}</span>
              <span>{voice.source}</span>
              <Badge tone={voice.consent === "affirmed" || voice.consent === "not required" ? "success" : "warning"}>
                {voice.consent}
              </Badge>
            </TableRow>
          ))}
        </Table>
      </Section>
    </section>
  );
}
