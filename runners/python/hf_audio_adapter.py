import json
import sys
from pathlib import Path


MODELS = {
    "bark-small": {
        "operation": "speech",
        "checkpoint": "suno/bark-small",
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


def respond(**payload):
    print(json.dumps(payload), flush=True)


def device_name(torch):
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


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
    checkpoint = spec["checkpoint"]
    if spec["kind"] == "bark":
        from transformers import AutoProcessor, BarkModel

        processor = AutoProcessor.from_pretrained(checkpoint)
        model = BarkModel.from_pretrained(checkpoint).to(device)
        inputs = processor(text).to(device)
        with torch.inference_mode():
            waveform = model.generate(**inputs)
        sample_rate = int(model.generation_config.sample_rate)
        audio = waveform[0].detach().cpu().float().numpy()
    elif spec["kind"] == "mms":
        from transformers import AutoTokenizer, VitsModel

        tokenizer = AutoTokenizer.from_pretrained(checkpoint)
        model = VitsModel.from_pretrained(checkpoint).to(device)
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
    )


def transcribe(request, spec):
    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    import torch
    from transformers import pipeline

    device = 0 if torch.cuda.is_available() else -1
    recognizer = pipeline(
        "automatic-speech-recognition",
        model=spec["checkpoint"],
        device=device,
    )
    result = recognizer(str(audio_path))
    text = str(result.get("text") or "").strip()
    if not text:
        raise RuntimeError("the ASR model returned an empty transcript")
    respond(ok=True, text=text)


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    spec = MODELS.get(model_id)
    if not spec:
        raise ValueError(f"unsupported Hugging Face audio model: {model_id}")
    operation = request.get("operation")
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
