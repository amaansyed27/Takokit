import json
import sys
from importlib.resources import files
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("F5-TTS adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    from f5_tts.api import F5TTS

    voice = request.get("voice")
    if voice and voice != "default":
        reference = Path(voice).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
    else:
        reference = Path(str(files("f5_tts").joinpath("infer/examples/basic/basic_ref_en.wav")))
    engine = F5TTS(model="F5TTS_v1_Base")
    _, sample_rate, _ = engine.infer(
        ref_file=str(reference),
        ref_text="",
        gen_text=text,
        file_wave=str(output_path),
        seed=None,
    )
    if not output_path.is_file():
        raise RuntimeError(f"F5-TTS did not create {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(sample_rate),
        voice=voice or "default",
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
