# Takokit 0.1.0 — first public Windows release

Takokit 0.1.0 is the first public Windows distribution of the local voice AI runtime.

## Included

- `tako.exe` CLI and interactive TUI
- managed `takokit.exe` local daemon
- React GUI served locally by the daemon and opened in the default browser with `tako gui`
- `takokit-updater.exe` updater helper
- TTS, STT, instant voice cloning, voice conversion, and Advanced RVC / Custom Voice Studio workflows
- model, runner, adapter, and managed Python runtime orchestration
- per-user Windows installer and portable ZIP
- production-signable release manifest, detached signature, checksums, and build provenance
- PowerShell bootstrap that resolves stable metadata and verifies the canonical installer SHA-256

Takokit does not include Tauri, a WebView desktop wrapper, or a root `Takokit.exe` GUI shell. The installed application contains `bin\tako.exe`, `bin\takokit.exe`, and `bin\takokit-updater.exe`.

## Platform and packaging notes

- Windows 10/11 x86_64 is the only packaged platform in v0.1.0.
- Linux and macOS packages are coming later.
- Model weights and managed runner dependencies download after installation as needed; they are not bundled in the installer.
- Application release-manifest signing is separate from Windows Authenticode and from user `.takovoice` signing keys.

## Deferred to later releases

- first-class `tako serve`
- OpenAI-compatible audio API guarantees
- Windows tray integration
- update notification UX
- Linux and macOS packages
