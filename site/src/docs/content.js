import { DEVELOPER_DOCS } from "./pages/developers.js";
import { GETTING_STARTED_DOCS } from "./pages/getting-started.js";
import { INTERFACE_DOCS } from "./pages/interfaces.js";
import { MANAGE_DOCS } from "./pages/manage.js";
import { RELEASE_DOCS } from "./pages/release.js";
import { VOICE_WORKFLOW_DOCS } from "./pages/voice-workflows.js";

export const DOC_GROUPS = [
  {
    title: "Getting started",
    pages: [
      ["install", "Install Takokit"],
      ["first-launch", "First launch"],
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
      ["advanced-rvc", "Advanced RVC studio"],
      ["voice-profiles", "Voice profiles"],
      ["custom-models", "Custom models"],
    ],
  },
  {
    title: "Interfaces",
    pages: [
      ["cli-reference", "CLI reference"],
      ["tui-guide", "TUI guide"],
      ["gui-guide", "GUI guide"],
      ["workspaces-sessions", "Workspaces and sessions"],
    ],
  },
  {
    title: "Developers",
    pages: [
      ["local-api", "API introduction"],
      ["openai-models", "Models"],
      ["openai-speech", "Text to speech"],
      ["openai-transcription", "Transcription"],
      ["openai-sdk", "OpenAI SDK examples"],
      ["api-security", "Authentication and network"],
      ["takokit-api", "Takokit API"],
      ["api-errors", "Errors"],
      ["model-references", "Model references and tags"],
      ["registry-api", "Registry API"],
    ],
  },
  {
    title: "Manage Takokit",
    pages: [
      ["models-storage", "Models and storage"],
      ["hardware", "Hardware"],
      ["server", "Server lifecycle"],
      ["updates", "Updates"],
      ["logs-diagnostics", "Logs and diagnostics"],
      ["reset-uninstall", "Reset and uninstall"],
      ["troubleshooting", "Troubleshooting"],
    ],
  },
  {
    title: "Release and safety",
    pages: [
      ["beta-status", "Beta status"],
      ["platform-support", "Platform support"],
      ["release-channels", "Release channels"],
      ["privacy-safety", "Privacy and voice safety"],
    ],
  },
];

export const DOCS = {
  ...GETTING_STARTED_DOCS,
  ...VOICE_WORKFLOW_DOCS,
  ...INTERFACE_DOCS,
  ...DEVELOPER_DOCS,
  ...MANAGE_DOCS,
  ...RELEASE_DOCS,
};

export const DOC_ORDER = DOC_GROUPS.flatMap((group) =>
  group.pages.map(([id, title]) => ({ id, title, group: group.title })),
);

export function findDoc(slug) {
  return DOCS[slug] || null;
}

export function findDocGroup(slug) {
  return DOC_GROUPS.find((group) => group.pages.some(([id]) => id === slug)) || null;
}

export function adjacentDocs(slug) {
  const index = DOC_ORDER.findIndex((item) => item.id === slug);
  return {
    previous: index > 0 ? DOC_ORDER[index - 1] : null,
    next: index >= 0 && index < DOC_ORDER.length - 1 ? DOC_ORDER[index + 1] : null,
  };
}
