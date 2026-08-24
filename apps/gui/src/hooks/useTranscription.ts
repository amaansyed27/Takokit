import { transcribeSessionAudio } from "../lib/sessionInference";
import type { TranscriptionApiResponse } from "../lib/types";
import { createWorkflowStore, useWorkflowStore } from "../lib/workflowState";

type TranscriptionInput = {
  model: string;
  filePath: string;
};

const transcriptionWorkflow = createWorkflowStore<TranscriptionApiResponse>();

export function useTranscription() {
  const state = useWorkflowStore(transcriptionWorkflow);

  async function transcribe(input: TranscriptionInput) {
    if (!input.filePath.trim()) {
      transcriptionWorkflow.fail("Choose an audio file before transcribing.");
      return;
    }
    if (!transcriptionWorkflow.start()) return;

    try {
      const response = await transcribeSessionAudio({
        model: input.model,
        file_path: input.filePath
      });
      transcriptionWorkflow.succeed(response);
    } catch (caught) {
      transcriptionWorkflow.fail(caught instanceof Error ? caught.message : "Transcription failed.");
    }
  }

  return {
    clearResult: transcriptionWorkflow.clear,
    error: state.error,
    isTranscribing: state.running,
    result: state.result,
    transcribe
  };
}
