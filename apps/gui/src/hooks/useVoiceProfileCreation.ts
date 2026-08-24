import { createVoiceProfile, type CreateVoiceProfileInput } from "../lib/voices";
import type { VoiceProfile } from "../lib/voiceTypes";
import { createWorkflowStore, useWorkflowStore } from "../lib/workflowState";

const voiceProfileWorkflow = createWorkflowStore<VoiceProfile>();

export function useVoiceProfileCreation() {
  const state = useWorkflowStore(voiceProfileWorkflow);

  async function create(input: CreateVoiceProfileInput): Promise<VoiceProfile | null> {
    if (!voiceProfileWorkflow.start()) return null;
    try {
      const profile = await createVoiceProfile(input);
      voiceProfileWorkflow.succeed(profile);
      return profile;
    } catch (caught) {
      voiceProfileWorkflow.fail(caught instanceof Error ? caught.message : "Voice creation failed.");
      return null;
    }
  }

  return {
    clearResult: voiceProfileWorkflow.clear,
    create,
    error: state.error,
    isCreating: state.running,
    result: state.result
  };
}
