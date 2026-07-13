import type { SessionRecord, SessionSummary } from "./types";
import { getWorkspaceContext, setWorkspaceContext, workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export async function initializeWorkspaceSession(): Promise<SessionRecord | null> {
  const context = getWorkspaceContext();
  if (!context.workspace) return null;
  const record = await openSession(context.workspace, context.session);
  setWorkspaceContext({ workspace: record.summary.workspace_root, session: record.summary.id });
  return record;
}

export async function createSession(title?: string): Promise<SessionRecord> {
  const context = getWorkspaceContext();
  if (!context.workspace) {
    throw new Error("The GUI was not launched with a Takokit project workspace.");
  }
  const record = await openSession(context.workspace, undefined, title);
  setWorkspaceContext({ workspace: record.summary.workspace_root, session: record.summary.id });
  return record;
}

export async function resumeSession(id: string): Promise<SessionRecord> {
  const context = getWorkspaceContext();
  if (!context.workspace) {
    throw new Error("The GUI was not launched with a Takokit project workspace.");
  }
  const record = await openSession(context.workspace, id);
  setWorkspaceContext({ workspace: record.summary.workspace_root, session: record.summary.id });
  return record;
}

export async function listSessions(query = ""): Promise<SessionSummary[]> {
  const suffix = query.trim() ? `?q=${encodeURIComponent(query.trim())}` : "";
  const response = await fetch(`${API_BASE}/v1/sessions${suffix}`, {
    headers: workspaceHeaders()
  });
  const body = await expectJson<{ data: SessionSummary[] }>(response);
  return body.data;
}

export async function getSession(id: string): Promise<SessionRecord> {
  const response = await fetch(`${API_BASE}/v1/sessions/${encodeURIComponent(id)}`, {
    headers: workspaceHeaders()
  });
  const body = await expectJson<{ data: SessionRecord }>(response);
  return body.data;
}

export async function removeSession(id: string): Promise<boolean> {
  const response = await fetch(`${API_BASE}/v1/sessions/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: workspaceHeaders()
  });
  const body = await expectJson<{ id: string; removed: boolean }>(response);
  return body.removed;
}

export async function loadSessionOutput(id: string, outputPath: string): Promise<string> {
  const filename = outputFilename(outputPath);
  const response = await fetch(
    `${API_BASE}/v1/sessions/${encodeURIComponent(id)}/outputs/${encodeURIComponent(filename)}`,
    { headers: workspaceHeaders() }
  );
  if (!response.ok) throw new Error(await responseError(response));
  return URL.createObjectURL(await response.blob());
}

export function outputFilename(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const components = normalized.split("/").filter(Boolean);
  return components.length > 0 ? components[components.length - 1] : "output";
}

async function openSession(workspace: string, sessionId?: string, title?: string): Promise<SessionRecord> {
  const response = await fetch(`${API_BASE}/v1/sessions/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      workspace,
      session_id: sessionId,
      title
    })
  });
  const body = await expectJson<{ data: SessionRecord }>(response);
  return body.data;
}

async function expectJson<T>(response: Response): Promise<T> {
  if (!response.ok) throw new Error(await responseError(response));
  return response.json() as Promise<T>;
}

async function responseError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: { message?: string } };
    if (body.error?.message) return body.error.message;
  } catch {
    // Use the status fallback below.
  }
  return `Takokit API request failed with ${response.status}`;
}
