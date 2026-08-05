export const RECOMMENDED_REFS = [
  "kokoro",
  "whisper:tiny",
  "chatterbox",
  "rvc",
];

const VERIFIED_REFS = new Set(["whisper:tiny"]);
const HARDWARE_BLOCKED_REFS = new Set(["qwen-omni:3"]);

const MODEL_PRESENTATION = {
  kokoro: {
    primary_task: "speech",
    short_summary: "Fast, lightweight local speech generation.",
    languages: ["English"],
    known_limitations: ["Uses bundled voices; it does not clone a reference voice."],
  },
  whisper: {
    primary_task: "transcription",
    short_summary: "Reliable local transcription with small CPU-friendly variants.",
    languages: ["Multilingual"],
    supported_platforms: ["Windows", "Linux"],
  },
  chatterbox: {
    primary_task: "cloning",
    short_summary: "Expressive speech and consent-backed zero-shot voice cloning.",
    known_limitations: ["Reference quality strongly affects the generated voice."],
  },
  rvc: {
    primary_task: "conversion",
    short_summary: "Convert a recording with a compatible custom voice checkpoint.",
    known_limitations: [
      "Requires a compatible custom .pth checkpoint.",
      "A matching .index file is recommended where available.",
      "Generated audio still requires perceptual listening review.",
    ],
  },
  piper: {
    primary_task: "speech",
    short_summary: "Small offline speech generation with the Lessac voice.",
    languages: ["English"],
  },
};

export const TASKS = {
  speech: {
    label: "Speech",
    longLabel: "Generate speech",
    registryTasks: ["tts"],
  },
  transcription: {
    label: "Transcription",
    longLabel: "Transcribe audio",
    registryTasks: ["stt", "live-transcription"],
  },
  cloning: {
    label: "Cloning",
    longLabel: "Clone a voice",
    registryTasks: ["voice-cloning"],
  },
  conversion: {
    label: "Conversion",
    longLabel: "Convert a voice",
    registryTasks: ["voice-conversion"],
  },
};

export const STATUS_LABELS = {
  verified: "Locally verified",
  executable: "Executable path",
  "hardware-blocked": "Hardware blocked",
  "metadata-only": "Metadata only",
};

export function canonicalRef(modelName, tag, defaultTag) {
  return tag && tag !== defaultTag ? `${modelName}:${tag}` : modelName;
}

export function releaseRef(model, release) {
  return `${model.name}:${release.tag}`;
}

function manifestMetadataOnly(release) {
  return /\bmetadata_only\s*=\s*true\b/.test(release.manifest_toml || "");
}

export function verificationStatus(model, release) {
  const ref = releaseRef(model, release);
  if (manifestMetadataOnly(release)) return "metadata-only";
  if (HARDWARE_BLOCKED_REFS.has(ref)) return "hardware-blocked";
  if (VERIFIED_REFS.has(ref)) return "verified";
  return "executable";
}

export function presentationFor(model) {
  return MODEL_PRESENTATION[model.name] || {};
}

export function primaryTask(model) {
  const declared = presentationFor(model).primary_task;
  if (declared) return declared;
  if (model.tasks.includes("voice-conversion")) return "conversion";
  if (model.tasks.includes("voice-cloning")) return "cloning";
  if (model.tasks.includes("stt")) return "transcription";
  return "speech";
}

export function taskLabels(model) {
  const labels = [];
  for (const [key, task] of Object.entries(TASKS)) {
    if (task.registryTasks.some((value) => model.tasks.includes(value))) {
      labels.push(task.label);
    }
  }
  return labels;
}

export function supportedPlatforms(model) {
  return presentationFor(model).supported_platforms || [];
}

export function languages(model) {
  return presentationFor(model).languages || [];
}

export function knownLimitations(model) {
  return presentationFor(model).known_limitations || [];
}

export function shortSummary(model) {
  return presentationFor(model).short_summary || model.summary;
}

export function isRecommended(model, release) {
  const ref = releaseRef(model, release);
  return RECOMMENDED_REFS.includes(ref) ||
    RECOMMENDED_REFS.includes(model.name) && release.tag === model.default_tag;
}
