import json
import sys
import time
from pathlib import Path


DEFAULT_RUNTIME_FILES = [
    "*.json",
    "*.txt",
    "*.model",
    "*.spm",
    "*.bin",
    "*.safetensors",
]

MODELS = {
    "bark-small": {
        "operation": "speech",
        "checkpoint": "suno/bark-small",
        "revision": "1dbd7a128513b8ae4a4e2130fed57b7ac9da5bcd",
        "runtime_files": [
            "config.json",
            "generation_config.json",
            "pytorch_model.bin",
            "speaker_embeddings/**",
            "speaker_embeddings_path.json",
            "special_tokens_map.json",
            "tokenizer.json",
            "tokenizer_config.json",
        ],
        "kind": "bark",
    },
    "mms-tts-eng": {
        "operation": "speech",
        "checkpoint": "facebook/mms-tts-eng",
        "kind": "mms",
    },
    "distil-whisper-large-v3": {
        "operation": "transcribe",
        "checkpoint": "distil-whisper/distil-large-v3",
        "kind": "asr-pipeline",
    },
    "wav2vec2-base-960h": {
        "operation": "transcribe",
        "checkpoint": "facebook/wav2vec2-base-960h",
        "kind": "asr-pipeline",
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


def device_name(torch):
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def device_detail(torch):
    device = device_name(torch)
    if device == "cuda":
        return (
            f"cuda ({torch.cuda.get_device_name(0)}; "
            f"torch {torch.__version__}; CUDA {torch.version.cuda})"
        )
    return f"{device} (torch {torch.__version__})"


def snapshot_options(spec):
    return {
        "repo_id": spec["checkpoint"],
        "revision": spec.get("revision"),
        "allow_patterns": spec.get("runtime_files", DEFAULT_RUNTIME_FILES),
    }


def is_retryable_download(error):
    message = f"{type(error).__name__}: {error}".lower()
    return any(
        marker in message
        for marker in (
            "429",
            "too many requests",
            "connection",
            "timeout",
            "timed out",
            "502",
            "503",
            "504",
        )
    )


def prefetch_checkpoint(spec):
    from huggingface_hub import snapshot_download

    last_error = None
    for attempt in range(1, 6):
        try:
            return snapshot_download(**snapshot_options(spec), max_workers=4)
        except Exception as error:
            last_error = error
            if attempt == 5 or not is_retryable_download(error):
                raise
            delay = min(2**attempt, 30)
            print(
                f"Checkpoint download attempt {attempt} failed; "
                f"retrying in {delay}s: {type(error).__name__}: {error}",
                file=sys.stderr,
                flush=True,
            )
            time.sleep(delay)
    raise last_error


def local_checkpoint(spec):
    from huggingface_hub import snapshot_download

    return snapshot_download(
        **snapshot_options(spec),
        local_files_only=True,
    )


def speech(request, spec):
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import torch
    from scipy.io.wavfile import write as write_wav

    device = device_name(torch)
    checkpoint = local_checkpoint(spec)
    if spec["kind"] == "bark":
        from transformers import AutoProcessor, BarkModel

        processor = AutoProcessor.from_pretrained(checkpoint, local_files_only=True)
        dtype = torch.float16 if device == "cuda" else torch.float32
        model = BarkModel.from_pretrained(
            checkpoint,
            torch_dtype=dtype,
            local_files_only=True,
        ).to(device)
        inputs = processor(text).to(device)
        with torch.inference_mode():
            waveform = model.generate(**inputs)
        sample_rate = int(model.generation_config.sample_rate)
        audio = waveform[0].detach().cpu().float().numpy()
    elif spec["kind"] == "mms":
        from transformers import AutoTokenizer, VitsModel

        tokenizer = AutoTokenizer.from_pretrained(checkpoint, local_files_only=True)
        model = VitsModel.from_pretrained(
            checkpoint,
            local_files_only=True,
        ).to(device)
        inputs = tokenizer(text, return_tensors="pt").to(device)
        with torch.inference_mode():
            waveform = model(**inputs).waveform
        sample_rate = int(model.config.sampling_rate)
        audio = waveform[0].detach().cpu().float().numpy()
    else:
        raise ValueError(f"unsupported TTS kind: {spec['kind']}")

    peak = float(np.max(np.abs(audio))) if audio.size else 0.0
    if peak > 1.0:
        audio = audio / peak
    write_wav(str(output_path), sample_rate, audio.astype(np.float32))
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=sample_rate,
        voice=request.get("voice") or "default",
        device=device_detail(torch),
    )


def transcribe(request, spec):
    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    import torch
    from transformers import pipeline

    checkpoint = local_checkpoint(spec)
    device = 0 if torch.cuda.is_available() else -1
    recognizer = pipeline(
        "automatic-speech-recognition",
        model=checkpoint,
        device=device,
        model_kwargs={"local_files_only": True},
    )
    result = recognizer(str(audio_path))
    text = str(result.get("text") or "").strip()
    if not text:
        raise RuntimeError("the ASR model returned an empty transcript")
    respond(ok=True, text=text, device=device_detail(torch))


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    spec = MODELS.get(model_id)
    if not spec:
        raise ValueError(f"unsupported Hugging Face audio model: {model_id}")
    operation = request.get("operation")
    if operation == "prefetch":
        import torch

        snapshot = prefetch_checkpoint(spec)
        respond(
            ok=True,
            detail=(
                f"Prefetched runtime files for {spec['checkpoint']} at {snapshot}; "
                f"{device_detail(torch)}"
            ),
            size_bytes=path_size(snapshot),
        )
        return
    if operation != spec["operation"]:
        raise ValueError(f"{model_id} does not support {operation}")
    if operation == "speech":
        speech(request, spec)
    else:
        transcribe(request, spec)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
