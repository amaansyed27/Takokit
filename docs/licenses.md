# Licenses

Takokit must keep runtime support claims separate from upstream model license claims. A model can be known to Takokit and still be metadata-only, non-commercial, or blocked.

## Current Notes

- `whisper-base`: MIT model/runtime path through whisper.cpp. Takokit uses the verified ggml artifact and official whisper.cpp binary release.
- `piper-lessac`: voice artifacts are verified, but current Piper runtime references include GPL-licensed upstream runtime code. Takokit must not vendor GPL runtime code without an explicit adapter boundary and release decision.
- `qwen3-tts`: upstream Qwen3-TTS material indicates an Apache 2.0 model family and a `qwen-tts` Python package path. Takokit has not installed or executed this adapter yet.
- `chatterbox`: upstream project is MIT. Takokit has not verified artifacts or installed the adapter yet.
- `f5-tts`: code is MIT, but common pretrained weights are CC-BY-NC. Treat runtime manifests as non-commercial until a commercial-safe artifact is selected and verified.
- `cosyvoice2`, `fish-speech`, `dia`, `sensevoice`, `parakeet`, `canary`, `openvoice`, `gpt-sovits`, and `rvc`: keep conservative license/commercial labels until Takokit verifies exact artifacts, dependency licenses, and redistribution/runtime terms.

## Policy

- Do not add fake artifact URLs or fake SHA256 values.
- Do not mark a model executable unless Takokit can run it locally.
- Do not auto-install blocked-license or non-commercial weights as normal supported models.
- Keep GPL or unclear runtime code behind an explicit adapter/process boundary and document the risk.
- Require consent gates for voice cloning and conversion adapters.
