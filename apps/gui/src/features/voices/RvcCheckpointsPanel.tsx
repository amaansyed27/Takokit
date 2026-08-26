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
      setMessage("Selected model version is now active.");
      await onChanged();
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Model activation failed."); }
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
      setVerification(await verifyRvcPackage(path));
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Package verification failed."); }
    finally { setBusy(false); }
  }

  return (
    <div className="tk-rvc-simple-model-panel">
      {detail.managed ? (
        <div className="tk-rvc-simple-ready-line"><Check size={14} /><span><strong>Model ready</strong> Takokit automatically selected and activated the validated model and index.</span></div>
      ) : <p className="tk-rvc-simple-note">No active trained model yet.</p>}

      <div className="tk-rvc-simple-package-tools">
        <div><strong>Export this voice</strong><p>Create a portable .takovoice package without the original training recordings.</p></div>
        <label><input type="checkbox" checked={sign} onChange={(event) => setSign(event.target.checked)} /> Sign package</label>
        <ProductButton tone="secondary" disabled={busy || !detail.managed} onClick={() => void exportPackage()}><Download size={14} /> Export</ProductButton>
        <ProductButton tone="secondary" disabled={busy} onClick={() => void verifyPackage()}><PackageCheck size={14} /> Verify package</ProductButton>
      </div>

      {verification ? (
        <div className={verification.errors.length === 0 && verification.hashes_valid ? "tk-rvc-verification is-valid" : "tk-rvc-verification is-invalid"}>
          <header><ShieldCheck size={16} /><strong>{verification.errors.length === 0 && verification.hashes_valid ? "Package integrity valid" : "Package verification failed"}</strong></header>
          <p>Hashes: {verification.hashes_valid ? "valid" : "invalid"} · Signature: {verification.signed ? verification.signature_valid ? "valid" : "invalid" : "unsigned"}</p>
          {verification.signer_fingerprint ? <p className="tk-rvc-fingerprint"><Fingerprint size={13} /> {verification.signer_fingerprint}</p> : null}
          {verification.errors.map((item) => <p key={item}>{item}</p>)}
        </div>
      ) : null}

      <details className="tk-rvc-simple-legacy">
        <summary>Choose another model version</summary>
        <p>Most users never need this. Takokit automatically activates the final validated model after training.</p>
        <div className="tk-rvc-checkpoint-fields">
          <label><span>Model</span><select value={selectedCheckpoint} onChange={(event) => { setSelectedCheckpoint(event.target.value); setSelectedIndex(""); }}><option value="">Choose model</option>{detail.checkpoints.map((item) => <option key={item.id} value={item.id}>{checkpointLabel(item.path, item.epoch)}{item.id === detail.project.active_checkpoint_id ? " · active" : ""}</option>)}</select></label>
          <label><span>Index</span><select value={selectedIndex} onChange={(event) => setSelectedIndex(event.target.value)}><option value="">No index</option>{indexes.map((item) => <option key={item.id} value={item.id}>{fileName(item.path)}{item.id === detail.project.active_index_id ? " · active" : ""}</option>)}</select></label>
          <ProductButton tone="secondary" disabled={busy || !selectedCheckpoint} onClick={() => void activate()}>Use this version</ProductButton>
        </div>
        <div className="tk-rvc-simple-artifact-list">
          {detail.checkpoints.map((item) => <p key={item.id}><strong>{fileName(item.path)}</strong> · {formatBytes(item.bytes)}{item.epoch ? ` · epoch ${item.epoch}` : ""}</p>)}
        </div>
      </details>

      {message ? <div className="tk-inline-success">{message}</div> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
    </div>
  );
}

function fileName(path: string): string { return path.split(/[\\/]/).pop() || path; }
function checkpointLabel(path: string, epoch?: number): string { return `${fileName(path)}${epoch ? ` · epoch ${epoch}` : ""}`; }
function formatBytes(bytes: number): string { return bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`; }
