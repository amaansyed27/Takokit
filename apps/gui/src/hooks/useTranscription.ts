import { useState } from "react";
import { transcribeSessionAudio } from "../lib/sessionInference";
import type { TranscriptionApiResponse } from "../lib/types";

type TranscriptionInput = {
  model: string;
  filePath: string;
};

export function useTranscription() {
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [result, setResult] = useState<TranscriptionApiResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function transcribe(input: TranscriptionInput) {
    if (!input.filePath.trim()) {
      setError("Choose an audio file before transcribing.");
      return;
    }

    setIsTranscribing(true);
    setError(null);

    try {
      const response = await transcribeSessionAudio({
        model: input.model,
        file_path: input.filePath
      });
      setResult(response);
    } catch (caught) {
      setResult(null);
      setError(caught instanceof Error ? caught.message : "Transcription failed.");
    } finally {
      setIsTranscribing(false);
    }
  }

  function clearResult() {
    setResult(null);
    setError(null);
  }

  return { clearResult, error, isTranscribing, result, transcribe };
}
