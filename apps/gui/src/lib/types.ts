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
  runner: string;
  runnerInstalled: boolean;
  hardwareNotes: string;
  executionStatus: string;
  artifactCount: number;
  runtime: "Rust" | "Python" | "ONNX" | "whisper.cpp" | "External";
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
  supported_model_families?: string[];
  supported_tasks?: string[];
  dependency_strategy?: string;
  install_state?: string;
  notes?: string;
  description: string;
  installed: boolean;
};

export type ModelPlan = {
  model_id: string;
  model_name: string;
  family: string;
  task: string;
  required_runner: string;
  artifact_state: "metadata-only" | "artifacts-ready" | "runner-ready" | "executable" | "failed";
  runner_contract_state: "runtime-missing" | "contract-installed" | "runtime-installed" | "ready" | "failed";
  runner_runtime_state: "runtime-missing" | "contract-installed" | "runtime-installed" | "ready" | "failed";
  executable: boolean;
  missing: string[];
  next_command: string;
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
