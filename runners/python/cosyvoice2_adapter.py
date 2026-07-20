"""Takokit adapter for the official CosyVoice2 repository API."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def load_model(model_dir: Path):
    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    sys.path.insert(0, str(source / "third_party" / "Matcha-TTS"))
    from cosyvoice.cli.cosyvoice import AutoModel

    return AutoModel(model_dir=str(model_dir))


def collect_audio(chunks):
    import torch

    tensors = [chunk["tts_speech"].detach().cpu() for chunk in chunks]
    if not tensors:
        raise RuntimeError("CosyVoice2 returned no audio chunks")
    return torch.cat(tensors, dim=1)


def main() -> None:
    request = json.load(sys.stdin)
    operation = request.get("operation")
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    if not model_dir.is_dir():
        raise FileNotFoundError(f"CosyVoice2 model directory is missing: {model_dir}")
    model = load_model(model_dir)

    if operation == "speech":
        text = str(request.get("input") or "").strip()
        if not text:
            raise ValueError("speech input cannot be empty")
        reference = request.get("voice")
        if not reference:
            raise ValueError(
                "CosyVoice2 requires --voice with a consent-backed reference sample"
            )
        reference_path = Path(reference).expanduser().resolve()
        if not reference_path.is_file():
            raise FileNotFoundError(f"reference audio does not exist: {reference_path}")
        instruction = str(request.get("instruction") or "").strip()
        reference_text = str(request.get("reference_text") or "").strip()
        if instruction:
            prompt = instruction
            if "<|endofprompt|>" not in prompt:
                prompt += "<|endofprompt|>"
            chunks = model.inference_instruct2(
                text, prompt, str(reference_path), stream=False
            )
        elif reference_text:
            chunks = model.inference_zero_shot(
                text, reference_text, str(reference_path), stream=False
            )
        else:
            chunks = model.inference_cross_lingual(
                text, str(reference_path), stream=False
            )
    elif operation == "convert":
        source_audio = Path(request["audio_path"]).expanduser().resolve()
        target_voice = Path(request["target_voice"]).expanduser().resolve()
        if not source_audio.is_file() or not target_voice.is_file():
            raise FileNotFoundError("CosyVoice2 conversion requires source and target WAV files")
        chunks = model.inference_vc(str(source_audio), str(target_voice), stream=False)
    else:
        raise ValueError(f"CosyVoice2 does not support operation: {operation}")

    import torchaudio

    audio = collect_audio(chunks)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    torchaudio.save(str(output_path), audio, int(model.sample_rate))
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"CosyVoice2 did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(model.sample_rate),
        voice=request.get("voice") or request.get("target_voice"),
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
