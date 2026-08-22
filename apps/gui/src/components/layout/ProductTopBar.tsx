import { FolderOpen, Moon, Server, Sun } from "lucide-react";
import { useTheme } from "../../hooks/useTheme";
import type { RuntimeSnapshot } from "../../lib/types";

type ProductTopBarProps = {
  runtime: RuntimeSnapshot;
  onChangeWorkspace: () => void;
};

export function ProductTopBar({ runtime, onChangeWorkspace }: ProductTopBarProps) {
  const online = runtime.server.status === "online";
  const workspaceName = workspaceDisplayName(runtime.workspacePath);
  const { theme, toggleTheme } = useTheme();

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
        <button
          className="tk-icon-button"
          type="button"
          onClick={toggleTheme}
          aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          title={theme === "dark" ? "Light mode" : "Dark mode"}
        >
          {theme === "dark" ? <Sun size={16} strokeWidth={1.8} /> : <Moon size={16} strokeWidth={1.8} />}
        </button>
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
  return parts.length > 0 ? parts[parts.length - 1] : path;
}
