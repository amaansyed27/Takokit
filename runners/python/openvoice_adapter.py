"""Takokit JSON adapter for OpenVoice V2."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("OpenVoice adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    voice = request.get("voice")
    if not voice:
        raise ValueError("OpenVoice requires a consent-backed voice profile")

    reference = Path(str(voice)).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(f"voice reference does not exist: {reference}")
    snapshot = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import torch
    from melo.api import TTS
    from openvoice import se_extractor
    from openvoice.api import ToneColorConverter

    device = "cuda:0" if torch.cuda.is_available() else "cpu"
    converter_root = snapshot / "checkpoints_v2" / "converter"
    converter_config = converter_root / "config.json"
    converter_checkpoint = converter_root / "checkpoint.pth"
    source_se_path = (
        snapshot
        / "checkpoints_v2"
        / "base_speakers"
        / "ses"
        / "en-default.pth"
    )
    for artifact in (converter_config, converter_checkpoint, source_se_path):
        if not artifact.is_file():
            raise FileNotFoundError(f"OpenVoice checkpoint is missing: {artifact}")

    converter = ToneColorConverter(str(converter_config), device=device)
    converter.load_ckpt(str(converter_checkpoint))
    target_se, _ = se_extractor.get_se(str(reference), converter, vad=True)

    base = TTS(language="EN", device=device)
    speaker_ids = base.hps.data.spk2id
    speaker_key = "EN-Default" if "EN-Default" in speaker_ids else next(iter(speaker_ids))
    speaker_id = speaker_ids[speaker_key]
    source_se = torch.load(str(source_se_path), map_location=device)

    with tempfile.TemporaryDirectory(prefix="takokit-openvoice-") as temporary:
        source_path = Path(temporary) / "source.wav"
        base.tts_to_file(text, speaker_id, str(source_path), speed=1.0)
        converter.convert(
            audio_src_path=str(source_path),
            src_se=source_se,
            tgt_se=target_se,
            output_path=str(output_path),
            message="Takokit local voice profile",
        )
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"OpenVoice did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=None,
        voice=str(reference),
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
