import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export type WorkspaceFile = {
  id: string;
  name: string;
  path: string;
  kind: "audio" | "text";
  content_type: string;
  bytes: number;
  modified_at: number;
};

export async function listWorkspaceFiles(): Promise<WorkspaceFile[]> {
  const response = await fetch(`${API_BASE}/api/v1/files`, {
    headers: workspaceHeaders()
  });
  if (!response.ok) throw new Error(await responseError(response));
  const body = (await response.json()) as { files: WorkspaceFile[] };
  return body.files;
}

export async function uploadWorkspaceFile(file: Blob, name: string): Promise<WorkspaceFile> {
  const query = new URLSearchParams({ name });
  const response = await fetch(`${API_BASE}/api/v1/files?${query.toString()}`, {
    method: "POST",
    headers: workspaceHeaders({
      "Content-Type": file.type || "application/octet-stream"
    }),
    body: file
  });
  if (!response.ok) throw new Error(await responseError(response));
  return (await response.json()) as WorkspaceFile;
}

export async function deleteWorkspaceFile(id: string): Promise<void> {
  const response = await fetch(`${API_BASE}/api/v1/files/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: workspaceHeaders()
  });
  if (!response.ok) throw new Error(await responseError(response));
}

export async function loadWorkspaceText(file: WorkspaceFile): Promise<string> {
  if (file.kind !== "text") throw new Error("Only text files can be loaded as text.");
  const response = await fetch(`${API_BASE}/api/v1/files/${encodeURIComponent(file.id)}/content`, {
    headers: workspaceHeaders()
  });
  if (!response.ok) throw new Error(await responseError(response));
  return response.text();
}

async function responseError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: { code?: string; message?: string } };
    if (body.error?.message) {
      return body.error.code ? `${body.error.code}: ${body.error.message}` : body.error.message;
    }
  } catch {
    // Use the status fallback.
  }
  return `Takokit file request failed with ${response.status}`;
}
