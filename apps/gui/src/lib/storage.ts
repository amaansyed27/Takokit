import type { StorageOverview } from "./types";
import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export async function getStorageOverview(): Promise<StorageOverview> {
  const response = await fetch(`${API_BASE}/v1/system/storage`, {
    headers: workspaceHeaders()
  });
  if (!response.ok) throw new Error(await responseError(response));
  const body = (await response.json()) as { data: StorageOverview };
  return body.data;
}

export type OpenStorageTarget = "storage" | "workspace" | "logs" | "voices";

export async function openStorageLocation(target: OpenStorageTarget): Promise<void> {
  const response = await fetch(`${API_BASE}/v1/system/open`, {
    method: "POST",
    headers: workspaceHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify({ target })
  });
  if (!response.ok) throw new Error(await responseError(response));
}

async function responseError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: { code?: string; message?: string } };
    if (body.error?.message) {
      return body.error.code ? `${body.error.code}: ${body.error.message}` : body.error.message;
    }
  } catch {
    // Fall through to HTTP status.
  }
  return `Takokit API request failed with ${response.status}`;
}
