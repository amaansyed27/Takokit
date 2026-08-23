import { useState } from "react";
import { convertSessionVoice } from "../lib/sessionInference";
import type { VoiceConversionApiRequest, VoiceConversionApiResponse } from "../lib/types";

export function useVoiceConversion() {
  const [isConverting, setIsConverting] = useState(false);
  const [result, setResult] = useState<VoiceConversionApiResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function convert(request: VoiceConversionApiRequest): Promise<VoiceConversionApiResponse | null> {
    if (isConverting) return null;
    setIsConverting(true);
    setError(null);
    setResult(null);
    try {
      const response = await convertSessionVoice(request);
      setResult(response);
      return response;
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Voice conversion failed.");
      return null;
    } finally {
      setIsConverting(false);
    }
  }

  function clearResult() {
    setResult(null);
    setError(null);
  }

  return { clearResult, convert, error, isConverting, result };
}
