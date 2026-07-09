"""Takokit's JSON stdin/stdout adapter for the MIT-licensed kokoro-onnx package.

This file is copied into ~/.takokit/runners/onnx/adapters during runner install.
It deliberately owns no model download logic: Rust verifies and pulls model assets before
invoking the adapter.
"""

from __future__ import annotations

import array
import json
import sys
import wave
from pathlib import Path

from kokoro_onnx import Kokoro


def fail(message: str) -> None:
    print(json.dumps({"ok": False, "error": message}), flush=True)
    raise SystemExit(1)


def main() -> None:
    try:
        request = json.load(sys.stdin)
        text = str(request["input"]).strip()
        if not text:
            fail("speech input cannot be empty")

        model_path = Path(request["model_path"])
        voices_path = Path(request["voices_path"])
        output_path = Path(request["output_path"])
        if not model_path.is_file():
            fail(f"Kokoro model artifact is missing: {model_path}")
        if not voices_path.is_file():
            fail(f"Kokoro voices artifact is missing: {voices_path}")

        # The adapter's default voice is an upstream English voice. A caller can
        # select another documented Kokoro voice explicitly through --voice.
        voice = request.get("voice") or "af_bella"
        kokoro = Kokoro(str(model_path), str(voices_path))
        samples, sample_rate = kokoro.create(text, voice=voice, speed=1.0, lang="en-us")

        output_path.parent.mkdir(parents=True, exist_ok=True)
        pcm = array.array("h")
        for sample in samples:
            value = max(-1.0, min(1.0, float(sample)))
            pcm.append(int(value * 32767.0))
        if sys.byteorder != "little":
            pcm.byteswap()
        with wave.open(str(output_path), "wb") as output:
            output.setnchannels(1)
            output.setsampwidth(2)
            output.setframerate(int(sample_rate))
            output.writeframes(pcm.tobytes())

        print(
            json.dumps(
                {
                    "ok": True,
                    "output_path": str(output_path),
                    "bytes": output_path.stat().st_size,
                    "sample_rate": int(sample_rate),
                    "voice": voice,
                }
            ),
            flush=True,
        )
    except Exception as error:  # Adapter errors are returned as structured output.
        fail(str(error))


if __name__ == "__main__":
    main()
