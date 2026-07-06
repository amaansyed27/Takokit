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
  status: "installed" | "planned";
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
