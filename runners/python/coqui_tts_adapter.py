import json
import sys
from pathlib import Path


MODELS = {
    "xtts-v2": "tts_models/multilingual/multi-dataset/xtts_v2",
    "yourtts": "tts_models/multilingual/multi-dataset/your_tts",
}


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Coqui adapter only supports speech")
    model_id = request.get("model_id")
    checkpoint = MODELS.get(model_id)
    if not checkpoint:
        raise ValueError(f"unsupported Coqui model: {model_id}")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    voice = request.get("voice")
    if not voice:
        raise ValueError(f"{model_id} requires a cloned voice profile or reference audio path")
    reference = Path(voice).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(f"voice reference does not exist: {reference}")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import torch
    from TTS.api import TTS

    device = "cuda" if torch.cuda.is_available() else "cpu"
    engine = TTS(checkpoint).to(device)
    engine.tts_to_file(
        text=text,
        speaker_wav=str(reference),
        language="en",
        file_path=str(output_path),
    )
    if not output_path.is_file():
        raise RuntimeError(f"Coqui did not create {output_path}")
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
