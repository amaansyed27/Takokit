import json
import sys
import tempfile
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("OpenVoice adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    voice = request.get("voice")
    if not voice:
        raise ValueError("OpenVoice requires a consent-backed voice profile or reference audio path")
    reference = Path(voice).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(f"voice reference does not exist: {reference}")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    cache_dir = Path(request["cache_dir"]).expanduser().resolve() / "openvoice-v2"
    cache_dir.mkdir(parents=True, exist_ok=True)

    import torch
    from huggingface_hub import snapshot_download
    from melo.api import TTS
    from openvoice import se_extractor
    from openvoice.api import ToneColorConverter

    device = "cuda:0" if torch.cuda.is_available() else "cpu"
    snapshot = Path(
        snapshot_download(
            repo_id="myshell-ai/OpenVoiceV2",
            local_dir=str(cache_dir),
            local_dir_use_symlinks=False,
        )
    )
    converter_root = snapshot / "checkpoints_v2" / "converter"
    converter = ToneColorConverter(str(converter_root / "config.json"), device=device)
    converter.load_ckpt(str(converter_root / "checkpoint.pth"))
    target_se, _ = se_extractor.get_se(str(reference), converter, vad=True)

    language = "EN"
    base = TTS(language=language, device=device)
    speaker_ids = base.hps.data.spk2id
    speaker_key = "EN-Default" if "EN-Default" in speaker_ids else next(iter(speaker_ids))
    speaker_id = speaker_ids[speaker_key]
    source_se_path = snapshot / "checkpoints_v2" / "base_speakers" / "ses" / "en-default.pth"
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
    if not output_path.is_file():
        raise RuntimeError(f"OpenVoice did not create {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=None,
        voice=str(reference),
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
