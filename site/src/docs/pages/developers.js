const OPENAI = "http://127.0.0.1:5050/v1";
const NATIVE = "http://127.0.0.1:5050/api/v1";

export const DEVELOPER_DOCS = {
  "local-api": {
    title: "Local API introduction",
    intro: "Takokit exposes one local runtime through an OpenAI-compatible audio subset and a separate Takokit-native API. It does not claim general OpenAI API compatibility.",
    sections: [
      {
        id: "run",
        title: "Run the server",
        commands: ["tako serve", "tako server start", "tako server status", "tako server logs"],
        note: "`tako serve` is foreground/developer-owned. `tako server start` uses the managed lifecycle.",
      },
      {
        id: "urls",
        title: "URLs",
        code: `${OPENAI}      OpenAI-compatible audio API
${NATIVE}  Takokit-native API
http://127.0.0.1:5050/gui
http://127.0.0.1:5050/openapi.json`,
      },
      {
        id: "compatibility",
        title: "OpenAI compatibility matrix",
        code: `Models                 Supported
Audio speech           Supported
Audio transcription    Supported
Chat completions       Not supported
Responses              Not supported
Embeddings             Not supported
Images                  Not supported`,
        warning: "Use the phrase OpenAI-compatible audio API. Do not describe Takokit as a general OpenAI-compatible server.",
      },
      {
        id: "openapi",
        title: "Machine-readable contract",
        body: ["Use `/openapi.json` as the current machine-readable local API contract. Release-facing docs and compatibility tests are checked against the executable router."],
      },
    ],
  },
  "openai-models": {
    title: "Models",
    intro: "The OpenAI-compatible models endpoint lists installed executable audio models from Takokit's canonical planner. Metadata-only and incomplete installs are excluded.",
    sections: [
      {
        id: "list",
        title: "List models",
        commands: [`curl ${OPENAI}/models`, `Invoke-RestMethod ${OPENAI}/models`],
      },
      {
        id: "retrieve",
        title: "Retrieve a model",
        commands: [`curl ${OPENAI}/models/kokoro`],
      },
    ],
  },
  "openai-speech": {
    title: "Text to speech",
    intro: "POST /v1/audio/speech executes Takokit's local planner and returns audio bytes rather than Takokit metadata JSON.",
    sections: [
      {
        id: "curl",
        title: "curl",
        code: `curl ${OPENAI}/audio/speech \\
  -H "Content-Type: application/json" \\
  -d '{"model":"kokoro","input":"Hello from Takokit","voice":"default","response_format":"wav"}' \\
  --output speech.wav`,
      },
      {
        id: "powershell",
        title: "PowerShell",
        code: `$body = @{ model="kokoro"; input="Hello from Takokit"; voice="default"; response_format="wav" } | ConvertTo-Json
Invoke-WebRequest "${OPENAI}/audio/speech" -Method Post -ContentType "application/json" -Body $body -OutFile speech.wav`,
      },
      {
        id: "subset",
        title: "Supported subset",
        body: ["The compatibility adapter validates the fields it supports and rejects unknown/unsupported values explicitly rather than pretending they worked. Consult `/openapi.json` for the exact current schema and limits."],
      },
    ],
  },
  "openai-transcription": {
    title: "Transcription",
    intro: "POST /v1/audio/transcriptions accepts a real multipart upload, executes an installed speech-to-text model, and cleans request staging after inference.",
    sections: [
      {
        id: "curl",
        title: "curl / PowerShell 7",
        commands: [`curl.exe ${OPENAI}/audio/transcriptions -F "file=@recording.wav" -F "model=whisper-tiny"`],
      },
      {
        id: "formats",
        title: "Formats and limits",
        body: ["The current OpenAPI contract is authoritative for accepted upload formats, size limits, response formats, and optional fields. Unsupported language/prompt/options are rejected instead of silently ignored."],
      },
    ],
  },
  "openai-sdk": {
    title: "OpenAI SDK examples",
    intro: "Takokit does not ship its own Python or JavaScript SDK. You can point the official OpenAI SDK at Takokit's local audio-compatible base URL for the supported audio subset.",
    sections: [
      {
        id: "python",
        title: "Python",
        code: `from pathlib import Path
from openai import OpenAI

client = OpenAI(base_url="${OPENAI}", api_key="takokit")
print(client.models.list())
audio = client.audio.speech.create(
    model="kokoro", input="Hello", voice="default", response_format="wav"
)
audio.write_to_file("speech.wav")
with Path("recording.wav").open("rb") as recording:
    print(client.audio.transcriptions.create(model="whisper-tiny", file=recording).text)`,
      },
      {
        id: "javascript",
        title: "JavaScript",
        code: `import OpenAI from "openai";
import fs from "node:fs";

const client = new OpenAI({ baseURL: "${OPENAI}", apiKey: "takokit" });
console.log(await client.models.list());
const result = await client.audio.transcriptions.create({
  model: "whisper-tiny",
  file: fs.createReadStream("recording.wav")
});
console.log(result.text);`,
      },
      {
        id: "scope",
        title: "SDK scope",
        body: ["Only SDK calls mapping to Takokit's documented Models, Audio Speech, and Audio Transcription subset are expected to work. Chat, Responses, embeddings, and images are outside the compatibility contract."],
      },
    ],
  },
  "api-security": {
    title: "Authentication and network access",
    intro: "Default loopback use is intentionally simple. Non-loopback binding requires authentication and privileged local-control routes receive additional Host/Origin protection.",
    sections: [
      {
        id: "loopback",
        title: "Loopback",
        body: ["On 127.0.0.1 the local API does not require users to manage a secret for ordinary local use. SDKs that require a non-empty key can use a placeholder such as `takokit`; it is not a remote credential."],
      },
      {
        id: "network",
        title: "Non-loopback binding",
        body: ["Bind deliberately to a specific non-loopback address and set a random Bearer token of at least the minimum length enforced by the server."],
        code: `# PowerShell
$env:TAKOKIT_API_TOKEN = "<generated-random-token>"
tako serve --host 192.168.1.20 --port 5050

# bash / zsh
export TAKOKIT_API_TOKEN="<generated-random-token>"
tako serve --host 192.168.1.20 --port 5050`,
      },
      {
        id: "logging",
        title: "Request IDs and redaction",
        body: ["Include `x-request-id` when reporting a local API failure. Release logging contracts avoid recording API tokens and raw speech/transcript/upload contents by default."],
      },
    ],
  },
  "takokit-api": {
    title: "Takokit-native API",
    intro: "Takokit-specific model management, runners/adapters, voices, cloning, conversion, Advanced RVC, workspaces, sessions, storage, updates, and diagnostics live under /api/v1.",
    sections: [
      {
        id: "examples",
        title: "Examples",
        commands: [`curl ${NATIVE}/status`, `curl ${NATIVE}/models`, `curl ${NATIVE}/voices/rvc`],
      },
      {
        id: "workspace",
        title: "Workspace/session semantics",
        body: ["Native inference can preserve Takokit workspace/session metadata and output references. Use `/v1` when you specifically need the OpenAI audio binary/multipart semantics."],
      },
      {
        id: "privileged",
        title: "Privileged local helpers",
        body: ["Folder picker, open-path/media, shutdown, update-apply, and destructive maintenance routes are local product-control helpers. They are not advertised as stable third-party integration APIs."],
      },
      {
        id: "legacy",
        title: "Legacy /v1 aliases",
        body: ["Non-conflicting historical native `/v1` aliases can remain for compatibility, but new native integrations should target `/api/v1`. `/v1/models`, `/v1/audio/speech`, and `/v1/audio/transcriptions` always use OpenAI-compatible semantics."],
      },
    ],
  },
  "api-errors": {
    title: "Errors",
    intro: "Compatibility failures use an OpenAI-style error envelope while native routes use Takokit's typed error contracts. Public errors should be actionable without leaking stack traces, secrets, or arbitrary filesystem paths.",
    sections: [
      {
        id: "openai",
        title: "Compatibility envelope",
        code: `{"error":{"message":"...","type":"invalid_request_error","param":"response_format","code":"invalid_request"}}`,
      },
      {
        id: "request",
        title: "Report a failure",
        body: ["Record the failing command/route, Takokit version, platform, model reference, and returned `x-request-id`. For runtime failures also include the relevant Takokit log excerpt with sensitive paths/content redacted as needed."],
      },
    ],
  },
  "model-references": {
    title: "Model references and tags",
    intro: "Takokit resolves versioned registry references instead of silently following mutable upstream branches.",
    sections: [
      {
        id: "format",
        title: "Reference format",
        code: "[namespace/]model[:tag][@sha256:digest]",
      },
      {
        id: "examples",
        title: "Examples",
        commands: ["tako pull kokoro", "tako pull whisper-tiny", "tako pull whisper:small", "tako pull qwen3-tts:0.6b-base"],
      },
      {
        id: "latest",
        title: "Defaults and latest",
        body: ["An omitted tag resolves to Takokit's declared default. `latest` means the latest default Takokit has curated/published; it does not mean an unpinned upstream branch."],
      },
    ],
  },
  "registry-api": {
    title: "Registry API",
    intro: "The public companion-site registry is a release/model catalog endpoint. It is separate from the local Takokit runtime API.",
    sections: [
      {
        id: "endpoint",
        title: "Endpoint",
        commands: ["curl https://takokit.dawnlightlabs.com/v1/registry.json"],
      },
      {
        id: "bytes",
        title: "Registry versus model bytes",
        body: ["The registry carries model metadata and pinned source information. Model/runtime bytes are still downloaded from the declared verified upstream sources when you pull them."],
      },
    ],
  },
};
