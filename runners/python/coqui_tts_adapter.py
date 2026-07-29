import json
import os
import sys
from pathlib import Path


MODELS = {
    "xtts-v2": "tts_models/multilingual/multi-dataset/xtts_v2",
    "yourtts": "tts_models/multilingual/multi-dataset/your_tts",
}


def path_size(path):
    root = Path(path)
    try:
        if root.is_file():
            return root.stat().st_size
        if not root.is_dir():
            return 0
    except OSError:
        return 0

    total = 0
    for item in root.rglob("*"):
        try:
            if item.is_file():
                total += item.stat().st_size
        except OSError:
            continue
    return total


def respond(**payload):
    print(json.dumps(payload), flush=True)


def ensure_compatible_transformers():
    import transformers

    version = str(getattr(transformers, "__version__", "0"))
    try:
        major = int(version.split(".", 1)[0])
    except ValueError:
        major = 0
    if major >= 5:
        raise RuntimeError(
            "Coqui TTS requires Transformers 4.x; Takokit should install "
            f"transformers==4.57.6 in this adapter overlay, but found {version}"
        )


def ensure_xtts_terms_accepted(model_id):
    if model_id != "xtts-v2":
        return
    if os.environ.get("COQUI_TOS_AGREED", "").strip() == "1":
        return
    raise RuntimeError(
        "XTTS v2 is licensed under the Coqui Public Model License (CPML). "
        "Review and accept the XTTS terms, then set COQUI_TOS_AGREED=1 "
        "before pulling or running this model. Takokit will not accept the "
        "license on your behalf."
    )


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    checkpoint = MODELS.get(model_id)
    if not checkpoint:
        raise ValueError(f"unsupported Coqui model: {model_id}")
    ensure_xtts_terms_accepted(model_id)
    if request.get("operation") == "prefetch":
        ensure_compatible_transformers()
        from TTS.api import TTS

        TTS(checkpoint)
        tts_home = Path(request["cache_dir"]) / "coqui"
        needle = checkpoint.rsplit("/", 1)[-1].replace("_", "").lower()
        model_roots = (
            [
                item
                for item in tts_home.iterdir()
                if item.is_dir()
                and needle in item.name.replace("_", "").lower()
            ]
            if tts_home.is_dir()
            else []
        )
        respond(
            ok=True,
            detail=f"Prefetched {checkpoint}",
            size_bytes=sum(path_size(item) for item in model_roots),
        )
        return
    if request.get("operation") != "speech":
        raise ValueError("Coqui adapter only supports speech")
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

    ensure_compatible_transformers()
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
    except SystemExit as error:
        respond(ok=False, error=f"SystemExit: {error}")
        raise
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
