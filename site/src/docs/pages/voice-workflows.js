export const VOICE_WORKFLOW_DOCS = {
  "voice-cloning": {
    title: "Voice cloning",
    intro: "Create a reusable voice profile only from audio you own or have permission to use.",
    sections: [
      {
        id: "consent-boundary",
        title: "Consent boundary",
        body: ["The consent flag confirms that you own the voice or have explicit permission to create the profile."],
        note: "Do not create a profile from another person's voice without permission.",
      },
      {
        id: "create-a-profile",
        title: "Create a profile",
        body: ["Use a clear reference recording and give the local profile a recognizable name."],
        commands: ['tako clone reference.wav --name "My Voice" --model chatterbox --consent'],
      },
      {
        id: "inspect-profiles",
        title: "Inspect profiles",
        body: ["Voice profiles are stored locally and can be listed from the CLI."],
        commands: ["tako voice list", "tako voice show chatterbox"],
      },
    ],
  },
  "voice-conversion": {
    title: "Voice conversion",
    intro: "Convert an existing recording with a compatible target voice package.",
    sections: [
      {
        id: "prepare-the-target",
        title: "Prepare the target",
        body: ["RVC requires a compatible custom checkpoint. A matching index file is recommended where available."],
      },
      {
        id: "convert-a-recording",
        title: "Convert a recording",
        body: ["Confirm that you have permission to use both the source recording and target voice."],
        commands: ["tako convert source.wav --target-voice ./owned-voice.pth --model rvc --consent"],
      },
      {
        id: "review-the-output",
        title: "Review the output",
        body: ["A successful WAV proves that the runtime executed. It does not prove perceptual similarity or production quality."],
      },
    ],
  },
  "voice-profiles": {
    title: "Voice profiles",
    intro: "Profiles keep consent-backed reference audio and reusable voice state local.",
    sections: [
      {
        id: "local-profile-state",
        title: "Local profile state",
        body: ["Profiles are shared by compatible CLI, TUI, GUI, and API workflows through Takokit's local state."],
      },
      {
        id: "inspect-a-profile",
        title: "Inspect a profile",
        commands: ["tako voice list", "tako voice show chatterbox"],
      },
    ],
  },
  "custom-models": {
    title: "Custom models",
    intro: "Register pinned custom manifests that extend a supported generic runner contract.",
    sections: [
      {
        id: "manifest-boundary",
        title: "Manifest boundary",
        body: ["Takokit does not execute arbitrary model repository scripts. Custom manifests must describe a supported runtime contract."],
      },
      {
        id: "register-a-model",
        title: "Register a model",
        commands: ["tako custom-model add manifest.toml", "tako custom-model list"],
      },
    ],
  },
  "rvc-packages": {
    title: "RVC packages",
    intro: "RVC is a voice-conversion runtime that requires a compatible custom checkpoint.",
    sections: [
      {
        id: "required-files",
        title: "Required files",
        body: ["A compatible .pth checkpoint is required. A matching .index file is recommended where available."],
      },
      {
        id: "model-policy",
        title: "Model policy",
        body: ["Takokit does not ship celebrity or public-figure impersonation checkpoints."],
      },
      {
        id: "quality-boundary",
        title: "Quality boundary",
        body: ["A successful conversion proves execution, not perceptual similarity. Listen to and review the result."],
        commands: ["tako pull rvc", "tako convert source.wav --target-voice ./owned-voice.pth --model rvc --consent"],
        note: "Custom RVC creation and training is planned separately under Issue #68.",
      },
    ],
  },
};
