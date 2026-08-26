import {
  ArrowRight,
  FileAudio,
  FileText,
  FolderOpen,
  Mic,
  Trash2,
  Upload
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { AudioRecorder } from "../../components/audio/AudioRecorder";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import {
  deleteWorkspaceFile,
  listWorkspaceFiles,
  loadWorkspaceText,
  uploadWorkspaceFile,
  type WorkspaceFile
} from "../../lib/files";
import {
  setCloneIntent,
  setSpeakIntent,
  setTranscribeIntent,
  setVoiceIntent
} from "../../lib/workflowIntent";

export function FilesPage({ onNavigate }: RouteComponentProps) {
  const [files, setFiles] = useState<WorkspaceFile[]>([]);
  const [filter, setFilter] = useState<"all" | "audio" | "text">("all");
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<WorkspaceFile | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const visibleFiles = useMemo(
    () => files.filter((file) => filter === "all" || file.kind === filter),
    [files, filter]
  );

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setFiles(await listWorkspaceFiles());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Workspace files could not be loaded.");
    } finally {
      setLoading(false);
    }
  }

  async function uploadSelected(selected: FileList | null) {
    if (!selected || selected.length === 0) return;
    setUploading(true);
    setError(null);
    try {
      for (const file of Array.from(selected)) {
        await uploadWorkspaceFile(file, file.name);
      }
      await refresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "One of the selected files could not be uploaded.");
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }

  async function useTextInSpeak(file: WorkspaceFile) {
    setError(null);
    try {
      const text = await loadWorkspaceText(file);
      setSpeakIntent({ text });
      onNavigate("speak");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The text file could not be opened.");
    }
  }

  async function removeSelected() {
    if (!deleteTarget) return;
    setLoading(true);
    try {
      await deleteWorkspaceFile(deleteTarget.id);
      setDeleteTarget(null);
      await refresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The workspace file could not be removed.");
      setLoading(false);
    }
  }

  function useAudio(file: WorkspaceFile, destination: "transcribe" | "voices" | "advanced" | "clone") {
    if (destination === "transcribe") {
      setTranscribeIntent({ filePath: file.path });
      onNavigate("transcribe");
      return;
    }
    if (destination === "voices") {
      setVoiceIntent({ samplePath: file.path, mode: "instant" });
      onNavigate("voices");
      return;
    }
    if (destination === "advanced") {
      setVoiceIntent({ samplePath: file.path, mode: "advanced" });
      onNavigate("voices");
      return;
    }
    setCloneIntent({ sourcePath: file.path, mode: "reference" });
    onNavigate("convert");
  }

  return (
    <section className="tk-page tk-files-page">
      <ProductPageHeader
        eyebrow="Workspace assets"
        title="Files"
        description="Keep reusable audio and text with this workspace, then send them directly into Takokit workflows without browsing for the same file again."
        actions={(
          <>
            <input
              ref={fileInputRef}
              className="tk-file-input-hidden"
              type="file"
              multiple
              accept="audio/*,.wav,.mp3,.flac,.ogg,.m4a,.aac,.wma,.txt,.md,.json,.csv"
              onChange={(event) => void uploadSelected(event.target.files)}
            />
            <ProductButton tone="primary" loading={uploading} onClick={() => fileInputRef.current?.click()}>
              <Upload size={15} /> Upload files
            </ProductButton>
          </>
        )}
      />

      <section className="tk-files-recorder">
        <div>
          <span className="tk-files-recorder__icon"><Mic size={18} strokeWidth={1.8} /></span>
          <div>
            <strong>Record into this workspace</strong>
            <p>Capture a clean WAV now. The recording is saved to Files and can be reused anywhere.</p>
          </div>
        </div>
        <AudioRecorder onSaved={(file) => setFiles((current) => [file, ...current])} compact label="Record a workspace clip" />
      </section>

      <div className="tk-files-toolbar">
        <div className="tk-files-tabs" role="tablist" aria-label="File type">
          {(["all", "audio", "text"] as const).map((value) => (
            <button
              key={value}
              type="button"
              className={filter === value ? "is-active" : ""}
              onClick={() => setFilter(value)}
            >
              {value === "all" ? "All files" : value === "audio" ? "Audio" : "Text"}
            </button>
          ))}
        </div>
        <span>{visibleFiles.length} {visibleFiles.length === 1 ? "file" : "files"}</span>
      </div>

      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      <section className="tk-files-list" aria-live="polite">
        {visibleFiles.map((file) => (
          <article className="tk-file-row" key={file.id}>
            <span className={file.kind === "audio" ? "tk-file-row__icon is-audio" : "tk-file-row__icon"}>
              {file.kind === "audio" ? <FileAudio size={19} strokeWidth={1.7} /> : <FileText size={19} strokeWidth={1.7} />}
            </span>
            <div className="tk-file-row__identity">
              <strong>{file.name}</strong>
              <span>{formatBytes(file.bytes)} · {formatTime(file.modified_at)}</span>
              <small title={file.path}>{file.path}</small>
            </div>

            {file.kind === "audio" ? (
              <div className="tk-file-row__preview">
                <LocalAudioPlayer path={file.path} compact defer label="Preview" />
              </div>
            ) : null}

            <div className="tk-file-row__actions">
              {file.kind === "text" ? (
                <button type="button" onClick={() => void useTextInSpeak(file)}>
                  Use in Speak <ArrowRight size={13} />
                </button>
              ) : (
                <>
                  <button type="button" onClick={() => useAudio(file, "transcribe")}>Transcribe</button>
                  <button type="button" onClick={() => useAudio(file, "voices")}>Instant voice</button>
                  <button type="button" onClick={() => useAudio(file, "advanced")}>Add to Voice Dataset</button>
                  <button type="button" onClick={() => useAudio(file, "clone")}>Clone audio <ArrowRight size={13} /></button>
                </>
              )}
              <button className="tk-file-row__delete" type="button" title={`Remove ${file.name}`} onClick={() => setDeleteTarget(file)}>
                <Trash2 size={14} />
              </button>
            </div>
          </article>
        ))}

        {!loading && visibleFiles.length === 0 ? (
          <div className="tk-files-empty">
            <FolderOpen size={25} strokeWidth={1.5} />
            <strong>{filter === "all" ? "No workspace files yet" : `No ${filter} files yet`}</strong>
            <span>Upload an existing file or record a new audio clip above.</span>
          </div>
        ) : null}
      </section>

      <ConfirmDialog
        open={Boolean(deleteTarget)}
        title="Remove this workspace file?"
        description={deleteTarget ? <>This removes <strong>{deleteTarget.name}</strong> from this workspace's Files library. Existing session outputs are not affected.</> : null}
        confirmLabel="Remove file"
        destructive
        busy={loading}
        onCancel={() => !loading && setDeleteTarget(null)}
        onConfirm={() => void removeSelected()}
      />
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

function formatTime(timestamp: number): string {
  if (!timestamp) return "Saved locally";
  return new Date(timestamp * 1000).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}
