"""Takokit adapter for Fish Speech S2 Pro using the official inference engine."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Fish Speech adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")

    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    codec_path = model_dir / "codec.pth"
    if not model_dir.is_dir() or not codec_path.is_file():
        raise FileNotFoundError(
            f"Fish Speech S2 Pro snapshot is incomplete below {model_dir}"
        )

    import soundfile as sf
    import torch
    from fish_speech.utils.file import audio_to_bytes
    from fish_speech.utils.schema import ServeReferenceAudio, ServeTTSRequest
    from tools.server.model_manager import ModelManager

    device = "cuda" if torch.cuda.is_available() else "cpu"
    manager = ModelManager(
        mode="tts",
        device=device,
        half=torch.cuda.is_available(),
        compile=False,
        llama_checkpoint_path=str(model_dir),
        decoder_checkpoint_path=str(codec_path),
        decoder_config_name="modded_dac_vq",
    )

    references = []
    voice = request.get("voice")
    if voice:
        reference = Path(voice).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
        references.append(
            ServeReferenceAudio(
                audio=audio_to_bytes(str(reference)),
                text=str(request.get("reference_text") or ""),
            )
        )

    inference_request = ServeTTSRequest(
        text=text,
        references=references,
        reference_id=None,
        format="wav",
        max_new_tokens=2048,
        chunk_length=300,
        top_p=0.8,
        repetition_penalty=1.1,
        temperature=0.8,
        streaming=False,
        use_memory_cache="off",
    )
    final_audio = None
    sample_rate = None
    for result in manager.tts_inference_engine.inference(inference_request):
        if result.code == "error":
            raise RuntimeError(str(result.error))
        if result.code == "final" and result.audio is not None:
            sample_rate, final_audio = result.audio
    if final_audio is None or sample_rate is None:
        raise RuntimeError("Fish Speech returned no final audio")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output_path), final_audio, int(sample_rate))
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"Fish Speech did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(sample_rate),
        voice=voice or "default",
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
