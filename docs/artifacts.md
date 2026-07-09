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

## Manifest Shape

```toml
[artifacts]
metadata_only = false

[[artifacts.weights]]
name = "en_US-lessac-medium.onnx"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
sha256 = "<verified sha256>"
bytes = 63200000
role = "model"

[[artifacts.configs]]
name = "en_US-lessac-medium.onnx.json"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
sha256 = "<verified sha256>"
bytes = 4890
role = "config"
```

If a manifest has artifact URLs but lacks checksums, normal artifact install fails unless the manifest or request is explicitly metadata-only. Takokit does not commit fake SHA256 values.

Metadata-only installs still write the manifest copy and installed-model record because they intentionally skip artifact download and verification.

## Current State

`piper-lessac` now records the real Piper Lessac medium ONNX model/config artifact shape, but its checksums are intentionally blank and the manifest is marked `metadata_only = true`. `takokit pull piper-lessac` installs metadata only until those checksums are finalized.

ONNX execution remains unimplemented. After valid model and runner metadata are installed, Piper/Kokoro-style requests still return typed `inference_not_implemented`.
