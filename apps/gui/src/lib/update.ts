import { apiConfig } from "./api";
import { workspaceHeaders } from "./workspace";

export type UpdateStatus = {
  current_version: string;
  available_version?: string | null;
  downloaded_version?: string | null;
  channel: "stable" | "preview";
  distribution_mode: string;
  manifest_source?: string | null;
  automatic_checks: boolean;
  automatic_download: boolean;
  last_check_unix?: number | null;
  last_error?: string | null;
  journal?: Record<string, unknown> | null;
};

export type UpdateCheck = {
  current_version: string;
  offered_version: string;
  channel: string;
  available: boolean;
  manifest_source: string;
  signing_key_id: string;
  artifact?: string | null;
  test_fixture: boolean;
};

export async function getUpdateStatus(): Promise<UpdateStatus> {
  return request<UpdateStatus>("/api/v1/system/update");
}

export async function checkForUpdates(): Promise<UpdateCheck> {
  return request<UpdateCheck>("/api/v1/system/update/check", { method: "POST" });
}

export async function applyUpdate(): Promise<{ accepted: boolean; message: string }> {
  return request<{ accepted: boolean; message: string }>("/api/v1/system/update/apply", {
    method: "POST"
  });
}

export async function configureUpdates(settings: {
  channel?: "stable" | "preview";
  automatic_checks?: boolean;
  automatic_download?: boolean;
}): Promise<UpdateStatus> {
  return request<UpdateStatus>("/api/v1/system/update/settings", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(settings)
  });
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiConfig.localBaseUrl}${path}`, {
    ...init,
    headers: workspaceHeaders(init?.headers)
  });
  if (response.ok) return response.json() as Promise<T>;

  let message = response.statusText || "Update request failed";
  try {
    const body = (await response.json()) as { error?: { message?: string } };
    if (body.error?.message) message = body.error.message;
  } catch {
    // Keep the HTTP status text when an unexpected non-JSON response is returned.
  }
  throw new Error(message);
}
