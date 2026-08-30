# Takokit v0.2.0 — local audio API and Windows tray

Takokit v0.2.0 adds a first-class local audio API to the Windows distribution. Start it in the foreground with `tako serve`, or manage the same runtime with `tako server` and the Windows system tray.

The OpenAI-compatible audio subset supports `GET /v1/models`, text-to-speech, and audio transcription with standard Python and JavaScript OpenAI SDK clients. Takokit-native model, runner, voice, cloning, conversion, RVC, session, diagnostics, and lifecycle routes remain available under `/api/v1`. OpenAPI and API examples document both surfaces.

The Windows tray exposes server status and controls, the GUI and canonical API URL, update checks, per-user startup integration, and version information. Existing v0.1.0 Windows installations can update to v0.2.0 through the signed updater path while preserving local models, voices, settings, and project state.

This release does not implement chat completions, the Responses API, embeddings, image generation, Linux or macOS packages, or a native desktop/WebView GUI. The GUI remains browser-based and locally served.
