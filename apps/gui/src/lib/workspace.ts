export type WorkspaceSource = "explicit" | "persisted" | "current_directory" | "safe_default" | "user_selected";

export type WorkspaceContext = {
  workspace?: string;
  session?: string;
  source?: WorkspaceSource;
};

const ACTIVE_WORKSPACE_KEY = "takokit.activeWorkspace";
const RECENT_WORKSPACES_KEY = "takokit.recentWorkspaces";

let currentContext: WorkspaceContext = initialContext();

export function getWorkspaceContext(): WorkspaceContext {
  return { ...currentContext };
}

export function workspaceNeedsSelection(): boolean {
  return !currentContext.workspace || currentContext.source === "safe_default";
}

export function getRecentWorkspaces(): string[] {
  try {
    const value = JSON.parse(window.localStorage.getItem(RECENT_WORKSPACES_KEY) ?? "[]") as unknown;
    return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return [];
  }
}

export function setWorkspaceContext(context: WorkspaceContext, updateUrl = true): void {
  const workspace = context.workspace?.trim() || undefined;
  currentContext = {
    workspace,
    session: context.session,
    source: context.source ?? (workspace ? "user_selected" : undefined)
  };

  if (workspace) {
    window.localStorage.setItem(ACTIVE_WORKSPACE_KEY, workspace);
    const recents = [workspace, ...getRecentWorkspaces().filter((item) => item !== workspace)].slice(0, 8);
    window.localStorage.setItem(RECENT_WORKSPACES_KEY, JSON.stringify(recents));
  }

  if (!updateUrl) return;
  const url = new URL(window.location.href);
  if (workspace) url.searchParams.set("workspace", workspace);
  else url.searchParams.delete("workspace");
  if (context.session) url.searchParams.set("session", context.session);
  else url.searchParams.delete("session");
  if (currentContext.source) url.searchParams.set("workspace_source", currentContext.source);
  else url.searchParams.delete("workspace_source");
  window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
}

export function selectWorkspace(workspace: string): void {
  const normalized = workspace.trim();
  if (!normalized) {
    throw new Error("Choose an absolute workspace path.");
  }
  setWorkspaceContext({ workspace: normalized, source: "user_selected" });
}

export function workspaceHeaders(headers?: HeadersInit): Headers {
  const result = new Headers(headers);
  if (currentContext.workspace) {
    result.set("X-Takokit-Workspace", encodeURIComponent(currentContext.workspace));
  }
  if (currentContext.session) {
    result.set("X-Takokit-Session", currentContext.session);
  }
  return result;
}

function initialContext(): WorkspaceContext {
  const url = contextFromUrl();
  if (url.workspace && url.source !== "safe_default") return url;

  const persisted = window.localStorage.getItem(ACTIVE_WORKSPACE_KEY)?.trim();
  if (persisted) {
    return { workspace: persisted, session: url.session, source: "persisted" };
  }
  return url;
}

function contextFromUrl(): WorkspaceContext {
  const parameters = new URLSearchParams(window.location.search);
  const source = parameters.get("workspace_source") as WorkspaceSource | null;
  return {
    workspace: parameters.get("workspace") || undefined,
    session: parameters.get("session") || undefined,
    source: source ?? undefined
  };
}
