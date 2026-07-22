import json
import sys
from pathlib import Path


CHECKPOINTS = {
    "canary": "nvidia/canary-1b-v2",
    "parakeet": "nvidia/parakeet-tdt-0.6b-v3",
}


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    checkpoint = CHECKPOINTS.get(model_id)
    if not checkpoint:
        raise ValueError(f"unsupported NeMo ASR model: {model_id}")

    if request.get("operation") == "prefetch":
        from huggingface_hub import snapshot_download

        snapshot = snapshot_download(repo_id=checkpoint)
        respond(ok=True, detail=f"Prefetched {checkpoint} at {snapshot}")
        return
    if request.get("operation") != "transcribe":
        raise ValueError("NeMo ASR adapter only supports transcription")

    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    import torch
    from nemo.collections.asr.models import ASRModel

    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = ASRModel.from_pretrained(model_name=checkpoint).to(device)
    results = model.transcribe([str(audio_path)])
    if not results:
        raise RuntimeError("NeMo returned no transcription result")
    result = results[0]
    text = getattr(result, "text", None) or str(result)
    text = text.strip()
    if not text:
        raise RuntimeError("NeMo returned an empty transcript")
    respond(ok=True, text=text, device=device)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
