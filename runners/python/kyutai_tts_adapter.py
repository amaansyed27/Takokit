import json
import sys
from pathlib import Path


DEFAULT_VOICE = "expresso/ex03-ex01_happy_001_channel1_334s.wav"
MODEL_REPO = "kyutai/tts-1.6b-en_fr"


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Kyutai DSM adapter only supports speech generation")
    if request.get("model_id") != "kyutai-tts-1.6b":
        raise ValueError(f"unsupported Kyutai TTS model: {request.get('model_id')}")

    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import sphn
    import torch
    from moshi.models.loaders import CheckpointInfo
    from moshi.models.tts import TTSModel

    if not torch.cuda.is_available():
        raise RuntimeError(
            "Kyutai TTS currently requires a CUDA-capable GPU in Takokit; "
            "the official MLX path will be added separately for Apple Silicon"
        )

    checkpoint = CheckpointInfo.from_hf_repo(MODEL_REPO)
    model = TTSModel.from_checkpoint_info(
        checkpoint,
        n_q=32,
        temp=0.6,
        device="cuda",
    )
    entries = model.prepare_script([text], padding_between=1)
    requested_voice = str(request.get("voice") or "default").strip()
    voice_name = DEFAULT_VOICE if requested_voice in {"", "default"} else requested_voice
    if Path(voice_name).is_absolute():
        raise ValueError(
            "Kyutai TTS expects a voice embedding name from kyutai/tts-voices, "
            "not an arbitrary local reference file"
        )
    voice_path = voice_name if voice_name.endswith(".safetensors") else model.get_voice_path(voice_name)
    attributes = model.make_condition_attributes([voice_path], cfg_coef=2.0)
    result = model.generate([entries], [attributes])

    with model.mimi.streaming(1), torch.no_grad():
        chunks = []
        for frame in result.frames[model.delay_steps :]:
            if (frame != -1).all():
                pcm = model.mimi.decode(frame[:, 1:, :]).cpu().numpy()
                chunks.append(np.clip(pcm[0, 0], -1, 1))
    if not chunks:
        raise RuntimeError("Kyutai TTS generated no audio frames")
    audio = np.concatenate(chunks, axis=-1)
    sphn.write_wav(str(output_path), audio, model.mimi.sample_rate)
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"Kyutai TTS did not create a valid WAV at {output_path}")

    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(model.mimi.sample_rate),
        voice=voice_name,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
