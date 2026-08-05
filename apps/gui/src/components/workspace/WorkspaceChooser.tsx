import { useMemo, useState } from "react";
import { Button } from "../ui/Button";
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

  return (
    <div className="workspace-dialog" role="dialog" aria-modal="true" aria-labelledby="workspace-dialog-title">
      <div className="workspace-dialog__panel">
        <header>
          <span className="eyebrow">Project storage</span>
          <h2 id="workspace-dialog-title">Choose a Takokit workspace</h2>
          <p>
            Models, runners and adapters remain global under <code>.takokit</code>. Sessions, transcripts and generated outputs are stored in this project&apos;s <code>.tako</code> directory only after the first workflow runs.
          </p>
        </header>

        <label className="field-label" htmlFor="workspace-path">Workspace path</label>
        <input
          id="workspace-path"
          className="search-input"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          placeholder="D:\\VoiceProjects\\Audiobook"
          autoFocus
        />

        {recent.length > 0 && (
          <div className="workspace-dialog__recents">
            <strong>Recent workspaces</strong>
            {recent.map((workspace) => (
              <button key={workspace} type="button" onClick={() => setPath(workspace)}>
                {workspace}
              </button>
            ))}
          </div>
        )}

        {error && <p className="notice-line" role="alert">{error}</p>}
        {switching && (
          <p className="notice-line">A Takokit operation is running. Workspace switching is disabled until it finishes.</p>
        )}

        <div className="workspace-dialog__actions">
          <Button type="button" variant="ghost" disabled={busy || switching || !path.trim()} onClick={() => apply(path)}>
            Open existing workspace
          </Button>
          <Button type="button" disabled={busy || switching || !path.trim()} loading={busy} onClick={() => apply(path)}>
            Choose or create workspace
          </Button>
          {defaultPath && (
            <Button type="button" variant="ghost" disabled={busy || switching} onClick={() => apply(defaultPath)}>
              Use default Takokit workspace
            </Button>
          )}
          {onClose && (
            <Button type="button" variant="ghost" disabled={busy} onClick={onClose}>Cancel</Button>
          )}
        </div>
      </div>
    </div>
  );
}
