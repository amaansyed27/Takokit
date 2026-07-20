"""Takokit adapter for OpenVoice V2 speech cloning and tone-colour conversion."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def load_runtime(model_dir: Path):
    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    import torch
    from openvoice import se_extractor
    from openvoice.api import ToneColorConverter

    checkpoint_root = model_dir / "checkpoints_v2"
    if not checkpoint_root.is_dir():
        checkpoint_root = model_dir
    converter_root = checkpoint_root / "converter"
    config = converter_root / "config.json"
    checkpoint = converter_root / "checkpoint.pth"
    if not config.is_file() or not checkpoint.is_file():
        raise FileNotFoundError(
            f"OpenVoice V2 converter files are missing below {checkpoint_root}"
        )
    device = "cuda:0" if torch.cuda.is_available() else "cpu"
    converter = ToneColorConverter(str(config), device=device)
    converter.load_ckpt(str(checkpoint))
    return checkpoint_root, converter, se_extractor, device


def target_embedding(reference: Path, converter, se_extractor, cache_dir: Path):
    if not reference.is_file():
        raise FileNotFoundError(f"target voice reference does not exist: {reference}")
    embedding, _ = se_extractor.get_se(
        str(reference), converter, target_dir=str(cache_dir), vad=True
    )
    return embedding


def run_speech(request: dict, model_dir: Path, output_path: Path) -> tuple[int, str]:
    checkpoint_root, converter, se_extractor, device = load_runtime(model_dir)
    text = str(request.get("input") or "").strip()
    reference = request.get("voice")
    if not text:
        raise ValueError("speech input cannot be empty")
    if not reference:
        raise ValueError(
            "OpenVoice requires --voice with a consent-backed reference sample"
        )
    reference_path = Path(reference).expanduser().resolve()
    language = str(request.get("language") or "EN").upper()
    speaker_key = "EN-US" if language.startswith("EN") else language

    from melo.api import TTS

    base_model = TTS(language=language, device=device)
    speaker_ids = base_model.hps.data.spk2id
    speaker_id = speaker_ids.get(speaker_key, next(iter(speaker_ids.values())))
    cache_dir = Path(request["cache_dir"]).expanduser().resolve() / "openvoice"
    cache_dir.mkdir(parents=True, exist_ok=True)
    target_se = target_embedding(reference_path, converter, se_extractor, cache_dir)
    source_se_path = (
        checkpoint_root
        / "base_speakers"
        / "ses"
        / ("en-us.pth" if language.startswith("EN") else f"{language.lower()}.pth")
    )
    if not source_se_path.is_file():
        fallback = checkpoint_root / "base_speakers" / "ses" / "en-default.pth"
        source_se_path = fallback if fallback.is_file() else source_se_path
    if not source_se_path.is_file():
        raise FileNotFoundError(f"OpenVoice source speaker embedding is missing: {source_se_path}")
    import torch

    source_se = torch.load(str(source_se_path), map_location=device)
    with tempfile.TemporaryDirectory(prefix="takokit-openvoice-") as temp:
        base_audio = Path(temp) / "base.wav"
        base_model.tts_to_file(text, speaker_id, str(base_audio), speed=1.0)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        converter.convert(
            audio_src_path=str(base_audio),
            src_se=source_se,
            tgt_se=target_se,
            output_path=str(output_path),
            message="@Takokit",
        )
    return 44100, str(reference_path)


def run_conversion(request: dict, model_dir: Path, output_path: Path) -> tuple[int, str]:
    _, converter, se_extractor, _ = load_runtime(model_dir)
    source_audio = Path(request["audio_path"]).expanduser().resolve()
    target_audio = Path(request["target_voice"]).expanduser().resolve()
    if not source_audio.is_file():
        raise FileNotFoundError(f"source audio does not exist: {source_audio}")
    cache_dir = Path(request["cache_dir"]).expanduser().resolve() / "openvoice"
    cache_dir.mkdir(parents=True, exist_ok=True)
    source_se, _ = se_extractor.get_se(
        str(source_audio), converter, target_dir=str(cache_dir / "source"), vad=True
    )
    target_se = target_embedding(target_audio, converter, se_extractor, cache_dir / "target")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    converter.convert(
        audio_src_path=str(source_audio),
        src_se=source_se,
        tgt_se=target_se,
        output_path=str(output_path),
        message="@Takokit",
    )
    return 44100, str(target_audio)


def main() -> None:
    request = json.load(sys.stdin)
    operation = request.get("operation")
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    if operation == "speech":
        sample_rate, voice = run_speech(request, model_dir, output_path)
    elif operation == "convert":
        sample_rate, voice = run_conversion(request, model_dir, output_path)
    else:
        raise ValueError(f"OpenVoice does not support operation: {operation}")
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"OpenVoice did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=sample_rate,
        voice=voice,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
