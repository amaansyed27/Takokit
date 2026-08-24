import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export async function loadLocalAudio(path: string): Promise<string> {
  const value = path.trim();
  if (!value) throw new Error("Choose an audio file first.");

  const query = new URLSearchParams({ path: value });
  const response = await fetch(`${API_BASE}/v1/system/audio?${query.toString()}`, {
    headers: workspaceHeaders()
  });

  if (!response.ok) {
    try {
      const body = (await response.json()) as { error?: { message?: string } };
      if (body.error?.message) throw new Error(body.error.message);
    } catch (error) {
      if (error instanceof Error) throw error;
    }
    throw new Error(`Audio preview failed with HTTP ${response.status}`);
  }

  const blob = await response.blob();
  return URL.createObjectURL(blob);
}
