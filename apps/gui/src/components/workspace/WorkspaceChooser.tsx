import { Clock3, FolderOpen, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ProductButton } from "../ui/ProductButton";
import { pickFolder } from "../../lib/nativePicker";
import { getRecentWorkspaces, getWorkspaceContext, selectWorkspace } from "../../lib/workspace";

type WorkspaceChooserProps = {
  open: boolean;
  switching?: boolean;
  onClose?: () => void;
  onSelected: () => Promise<void> | void;
};

export function WorkspaceChooser({ open, switching = false, onClose, onSelected }: WorkspaceChooserProps) {
  const context = getWorkspaceContext();
  const defaultPath = context.workspace ?? "";
  const recent = useMemo(() => getRecentWorkspaces(), [open]);
  const [path, setPath] = useState(recent[0] ?? defaultPath);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pickerBusy, setPickerBusy] = useState(false);

  useEffect(() => {
    if (open) setPath(getRecentWorkspaces()[0] ?? getWorkspaceContext().workspace ?? "");
  }, [open]);

  if (!open) return null;

  async function apply(selectedPath: string) {
    setError(null);
    setBusy(true);
    try {
      selectWorkspace(selectedPath);
      await onSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Workspace selection failed.");
    } finally {
      setBusy(false);
    }
  }

  async function browse() {
    setPickerBusy(true);
    setError(null);
    try {
      const selected = await pickFolder();
      if (selected) setPath(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The folder picker could not be opened.");
    } finally {
      setPickerBusy(false);
    }
  }

  return (
    <div className="tk-workspace-backdrop" role="presentation" onMouseDown={onClose}>
      <div className="tk-workspace-dialog" role="dialog" aria-modal="true" aria-labelledby="tk-workspace-title" onMouseDown={(event) => event.stopPropagation()}>
        <header className="tk-workspace-dialog__header">
          <span className="tk-workspace-dialog__icon"><FolderOpen size={19} strokeWidth={1.8} /></span>
          <div><span>Project workspace</span><h2 id="tk-workspace-title">Choose where this project lives</h2><p>Sessions, transcripts, and outputs stay in this workspace. Models, runners, and reusable voices remain in the shared Takokit store.</p></div>
          {onClose ? <button type="button" className="tk-dialog__close" onClick={onClose} aria-label="Close workspace chooser"><X size={16} /></button> : null}
        </header>

        <div className="tk-workspace-dialog__body">
          <label className="tk-workspace-path">
            <span>Workspace folder</span>
            <div><input value={path} onChange={(event) => setPath(event.target.value)} placeholder="D:\\VoiceProjects\\Audiobook" autoFocus spellCheck={false} /><ProductButton tone="secondary" loading={pickerBusy} onClick={() => void browse()}><FolderOpen size={14} /> Browse</ProductButton></div>
          </label>

          {recent.length > 0 ? (
            <div className="tk-workspace-recents">
              <div className="tk-workspace-recents__heading"><Clock3 size={14} /><span>Recent workspaces</span></div>
              {recent.map((workspace) => <button className={workspace === path ? "is-active" : ""} key={workspace} type="button" onClick={() => setPath(workspace)}><strong>{workspaceName(workspace)}</strong><span>{workspace}</span></button>)}
            </div>
          ) : null}

          {switching ? <div className="tk-inline-warning">A Takokit operation is running. Workspace switching is disabled until it finishes.</div> : null}
          {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
        </div>

        <footer className="tk-workspace-dialog__actions">
          {onClose ? <ProductButton tone="ghost" disabled={busy} onClick={onClose}>Cancel</ProductButton> : <span />}
          <ProductButton tone="primary" loading={busy} disabled={switching || !path.trim()} onClick={() => void apply(path)}>Use workspace</ProductButton>
        </footer>
      </div>
    </div>
  );
}

function workspaceName(path: string): string {
  const normalized = path.replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}
