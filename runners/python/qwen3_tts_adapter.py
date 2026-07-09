"""Takokit's JSON stdin/stdout adapter for the official qwen-tts package.

Takokit pulls every declared model file into ~/.takokit/models/qwen3-tts before
this adapter is invoked. The adapter only loads that local directory and writes
the requested WAV; it never calls a hosted inference service.
"""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

import soundfile as sf


def fail(message: str) -> None:
    print(json.dumps({"ok": False, "error": message}), flush=True)
    raise SystemExit(1)


def main() -> None:
    try:
        request = json.load(sys.stdin)
        text = str(request["input"]).strip()
        if not text:
            fail("speech input cannot be empty")
        model_dir = Path(request["model_dir"])
        output_path = Path(request["output_path"])
        if not model_dir.is_dir():
            fail(f"Qwen3-TTS model directory is missing: {model_dir}")

        # Some upstream packages print non-JSON progress and feature warnings.
        # Preserve our stdout contract by routing their output to stderr.
        with redirect_stdout(sys.stderr):
            import torch
            from qwen_tts import Qwen3TTSModel

            if torch.cuda.is_available():
                device = "cuda:0"
                dtype = torch.bfloat16
            else:
                device = "cpu"
                dtype = torch.float32

            model = Qwen3TTSModel.from_pretrained(
                str(model_dir), device_map=device, dtype=dtype
            )
            speaker = request.get("voice") or "Ryan"
            wavs, sample_rate = model.generate_custom_voice(
                text=text,
                language="English",
                speaker=speaker,
                instruct="",
            )
        output_path.parent.mkdir(parents=True, exist_ok=True)
        sf.write(str(output_path), wavs[0], sample_rate)
        print(
            json.dumps(
                {
                    "ok": True,
                    "output_path": str(output_path),
                    "bytes": output_path.stat().st_size,
                    "sample_rate": int(sample_rate),
                    "voice": speaker,
                    "device": device,
                }
            ),
            flush=True,
        )
    except Exception as error:
        fail(str(error))


if __name__ == "__main__":
    main()
