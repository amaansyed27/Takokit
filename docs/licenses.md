# Licenses

Takokit must keep runtime support claims separate from upstream model license claims. A model can be known to Takokit and still be metadata-only, non-commercial, or blocked.

## Current Notes

- `whisper-base`: MIT model/runtime path through whisper.cpp. Takokit uses the verified ggml artifact and official whisper.cpp binary release.
- `piper-lessac`: voice artifacts are verified, but current Piper runtime references include GPL-licensed upstream runtime code. Takokit must not vendor GPL runtime code without an explicit adapter boundary and release decision.
- `kokoro`: Takokit pins and verifies the v1.0 INT8 ONNX model and voice bundle from the `kokoro-onnx` release. The model weights are Apache-2.0 and `kokoro-onnx` is MIT, but its upstream phonemization stack declares `espeakng-loader` and `phonemizer-fork`. Takokit invokes that stack only as an isolated JSON subprocess adapter; it does not vendor GPL runtime code into the Rust binary. The model card/GUI carries this runtime-boundary warning.
- `qwen3-tts`: Takokit pins the official Apache-2.0 0.6B CustomVoice revision, verifies every pulled file, and installs the Apache-2.0 `qwen-tts` package inside its own managed environment. The currently supported speak path selects built-in voices only; reference-audio cloning remains consent-blocked.
- `chatterbox`: upstream project is MIT. Takokit has not verified artifacts or installed the adapter yet.
- `f5-tts`: code is MIT, but common pretrained weights are CC-BY-NC. Treat runtime manifests as non-commercial until a commercial-safe artifact is selected and verified.
- `cosyvoice2`, `fish-speech`, `dia`, `sensevoice`, `parakeet`, `canary`, `openvoice`, `gpt-sovits`, and `rvc`: keep conservative license/commercial labels until Takokit verifies exact artifacts, dependency licenses, and redistribution/runtime terms.
- `voxtral` and `qwen3-omni`: remain metadata-only. They need exact selected checkpoints plus a managed Transformers adapter; do not infer commercial permission from the framework package alone.

## Policy

- Do not add fake artifact URLs or fake SHA256 values.
- Do not mark a model executable unless Takokit can run it locally.
- Do not auto-install blocked-license or non-commercial weights as normal supported models.
- Keep GPL or unclear runtime code behind an explicit adapter/process boundary and document the risk.
- Require consent gates for voice cloning and conversion adapters.
- Preserve upstream attribution and pinned revision metadata in the local model record for every supported pull.
