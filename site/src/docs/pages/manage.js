export const MANAGE_DOCS = {
  "models-storage": {
    title: "Models and storage",
    intro: "Takokit separates reusable global runtime data from project-local sessions and outputs, then provides safe inspection/cleanup commands instead of requiring manual cache deletion.",
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
        body: ["Managed uv/runtime tools and adapter environments also live inside Takokit-owned global state. Exact subdirectories can evolve without changing the user-facing global-versus-project contract."],
      },
      {
        id: "project",
        title: "Project data",
        code: `<workspace>/.tako/
  active-session
  version
  sessions/
    <session-id>/
      session.json
      events.jsonl
      outputs/`,
      },
      {
        id: "report",
        title: "Storage report",
        commands: ["tako storage", "tako storage --json", "tako storage status"],
      },
      {
        id: "safe-clean",
        title: "Safe cleanup",
        commands: ["tako storage clean --dry-run", "tako storage clean --scope downloads", "tako storage clean --scope uv", "tako storage clean --scope unused", "tako storage clean --scope all-safe"],
        note: "Safe cleanup targets reconstructible data only. Provider/model checkpoint caches are not implicit deletion targets when Takokit cannot prove they are safely reconstructible/unreferenced.",
      },
    ],
  },
  hardware: {
    title: "Hardware",
    intro: "Distribution support and model support are separate. Inspect each model's declared requirements before pulling or training it.",
    sections: [
      {
        id: "doctor",
        title: "Machine diagnostics",
        commands: ["tako doctor", "tako capabilities", "tako doctor --json"],
      },
      {
        id: "plan",
        title: "Model planning",
        commands: ["tako plan kokoro", "tako plan qwen3-tts:0.6b-base", "tako show whisper-tiny"],
        body: ["Plans expose the chosen runner/adapter, declared RAM/VRAM/device requirements, license boundaries, and the next action where available."],
      },
      {
        id: "accelerators",
        title: "CUDA, MPS, and CPU",
        body: ["Small packaged acceptance uses CPU paths on Linux/macOS. CUDA support is strongest on compatible Windows/Linux model stacks. Apple Silicon MPS is used only where the upstream model/runtime actually supports it; Takokit does not translate CUDA requirements into fake MPS support."],
      },
      {
        id: "unknown",
        title: "Unknown requirements",
        body: ["Unknown requirements are reported as unknown/not declared rather than invented. Hardware-blocked means the runtime path exists but the current machine does not satisfy the declared requirement."],
      },
    ],
  },
  server: {
    title: "Server lifecycle",
    intro: "CLI, TUI, browser GUI, resident controllers, and API workflows share the same Takokit server identity and ownership model.",
    sections: [
      {
        id: "managed",
        title: "Managed server",
        commands: ["tako start", "tako server start", "tako server status", "tako server restart", "tako server logs", "tako stop"],
      },
      {
        id: "foreground",
        title: "Foreground serve",
        commands: ["tako serve", "tako serve --host 127.0.0.1 --port 5050"],
        body: ["`tako serve` is developer-owned and runs in the foreground. Ctrl+C stops it. If a verified Takokit server is already running on the configured address, Takokit reports that cleanly instead of surfacing a raw socket-bind error."],
      },
      {
        id: "ownership",
        title: "Ownership rules",
        items: ["Resident-owned server: resident Quit may stop it", "Developer-owned `tako serve`: resident Quit must preserve it", "Foreign process on port 5050: Takokit must never kill it merely because it occupies the port", "Server dies while resident remains: resident reports Stopped and can start it again"],
      },
      {
        id: "legacy",
        title: "Daemon alias",
        body: ["`tako daemon start|stop|restart|status|logs` remains for backwards compatibility. Prefer `tako server ...` in new scripts and documentation."],
      },
    ],
  },
  updates: {
    title: "Updates",
    intro: "Takokit uses signed release metadata, platform/architecture checks, artifact checksums, staging, and rollback-aware replacement. Installation remains an explicit action.",
    sections: [
      {
        id: "inspect",
        title: "Check update status",
        commands: ["tako update status", "tako update check"],
      },
      {
        id: "configure",
        title: "Configure automatic checks",
        commands: ["tako update channel stable", "tako update configure --automatic-checks on --automatic-download off"],
      },
      {
        id: "manual",
        title: "Download and apply",
        commands: ["tako update download", "tako update apply"],
        body: ["Release artifacts are verified before replacement. Mutable `~/.takokit` state and project `.tako` data are outside the immutable application replacement set."],
      },
      {
        id: "platform-not-published",
        title: "Platform not published yet",
        body: ["During a beta transition, a platform can legitimately have no stable release manifest yet. Takokit should report that state cleanly rather than expose a raw 404 or falsely claim the candidate is up to date."],
      },
    ],
  },
  "logs-diagnostics": {
    title: "Logs and diagnostics",
    intro: "Collect evidence from Takokit before changing managed environments manually. The runtime is designed to self-heal common interrupted/stale managed-tool states.",
    sections: [
      {
        id: "doctor",
        title: "Doctor",
        commands: ["tako doctor", "tako doctor --json", "tako deps doctor"],
      },
      {
        id: "server",
        title: "Server logs",
        commands: ["tako server logs"],
      },
      {
        id: "runtime",
        title: "Managed runtime logs",
        body: ["Model/runner bootstrap diagnostics are stored under `~/.takokit/logs`. For example, managed uv bootstrap logging distinguishes the requested version, source kind/path, observed SHA, expected SHA, platform, architecture, and managed path."],
      },
      {
        id: "report",
        title: "Useful bug-report evidence",
        items: ["`tako version`", "Operating system and architecture", "Exact model reference/command", "`tako doctor` summary", "Relevant request ID or log excerpt", "Whether the install is packaged, portable, or source-built", "Screenshots only when the problem is visual"],
      },
    ],
  },
  "reset-uninstall": {
    title: "Reset and uninstall",
    intro: "Application uninstall and user-data deletion are deliberately separate operations on every supported platform.",
    sections: [
      {
        id: "remove-model",
        title: "Remove a model",
        commands: ["tako rm whisper-tiny --dry-run", "tako rm whisper-tiny"],
      },
      {
        id: "reset-preview",
        title: "Preview destructive reset",
        commands: ["tako reset --dry-run"],
        warning: "Do not manually delete paths copied from another machine. Takokit's destructive reset requires acknowledgements matching the resolved local data paths.",
      },
      {
        id: "windows",
        title: "Windows uninstall",
        body: ["Use Windows Installed apps / the Takokit uninstaller. It removes the application, resident/startup integration, and installed binaries while preserving global `~/.takokit` and project `.tako` state by default."],
      },
      {
        id: "unix",
        title: "Linux and macOS uninstall",
        body: ["The packaged Unix install includes the supported uninstall script in the install tree. Normal uninstall removes Takokit-owned application/desktop/PATH integration and preserves `~/.takokit` plus project `.tako` state."],
        commands: ["~/.local/share/takokit/uninstall.sh"],
      },
      {
        id: "full-data",
        title: "Full data deletion",
        body: ["If you intentionally want models, voices, logs, and runtime caches removed too, use the explicit Takokit reset/purge path after reviewing what will be deleted. Project `.tako` directories are separate and should only be deleted when you also want project history/outputs gone."],
      },
    ],
  },
  troubleshooting: {
    title: "Troubleshooting",
    intro: "Start with the exact command/model plan, server state, doctor output, and relevant Takokit logs. Avoid solving managed-runtime problems by installing random global dependencies.",
    sections: [
      {
        id: "server",
        title: "Server unavailable or port 5050 conflict",
        commands: ["tako server status", "tako doctor", "tako serve"],
        body: ["If the port is occupied, Takokit verifies whether the listener belongs to Takokit. A foreign process is never killed solely because it uses port 5050."],
      },
      {
        id: "pull",
        title: "Failed or interrupted pull",
        commands: ["tako plan MODEL", "tako pull MODEL", "tako doctor"],
        body: ["Retry through Takokit first. Managed downloads/bootstrap state is staged and verified; do not bypass the planner by cloning repositories or globally installing packages unless a contributor guide explicitly requires it."],
      },
      {
        id: "uv",
        title: "Managed uv/runtime bootstrap",
        body: ["Takokit owns its pinned managed uv runtime and can download/verify it itself. A stale Homebrew/system uv should not be silently adopted as Takokit's managed runtime. `UV=/absolute/path/to/uv` is an explicit advanced override and must match the accepted pin."],
      },
      {
        id: "memory",
        title: "RAM / VRAM / committed-memory failures",
        body: ["Use `tako plan MODEL` and the model page before retrying. Reduce workload/select a smaller model or use compatible hardware; do not label a hardware-blocked model as a runtime defect without satisfying the requirement."],
      },
      {
        id: "cuda",
        title: "CUDA / CPU / MPS fallback",
        body: ["Fallback is model-specific. If an upstream runtime is CUDA-only, Takokit should report that boundary instead of pretending CPU/MPS parity. Check the model/runner status and logs."],
      },
      {
        id: "audio",
        title: "Invalid audio or reference files",
        body: ["Confirm the file exists, is readable, and uses a supported format. For cloning/training, use clear owned/consented speech. For RVC, verify the checkpoint and index belong together."],
      },
      {
        id: "workspace",
        title: "Missing output or wrong history",
        commands: ["tako status", "tako sessions list"],
        body: ["Check the active workspace/session. Outputs/history are project-local under `.tako`, so launching Takokit from another directory can legitimately show a different project history."],
      },
      {
        id: "macos",
        title: "macOS Gatekeeper or app launch",
        body: ["Record the exact macOS warning and the release's Apple signing/notarization status. Do not disable Gatekeeper or run broad quarantine/security bypass commands as a normal fix."],
      },
      {
        id: "windows-lock",
        title: "Windows executable locked during development",
        body: ["Stop the relevant Takokit server/resident process before replacing a development executable. Packaged update/uninstall flows already coordinate the installed-product lifecycle."],
      },
    ],
  },
};
