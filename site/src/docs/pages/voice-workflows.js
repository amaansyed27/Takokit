const ADVANCED_RVC = {
  title: "Advanced RVC studio",
  intro: "Takokit includes a local RVC project workflow for owned/consented data: dataset curation, preflight, preparation, training, checkpoint/index management, testing, import/export, and package verification.",
  sections: [
    {
      id: "create-project",
      title: "Create a voice project",
      body: ["Create a named project and record the consent boundary before adding training data."],
      commands: ["tako voice rvc create --name \"My Voice\" --consent", "tako voice rvc list"],
    },
    {
      id: "samples",
      title: "Add and inspect samples",
      body: ["Use owned or explicitly permitted recordings. Dataset inspection and preflight happen before training."],
      commands: ["tako voice rvc samples my-voice add ./samples/*.wav", "tako voice rvc samples my-voice list", "tako voice rvc inspect my-voice", "tako voice rvc preflight my-voice --preset balanced"],
    },
    {
      id: "train",
      title: "Prepare and train",
      commands: ["tako voice rvc prepare my-voice --preset balanced", "tako voice rvc train my-voice --preset balanced", "tako voice rvc status my-voice", "tako voice rvc logs my-voice"],
      note: "Quick, balanced, and high-quality presets keep verified backend-owned values. Advanced epochs/batch/device/precision overrides require --preset custom.",
    },
    {
      id: "checkpoints",
      title: "Checkpoints and indexes",
      commands: ["tako voice rvc checkpoints my-voice", "tako voice rvc indexes my-voice"],
      body: ["Activate an explicit checkpoint/index pair before testing or packaging. A valid generated WAV proves execution, not perceptual similarity."],
    },
    {
      id: "import-export",
      title: "Import and export",
      body: ["Takokit can import an owned checkpoint/index pair, export a project package, verify a package, and import a verified package into another Takokit installation."],
      commands: ["tako voice rvc import ./voice.pth --index ./voice.index --name \"Imported Voice\" --consent", "tako voice rvc export my-voice ./my-voice.tako-rvc --sign", "tako voice rvc verify ./my-voice.tako-rvc", "tako voice rvc import-package ./my-voice.tako-rvc --consent"],
    },
    {
      id: "platform",
      title: "Training hardware",
      body: ["RVC inference and RVC training have different hardware boundaries. Windows and Linux CUDA are the primary accelerated paths; macOS RVC training is not advertised as a v0.3.0 acceptance target and no CUDA-to-MPS parity is claimed."],
      warning: "Do not interpret packaging support as proof that every training backend is supported on every operating system.",
    },
  ],
};

export const VOICE_WORKFLOW_DOCS = {
  "voice-cloning": {
    title: "Voice cloning",
    intro: "Create and reuse local voice profiles only from reference audio you own or have explicit permission to use.",
    sections: [
      {
        id: "reference",
        title: "Reference audio",
        body: ["Use a clean recording with minimal background noise and a single speaker. Model-specific pages describe reference-text, duration, language, and hardware requirements when known."],
      },
      {
        id: "consent",
        title: "Consent boundary",
        body: ["CLI, TUI, GUI, and API share the same consent requirement for cloning and training actions."],
        warning: "Do not clone another person's voice without ownership or explicit permission.",
      },
      {
        id: "create",
        title: "Create a profile",
        commands: ["tako clone ./reference.wav --name \"My Voice\" --model chatterbox --consent", "tako voice list"],
      },
      {
        id: "reuse",
        title: "Reuse a profile",
        body: ["Compatible models can reference the locally stored profile from Speak, the GUI, TUI, and Takokit-native API."],
        commands: ["tako speak \"Profile reuse test\" --model chatterbox --voice my-voice"],
      },
      {
        id: "quality",
        title: "Quality expectations",
        body: ["A successful output proves that the selected runtime completed. Similarity, naturalness, accent preservation, and robustness remain model- and recording-dependent and should be judged by listening."],
      },
    ],
  },
  "voice-conversion": {
    title: "Voice conversion",
    intro: "Convert an existing recording into a compatible target voice without treating conversion as TTS or reference cloning.",
    sections: [
      {
        id: "openvoice",
        title: "Reference-target conversion",
        body: ["OpenVoice-style conversion uses a source recording plus a permitted target reference/profile when the selected model supports that contract."],
        commands: ["tako convert ./source.wav --target-voice ./target-reference.wav --model openvoice --consent"],
      },
      {
        id: "rvc",
        title: "RVC conversion",
        body: ["RVC uses a compatible .pth checkpoint. A matching .index is recommended where the package supplies one."],
        commands: ["tako pull rvc", "tako convert ./source.wav --target-voice ./owned-voice.pth --model rvc --consent"],
      },
      {
        id: "controls",
        title: "RVC controls",
        body: ["The conversion command exposes F0 method, pitch shift, index rate, RMS mix rate, protect, and filter radius. Start with defaults and change one control at a time."],
        code: "--f0-method rmvpe --pitch-shift 0 --index-rate 0.75 --rms-mix-rate 0.25 --protect 0.33 --filter-radius 3",
      },
      {
        id: "review",
        title: "Review the output",
        body: ["A valid WAV proves execution only. Listen for intelligibility, target similarity, pitch artifacts, breath/noise behavior, and source leakage before accepting the result."],
      },
    ],
  },
  "advanced-rvc": ADVANCED_RVC,
  "rvc-packages": ADVANCED_RVC,
  "voice-profiles": {
    title: "Voice profiles",
    intro: "Profiles keep consent-backed reference material and reusable voice state local to Takokit.",
    sections: [
      {
        id: "state",
        title: "Shared local state",
        body: ["Compatible CLI, TUI, GUI, and API workflows see the same profiles under the Takokit global data root."],
        commands: ["tako voice list", "tako voice show chatterbox"],
      },
      {
        id: "add",
        title: "Add a reusable profile",
        commands: ["tako voice add ./reference.wav --name \"Narrator\" --model xtts-v2 --consent"],
      },
      {
        id: "data",
        title: "Local data",
        body: ["Voice references can be sensitive personal data. Back up, share, and delete ~/.takokit/voices intentionally."],
      },
    ],
  },
  "custom-models": {
    title: "Custom models",
    intro: "Register pinned custom checkpoints only when they extend a Takokit-supported runner contract.",
    sections: [
      {
        id: "boundary",
        title: "Runtime boundary",
        body: ["A custom manifest may describe pinned files, metadata, runner selection, and compatible adapter behavior. Takokit does not execute arbitrary repository scripts supplied by a model manifest."],
      },
      {
        id: "register",
        title: "Register and inspect",
        commands: ["tako custom-model add ./manifest.toml", "tako custom-model list", "tako custom-model show my-model"],
      },
      {
        id: "remove",
        title: "Remove registration",
        commands: ["tako custom-model rm my-model"],
        note: "Removal follows Takokit's storage/reference rules; inspect model/storage state before deleting shared artifacts manually.",
      },
    ],
  },
};
