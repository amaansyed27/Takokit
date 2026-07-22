import json
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") == "prefetch":
        from funasr import AutoModel

        AutoModel(model="iic/SenseVoiceSmall", vad_model="fsmn-vad", device="cpu")
        respond(ok=True, detail="Prefetched iic/SenseVoiceSmall and fsmn-vad")
        return
    if request.get("operation") != "transcribe":
        raise ValueError("SenseVoice adapter only supports transcription")
    audio_path = Path(request["audio_path"]).expanduser().resolve()
    if not audio_path.is_file():
        raise FileNotFoundError(f"audio file does not exist: {audio_path}")

    import torch
    from funasr import AutoModel
    from funasr.utils.postprocess_utils import rich_transcription_postprocess

    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = AutoModel(
        model="iic/SenseVoiceSmall",
        vad_model="fsmn-vad",
        device=device,
    )
    result = model.generate(
        input=str(audio_path),
        cache={},
        language="auto",
        use_itn=True,
        batch_size_s=60,
        merge_vad=True,
        merge_length_s=15,
    )
    if not result:
        raise RuntimeError("SenseVoice returned no transcription result")
    raw_text = result[0].get("text") or ""
    text = rich_transcription_postprocess(raw_text).strip()
    if not text:
        raise RuntimeError("SenseVoice returned an empty transcript")
    respond(ok=True, text=text)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
