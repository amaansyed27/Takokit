import json
import sys
from pathlib import Path


CHECKPOINTS = {
    "canary": {
        "repo": "nvidia/canary-1b-v2",
        "file": "canary-1b-v2.nemo",
    },
    "parakeet": {
        "repo": "nvidia/parakeet-tdt-0.6b-v3",
        "file": "parakeet-tdt-0.6b-v3.nemo",
    },
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


def cuda_device(torch):
    if not torch.cuda.is_available():
        raise RuntimeError(
            "CUDA is unavailable to the managed PyTorch environment. "
            f"torch={torch.__version__}; rebuild this adapter with Takokit's "
            "automatic PyTorch accelerator selection."
        )
    return f"cuda ({torch.cuda.get_device_name(0)}; torch {torch.__version__}; CUDA {torch.version.cuda})"


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    spec = CHECKPOINTS.get(model_id)
    if not spec:
        raise ValueError(f"unsupported NeMo ASR model: {model_id}")

    import torch

    device_detail = cuda_device(torch)
    if request.get("operation") == "prefetch":
        from huggingface_hub import hf_hub_download

        checkpoint_path = hf_hub_download(
            repo_id=spec["repo"],
            filename=spec["file"],
        )
        respond(
            ok=True,
            detail=f"Prefetched {spec['repo']}/{spec['file']} at {checkpoint_path}; {device_detail}",
            size_bytes=path_size(checkpoint_path),
        )
        return
    if request.get("operation") != "transcribe":
        raise ValueError("NeMo ASR adapter only supports transcription")

    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    from nemo.collections.asr.models import ASRModel

    model = ASRModel.from_pretrained(model_name=spec["repo"]).to("cuda")
    results = model.transcribe([str(audio_path)])
    if not results:
        raise RuntimeError("NeMo returned no transcription result")
    result = results[0]
    text = getattr(result, "text", None) or str(result)
    text = text.strip()
    if not text:
        raise RuntimeError("NeMo returned an empty transcript")
    respond(ok=True, text=text, device=device_detail)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
