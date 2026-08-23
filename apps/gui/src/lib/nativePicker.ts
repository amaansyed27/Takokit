import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

type PickerResponse = {
  path: string | null;
};

export async function pickAudioFile(): Promise<string | null> {
  return pickPath("/v1/system/picker/audio", "open audio picker");
}

export async function pickFolder(): Promise<string | null> {
  return pickPath("/v1/system/picker/folder", "open folder picker");
}

async function pickPath(path: string, operation: string): Promise<string | null> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: workspaceHeaders()
  });

  if (!response.ok) {
    try {
      const body = (await response.json()) as { error?: { message?: string } };
      if (body.error?.message) throw new Error(body.error.message);
    } catch (error) {
      if (error instanceof Error) throw error;
    }
    throw new Error(`${operation} failed with HTTP ${response.status}`);
  }

  const body = (await response.json()) as PickerResponse;
  return body.path?.trim() || null;
}
