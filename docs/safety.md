# Safety

Takokit is local-first, but voice tooling still needs clear safety boundaries.

## Voice Cloning Consent

Voice cloning should require explicit user confirmation that they have permission to use the source voice. Consent state should be recorded with the voice profile.

Voice cloning and conversion commands must not silently run against arbitrary audio. Before adapters such as OpenVoice, RVC, GPT-SoVITS, Chatterbox cloning, or Qwen3-TTS cloning become executable, the CLI/API must require an explicit consent flag or recorded voice-profile consent.

Plain TTS through Qwen3-TTS CustomVoice uses only the upstream named built-in voices. Takokit does not expose its reference-audio cloning path until a consent-gated voice-profile command is implemented.

## Model Licenses

Every model entry should include a license label before download or execution. Some open-source voice models are not commercial-safe.

Non-commercial weights must not be marked commercial-safe. Models with GPL runtime risk or unclear redistribution rights must stay metadata-only or blocked until Takokit has an adapter boundary and license note.

## Watermarking And Metadata

Future output pipelines should support watermarking or metadata tags that identify generated audio when practical.

## Local-First Privacy

Takokit should not make hidden cloud calls. Audio, transcripts, voice profiles, and training datasets should remain local unless the user explicitly exports or uploads them.

## User Control

Model downloads should be explicit. Training should never start from user audio without an explicit action.

## Conversion And Cloning Gate

`openvoice`, `rvc`, and `gpt-sovits` remain non-executable even when their runner contract is installed. A future conversion or cloning request must require `--consent` and a local voice profile whose consent record names the authorized speaker and source audio. The API must reject arbitrary reference-audio paths without the same recorded consent boundary.

## Logs

Logs should help debug local runtime behavior without silently collecting private audio content.
