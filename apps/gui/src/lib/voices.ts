import type { VoiceProfile } from "./voiceTypes";
import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export type CreateVoiceProfileInput = {
  sample_path: string;
  name: string;
  model: string;
  consent_affirmed: boolean;
  consent_note?: string;
};

export async function createVoiceProfile(
  input: CreateVoiceProfileInput
): Promise<VoiceProfile> {
  const response = await fetch(`${API_BASE}/v1/voices/clone`, {
    method: "POST",
    headers: workspaceHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify(input)
  });
  if (!response.ok) throw new Error(await responseError(response));
  const body = (await response.json()) as { data: VoiceProfile };
  return body.data;
}

async function responseError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: { code?: string; message?: string } };
    if (body.error?.message) {
      return body.error.code
        ? `${body.error.code}: ${body.error.message}`
        : body.error.message;
    }
  } catch {
    // Use the status fallback.
  }
  return `Takokit API request failed with ${response.status}`;
}
