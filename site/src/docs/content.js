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
      ["Prerequisites", "Install Git, Rust stable, Node.js LTS, and npm. A compatible GPU driver is required only when the selected model requires a GPU."],
      ["Build", "Build the React GUI assets first, then build the locked Rust workspace from the repository root."],
    ],
    commands: [
      "npm ci --prefix apps/gui",
      "npm run build --prefix apps/gui",
      "cargo build --release --locked",
      ".\\target\\release\\tako.exe doctor",
    ],
  },
  "pull-first-model": {
    title: "Pull your first model",
    intro: "Pull resolves the model reference, required runner, adapter, and pinned artifacts.",
    sections: [["Start with Kokoro", "Kokoro is a compact CPU-friendly speech model and a practical first pull."]],
    commands: ["tako pull kokoro", "tako show kokoro", "tako plan kokoro"],
  },
  "generate-speech": {
    title: "Generate speech",
    intro: "Use the speak command with a model that declares text-to-speech capability.",
    commands: ['tako speak "Hello from Takokit" --model kokoro'],
  },
  "transcribe-audio": {
    title: "Transcribe audio",
    intro: "Whisper Tiny is the currently documented locally verified transcription variant.",
    commands: ["tako pull whisper-tiny", "tako transcribe recording.wav --model whisper-tiny"],
  },
  "voice-cloning": {
    title: "Voice cloning",
    intro: "Create a reusable voice profile only from audio you own or have permission to use.",
    commands: ['tako clone reference.wav --name "My Voice" --model chatterbox --consent', "tako list voices"],
  },
  "voice-conversion": {
    title: "Voice conversion",
    intro: "Conversion changes an existing recording using a compatible target voice package.",
    commands: ["tako convert source.wav --target-voice ./owned-voice.pth --model rvc --consent"],
  },
  "voice-profiles": {
    title: "Voice profiles",
    intro: "Profiles store consent-backed reference audio locally and can be reused by compatible models.",
    commands: ["tako voice list", "tako voice show chatterbox"],
  },
  "custom-models": {
    title: "Custom models",
    intro: "Takokit accepts pinned custom manifests only when they extend a verified generic runner contract. Arbitrary model scripts are not executed.",
    commands: ["tako custom-model add manifest.toml", "tako custom-model list"],
  },
  "rvc-packages": {
    title: "RVC packages",
    intro: "RVC requires a compatible custom .pth checkpoint. A matching .index file is recommended.",
    sections: [
      ["Quality boundary", "A successful conversion proves execution, not perceptual similarity. Listen to the result before treating it as useful."],
      ["Model policy", "Takokit does not ship celebrity or public-figure impersonation checkpoints."],
    ],
    commands: ["tako pull rvc", "tako convert source.wav --target-voice ./owned-voice.pth --model rvc --consent"],
  },
  "local-api": {
    title: "Local API",
    intro: "The managed daemon serves the local API at http://127.0.0.1:5050 by default.",
    commands: ["tako daemon start", "curl http://127.0.0.1:5050/health"],
  },
  "python-http": {
    title: "Python HTTP examples",
    intro: "Takokit does not currently claim an official Python SDK. Use a standard HTTP client.",
    code: `import requests\n\nresponse = requests.post(\n    "http://127.0.0.1:5050/v1/audio/speech",\n    json={\n        "model": "kokoro",\n        "input": "Hello from Takokit",\n        "voice": "default",\n        "response_format": "wav",\n    },\n    timeout=300,\n)\nresponse.raise_for_status()\nprint(response.json())`,
  },
  "javascript-http": {
    title: "JavaScript HTTP examples",
    intro: "Takokit does not currently claim an official JavaScript SDK. Use fetch or another HTTP client.",
    code: `const response = await fetch("http://127.0.0.1:5050/v1/audio/transcriptions", {\n  method: "POST",\n  headers: { "Content-Type": "application/json" },\n  body: JSON.stringify({ file_path: "recording.wav", model: "whisper-tiny" }),\n});\nif (!response.ok) throw new Error(await response.text());\nconsole.log(await response.json());`,
  },
  "model-references": {
    title: "Model references and tags",
    intro: "An omitted tag resolves to Takokit's tested default. Explicit tags select immutable registry variants.",
    commands: ["tako pull whisper-tiny", "tako pull whisper:small", "tako library show qwen3-tts:0.6b-base"],
  },
  "registry-api": {
    title: "Registry API",
    intro: "The public companion site exposes the canonical registry through /v1/registry.json.",
    commands: ["curl https://takokit-library.vercel.app/v1/registry.json"],
  },
  "models-storage": {
    title: "Models and storage",
    intro: "Reusable runtime files live under ~/.takokit. Project outputs and history live under the launching project's .tako directory.",
    code: `~/.takokit/\n  models/\n  runners/\n  blobs/\n  cache/\n  manifests/\n  voices/\n  logs/\n\nproject/.tako/sessions/`,
  },
  hardware: {
    title: "Hardware",
    intro: "Check each model page before pulling. Unknown requirements remain labelled Not declared rather than inferred.",
    commands: ["tako doctor", "tako plan qwen3-tts:0.6b-base"],
  },
  daemon: {
    title: "Daemon",
    intro: "CLI, TUI, GUI, and API share the managed daemon and one local registry state.",
    commands: ["tako daemon start", "tako daemon status", "tako daemon restart", "tako daemon stop"],
  },
  "logs-diagnostics": {
    title: "Logs and diagnostics",
    intro: "Use doctor and daemon logs before changing local model environments manually.",
    commands: ["tako doctor", "tako daemon logs"],
  },
  "reset-uninstall": {
    title: "Reset and uninstall",
    intro: "Remove individual models with the CLI. Full installer-backed uninstall instructions will be published with real packages.",
    commands: ["tako rm whisper-tiny", "tako storage clean --dry-run"],
  },
  troubleshooting: {
    title: "Troubleshooting",
    intro: "Start with the model plan, doctor output, and daemon logs. Do not report a metadata-only or hardware-blocked model as ready.",
    commands: ["tako plan MODEL", "tako doctor", "tako daemon logs"],
  },
};

export function findDoc(slug) {
  return DOCS[slug] || null;
}
