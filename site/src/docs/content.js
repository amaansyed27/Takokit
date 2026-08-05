export const DOC_GROUPS = [
  {
    title: "Getting started",
    pages: [
      ["install", "Install Takokit"],
      ["pull-first-model", "Pull your first model"],
      ["generate-speech", "Generate speech"],
      ["transcribe-audio", "Transcribe audio"],
    ],
  },
  {
    title: "Voice workflows",
    pages: [
      ["voice-cloning", "Voice cloning"],
      ["voice-conversion", "Voice conversion"],
      ["voice-profiles", "Voice profiles"],
      ["custom-models", "Custom models"],
      ["rvc-packages", "RVC packages"],
    ],
  },
  {
    title: "Developers",
    pages: [
      ["local-api", "Local API"],
      ["python-http", "Python HTTP examples"],
      ["javascript-http", "JavaScript HTTP examples"],
      ["model-references", "Model references and tags"],
      ["registry-api", "Registry API"],
    ],
  },
  {
    title: "Manage Takokit",
    pages: [
      ["models-storage", "Models and storage"],
      ["hardware", "Hardware"],
      ["daemon", "Daemon"],
      ["logs-diagnostics", "Logs and diagnostics"],
      ["reset-uninstall", "Reset and uninstall"],
      ["troubleshooting", "Troubleshooting"],
    ],
  },
];

