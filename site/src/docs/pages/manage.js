export const MANAGE_DOCS = {
  "models-storage": {
    title: "Models and storage",
    intro: "Reusable runtime files and project-specific output history use separate local locations.",
    sections: [
      {
        id: "shared-runtime-data",
        title: "Shared runtime data",
        body: ["Models, runners, manifests, voices, cache data, and logs live under ~/.takokit."],
        code: `~/.takokit/
  models/
  runners/
  blobs/
  cache/
  manifests/
  voices/
  logs/`,
      },
      {
        id: "project-data",
        title: "Project data",
        body: ["Sessions and outputs belong to the project that launched the workflow."],
        code: `project/.tako/sessions/`,
      },
    ],
  },
  hardware: {
    title: "Hardware",
    intro: "Check the selected model's declared requirements before pulling it.",
    sections: [
      {
        id: "inspect-requirements",
        title: "Inspect requirements",
        body: ["Model pages present CPU/GPU support, RAM, VRAM, and approximate download size when the registry declares them."],
        commands: ["tako doctor", "tako plan qwen3-tts:0.6b-base"],
      },
      {
        id: "unknown-values",
        title: "Unknown values",
        body: ["Unknown requirements are shown as Not declared rather than inferred or displayed as zero."],
      },
    ],
  },
  daemon: {
    title: "Daemon",
    intro: "CLI, TUI, GUI, and API workflows share the managed daemon and one local registry state.",
    sections: [
      {
        id: "lifecycle",
        title: "Lifecycle",
        commands: ["tako daemon start", "tako daemon status", "tako daemon restart", "tako daemon stop"],
      },
      {
        id: "logs",
        title: "Daemon logs",
        commands: ["tako daemon logs"],
      },
    ],
  },
  "logs-diagnostics": {
    title: "Logs and diagnostics",
    intro: "Use Takokit's diagnostics before changing model environments manually.",
    sections: [
      {
        id: "doctor",
        title: "Run doctor",
        commands: ["tako doctor", "tako doctor --json"],
      },
      {
        id: "inspect-logs",
        title: "Inspect daemon logs",
        commands: ["tako daemon logs"],
      },
    ],
  },
  "reset-uninstall": {
    title: "Reset and uninstall",
    intro: "Remove individual models safely; full installer-backed uninstall instructions will arrive with release packages.",
    sections: [
      {
        id: "remove-a-model",
        title: "Remove a model",
        commands: ["tako rm whisper-tiny", "tako rm whisper-tiny --dry-run"],
      },
      {
        id: "clean-runtime-cache",
        title: "Clean runtime cache",
        commands: ["tako storage clean --dry-run", "tako storage clean"],
      },
      {
        id: "full-uninstall",
        title: "Full uninstall",
        body: ["A supported full-uninstall flow will be documented after packaged installers exist."],
      },
    ],
  },
  troubleshooting: {
    title: "Troubleshooting",
    intro: "Start with the model plan, doctor output, and daemon logs.",
    sections: [
      {
        id: "inspect-the-plan",
        title: "Inspect the plan",
        body: ["The plan explains the selected model, runner, hardware boundary, and next command."],
        commands: ["tako plan MODEL"],
      },
      {
        id: "run-diagnostics",
        title: "Run diagnostics",
        commands: ["tako doctor", "tako daemon logs"],
      },
      {
        id: "status-boundary",
        title: "Status boundary",
        body: ["Do not treat a metadata-only or hardware-blocked model as ready to execute."],
      },
    ],
  },
};
