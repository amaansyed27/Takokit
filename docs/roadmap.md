# Roadmap

This file tracks near-term direction without phase gates. The source of truth for current work is [../TASKS.md](../TASKS.md).

## Next Useful Increments

- Make package pull write a fuller installed model record, including artifact slots and checksum placeholders.
- Add installed runner registry support.
- Add a runner selection layer that returns typed unsupported errors before execution.
- Add config loading from `~/.takokit/config.toml`.
- Add browser GUI controls for pulling/removing manifests through the API.
- Add API tests for model detail, runner listing, pull, and delete.
- Add real artifact download only after checksum verification is in place.

## Keep Out For Now

- Tauri app scaffolding.
- Fake Kokoro or Whisper inference.
- Hidden cloud calls.
- Model-specific dependency instructions as the primary user path.
