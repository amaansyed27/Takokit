"""Takokit adapter for a locally pulled F5-TTS checkpoint."""

from __future__ import annotations

import json
import sys
from importlib.resources import files
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("F5-TTS adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")

    model_dir = Path(request["model_dir"]).expanduser().resolve()
    cache_dir = Path(request["cache_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    checkpoint = model_dir / "F5TTS_v1_Base" / "model_1250000.safetensors"
    if not checkpoint.is_file():
        candidates = list(model_dir.rglob("model_1250000.safetensors"))
        checkpoint = candidates[0] if candidates else checkpoint
    if not checkpoint.is_file():
        raise FileNotFoundError(f"F5-TTS checkpoint is missing below {model_dir}")
    vocab_candidates = list(model_dir.rglob("vocab.txt"))
    vocab = str(vocab_candidates[0]) if vocab_candidates else ""
    output_path.parent.mkdir(parents=True, exist_ok=True)

    from f5_tts.api import F5TTS

    voice = request.get("voice")
    if voice and voice != "default":
        reference = Path(str(voice)).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
    else:
        reference = Path(
            str(files("f5_tts").joinpath("infer/examples/basic/basic_ref_en.wav"))
        )
    engine = F5TTS(
        model="F5TTS_v1_Base",
        ckpt_file=str(checkpoint),
        vocab_file=vocab,
        hf_cache_dir=str(cache_dir / "f5-tts"),
    )
    _, sample_rate, _ = engine.infer(
        ref_file=str(reference),
        ref_text=str(request.get("reference_text") or ""),
        gen_text=text,
        file_wave=str(output_path),
        seed=None,
    )
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"F5-TTS did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(sample_rate),
        voice=voice or "default",
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
