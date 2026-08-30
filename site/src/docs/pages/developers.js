const OPENAI = "http://127.0.0.1:5050/v1";
const NATIVE = "http://127.0.0.1:5050/api/v1";

export const DEVELOPER_DOCS = {
  "local-api": {
    title: "Local API introduction",
    intro: "Takokit 0.2.0 uses one local runtime for an OpenAI-compatible audio subset and a separate Takokit-native API.",
    sections: [
      { id: "run", title: "Running Takokit", commands: ["tako serve", "tako server start", "tako server status", "tako server logs"] },
      { id: "urls", title: "URLs", code: `${OPENAI}      OpenAI-compatible audio API
${NATIVE}  Takokit-native API
http://127.0.0.1:5050/gui
http://127.0.0.1:5050/openapi.json` },
      { id: "matrix", title: "Compatibility matrix", code: `Models                 Supported
Audio speech           Supported
Audio transcription    Supported
Chat completions       Not supported
Responses              Not supported
Embeddings             Not supported
Images                  Not supported`, note: "Takokit claims OpenAI-compatible audio endpoints, not general OpenAI API compatibility." },
    ],
  },
  "openai-models": {
    title: "Models",
    intro: "Only installed, executable audio models from Takokit's canonical planner are listed. Metadata-only and incomplete installs are excluded.",
    sections: [{ id: "list", title: "List and retrieve", commands: [`curl ${OPENAI}/models`, `Invoke-RestMethod ${OPENAI}/models`, `curl ${OPENAI}/models/kokoro`] }],
  },
  "openai-speech": {
    title: "Text to speech",
    intro: "POST /v1/audio/speech returns audio bytes rather than Takokit metadata JSON.",
    sections: [
      { id: "curl", title: "curl", code: `curl ${OPENAI}/audio/speech \\
  -H "Authorization: Bearer takokit" -H "Content-Type: application/json" \\
  -d '{"model":"kokoro","input":"Hello","voice":"default","response_format":"wav"}' \\
  --output speech.wav` },
      { id: "powershell", title: "PowerShell", code: `$body = @{model="kokoro";input="Hello";voice="default";response_format="wav"}|ConvertTo-Json
Invoke-WebRequest "${OPENAI}/audio/speech" -Method Post -ContentType "application/json" -Body $body -OutFile speech.wav` },
      { id: "subset", title: "Supported subset", body: ["Input is limited to 4096 characters. WAV and speed 1.0 are guaranteed. Unsupported or unknown parameters are rejected explicitly."] },
    ],
  },
  "openai-transcription": {
    title: "Transcription",
    intro: "POST /v1/audio/transcriptions accepts an actual multipart upload and cleans temporary request files after inference.",
    sections: [
      { id: "curl", title: "curl and PowerShell 7", commands: [`curl.exe ${OPENAI}/audio/transcriptions -H "Authorization: Bearer takokit" -F "file=@recording.wav" -F "model=whisper-tiny"`] },
      { id: "limits", title: "Formats and limits", body: ["Uploads are limited to 25 MiB. WAV, MP3/MPEG, FLAC, OGG, M4A/MP4, and WebM are accepted. response_format may be json or text. Unsupported language or prompt values are rejected."] },
    ],
  },
  "openai-sdk": {
    title: "OpenAI SDK examples",
    intro: "Use the official SDK with the local base URL. The placeholder key works for default loopback use.",
    sections: [
      { id: "python", title: "Python", code: `from pathlib import Path
from openai import OpenAI
client = OpenAI(base_url="${OPENAI}", api_key="takokit")
print(client.models.list())
audio = client.audio.speech.create(model="kokoro", input="Hello", voice="default", response_format="wav")
audio.write_to_file("speech.wav")
with Path("recording.wav").open("rb") as f:
    print(client.audio.transcriptions.create(model="whisper-tiny", file=f).text)` },
      { id: "javascript", title: "JavaScript", code: `import OpenAI from "openai";
import fs from "node:fs";
const client = new OpenAI({baseURL:"${OPENAI}", apiKey:"takokit"});
console.log(await client.models.list());
console.log((await client.audio.transcriptions.create({model:"whisper-tiny",file:fs.createReadStream("recording.wav")})).text);` },
    ],
  },
  "api-security": {
    title: "Authentication and network access",
    intro: "Loopback is zero-configuration. Non-loopback binding never exposes machine-control routes anonymously.",
    sections: [
      { id: "loopback", title: "Loopback", body: ["api_key=\"takokit\" works locally. Host and browser Origin checks protect privileged routes from DNS rebinding and hostile pages."] },
      { id: "network", title: "Network binding", body: ["Choose an explicit IP and a generated token of at least 24 characters. All requests then require that Bearer token."], commands: ["$env:TAKOKIT_API_TOKEN='<generated-random-token>'", "tako serve --host 192.168.1.20 --port 5050"] },
      { id: "logging", title: "Logging", body: ["x-request-id is returned. Logs omit speech text, transcripts, uploads, and API tokens by default."] },
    ],
  },
  "takokit-api": {
    title: "Takokit-native API",
    intro: "Model management, runners, adapters, voices, cloning, conversion, RVC, sessions, storage, updates, and diagnostics live under /api/v1.",
    sections: [
      { id: "examples", title: "Examples", commands: [`curl ${NATIVE}/status`, `curl ${NATIVE}/models`, `curl ${NATIVE}/voices/rvc`] },
      { id: "semantics", title: "Native semantics", body: ["Native inference preserves Takokit workspace/session metadata. Use /v1 for OpenAI binary and multipart semantics."] },
      { id: "legacy", title: "v0.1 transition", body: ["Common non-conflicting /v1 aliases remain temporarily. /v1/models, /v1/audio/speech, and /v1/audio/transcriptions always use OpenAI-compatible semantics in v0.2."] },
      { id: "privileged", title: "Privileged helpers", body: ["Picker, open-path, shutdown, maintenance, and update-apply routes are local GUI/control helpers, not stable public integration contracts."] },
    ],
  },
  "api-errors": {
    title: "Errors",
    intro: "Compatibility errors use an OpenAI-style envelope and do not expose stack traces, secrets, or arbitrary filesystem paths.",
    sections: [{ id: "shape", title: "Envelope", code: `{"error":{"message":"...","type":"invalid_request_error","param":"response_format","code":"invalid_request"}}` }, { id: "request", title: "Request ID", body: ["Include x-request-id when reporting a server failure."] }],
  },
  "model-references": {
    title: "Model references and tags",
    intro: "An omitted tag resolves to the declared default; an explicit tag selects a registry variant.",
    sections: [{ id: "refs", title: "Examples", commands: ["tako pull kokoro", "tako pull whisper-tiny", "tako pull whisper:small"] }],
  },
  "registry-api": {
    title: "Registry API",
    intro: "The companion site's public registry JSON is separate from the local runtime API.",
    sections: [{ id: "endpoint", title: "Endpoint", commands: ["curl https://takokit-library.vercel.app/v1/registry.json"] }],
  },
};
