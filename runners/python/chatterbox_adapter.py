"""Takokit adapter for locally pulled Chatterbox checkpoints."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Chatterbox adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    if not model_dir.is_dir():
        raise FileNotFoundError(f"Chatterbox snapshot is missing: {model_dir}")
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import torch
    import torchaudio
    from chatterbox.tts import ChatterboxTTS

    if torch.cuda.is_available():
        device = "cuda"
    elif torch.backends.mps.is_available():
        device = "mps"
    else:
        device = "cpu"

    model = ChatterboxTTS.from_local(model_dir, device=device)
    voice = request.get("voice")
    options = {}
    if voice and voice != "default":
        reference = Path(str(voice)).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
        options["audio_prompt_path"] = str(reference)
    waveform = model.generate(text, **options)
    torchaudio.save(str(output_path), waveform.detach().cpu(), model.sr)
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"Chatterbox did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(model.sr),
        voice=voice or "default",
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
