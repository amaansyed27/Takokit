export const GETTING_STARTED_DOCS = {
  install: {
    title: "Install Takokit",
    intro: "Takokit 0.x releases are public beta builds. v0.3.0 targets Windows x86_64, Linux x86_64, and macOS Apple Silicon with one shared runtime and signed release metadata.",
    sections: [
      {
        id: "windows",
        title: "Windows",
        body: ["Run the PowerShell bootstrap or use the installer download. Both resolve the same stable metadata and verify the installer SHA-256 before installation."],
        commands: ["irm https://takokit.dawnlightlabs.com/install.ps1 | iex"],
        items: ["Windows 10 or 11", "x86_64", "Per-user Inno installer", "Takokit app + notification-area resident controller"],
      },
      {
        id: "linux",
        title: "Linux",
        body: ["The POSIX bootstrap detects Linux x86_64, verifies the published archive, installs Takokit per-user under ~/.local, and adds the freedesktop launcher where supported."],
        commands: ["curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh"],
        items: ["Linux x86_64 is the published v0.3.0 target", "Linux ARM64 is not published in v0.3.0", "No root, Cargo, Node, or system Python is required for the packaged runtime"],
      },
      {
        id: "macos",
        title: "macOS",
        body: ["The same POSIX bootstrap detects Apple Silicon, verifies the release archive, installs the CLI/runtime per-user, and places Takokit.app in the user Applications directory."],
        commands: ["curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh"],
        items: ["macOS 12+", "Apple Silicon / arm64", "Takokit.app + menu-bar resident controller", "Intel macOS remains experimental in v0.3.0"],
        note: "Production Apple Developer ID signing/notarization status is reported with each release. Takokit never disables Gatekeeper or changes macOS security settings for you.",
      },
      {
        id: "verify",
        title: "Verify the installation",
        body: ["Open a new terminal after installation so PATH changes are visible, then verify the runtime and diagnostics."],
        commands: ["tako version", "tako doctor", "tako update status"],
      },
    ],
  },
  "first-launch": {
    title: "First launch",
    intro: "Takokit exposes the same local state through the native app launcher, browser GUI, TUI, CLI, and API.",
    sections: [
      {
        id: "normal-app",
        title: "Normal application launch",
        body: ["On Windows, open Takokit from the Start menu. On macOS, open Takokit.app. The resident controller starts or reuses the verified local server and opens the browser GUI on an explicit launch."],
        items: ["Windows: notification-area icon remains resident", "macOS: menu-bar icon remains resident", "Linux: the desktop launcher starts/reuses the server and opens the browser GUI"],
      },
      {
        id: "tui",
        title: "Open the TUI",
        body: ["Run Takokit without a subcommand. The TUI uses the same registry, workspaces, sessions, models, and server state as the other interfaces."],
        commands: ["tako"],
      },
      {
        id: "browser-gui",
        title: "Open the browser GUI",
        body: ["The GUI is served locally from the Takokit server. It is not Electron, Tauri, or an embedded WebView."],
        commands: ["tako gui"],
        code: "http://127.0.0.1:5050/gui",
      },
      {
        id: "server",
        title: "Server controls",
        commands: ["tako start", "tako server status", "tako stop", "tako serve"],
        note: "tako serve is the developer-owned foreground server. The resident app must not kill a separately started foreground server merely because the resident exits.",
      },
    ],
  },
  "pull-first-model": {
    title: "Pull your first model",
    intro: "Pull resolves the model reference, runner, adapter, managed runtime, pinned artifacts, license boundary, and readiness checks.",
    sections: [
      {
        id: "choose",
        title: "Choose a model",
        body: ["Use the Models page or CLI catalog to compare task, hardware, size, license, and support status before downloading anything."],
        commands: ["tako models", "tako plan kokoro", "tako plan whisper-tiny"],
      },
      {
        id: "tts",
        title: "First TTS model",
        body: ["Kokoro is the representative lightweight speech model used by the cross-platform package acceptance suite."],
        commands: ["tako pull kokoro", "tako show kokoro"],
      },
      {
        id: "stt",
        title: "First STT model",
        body: ["Whisper Tiny is the representative lightweight transcription model used by the Linux/macOS package acceptance suite."],
        commands: ["tako pull whisper-tiny", "tako show whisper-tiny"],
      },
      {
        id: "offline",
        title: "After the pull",
        body: ["Installed model artifacts and managed runtimes are reused locally. A later execution does not redownload the same verified artifacts unless the selected model/runtime requires recovery or an update."],
      },
    ],
  },
  "generate-speech": {
    title: "Generate speech",
    intro: "Use any installed model that declares text-to-speech capability. CLI, GUI, TUI, and API all execute through the same runtime planner.",
    sections: [
      {
        id: "cli",
        title: "CLI",
        commands: ["tako pull kokoro", "tako speak \"Hello from Takokit\" --model kokoro"],
      },
      {
        id: "run",
        title: "Unified run command",
        commands: ["tako run kokoro \"Hello from the unified run command\""],
      },
      {
        id: "outputs",
        title: "Outputs",
        body: ["Output files and session events are written under the active workspace's .tako directory. Use --workspace to choose a project explicitly."],
        commands: ["tako --workspace ./voice-project speak \"Project narration\" --model kokoro"],
      },
      {
        id: "voice-options",
        title: "Voice controls",
        body: ["Compatible models may expose preset voices, local voice profiles, language, reference text, or natural-language delivery instructions. Unsupported options are rejected rather than silently ignored."],
      },
    ],
  },
  "transcribe-audio": {
    title: "Transcribe audio",
    intro: "Use an installed speech-to-text model to turn local audio into text while keeping the workflow and outputs local.",
    sections: [
      {
        id: "cli",
        title: "CLI",
        commands: ["tako pull whisper-tiny", "tako transcribe recording.wav --model whisper-tiny"],
      },
      {
        id: "run",
        title: "Unified run command",
        commands: ["tako run whisper-tiny --file ./recording.wav"],
      },
      {
        id: "api",
        title: "OpenAI-compatible audio endpoint",
        body: ["Takokit's /v1 compatibility surface includes multipart audio transcription. See the API guide for upload limits and supported formats."],
        code: "POST http://127.0.0.1:5050/v1/audio/transcriptions",
      },
      {
        id: "privacy",
        title: "Local processing boundary",
        body: ["Takokit does not require sending your recording to a hosted inference API. Network access is still used when you explicitly download models, registries, or updates."],
      },
    ],
  },
};
