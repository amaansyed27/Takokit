# Artifacts

Takokit model pulls are explicit. The CLI, API, and GUI must not download model files during list/show/status requests.

Artifact-backed pulls use this flow:

```txt
artifact manifest
  -> download into ~/.takokit/cache/downloads/
  -> validate byte size when bytes is present
  -> compute SHA256 before install
  -> compare with manifest checksum
  -> delete temporary file on byte/checksum mismatch
  -> move verified file into ~/.takokit/blobs/sha256/<hash>
  -> stage installed model manifest and installed-model record
  -> move staged manifest and record into final paths
```

Unverified files are never placed in the final blob directory. Failed artifact-backed installs must not leave a final model manifest or installed-model record, so `is_model_installed(model)` remains false after a failed first install.

Expected artifact failures return typed errors:

- `artifact_url_missing`
- `artifact_checksum_missing`
- `artifact_download_failed`
- `artifact_checksum_mismatch`
- `artifact_install_failed`
- `artifact_missing`
- `artifact_not_downloaded`
- `artifact_config_invalid`

## Manifest Shape

```toml
[artifacts]
metadata_only = false

[[artifacts.weights]]
name = "en_US-lessac-medium.onnx"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
sha256 = "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f"
bytes = 63201294
role = "model"

[[artifacts.configs]]
name = "en_US-lessac-medium.onnx.json"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
sha256 = "efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0"
bytes = 4885
role = "config"
```

If a manifest has artifact URLs but lacks checksums, normal artifact install fails unless the manifest or request is explicitly metadata-only. Takokit does not commit fake SHA256 values.

Metadata-only installs still write the manifest copy and installed-model record because they intentionally skip artifact download and verification.

## Current State

`piper-lessac` now records verified Piper Lessac medium ONNX model/config artifacts. `takokit pull piper-lessac` downloads both files, validates byte sizes and SHA256 values, stores them under `~/.takokit/blobs/sha256/`, and writes installed artifact records with `downloaded = true`.

ONNX TTS execution remains blocked. After valid model and runner metadata are installed, `piper-lessac` resolves artifacts/config and returns typed `piper_text_frontend_not_implemented`; generic ONNX models such as metadata-only Kokoro still return typed `inference_not_implemented`.

The Piper ONNX runner scaffold now reads the installed `piper-lessac` artifact records, resolves the local blob paths for `en_US-lessac-medium.onnx` and `en_US-lessac-medium.onnx.json`, and parses the JSON config before stopping at the typed text-frontend boundary.
