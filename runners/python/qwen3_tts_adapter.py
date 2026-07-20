"""Takokit JSON adapter for every official Qwen3-TTS checkpoint type."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

import soundfile as sf


CUSTOM_MODELS = {"qwen3-tts", "qwen3-tts-1.7b-custom"}
BASE_MODELS = {"qwen3-tts-0.6b-base", "qwen3-tts-1.7b-base"}
VOICE_DESIGN_MODELS = {"qwen3-tts-1.7b-voice-design"}


def fail(message: str) -> None:
    print(json.dumps({"ok": False, "error": message}), flush=True)
    raise SystemExit(1)


def main() -> None:
    try:
        request = json.load(sys.stdin)
        if request.get("operation") != "speech":
            fail("Qwen3-TTS adapter only supports speech")
        model_id = str(request["model_id"])
        text = str(request["input"]).strip()
        if not text:
            fail("speech input cannot be empty")
        model_dir = Path(request["model_dir"])
        output_path = Path(request["output_path"])
        if not model_dir.is_dir():
            fail(f"Qwen3-TTS model directory is missing: {model_dir}")

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
            language = request.get("language") or "Auto"
            instruction = request.get("instruction") or ""
            voice = request.get("voice")
            reference_text = request.get("reference_text")

            if model_id in CUSTOM_MODELS:
                speaker = voice or "Ryan"
                wavs, sample_rate = model.generate_custom_voice(
                    text=text,
                    language=language,
                    speaker=speaker,
                    instruct=instruction,
                )
                reported_voice = speaker
            elif model_id in BASE_MODELS:
                if not voice:
                    fail(
                        "Qwen3-TTS Base requires --voice with a consent-backed voice profile or reference audio path"
                    )
                kwargs = {
                    "text": text,
                    "language": language,
                    "ref_audio": voice,
                    "x_vector_only_mode": not bool(reference_text),
                }
                if reference_text:
                    kwargs["ref_text"] = reference_text
                wavs, sample_rate = model.generate_voice_clone(**kwargs)
                reported_voice = voice
            elif model_id in VOICE_DESIGN_MODELS:
                if not instruction:
                    fail("Qwen3-TTS VoiceDesign requires --instruction")
                wavs, sample_rate = model.generate_voice_design(
                    text=text,
                    language=language,
                    instruct=instruction,
                )
                reported_voice = "voice-design"
            else:
                fail(f"unsupported Qwen3-TTS model id: {model_id}")

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
                    "voice": reported_voice,
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
