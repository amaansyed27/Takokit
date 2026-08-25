import { ArrowRight, AudioLines, FileArchive, Plus, RotateCcw, ShieldCheck } from "lucide-react";
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

type CreateMode = "new" | "import" | "package";

export function AdvancedVoiceStudio({ onNavigate, onRefresh, initialSamplePath }: Props) {
  const [projects, setProjects] = useState<RvcProject[]>([]);
  const [activeVoice, setActiveVoice] = useState<string | null>(null);
  const [mode, setMode] = useState<CreateMode>("new");
  const [name, setName] = useState("");
  const [checkpoint, setCheckpoint] = useState("");
  const [index, setIndex] = useState("");
  const [packagePath, setPackagePath] = useState("");
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { void refreshProjects(); }, []);

  async function refreshProjects() {
    setError(null);
    try {
      setProjects(await listRvcVoices());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Advanced voices could not be loaded.");
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
      setError(caught instanceof Error ? caught.message : `Could not choose the RVC ${kind}.`);
    } finally {
      setBusy(false);
    }
  }

  async function createOrImport() {
    if (!consent || !name.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const project = mode === "new"
        ? await createRvcVoice(name.trim(), "Permission acknowledged in Voice Studio.")
        : mode === "import"
          ? await importRvcVoice(checkpoint.trim(), index.trim() || undefined, name.trim())
          : await importRvcPackage(packagePath.trim(), name.trim());
      await refreshProjects();
      await onRefresh();
      setActiveVoice(project.id);
      setName("");
      setCheckpoint("");
      setIndex("");
      setPackagePath("");
      setConsent(false);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The advanced voice could not be created.");
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

  const canSubmit = consent && Boolean(name.trim()) && !busy && (
    mode === "new" || (mode === "import" && Boolean(checkpoint.trim())) || (mode === "package" && Boolean(packagePath.trim()))
  );

  return (
    <section className="tk-rvc-landing">
      <div className="tk-rvc-intro">
        <div>
          <span className="tk-voice-builder__eyebrow">Advanced clone</span>
          <h2>Custom RVC Voice Studio</h2>
          <p>Build a persistent local RVC voice from recordings, or bring an existing checkpoint/package into the same managed workflow.</p>
        </div>
        <span className="tk-rvc-intro__mark"><AudioLines size={20} strokeWidth={1.6} /></span>
      </div>

      <div className="tk-rvc-mode-grid" role="tablist" aria-label="Advanced voice source">
        <button className={mode === "new" ? "is-active" : ""} type="button" onClick={() => setMode("new")}>
          <Plus size={18} /><strong>New custom voice</strong><span>Record or upload multiple samples, inspect, prepare, train, checkpoint and test.</span>
        </button>
        <button className={mode === "import" ? "is-active" : ""} type="button" onClick={() => setMode("import")}>
          <RotateCcw size={18} /><strong>Import existing RVC</strong><span>Copy an existing .pth and optional .index into Takokit-managed voice storage.</span>
        </button>
        <button className={mode === "package" ? "is-active" : ""} type="button" onClick={() => setMode("package")}>
          <FileArchive size={18} /><strong>Import .takovoice</strong><span>Verify package integrity/signature metadata before importing managed artifacts.</span>
        </button>
      </div>

      <div className="tk-rvc-create-grid">
        <label className="tk-field">
          <span className="tk-field__label">Voice name</span>
          <input className="tk-input" value={name} onChange={(event) => setName(event.target.value)} placeholder="For example, Narration voice" />
        </label>
        {mode === "import" ? (
          <>
            <ArtifactField label="Checkpoint (.pth)" value={checkpoint} required onBrowse={() => void browse("checkpoint")} onChange={setCheckpoint} />
            <ArtifactField label="Index (.index, optional)" value={index} onBrowse={() => void browse("index")} onChange={setIndex} />
          </>
        ) : null}
        {mode === "package" ? (
          <ArtifactField label="Voice package (.takovoice)" value={packagePath} required onBrowse={() => void browse("package")} onChange={setPackagePath} />
        ) : null}
      </div>

      <label className="tk-rvc-consent">
        <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
        <span><ShieldCheck size={16} /><strong>I own this voice or have explicit permission to create/import it.</strong><small>Takokit records this acknowledgement locally. Package signatures prove artifact integrity, not speaker identity or legal ownership.</small></span>
      </label>

      {initialSamplePath && mode === "new" ? <div className="tk-inline-note">The audio sent from Files will be copied into this voice dataset after creation.</div> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
      <div className="tk-rvc-create-actions">
        <ProductButton tone="primary" disabled={!canSubmit} loading={busy} onClick={() => void createOrImport()}>
          {mode === "new" ? "Create studio" : "Import voice"} <ArrowRight size={15} />
        </ProductButton>
      </div>

      <section className="tk-rvc-projects">
        <header><div><strong>Your advanced voices</strong><span>{projects.length} managed project{projects.length === 1 ? "" : "s"}</span></div></header>
        {projects.map((project) => (
          <button type="button" key={project.id} onClick={() => setActiveVoice(project.id)}>
            <span><strong>{project.name}</strong><small>{project.imported ? "Imported RVC" : "Custom RVC"} · {stateLabel(project.state)}</small></span>
            <ArrowRight size={15} />
          </button>
        ))}
        {projects.length === 0 ? <div className="tk-rvc-empty">No advanced RVC voice projects yet.</div> : null}
      </section>
    </section>
  );
}

function ArtifactField({ label, value, required, onBrowse, onChange }: {
  label: string; value: string; required?: boolean; onBrowse: () => void; onChange: (value: string) => void;
}) {
  return (
    <label className="tk-field tk-rvc-artifact-field">
      <span className="tk-field__label">{label}{required ? " *" : ""}</span>
      <div><input className="tk-input" value={value} onChange={(event) => onChange(event.target.value)} placeholder="Choose a local file or enter its path" /><button type="button" onClick={onBrowse}>Browse</button></div>
    </label>
  );
}

function stateLabel(state: string): string {
  return state.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
