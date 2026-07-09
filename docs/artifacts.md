# Artifacts

Takokit model pulls are explicit. The CLI, API, and GUI must not download model files during list/show/status requests.

Artifact-backed pulls use this flow:

```txt
artifact manifest
  -> download into ~/.takokit/cache/downloads/
  -> compute SHA256 before install
  -> compare with manifest checksum
  -> delete temporary file on mismatch
  -> move verified file into ~/.takokit/blobs/sha256/<hash>
  -> write installed artifact record with local_path and downloaded=true
```

Unverified files are never placed in the final blob directory. Expected artifact failures return typed errors:

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

## Current State

`piper-lessac` now records the real Piper Lessac medium ONNX model/config artifact shape, but its checksums are intentionally blank and the manifest is marked `metadata_only = true`. `takokit pull piper-lessac` installs metadata only until those checksums are finalized.

ONNX execution remains unimplemented. After valid model and runner metadata are installed, Piper/Kokoro-style requests still return typed `inference_not_implemented`.
