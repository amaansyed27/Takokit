# Python Runner

This folder is reserved for isolated PyTorch model runners such as Kokoro, Chatterbox, GPT-SoVITS, and Qwen3-TTS.

Rules:

- Keep Python dependencies out of the Rust server process.
- Communicate through explicit request/response contracts.
- Do not make hidden cloud calls.
- Record model license and hardware requirements before execution.

