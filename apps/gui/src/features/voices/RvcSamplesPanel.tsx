import { FileAudio, FolderPlus, RefreshCw, Trash2, Upload } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { AudioRecorder } from "../../components/audio/AudioRecorder";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { listWorkspaceFiles, uploadWorkspaceFile, type WorkspaceFile } from "../../lib/files";
import {
  addRvcSamples,
  inspectRvcDataset,
  removeRvcSample,
  setRvcSampleIncluded,
  type RvcVoiceDetail
} from "../../lib/rvcApi";

type Props = {
  detail: RvcVoiceDetail;
  initialSamplePath?: string;
  onChanged: () => Promise<void>;
};

export function RvcSamplesPanel({ detail, initialSamplePath, onChanged }: Props) {
  const voice = detail.project.id;
  const inputRef = useRef<HTMLInputElement | null>(null);
  const initialHandled = useRef(false);
  const [workspaceFiles, setWorkspaceFiles] = useState<WorkspaceFile[]>([]);
  const [filesOpen, setFilesOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const audioFiles = useMemo(() => workspaceFiles.filter((file) => file.kind === "audio"), [workspaceFiles]);

  useEffect(() => {
    if (!initialSamplePath || initialHandled.current) return;
    initialHandled.current = true;
    if (detail.samples.some((sample) => sample.source_path === initialSamplePath || sample.managed_path === initialSamplePath)) return;
    void addPaths([initialSamplePath]);
  }, [initialSamplePath, detail.samples]);

  async function addPaths(paths: string[]) {
    if (paths.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      await addRvcSamples(voice, paths);
      await inspectRvcDataset(voice);
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Recordings could not be added to this voice.");
    } finally {
      setBusy(false);
    }
  }

  async function uploadSelected(files: FileList | null) {
    if (!files?.length) return;
    setBusy(true);
    setError(null);
    try {
      const uploaded: WorkspaceFile[] = [];
      for (const file of Array.from(files)) uploaded.push(await uploadWorkspaceFile(file, file.name));
      await addRvcSamples(voice, uploaded.map((file) => file.path));
      await inspectRvcDataset(voice);
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "One of the selected recordings could not be added.");
    } finally {
      setBusy(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  }

  async function showWorkspaceFiles() {
    setBusy(true);
    setError(null);
    try {
      setWorkspaceFiles(await listWorkspaceFiles());
      setFilesOpen(true);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Workspace Files could not be loaded.");
    } finally {
      setBusy(false);
    }
  }

  async function inspect() {
    setBusy(true);
    setError(null);
    try {
      await inspectRvcDataset(voice);
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Audio check failed.");
    } finally {
      setBusy(false);
    }
  }

  async function toggleSample(sampleId: string, included: boolean) {
    setBusy(true);
    setError(null);
    try {
      await setRvcSampleIncluded(voice, sampleId, included);
      await inspectRvcDataset(voice);
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The recording could not be updated.");
    } finally {
      setBusy(false);
    }
  }

  async function removeSample(sampleId: string) {
    setBusy(true);
    setError(null);
    try {
      await removeRvcSample(voice, sampleId);
      if (detail.samples.length > 1) await inspectRvcDataset(voice);
      await onChanged();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The recording could not be removed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="tk-rvc-simple-panel tk-rvc-simple-data">
      <header className="tk-rvc-simple-section-heading">
        <div><h3>Add voice recordings</h3><p>Use clear audio from one speaker. Takokit checks each recording automatically after you add it.</p></div>
        <button type="button" className="tk-text-button" disabled={busy || detail.samples.length === 0} onClick={() => void inspect()}><RefreshCw size={13} /> Recheck audio</button>
      </header>

      <div className="tk-rvc-simple-actions">
        <input ref={inputRef} className="tk-file-input-hidden" type="file" multiple accept="audio/*,.wav,.mp3,.flac,.ogg,.m4a,.aac,.wma" onChange={(event) => void uploadSelected(event.target.files)} />
        <ProductButton tone="primary" disabled={busy} onClick={() => inputRef.current?.click()}><Upload size={14} /> Upload recordings</ProductButton>
        <ProductButton tone="secondary" disabled={busy} onClick={() => void showWorkspaceFiles()}><FolderPlus size={14} /> Add from Files</ProductButton>
      </div>

      <AudioRecorder reviewBeforeSave onSaved={(file) => void addPaths([file.path])} compact label="Record now" />

      {filesOpen ? (
        <div className="tk-rvc-simple-picker">
          <header><strong>Workspace audio</strong><button type="button" onClick={() => setFilesOpen(false)}>Close</button></header>
          {audioFiles.map((file) => (
            <button type="button" key={file.id} disabled={busy} onClick={() => void addPaths([file.path])}>
              <FileAudio size={15} /><span><strong>{file.name}</strong><small>{formatBytes(file.bytes)}</small></span><span>Add</span>
            </button>
          ))}
          {audioFiles.length === 0 ? <p>No audio files are saved in this workspace yet.</p> : null}
        </div>
      ) : null}

      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      {detail.samples.length > 0 ? (
        <div className="tk-rvc-simple-dataset-line">
          <span><strong>{detail.dataset.included_sample_count}</strong> included</span>
          <span><strong>{formatDuration(detail.dataset.usable_duration_ms)}</strong> usable audio</span>
          <span className={detail.dataset.warning_count ? "has-warning" : ""}><strong>{detail.dataset.warning_count}</strong> warnings</span>
          {detail.dataset.duplicate_count ? <span className="has-warning"><strong>{detail.dataset.duplicate_count}</strong> duplicates</span> : null}
        </div>
      ) : null}

      {detail.dataset.warnings.length > 0 ? (
        <div className="tk-rvc-warning-stack">
          {detail.dataset.warnings.map((warning, index) => <p key={`${warning.code}-${index}`}><strong>{labelWarning(warning.code)}</strong> {warning.message}</p>)}
        </div>
      ) : null}

      <div className="tk-rvc-simple-recordings">
        {detail.samples.map((sample) => (
          <article key={sample.id} className={sample.included ? "" : "is-excluded"}>
            <div className="tk-rvc-simple-recording-name">
              <FileAudio size={16} />
              <span><strong>{sample.display_name}</strong><small>{formatDuration(sample.inspection?.duration_ms ?? 0)} · {formatBytes(sample.bytes)}{sample.warnings.length ? ` · ${sample.warnings.length} warning${sample.warnings.length === 1 ? "" : "s"}` : " · checked"}</small></span>
            </div>
            <LocalAudioPlayer path={sample.managed_path} compact defer label="Play recording" />
            <div className="tk-rvc-simple-recording-actions">
              <label><input type="checkbox" checked={sample.included} disabled={busy} onChange={(event) => void toggleSample(sample.id, event.target.checked)} /> Use for training</label>
              <button type="button" disabled={busy} title="Remove recording" onClick={() => void removeSample(sample.id)}><Trash2 size={14} /></button>
            </div>
          </article>
        ))}
        {detail.samples.length === 0 ? <p className="tk-rvc-empty">Add at least one recording to begin.</p> : null}
      </div>
    </div>
  );
}

function formatDuration(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.round(milliseconds / 1000));
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, "0")}`;
}
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
function labelWarning(code: string): string {
  return code.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
