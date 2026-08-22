import { FolderOpen, Server } from "lucide-react";
import type { RuntimeSnapshot } from "../../lib/types";

type ProductTopBarProps = {
  runtime: RuntimeSnapshot;
  onChangeWorkspace: () => void;
};

export function ProductTopBar({ runtime, onChangeWorkspace }: ProductTopBarProps) {
  const online = runtime.server.status === "online";
  const workspaceName = workspaceDisplayName(runtime.workspacePath);

  return (
    <header className="tk-topbar">
      <button className="tk-workspace-button" type="button" onClick={onChangeWorkspace} title={runtime.workspacePath}>
        <FolderOpen size={16} strokeWidth={1.8} aria-hidden="true" />
        <span>
          <small>Workspace</small>
          <strong>{workspaceName}</strong>
        </span>
      </button>

      <div className="tk-topbar__meta">
        <span className={online ? "tk-runtime-pill is-online" : "tk-runtime-pill"}>
          <Server size={14} strokeWidth={1.8} aria-hidden="true" />
          {online ? "Local runtime" : "Offline"}
        </span>
      </div>
    </header>
  );
}

function workspaceDisplayName(path: string): string {
  if (!path || path === "Not selected") return "Choose workspace";
  const normalized = path.replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.at(-1) ?? path;
}
