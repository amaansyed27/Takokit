import type {
  SpeechApiRequest,
  SpeechApiResponse,
  TranscriptionApiRequest,
  TranscriptionApiResponse
} from "./types";
import { workspaceHeaders } from "./workspace";

const API_BASE = window.location.origin;

export function generateSessionSpeech(request: SpeechApiRequest): Promise<SpeechApiResponse> {
  return requestJson<SpeechApiResponse>("/v1/audio/speech", request);
}

export function transcribeSessionAudio(
  request: TranscriptionApiRequest
): Promise<TranscriptionApiResponse> {
  return requestJson<TranscriptionApiResponse>("/v1/audio/transcriptions", request);
}

async function requestJson<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    method: "POST",
    headers: workspaceHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify(body)
  });
  if (!response.ok) throw new Error(await responseError(response));
  return response.json() as Promise<T>;
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
    // Use status fallback.
  }
  return `Takokit API request failed with ${response.status}`;
}
