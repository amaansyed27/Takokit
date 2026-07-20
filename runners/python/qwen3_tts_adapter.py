"""Takokit JSON adapter for the official Qwen3-TTS checkpoints."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

import soundfile as sf


def fail(message: str) -> None:
    print(json.dumps({"ok": False, "error": message}), flush=True)
    raise SystemExit(1)


def load_reference(value: object) -> Path:
    if not value:
        fail("this Qwen3-TTS checkpoint requires a consent-backed reference voice")
    reference = Path(str(value)).expanduser().resolve()
    if not reference.is_file():
        fail(f"voice reference does not exist: {reference}")
    return reference


def main() -> None:
    try:
        request = json.load(sys.stdin)
        text = str(request.get("input") or "").strip()
        if request.get("operation") != "speech" or not text:
            fail("Qwen3-TTS requires a non-empty speech request")

        model_id = str(request["model_id"])
        model_dir = Path(request["model_dir"]).expanduser().resolve()
        output_path = Path(request["output_path"]).expanduser().resolve()
        if not model_dir.is_dir():
            fail(f"Qwen3-TTS model directory is missing: {model_dir}")

        with redirect_stdout(sys.stderr):
            import torch
            from qwen_tts import Qwen3TTSModel

            device = "cuda:0" if torch.cuda.is_available() else "cpu"
            dtype = torch.bfloat16 if torch.cuda.is_available() else torch.float32
            model = Qwen3TTSModel.from_pretrained(
                str(model_dir),
                device_map=device,
                dtype=dtype,
            )
            voice = request.get("voice")
            if model_id.endswith("-base"):
                reference = load_reference(voice)
                wavs, sample_rate = model.generate_voice_clone(
                    text=text,
                    language="English",
                    ref_audio=str(reference),
                    ref_text=None,
                    x_vector_only_mode=True,
                )
                voice_label = str(reference)
            elif model_id.endswith("voice-design"):
                instruction = str(
                    voice or "A clear, natural, warm and expressive speaking voice."
                )
                wavs, sample_rate = model.generate_voice_design(
                    text=text,
                    language="English",
                    instruct=instruction,
                )
                voice_label = instruction
            else:
                speaker = str(voice or "Ryan")
                wavs, sample_rate = model.generate_custom_voice(
                    text=text,
                    language="English",
                    speaker=speaker,
                    instruct="",
                )
                voice_label = speaker

        output_path.parent.mkdir(parents=True, exist_ok=True)
        sf.write(str(output_path), wavs[0], sample_rate)
        if not output_path.is_file() or output_path.stat().st_size <= 44:
            fail(f"Qwen3-TTS did not create a valid WAV at {output_path}")
        print(
            json.dumps(
                {
                    "ok": True,
                    "output_path": str(output_path),
                    "bytes": output_path.stat().st_size,
                    "sample_rate": int(sample_rate),
                    "voice": voice_label,
                    "device": device,
                }
            ),
            flush=True,
        )
    except SystemExit:
        raise
    except Exception as error:
        fail(f"{type(error).__name__}: {error}")


if __name__ == "__main__":
    main()
