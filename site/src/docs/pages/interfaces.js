export const INTERFACE_DOCS = {
  "cli-reference": {
    title: "CLI reference",
    intro: "The `tako` command is the canonical command-line entry point. Global output, workspace, session, and direct-execution flags apply across the command tree.",
    sections: [
      {
        id: "global-options",
        title: "Global options",
        code: `--direct                 Execute without delegating to a managed server where supported
--output human|json      Human-readable output by default; JSON is machine-readable stdout
--workspace <path>       Use a specific project directory and its .tako state
--session <uuid>         Resume a specific project session`,
      },
      {
        id: "runtime",
        title: "Runtime and server",
        commands: ["tako version", "tako status", "tako doctor", "tako capabilities", "tako start", "tako stop", "tako serve", "tako server start", "tako server status", "tako server restart", "tako server logs", "tako server stop"],
        note: "`tako daemon ...` remains a compatibility alias. New documentation and integrations should use `tako server ...`. `tako serve` is the explicit developer-owned foreground server.",
      },
      {
        id: "models",
        title: "Models and registry",
        commands: ["tako models", "tako list", "tako show kokoro", "tako plan whisper-tiny", "tako pull kokoro", "tako rm kokoro --dry-run", "tako rm kokoro", "tako library sync", "tako library models", "tako library runners", "tako library show whisper:small"],
      },
      {
        id: "runners",
        title: "Runners and adapters",
        commands: ["tako runners", "tako runner show takokit-onnx", "tako runner pull takokit-onnx", "tako runner install takokit-onnx", "tako runner doctor takokit-onnx", "tako runner rm takokit-onnx", "tako adapter list", "tako adapter install ADAPTER", "tako adapter doctor ADAPTER"],
      },
      {
        id: "voice-workflows",
        title: "Voice workflows",
        commands: ["tako speak \"Hello\" --model kokoro", "tako transcribe recording.wav --model whisper-tiny", "tako run kokoro \"Hello\"", "tako run whisper-tiny --file recording.wav", "tako clone reference.wav --name \"My Voice\" --model chatterbox --consent", "tako convert source.wav --target-voice TARGET --model rvc --consent", "tako train ./dataset --name \"My Voice\" --model gpt-sovits --consent"],
      },
      {
        id: "voices",
        title: "Voice profiles and Advanced RVC",
        commands: ["tako voice list", "tako voice show MODEL", "tako voice add reference.wav --name \"Narrator\" --model xtts-v2 --consent", "tako voice rvc list", "tako voice rvc presets"],
        body: ["See Advanced RVC studio for the full project, dataset, training, checkpoint, test, and package command set."],
      },
      {
        id: "sessions",
        title: "Sessions",
        commands: ["tako sessions list", "tako sessions new --title \"Narration tests\"", "tako sessions show SESSION_UUID", "tako sessions open SESSION_UUID", "tako sessions rm SESSION_UUID"],
      },
      {
        id: "storage-updates",
        title: "Storage, updates, and reset",
        commands: ["tako storage", "tako storage status", "tako storage clean --dry-run", "tako storage clean --scope all-safe", "tako update check", "tako update status", "tako update channel stable", "tako update configure --automatic-checks on --automatic-download off", "tako reset --dry-run"],
        warning: "Full reset is intentionally confirmation-gated. Use `tako reset --help` and inspect the resolved data paths before destructive cleanup.",
      },
      {
        id: "other",
        title: "Other management commands",
        commands: ["tako gui", "tako ps", "tako custom-model list", "tako licenses list", "tako deps doctor", "tako quickstart", "tako samples create", "tako test --suite launch --json"],
      },
      {
        id: "output-contract",
        title: "Output contract",
        body: ["`--output human` produces readable text. `--output json` keeps stdout valid JSON without trailing completion text, so scripts can parse it safely. Diagnostics that are not part of JSON stdout belong on stderr or in the structured response."],
      },
    ],
  },
  "tui-guide": {
    title: "TUI guide",
    intro: "Run `tako` with no subcommand for the terminal interface. It is a view over the same runtime state used by the CLI, GUI, and API.",
    sections: [
      {
        id: "launch",
        title: "Launch and exit",
        commands: ["tako"],
        body: ["The TUI opens in the current workspace unless `--workspace` is supplied. Exit normally with the visible quit action or Ctrl+C; the terminal should be restored cleanly."],
      },
      {
        id: "navigation",
        title: "Keyboard navigation",
        code: `Arrow keys          Navigate lists and sections
Tab / Shift+Tab     Move through form fields
Enter               Activate the selected action
F1                  Open help
Ctrl+C              Exit safely`,
      },
      {
        id: "workflow-areas",
        title: "Workflow areas",
        items: ["Models — browse, inspect, plan, pull, and remove", "Speak — text-to-speech inputs and voice controls", "Transcribe — local audio transcription", "Clone / Convert / RVC — consent-backed voice workflows", "Sessions — create, open, inspect, and resume work", "Runners / System — runtime state, diagnostics, and maintenance"],
      },
      {
        id: "workspace",
        title: "Workspace context",
        body: ["Confirm the active workspace before running a task. Outputs and session history belong to that workspace's `.tako` directory, while downloaded models/runners remain global under `~/.takokit`."],
      },
    ],
  },
  "gui-guide": {
    title: "GUI guide",
    intro: "Takokit's GUI is a React browser application served by the local Takokit server. It shares runtime semantics with the CLI/TUI; it is not a separate desktop runtime.",
    sections: [
      {
        id: "open",
        title: "Open the GUI",
        commands: ["tako gui"],
        code: "http://127.0.0.1:5050/gui",
      },
      {
        id: "navigation",
        title: "Navigation",
        items: ["Home — runtime and project overview", "Speak — text-to-speech", "Transcribe — speech-to-text", "Clone audio — cloning and conversion workflows", "Voices — reusable local voice profiles", "Files — workspace outputs and files", "Models — registry and installed-model management", "Runners — execution runtime state", "History — sessions and previous operations", "Storage — local disk usage and safe cleanup", "Diagnostics — server/runtime evidence", "Settings — workspace and runtime preferences"],
      },
      {
        id: "workspace-picker",
        title: "Choose a workspace",
        body: ["Use Browse to open the native folder chooser. Takokit returns the selected absolute local path through a narrowly scoped local control route, then creates/uses `.tako` project state there."],
        note: "The picker is a local GUI helper, not a general third-party filesystem API.",
      },
      {
        id: "model-install",
        title: "Install a model",
        body: ["Open Models, inspect the selected model's hardware/license/runtime information, then Pull. Progress and actionable errors come from the same pull planner as the CLI."],
      },
      {
        id: "failures",
        title: "Failures and retry",
        body: ["When an operation fails, use Diagnostics and the displayed request/error ID before manually changing runtime files. Interrupted managed downloads/bootstrap operations are designed to recover or be safely retried."],
      },
    ],
  },
  "workspaces-sessions": {
    title: "Workspaces and sessions",
    intro: "Takokit deliberately separates reusable global runtime data from project-local history and outputs.",
    sections: [
      {
        id: "global",
        title: "Global runtime data",
        code: `~/.takokit/
  models/
  runners/
  blobs/
  cache/
  manifests/
  voices/
  logs/`,
        body: ["This data is reused across projects and interfaces. On Windows, `~` resolves to the current user's profile directory."],
      },
      {
        id: "project",
        title: "Project-local state",
        code: `<workspace>/.tako/
  active-session
  version
  sessions/
    <session-id>/
      session.json
      events.jsonl
      outputs/`,
        body: ["Do not put downloaded model runtimes inside `.tako`; do not put project outputs into the global model cache."],
      },
      {
        id: "select",
        title: "Select context",
        commands: ["tako --workspace ./voice-project status", "tako --workspace ./voice-project sessions new --title \"Experiment\"", "tako --workspace ./voice-project --session SESSION_UUID speak \"Resume this work\" --model kokoro"],
      },
      {
        id: "gui-tui",
        title: "GUI and TUI",
        body: ["The GUI workspace picker and TUI workspace/session surfaces change the same shared project context. Switching interfaces does not create a second copy of the project state."],
      },
      {
        id: "backup",
        title: "Backup and deletion",
        body: ["Back up `.tako` with the project when session history/outputs matter. Normal application uninstall preserves `~/.takokit` and project `.tako` state. Destructive global/project reset is separate and explicitly confirmed."],
      },
    ],
  },
};
