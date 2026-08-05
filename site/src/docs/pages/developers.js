export const DEVELOPER_DOCS = {
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
};
