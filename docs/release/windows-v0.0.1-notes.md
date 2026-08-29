# Takokit 0.0.1 — Windows public-test candidate

Takokit 0.0.1 is the first Windows distribution candidate that is intended to behave as an installed application rather than a repository build.

## Included

- `tako.exe` CLI and Ratatui TUI
- managed `takokit.exe` daemon/API
- packaged React web GUI served locally by the Takokit daemon and opened with `tako gui`
- `takokit-updater.exe` staged updater helper
- canonical model/runner registry
- managed runner and Python-adapter bootstrap architecture
- TTS, STT, voice cloning and conversion flows from Slices 1–2
- Slice 3 Advanced RVC preparation, training, recovery, managed trained voices, conversion and `.takovoice` package flows
- per-user Windows installer and portable ZIP
- provider-aware durable checkpoint ownership and safe storage cleanup
- reset/uninstall safety contracts
- signed release metadata and staged Windows updater

The Windows package does not include a separate native desktop/WebView host. The GUI is the existing React application served at the local daemon `/gui` route and opened in the user's default browser.

## Test-signing notice

Workflow artifacts produced without the private application release signing secret are deliberately marked as **test fixtures** and use Takokit's deterministic non-production test key. They are suitable only for the Slice 4 updater acceptance tests using the explicit test-channel flags. They are not public-release trust artifacts.

The application release signing key is separate from all user `.takovoice` signing keys.

## Known release-candidate limitations

- Windows x86_64 is the only packaged platform in this slice.
- Linux and macOS distribution are deferred.
- The comprehensive public website/docs/API redesign is deferred.
- Models and managed runner dependencies continue to download after installation according to the registry; model weights are not bundled into the application installer.
- A production GitHub Release is intentionally not published until Windows acceptance is complete.
