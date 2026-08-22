import { useState } from "react";
import { generateSessionSpeech } from "../lib/sessionInference";
import type { SpeechApiRequest, SpeechApiResponse } from "../lib/types";

export function useSpeechGeneration() {
  const [isGenerating, setIsGenerating] = useState(false);
  const [result, setResult] = useState<SpeechApiResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function generate(request: SpeechApiRequest) {
    if (!request.input.trim()) {
      setError("Enter some text before generating speech.");
      return;
    }

    setIsGenerating(true);
    setError(null);

    try {
      const response = await generateSessionSpeech({
        ...request,
        response_format: request.response_format ?? "wav"
      });
      setResult(response);
    } catch (caught) {
      setResult(null);
      setError(caught instanceof Error ? caught.message : "Speech generation failed.");
    } finally {
      setIsGenerating(false);
    }
  }

  function clearResult() {
    setResult(null);
    setError(null);
  }

  return { error, generate, isGenerating, result, clearResult };
}
