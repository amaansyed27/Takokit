"""Shared local-only Transformers adapter for TTS and ASR model families."""

from __future__ import annotations

import json
import sys
from pathlib import Path


MODELS = {
    "bark-small": {"operation": "speech", "kind": "bark"},
    "mms-tts-eng": {"operation": "speech", "kind": "mms"},
    "distil-whisper-large-v3": {
        "operation": "transcribe",
        "kind": "asr-pipeline",
    },
    "wav2vec2-base-960h": {
        "operation": "transcribe",
        "kind": "asr-pipeline",
    },
}


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def device_name(torch):
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def speech(request: dict[str, object], spec: dict[str, str], checkpoint: Path) -> None:
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(str(request["output_path"])).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import torch
    from scipy.io.wavfile import write as write_wav

    device = device_name(torch)
    if spec["kind"] == "bark":
        from transformers import AutoProcessor, BarkModel

        processor = AutoProcessor.from_pretrained(
            str(checkpoint), local_files_only=True
        )
        model = BarkModel.from_pretrained(
            str(checkpoint), local_files_only=True
        ).to(device)
        inputs = processor(text).to(device)
        with torch.inference_mode():
            waveform = model.generate(**inputs)
        sample_rate = int(model.generation_config.sample_rate)
        audio = waveform[0].detach().cpu().float().numpy()
    elif spec["kind"] == "mms":
        from transformers import AutoTokenizer, VitsModel

        tokenizer = AutoTokenizer.from_pretrained(
            str(checkpoint), local_files_only=True
        )
        model = VitsModel.from_pretrained(
            str(checkpoint), local_files_only=True
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
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"model did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=sample_rate,
        voice=request.get("voice") or "default",
        device=device,
    )


def transcribe(request: dict[str, object], checkpoint: Path) -> None:
    audio_path = Path(str(request["audio_path"])).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    import torch
    from transformers import pipeline

    device = 0 if torch.cuda.is_available() else -1
    recognizer = pipeline(
        "automatic-speech-recognition",
        model=str(checkpoint),
        device=device,
        local_files_only=True,
    )
    result = recognizer(str(audio_path))
    text = str(result.get("text") or "").strip()
    if not text:
        raise RuntimeError("the ASR model returned an empty transcript")
    respond(ok=True, text=text)


def main() -> None:
    request = json.load(sys.stdin)
    model_id = str(request.get("model_id") or "")
    spec = MODELS.get(model_id)
    if not spec:
        raise ValueError(f"unsupported Transformers audio model: {model_id}")
    checkpoint = Path(str(request["model_dir"])).expanduser().resolve()
    if not checkpoint.is_dir():
        raise FileNotFoundError(f"local model snapshot is missing: {checkpoint}")
    operation = request.get("operation")
    if operation != spec["operation"]:
        raise ValueError(f"{model_id} does not support {operation}")
    if operation == "speech":
        speech(request, spec, checkpoint)
    else:
        transcribe(request, checkpoint)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
