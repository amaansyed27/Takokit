import { useState } from "react";
import { generateSpeech } from "../lib/api";

type GenerateInput = {
  model: string;
  voice: string;
  input: string;
};

export function useMockGeneration() {
  const [isGenerating, setIsGenerating] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function generate(input: GenerateInput) {
    if (!input.input.trim()) {
      setError("Enter text before generating speech.");
      return;
    }

    setIsGenerating(true);
    setError(null);

    try {
      const response = await generateSpeech({
        model: input.model,
        voice: input.voice,
        input: input.input,
        response_format: "wav"
      });
      setResult(response?.output_path ?? "Local output prepared.");
    } catch {
      setResult("Local API unavailable; no inference was run.");
    } finally {
      setIsGenerating(false);
    }
  }

  return { error, generate, isGenerating, result };
}
