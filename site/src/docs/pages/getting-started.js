export const GETTING_STARTED_DOCS = {
  install: {
    title: "Install Takokit",
    intro: "Takokit v0.1.0 is distributed for Windows through one verified installer.",
    sections: [
      {
        id: "prerequisites",
        title: "Prerequisites",
        body: [
          "Use Windows 10 or Windows 11 on x86_64. A compatible GPU driver is required only when the model you choose requires a GPU.",
        ],
      },
      {
        id: "install-windows",
        title: "Install on Windows",
        body: [
          "Run the PowerShell bootstrap, or use Download for Windows on the download page. Both resolve and verify the same canonical installer.",
        ],
        commands: [
          "irm https://takokit.dawnlightlabs.com/install.ps1 | iex",
        ],
      },
      {
        id: "verify-installation",
        title: "Verify the installation",
        body: ["Open a new terminal, verify the installed CLI, then launch either interface."],
        commands: ["tako version", "tako doctor", "tako", "tako gui"],
        note: "The GUI is served locally by Takokit and opens in your default browser. Linux and macOS packages are coming later.",
      },
    ],
  },
  "pull-first-model": {
    title: "Pull your first model",
    intro: "Pull resolves the model reference, required runner, adapter, and pinned artifacts.",
    sections: [
      {
        id: "choose-a-model",
        title: "Choose a model",
        body: ["Use the Models page to compare task, hardware, size, and verification status before downloading anything."],
      },
      {
        id: "pull-and-inspect",
        title: "Pull and inspect",
        body: ["Kokoro is a compact speech model and a practical first pull on a CPU-capable machine."],
        commands: ["tako pull kokoro", "tako show kokoro", "tako plan kokoro"],
      },
    ],
  },
  "generate-speech": {
    title: "Generate speech",
    intro: "Use the speak command with a model that declares text-to-speech capability.",
    sections: [
      {
        id: "prepare-the-model",
        title: "Prepare the model",
        body: ["Pull the model once. Takokit reuses the installed model and shared runtime on later runs."],
        commands: ["tako pull kokoro"],
      },
      {
        id: "generate-audio",
        title: "Generate audio",
        body: ["Pass the text as the first argument and select the model explicitly."],
        commands: ['tako speak "Hello from Takokit" --model kokoro'],
      },
    ],
  },
  "transcribe-audio": {
    title: "Transcribe audio",
    intro: "Use a speech-to-text model to turn a local recording into text.",
    sections: [
      {
        id: "prepare-whisper",
        title: "Prepare Whisper",
        body: ["Whisper Tiny is the currently documented lightweight transcription variant."],
        commands: ["tako pull whisper-tiny"],
      },
      {
        id: "transcribe-a-file",
        title: "Transcribe a file",
        body: ["Provide the path to the recording and the installed model reference."],
        commands: ["tako transcribe recording.wav --model whisper-tiny"],
      },
    ],
  },
};
