import { generateSessionSpeech } from "../lib/sessionInference";
import type { SpeechApiRequest, SpeechApiResponse } from "../lib/types";
import { createWorkflowStore, useWorkflowStore } from "../lib/workflowState";

const speechWorkflow = createWorkflowStore<SpeechApiResponse>();

export function useSpeechGeneration() {
  const state = useWorkflowStore(speechWorkflow);

  async function generate(request: SpeechApiRequest) {
    if (!request.input.trim()) {
      speechWorkflow.fail("Enter some text before generating speech.");
      return;
    }
    if (!speechWorkflow.start()) return;

    try {
      const response = await generateSessionSpeech({
        ...request,
        response_format: request.response_format ?? "wav"
      });
      speechWorkflow.succeed(response);
    } catch (caught) {
      speechWorkflow.fail(caught instanceof Error ? caught.message : "Speech generation failed.");
    }
  }

  return {
    error: state.error,
    generate,
    isGenerating: state.running,
    result: state.result,
    clearResult: speechWorkflow.clear
  };
}
