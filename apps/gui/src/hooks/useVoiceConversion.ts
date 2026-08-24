import { convertSessionVoice } from "../lib/sessionInference";
import type { VoiceConversionApiRequest, VoiceConversionApiResponse } from "../lib/types";
import { createWorkflowStore, useWorkflowStore } from "../lib/workflowState";

const cloningWorkflow = createWorkflowStore<VoiceConversionApiResponse>();

export function useVoiceConversion() {
  const state = useWorkflowStore(cloningWorkflow);

  async function convert(request: VoiceConversionApiRequest): Promise<VoiceConversionApiResponse | null> {
    if (!cloningWorkflow.start()) return null;
    try {
      const response = await convertSessionVoice(request);
      cloningWorkflow.succeed(response);
      return response;
    } catch (caught) {
      cloningWorkflow.fail(caught instanceof Error ? caught.message : "Voice cloning failed.");
      return null;
    }
  }

  return {
    clearResult: cloningWorkflow.clear,
    convert,
    error: state.error,
    isConverting: state.running,
    result: state.result
  };
}
