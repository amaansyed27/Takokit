export type WorkspaceContext = {
  workspace?: string;
  session?: string;
};

let currentContext: WorkspaceContext = contextFromUrl();

export function getWorkspaceContext(): WorkspaceContext {
  return { ...currentContext };
}

export function setWorkspaceContext(context: WorkspaceContext, updateUrl = true): void {
  currentContext = { ...context };
  if (!updateUrl) return;
  const url = new URL(window.location.href);
  if (context.workspace) url.searchParams.set("workspace", context.workspace);
  else url.searchParams.delete("workspace");
  if (context.session) url.searchParams.set("session", context.session);
  else url.searchParams.delete("session");
  window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
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

function contextFromUrl(): WorkspaceContext {
  const parameters = new URLSearchParams(window.location.search);
  return {
    workspace: parameters.get("workspace") || undefined,
    session: parameters.get("session") || undefined
  };
}
