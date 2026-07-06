export type ModelCapability = "tts" | "stt" | "clone" | "train" | "convert" | "streaming";

export type ModelSummary = {
  id: string;
  name: string;
  purpose: string;
  params: string;
  size: string;
  language: string;
  backend: string;
  runtime: "Rust" | "Python" | "ONNX" | "whisper.cpp";
  status: "installed" | "available" | "planned";
  license: string;
  capabilities: ModelCapability[];
};

export type VoiceSummary = {
  id: string;
  name: string;
  label: string;
  source: string;
  model: string;
  description: string;
  consent: "not required" | "required";
};

export type RuntimeSnapshot = {
  storagePath: string;
  server: {
    status: "offline" | "online";
    url: string;
    uptime: string;
  };
  models: ModelSummary[];
  voices: VoiceSummary[];
};

export type SpeechApiRequest = {
  model: string;
  input: string;
  voice?: string;
  response_format?: "wav" | "mp3" | "json";
};

export type SpeechApiResponse = {
  id: string;
  model: string;
  voice?: string;
  engine: string;
  output_path: string;
  content_type: string;
  bytes: number;
};

