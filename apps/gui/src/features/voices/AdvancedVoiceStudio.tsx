import { ArrowRight, AudioWaveform, FileArchive, FolderOpen, Plus, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductButton } from "../../components/ui/ProductButton";
import { pickRvcArtifact } from "../../lib/nativePicker";
import {
  createRvcVoice,
  importRvcPackage,
  importRvcVoice,
  listRvcVoices,
  type RvcProject
} from "../../lib/rvcApi";
import { RvcStudioWorkspace } from "./RvcStudioWorkspace";

type Props = Pick<RouteComponentProps, "onNavigate" | "onRefresh"> & {
  initialSamplePath?: string;
};

export function AdvancedVoiceStudio({ onNavigate, onRefresh, initialSamplePath }: Props) {
  const [projects, setProjects] = useState<RvcProject[]>([]);
  const [activeVoice, setActiveVoice] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [consent, setConsent] = useState(false);
  const [packagePath, setPackagePath] = useState("");
  const [legacyName, setLegacyName] = useState("");
  const [checkpoint, setCheckpoint] = useState("");
  const [index, setIndex] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { void refreshProjects(); }, []);

  async function refreshProjects() {
    setError(null);
    try {
      setProjects(await listRvcVoices());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Trained voices could not be loaded.");
    }
  }

  async function createVoice() {
    if (!name.trim() || !consent || busy) return;
    setBusy(true);
    setError(null);
    try {
      const project = await createRvcVoice(name.trim(), "Permission acknowledged in Voice Studio.");
      await refreshProjects();
      await onRefresh();
      setName("");
      setConsent(false);
      setActiveVoice(project.id);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The voice could not be created.");
    } finally {
      setBusy(false);
    }
  }

  async function browse(kind: "checkpoint" | "index" | "package") {
    setBusy(true);
    setError(null);
    try {
      const selected = await pickRvcArtifact(kind);
      if (!selected) return;
      if (kind === "checkpoint") setCheckpoint(selected);
      if (kind === "index") setIndex(selected);
      if (kind === "package") setPackagePath(selected);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : `Could not choose the ${kind} file.`);
    } finally {
      setBusy(false);
    }
  }

  async function importPackage() {
    if (!packagePath.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const project = await importRvcPackage(packagePath.trim());
      await refreshProjects();
      await onRefresh();
      setPackagePath("");
      setActiveVoice(project.id);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The voice package could not be imported.");
    } finally {
      setBusy(false);
    }
  }

  async function importLegacy() {
    if (!legacyName.trim() || !checkpoint.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const project = await importRvcVoice(checkpoint.trim(), index.trim() || undefined, legacyName.trim());
      await refreshProjects();
      await onRefresh();
      setLegacyName("");
      setCheckpoint("");
      setIndex("");
      setActiveVoice(project.id);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The legacy RVC voice could not be imported.");
    } finally {
      setBusy(false);
    }
  }

  if (activeVoice) {
    return (
      <RvcStudioWorkspace
        voice={activeVoice}
        initialSamplePath={initialSamplePath}
        onBack={() => { setActiveVoice(null); void refreshProjects(); }}
        onNavigate={onNavigate}
        onRefresh={onRefresh}
      />
    );
  }

  return (
    <section className="tk-rvc-home">
      <div className="tk-rvc-home__grid">
        <section className="tk-rvc-create-card" aria-label="Create trained voice">
          <header className="tk-rvc-create-card__header">
            <div>
              <span className="tk-rvc-kicker">Trained voice</span>
              <h2>New trained voice</h2>
              <p>Start with a name. Add recordings and train on the next screen.</p>
            </div>
            <span className="tk-rvc-create-card__icon"><AudioWaveform size={21} strokeWidth={1.7} /></span>
          </header>

          <div className="tk-rvc-create-card__body">
            <label className="tk-field">
              <span className="tk-field__label">Voice name</span>
              <input
                className="tk-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="For example, Studio narrator"
              />
              <span className="tk-field__hint">You can add multiple recordings after creating the voice.</span>
            </label>

            <div className="tk-rvc-create-flow" aria-label="Training flow">
              <div className="is-current"><span>1</span><p><strong>Create</strong><small>Name the voice</small></p></div>
              <div><span>2</span><p><strong>Add audio</strong><small>Upload or record</small></p></div>
              <div><span>3</span><p><strong>Train</strong><small>Takokit handles the rest</small></p></div>
            </div>

            <label className={consent ? "tk-rvc-permission is-checked" : "tk-rvc-permission"}>
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              <span className="tk-rvc-permission__check">{consent ? "✓" : ""}</span>
              <ShieldCheck size={18} strokeWidth={1.8} />
              <span>
                <strong>I own this voice or have explicit permission to use it.</strong>
                <small>Required before Takokit creates a persistent trained voice.</small>
              </span>
            </label>

            {initialSamplePath ? <div className="tk-rvc-seeded-note">Your selected audio will be added automatically after creation.</div> : null}
            {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
          </div>

          <footer className="tk-rvc-create-card__footer">
            <span>Local RVC training</span>
            <ProductButton tone="primary" loading={busy} disabled={busy || !name.trim() || !consent} onClick={() => void createVoice()}>
              <Plus size={15} /> Create trained voice
            </ProductButton>
          </footer>
        </section>

        <aside className="tk-rvc-library-card" aria-label="Trained voices">
          <header className="tk-rvc-library-card__header">
            <div><strong>Trained voices</strong><small>Open one to continue training or test it.</small></div>
            <span>{projects.length}</span>
          </header>

          <div className="tk-rvc-library-card__list">
            {projects.map((project) => (
              <button type="button" key={project.id} onClick={() => setActiveVoice(project.id)}>
                <span className="tk-rvc-library-card__voice-icon"><AudioWaveform size={16} strokeWidth={1.8} /></span>
                <span className="tk-rvc-library-card__identity">
                  <strong>{project.name}</strong>
                  <small>{project.imported ? "Imported voice" : stateLabel(project.state)}</small>
                </span>
                <ArrowRight size={14} />
              </button>
            ))}
            {projects.length === 0 ? <div className="tk-rvc-library-card__empty">No trained voices yet.</div> : null}
          </div>

          <details className="tk-rvc-import">
            <summary>Import an existing voice</summary>
            <div className="tk-rvc-import__body">
              <div>
                <strong>Takokit voice package</strong>
                <p>Recommended for moving a trained voice between Takokit installs.</p>
              </div>
              <div className="tk-rvc-simple-file-row">
                <input className="tk-input" value={packagePath} onChange={(event) => setPackagePath(event.target.value)} placeholder="Choose a .takovoice package" />
                <ProductButton tone="secondary" disabled={busy} onClick={() => void browse("package")}><FolderOpen size={14} /> Browse</ProductButton>
              </div>
              <ProductButton tone="primary" disabled={busy || !packagePath.trim()} onClick={() => void importPackage()}><FileArchive size={14} /> Import package</ProductButton>

              <details className="tk-rvc-simple-legacy">
                <summary>Legacy RVC files (.pth / .index)</summary>
                <p>Only use this for RVC models created outside Takokit.</p>
                <label className="tk-field"><span className="tk-field__label">Voice name</span><input className="tk-input" value={legacyName} onChange={(event) => setLegacyName(event.target.value)} /></label>
                <ArtifactRow label="Model (.pth)" value={checkpoint} onChange={setCheckpoint} onBrowse={() => void browse("checkpoint")} />
                <ArtifactRow label="Index (.index, optional)" value={index} onChange={setIndex} onBrowse={() => void browse("index")} />
                <ProductButton tone="secondary" disabled={busy || !legacyName.trim() || !checkpoint.trim()} onClick={() => void importLegacy()}>Import legacy voice</ProductButton>
              </details>
            </div>
          </details>
        </aside>
      </div>
    </section>
  );
}

function ArtifactRow({ label, value, onChange, onBrowse }: { label: string; value: string; onChange: (value: string) => void; onBrowse: () => void }) {
  return (
    <label className="tk-field">
      <span className="tk-field__label">{label}</span>
      <div className="tk-rvc-simple-file-row">
        <input className="tk-input" value={value} onChange={(event) => onChange(event.target.value)} />
        <ProductButton tone="secondary" onClick={onBrowse}><FolderOpen size={14} /> Browse</ProductButton>
      </div>
    </label>
  );
}

function stateLabel(state: string): string {
  if (state === "ready") return "Ready";
  if (["created", "collecting_samples"].includes(state)) return "Needs recordings";
  if (["ready_for_preparation", "ready_to_train"].includes(state)) return "Ready to train";
  if (["preprocessing", "extracting_f0", "extracting_features", "training", "building_index", "validating_artifacts"].includes(state)) return "Training";
  if (["failed", "cancelled"].includes(state)) return "Needs attention";
  return state.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
