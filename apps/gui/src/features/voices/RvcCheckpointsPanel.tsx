import { Check, Download, Fingerprint, PackageCheck, ShieldCheck } from "lucide-react";
import { useMemo, useState } from "react";
import { ProductButton } from "../../components/ui/ProductButton";
import { pickFolder, pickRvcArtifact } from "../../lib/nativePicker";
import {
  activateRvcCheckpoint,
  exportRvcVoice,
  verifyRvcPackage,
  type PackageVerification,
  type RvcVoiceDetail
} from "../../lib/rvcApi";

type Props = { detail: RvcVoiceDetail; onChanged: () => Promise<void> };

export function RvcCheckpointsPanel({ detail, onChanged }: Props) {
  const voice = detail.project.id;
  const [selectedCheckpoint, setSelectedCheckpoint] = useState(detail.project.active_checkpoint_id ?? detail.checkpoints[0]?.id ?? "");
  const [selectedIndex, setSelectedIndex] = useState(detail.project.active_index_id ?? "");
  const [sign, setSign] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [verification, setVerification] = useState<PackageVerification | null>(null);

  const indexes = useMemo(
    () => detail.indexes.filter((item) => !selectedCheckpoint || !item.checkpoint_id || item.checkpoint_id === selectedCheckpoint),
    [detail.indexes, selectedCheckpoint]
  );

  async function activate() {
    if (!selectedCheckpoint) return;
    setBusy(true); setError(null); setMessage(null);
    try {
      await activateRvcCheckpoint(voice, selectedCheckpoint, selectedIndex || undefined);
      setMessage("This checkpoint is now the managed conversion target.");
      await onChanged();
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Checkpoint activation failed."); }
    finally { setBusy(false); }
  }

  async function exportPackage() {
    setBusy(true); setError(null); setMessage(null);
    try {
      const folder = await pickFolder();
      if (!folder) return;
      const separator = folder.includes("\\") ? "\\" : "/";
      const safeName = detail.project.name.replace(/[<>:"/\\|?*\x00-\x1f]+/g, "-").trim() || "takokit-voice";
      const output = `${folder.replace(/[\\/]$/, "")}${separator}${safeName}.takovoice`;
      const saved = await exportRvcVoice(voice, output, sign, false);
      setMessage(`Exported ${saved}. Training recordings are not included.`);
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Voice package export failed."); }
    finally { setBusy(false); }
  }

  async function verifyPackage() {
    setBusy(true); setError(null); setMessage(null); setVerification(null);
    try {
      const path = await pickRvcArtifact("package");
      if (!path) return;
      const report = await verifyRvcPackage(path);
      setVerification(report);
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Package verification failed."); }
    finally { setBusy(false); }
  }

  return (
    <div className="tk-rvc-panel tk-rvc-checkpoints-panel">
      <section className="tk-rvc-checkpoint-selector">
        <header><div><strong>Active checkpoint</strong><p>Select which validated checkpoint/index pair the normal RVC Convert workflow should use for this managed voice.</p></div>{detail.managed ? <span className="tk-rvc-ready-pill"><Check size={13} /> Ready</span> : null}</header>
        <div className="tk-rvc-checkpoint-fields">
          <label><span>Checkpoint</span><select value={selectedCheckpoint} onChange={(event) => { setSelectedCheckpoint(event.target.value); setSelectedIndex(""); }}><option value="">Choose checkpoint</option>{detail.checkpoints.map((item) => <option key={item.id} value={item.id}>{checkpointLabel(item.path, item.epoch)}{item.id === detail.project.active_checkpoint_id ? " · active" : ""}</option>)}</select></label>
          <label><span>Index</span><select value={selectedIndex} onChange={(event) => setSelectedIndex(event.target.value)}><option value="">No index</option>{indexes.map((item) => <option key={item.id} value={item.id}>{fileName(item.path)}{item.id === detail.project.active_index_id ? " · active" : ""}</option>)}</select></label>
          <ProductButton tone="primary" disabled={busy || !selectedCheckpoint} onClick={() => void activate()}>Use checkpoint</ProductButton>
        </div>
      </section>

      <section className="tk-rvc-artifact-list">
        <header><strong>Checkpoints</strong><span>{detail.checkpoints.length} validated artifact{detail.checkpoints.length === 1 ? "" : "s"}</span></header>
        {detail.checkpoints.map((item) => (
          <article key={item.id} className={item.id === detail.project.active_checkpoint_id ? "is-active" : ""}>
            <div><strong>{fileName(item.path)}</strong><small>{formatBytes(item.bytes)} · {item.model_version ?? "version unknown"} · {item.sample_rate_hz ? `${item.sample_rate_hz / 1000} kHz` : "sample rate unknown"}</small></div>
            <span>{item.valid_for_inference ? "Validated" : "Invalid"}</span>
          </article>
        ))}
        {detail.checkpoints.length === 0 ? <div className="tk-rvc-empty">No checkpoint has been imported or produced yet.</div> : null}
      </section>

      <section className="tk-rvc-package-tools">
        <article><span><Download size={17} /></span><div><strong>Export .takovoice</strong><p>Portable checkpoint/index package. Dataset recordings are excluded by default.</p><label><input type="checkbox" checked={sign} onChange={(event) => setSign(event.target.checked)} /> Sign manifest with this Takokit install's Ed25519 voice-package key</label></div><ProductButton tone="secondary" disabled={busy || !detail.managed} onClick={() => void exportPackage()}>Export</ProductButton></article>
        <article><span><PackageCheck size={17} /></span><div><strong>Verify a package</strong><p>Check manifest bounds, SHA-256 artifact hashes and optional Ed25519 signature metadata before import.</p></div><ProductButton tone="secondary" disabled={busy} onClick={() => void verifyPackage()}>Choose & verify</ProductButton></article>
      </section>

      {verification ? (
        <section className={verification.errors.length === 0 && verification.hashes_valid ? "tk-rvc-verification is-valid" : "tk-rvc-verification is-invalid"}>
          <header><ShieldCheck size={16} /><strong>{verification.errors.length === 0 && verification.hashes_valid ? "Package integrity valid" : "Package verification failed"}</strong></header>
          <p>Hashes: {verification.hashes_valid ? "valid" : "invalid"} · Signature: {verification.signed ? verification.signature_valid ? "valid" : "invalid" : "unsigned"}</p>
          {verification.signer_fingerprint ? <p className="tk-rvc-fingerprint"><Fingerprint size={13} /> {verification.signer_fingerprint}</p> : null}
          {verification.errors.map((item) => <p key={item}>{item}</p>)}
        </section>
      ) : null}
      {message ? <div className="tk-inline-success">{message}</div> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
    </div>
  );
}

function fileName(path: string): string { return path.split(/[\\/]/).pop() || path; }
function checkpointLabel(path: string, epoch?: number): string { return `${fileName(path)}${epoch ? ` · epoch ${epoch}` : ""}`; }
function formatBytes(bytes: number): string { return bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`; }
