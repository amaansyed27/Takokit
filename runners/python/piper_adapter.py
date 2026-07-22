"""Takokit adapter for the official Piper Python runtime."""

from __future__ import annotations

import json
import sys
import wave
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Piper adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")

    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    model_path = next(model_dir.glob("*.onnx"), None)
    if model_path is None:
        raise FileNotFoundError(f"Piper ONNX model is missing below {model_dir}")
    config_path = model_path.with_suffix(model_path.suffix + ".json")
    if not config_path.is_file():
        configs = list(model_dir.glob("*.json"))
        config_path = configs[0] if configs else config_path
    if not config_path.is_file():
        raise FileNotFoundError(f"Piper voice config is missing below {model_dir}")

    # This adapter is installed as piper.py, so remove its directory from
    # module lookup before importing the separately installed piper package.
    adapter_dir = Path(__file__).resolve().parent
    sys.path = [
        entry
        for entry in sys.path
        if Path(entry or ".").resolve() != adapter_dir
    ]
    from piper import PiperVoice

    voice = PiperVoice.load(str(model_path), config_path=str(config_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(output_path), "wb") as wav_file:
        voice.synthesize_wav(text, wav_file)
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"Piper did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(voice.config.sample_rate),
        voice="en_US-lessac-medium",
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
