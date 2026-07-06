import type { SpeechApiRequest, SpeechApiResponse } from "./types";

const LOCAL_API_BASE_URL = "http://127.0.0.1:5050";

export async function generateSpeech(request: SpeechApiRequest): Promise<SpeechApiResponse> {
  const response = await fetch(`${LOCAL_API_BASE_URL}/v1/audio/speech`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });

  if (!response.ok) {
    throw new Error(`Speech generation failed with ${response.status}`);
  }

  return response.json() as Promise<SpeechApiResponse>;
}

export const apiConfig = {
  localBaseUrl: LOCAL_API_BASE_URL
};

