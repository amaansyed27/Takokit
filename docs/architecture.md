# Architecture

Takokit is organized as:

```txt
Rust CLI
  |
Rust daemon / local HTTP API
  |
local browser GUI
  |
package registry + installed manifests
  |
model adapters + future runners
```

The GUI is a normal React + TypeScript + Vite web app in `apps/gui`. The daemon serves the built GUI at `/gui`, and `takokit gui` opens `http://127.0.0.1:5050/gui`.

## Crates

- `takokit-core`: API request/response types, shared model metadata, runtime config, and typed errors.
- `takokit-server`: Axum router, daemon entry point, API handlers, and static GUI serving.
- `takokit-package`: model manifests, runner manifests, local mock registry lookup, and installed manifest registry.
- `takokit-models`: adapter traits and mock TTS implementation.
- `takokit-audio`: audio helpers and valid mock WAV writing.
- `takokit-store`: local filesystem layout under `~/.takokit`.
- `takokit-safety`: consent, license, and safety policy primitives.

## Apps

- `apps/cli`: Clap CLI. It starts the daemon, opens the GUI, and calls package/model functionality.
- `apps/gui`: Browser GUI. It calls the local API through `src/lib/api.ts`.

## Package Manager Boundary

Takokit should eventually own model and runner installation:

```txt
model manifest -> runner manifest -> content-addressed blobs -> installed registry -> adapter dispatch
```

The current implementation installs local mock manifests only. It does not download model weights, install Python packages, or execute real runners.

## Runner Isolation

Runner types are modeled now for future backends:

- `native`
- `onnx`
- `whispercpp`
- `python-managed`
- `external`

Runners must communicate through explicit contracts. UI and API callers should only see model IDs, voice IDs, request contracts, package metadata, and typed errors.
