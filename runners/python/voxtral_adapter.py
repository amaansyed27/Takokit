import json
import sys
from pathlib import Path


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


def main():
    request = json.load(sys.stdin)
    checkpoint = "mistralai/Voxtral-Mini-3B-2507"
    if request.get("operation") == "prefetch":
        from huggingface_hub import snapshot_download

        snapshot = snapshot_download(repo_id=checkpoint)
        respond(
            ok=True,
            detail=f"Prefetched {checkpoint} at {snapshot}",
            size_bytes=path_size(snapshot),
        )
        return
    if request.get("operation") != "transcribe":
        raise ValueError("Voxtral adapter only supports transcription")
    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    from transformers import AutoProcessor, VoxtralForConditionalGeneration

    processor = AutoProcessor.from_pretrained(checkpoint)
    model = VoxtralForConditionalGeneration.from_pretrained(
        checkpoint,
        device_map="auto",
        low_cpu_mem_usage=True,
    )
    inputs = processor.apply_transcription_request(
        audio=str(audio_path),
        model_id=checkpoint,
    )
    inputs = inputs.to(model.device)
    outputs = model.generate(**inputs, max_new_tokens=1024)
    prompt_length = inputs.input_ids.shape[1]
    decoded = processor.batch_decode(
        outputs[:, prompt_length:],
        skip_special_tokens=True,
    )
    text = decoded[0].strip() if decoded else ""
    if not text:
        raise RuntimeError("Voxtral returned an empty transcript")
    respond(ok=True, text=text)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
