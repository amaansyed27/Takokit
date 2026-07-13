export type ModelCapability = "tts" | "stt" | "voice_cloning" | "live_transcription" | "live_audio";

export type CapabilitySummary = {
  id: ModelCapability;
  label: string;
  description: string;
};

export type ModelSummary = {
  id: string;
  name: string;
  family: string;
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
  licenseWarning?: string;
  lifecycleState: ModelPlan["lifecycle_state"];
  runnerRuntimeState: ModelPlan["runner_runtime_state"];
  executable: boolean;
  missing: string[];
  nextCommand: string;
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
  lifecycle_state: "metadata-only" | "artifacts-ready" | "runner-ready" | "executable" | "failed";
  artifact_state: "metadata-only" | "artifacts-ready" | "runner-ready" | "executable" | "failed";
  runner_contract_state: "runtime-missing" | "contract-installed" | "runtime-installed" | "ready" | "failed";
  runner_runtime_state: "runtime-missing" | "contract-installed" | "runtime-installed" | "ready" | "failed";
  executable: boolean;
  missing: string[];
  next_command: string;
};

export type InstallStepState = "not-requested" | "already-ready" | "installed" | "repaired" | "metadata-only" | "failed";

export type InstallStep = {
  state: InstallStepState;
  newly_installed: boolean;
  detail: string;
};

export type ModelInstallResponse = {
  model_id: string;
  required_runner: string;
  required_adapter: string | null;
  artifacts: InstallStep;
  runner_contract: InstallStep;
  runner_runtime: InstallStep;
  adapter: InstallStep | null;
  executable: boolean;
  missing: string[];
  logs_path: string;
};

export type VoiceSummary = {
  id: string;
  name: string;
  label: string;
  source: string;
  model: string;
  description: string;
  consent: "not required" | "required" | "affirmed";
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
  sample_rate?: number;
};

export type TranscriptionApiRequest = {
  model?: string;
  file_path: string;
};

export type TranscriptionApiResponse = {
  id: string;
  model: string;
  text: string;
};

export type SessionTask =
  | "text_to_speech"
  | "speech_to_text"
  | "voice_cloning"
  | "voice_training"
  | "voice_conversion"
  | "model_install"
  | "runner_install"
  | "diagnostics"
  | "system";

export type SessionEventState = "started" | "completed" | "failed";

export type SessionEvent = {
  id: string;
  session_id: string;
  timestamp: number;
  task: SessionTask;
  state: SessionEventState;
  model?: string;
  input?: string;
  source_path?: string;
  output_path?: string;
  text?: string;
  message?: string;
};

export type SessionSummary = {
  id: string;
  title: string;
  workspace_root: string;
  created_at: number;
  updated_at: number;
  event_count: number;
  output_count: number;
  last_task?: SessionTask;
  last_model?: string;
};

export type SessionRecord = {
  summary: SessionSummary;
  events: SessionEvent[];
};

export type DoctorCheck = {
  section: string;
  label: string;
  status: "ok" | "warn" | "fail";
  detail?: string;
};

export type DoctorResponse = {
  storage_root: string;
  server: string;
  checks: DoctorCheck[];
  executable_models: string[];
  logs_path: string;
};