export const DOCS = {
  install: {
    title: "Install Takokit",
    intro: "Takokit is currently source-distributed while Windows-first packaging is prepared.",
    sections: [
      {
        id: "prerequisites",
        title: "Prerequisites",
        body: [
          "Install Git, Rust stable, Node.js LTS, and npm. A compatible GPU driver is required only when the model you choose requires a GPU.",
        ],
      },
      {
        id: "build-from-source",
        title: "Build from source",
        body: [
          "Build the React GUI assets first, then build the locked Rust workspace from the repository root.",
        ],
        commands: [
          "npm ci --prefix apps/gui",
          "npm run build --prefix apps/gui",
          "cargo build --release --locked",
        ],
      },
      {
        id: "verify-installation",
        title: "Verify the installation",
        body: ["Run the generated binary directly before adding it to PATH."],
        commands: [".\\target\\release\\tako.exe version", ".\\target\\release\\tako.exe doctor"],
        note: "Signed installers and package-manager commands will be documented only after real release artifacts exist.",
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
  "local-api": {
    title: "Local API",
    intro: "The managed daemon serves Takokit's local HTTP API at 127.0.0.1:5050 by default.",
    sections: [
      {
        id: "start-the-daemon",
        title: "Start the daemon",
        commands: ["tako daemon start", "tako daemon status"],
      },
      {
        id: "base-url",
        title: "Base URL",
        body: ["The API is local by default. Health and versioned runtime routes are served from the same process."],
        commands: ["curl http://127.0.0.1:5050/health"],
      },
      {
        id: "speech-request",
        title: "Speech request",
        code: `curl -X POST http://127.0.0.1:5050/v1/audio/speech \\
  -H "Content-Type: application/json" \\
  -d '{"model":"kokoro","input":"Hello from Takokit","voice":"default","response_format":"wav"}'`,
      },
    ],
  },
  "python-http": {
    title: "Python HTTP examples",
    intro: "Takokit does not currently claim an official Python SDK. Use a standard HTTP client.",
    sections: [
      {
        id: "install-the-client",
        title: "Install the client",
        commands: ["python -m pip install requests"],
      },
      {
        id: "send-a-speech-request",
        title: "Send a speech request",
        code: `import requests

response = requests.post(
    "http://127.0.0.1:5050/v1/audio/speech",
    json={
        "model": "kokoro",
        "input": "Hello from Takokit",
        "voice": "default",
        "response_format": "wav",
    },
    timeout=300,
)
response.raise_for_status()
print(response.json())`,
      },
    ],
  },
  "javascript-http": {
    title: "JavaScript HTTP examples",
    intro: "Takokit does not currently claim an official JavaScript SDK. Use fetch or another HTTP client.",
    sections: [
      {
        id: "transcription-request",
        title: "Send a transcription request",
        code: `const response = await fetch("http://127.0.0.1:5050/v1/audio/transcriptions", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    file_path: "recording.wav",
    model: "whisper-tiny",
  }),
});

if (!response.ok) throw new Error(await response.text());
console.log(await response.json());`,
      },
    ],
  },
  "model-references": {
    title: "Model references and tags",
    intro: "An omitted tag resolves to Takokit's declared default; an explicit tag selects a registry variant.",
    sections: [
      {
        id: "default-references",
        title: "Default references",
        body: ["Use the family name when the declared default variant is appropriate."],
        commands: ["tako pull kokoro", "tako pull whisper-tiny"],
      },
      {
        id: "explicit-tags",
        title: "Explicit tags",
        body: ["Use an explicit tag when you need a particular size or capability variant."],
        commands: ["tako pull whisper:small", "tako library show qwen3-tts:0.6b-base"],
      },
    ],
  },
  "registry-api": {
    title: "Registry API",
    intro: "The companion site exposes the canonical model registry through a public JSON endpoint.",
    sections: [
      {
        id: "endpoint",
        title: "Endpoint",
        commands: ["curl https://takokit-library.vercel.app/v1/registry.json"],
      },
      {
        id: "source-of-truth",
        title: "Source of truth",
        body: ["The endpoint follows the registry revision deployed with the site. The React application does not maintain a second hard-coded catalog."],
      },
    ],
  },
  "models-storage": {
    title: "Models and storage",
    intro: "Reusable runtime files and project-specific output history use separate local locations.",
    sections: [
      {
        id: "shared-runtime-data",
        title: "Shared runtime data",
        body: ["Models, runners, manifests, voices, cache data, and logs live under ~/.takokit."],
        code: `~/.takokit/
  models/
  runners/
  blobs/
  cache/
  manifests/
  voices/
  logs/`,
      },
      {
        id: "project-data",
        title: "Project data",
        body: ["Sessions and outputs belong to the project that launched the workflow."],
        code: `project/.tako/sessions/`,
      },
    ],
  },
  hardware: {
    title: "Hardware",
    intro: "Check the selected model's declared requirements before pulling it.",
    sections: [
      {
        id: "inspect-requirements",
        title: "Inspect requirements",
        body: ["Model pages present CPU/GPU support, RAM, VRAM, and approximate download size when the registry declares them."],
        commands: ["tako doctor", "tako plan qwen3-tts:0.6b-base"],
      },
      {
        id: "unknown-values",
        title: "Unknown values",
        body: ["Unknown requirements are shown as Not declared rather than inferred or displayed as zero."],
      },
    ],
  },
  daemon: {
    title: "Daemon",
    intro: "CLI, TUI, GUI, and API workflows share the managed daemon and one local registry state.",
    sections: [
      {
        id: "lifecycle",
        title: "Lifecycle",
        commands: ["tako daemon start", "tako daemon status", "tako daemon restart", "tako daemon stop"],
      },
      {
        id: "logs",
        title: "Daemon logs",
        commands: ["tako daemon logs"],
      },
    ],
  },
  "logs-diagnostics": {
    title: "Logs and diagnostics",
    intro: "Use Takokit's diagnostics before changing model environments manually.",
    sections: [
      {
        id: "doctor",
        title: "Run doctor",
        commands: ["tako doctor", "tako doctor --json"],
      },
      {
        id: "inspect-logs",
        title: "Inspect daemon logs",
        commands: ["tako daemon logs"],
      },
    ],
  },
  "reset-uninstall": {
    title: "Reset and uninstall",
    intro: "Remove individual models safely; full installer-backed uninstall instructions will arrive with release packages.",
    sections: [
      {
        id: "remove-a-model",
        title: "Remove a model",
        commands: ["tako rm whisper-tiny", "tako rm whisper-tiny --dry-run"],
      },
      {
        id: "clean-runtime-cache",
        title: "Clean runtime cache",
        commands: ["tako storage clean --dry-run", "tako storage clean"],
      },
      {
        id: "full-uninstall",
        title: "Full uninstall",
        body: ["A supported full-uninstall flow will be documented after packaged installers exist."],
      },
    ],
  },
  troubleshooting: {
    title: "Troubleshooting",
    intro: "Start with the model plan, doctor output, and daemon logs.",
    sections: [
      {
        id: "inspect-the-plan",
        title: "Inspect the plan",
        body: ["The plan explains the selected model, runner, hardware boundary, and next command."],
        commands: ["tako plan MODEL"],
      },
      {
        id: "run-diagnostics",
        title: "Run diagnostics",
        commands: ["tako doctor", "tako daemon logs"],
      },
      {
        id: "status-boundary",
        title: "Status boundary",
        body: ["Do not treat a metadata-only or hardware-blocked model as ready to execute."],
      },
    ],
  },
};

export const DOC_ORDER = DOC_GROUPS.flatMap((group) =>
  group.pages.map(([id, title]) => ({ id, title, group: group.title })),
);

export function findDoc(slug) {
  return DOCS[slug] || null;
}

export function findDocGroup(slug) {
  return DOC_GROUPS.find((group) => group.pages.some(([id]) => id === slug)) || null;
}

export function adjacentDocs(slug) {
  const index = DOC_ORDER.findIndex((item) => item.id === slug);
  return {
    previous: index > 0 ? DOC_ORDER[index - 1] : null,
    next: index >= 0 && index < DOC_ORDER.length - 1 ? DOC_ORDER[index + 1] : null,
  };
}
