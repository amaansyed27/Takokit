export type ModelCapability = "tts" | "stt" | "voice_cloning" | "live_transcription" | "live_audio";

export type CapabilitySummary = {
  id: ModelCapability;
  label: string;
  description: string;
};

export type ModelSummary = {
  id: string;
  name: string;
  purpose: string;
  version: string;
  params?: string;
  size?: string;
  language: string;
  backend: string;
  runtime: "Rust" | "Python" | "ONNX" | "whisper.cpp";
  status: "installed" | "available" | "planned";
  license: string;
  capabilities: ModelCapability[];
};

export type RunnerSummary = {
  id: string;
  name: string;
  version: string;
  kind: string;
  platforms: string[];
  description: string;
  installed: boolean;
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
  runners: RunnerSummary[];
  voices: VoiceSummary[];
  capabilities: CapabilitySummary[];
  modeNote: string;
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
