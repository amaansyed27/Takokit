import json
import sys
from importlib.resources import files
from pathlib import Path


DEFAULT_REFERENCE_TEXT = "Some call me nature, others call me mother nature."


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


def load_f5tts_api():
    # Takokit deploys this runner as `f5_tts.py`. Without removing the runner
    # directory from sys.path, Python resolves that file instead of the real
    # installed `f5_tts` package and reports that `f5_tts` is not a package.
    adapter_dir = Path(__file__).resolve().parent
    original_path = list(sys.path)
    try:
        sys.path[:] = [
            entry
            for entry in sys.path
            if Path(entry or ".").resolve() != adapter_dir
        ]
        from f5_tts.api import F5TTS

        return F5TTS
    finally:
        sys.path[:] = original_path


def main():
    request = json.load(sys.stdin)
    if request.get("operation") == "prefetch":
        F5TTS = load_f5tts_api()
        F5TTS(model="F5TTS_v1_Base")
        hub = Path(request["cache_dir"]) / "huggingface" / "hub"
        model_roots = (
            [
                item
                for item in hub.iterdir()
                if item.is_dir() and "f5" in item.name.lower()
            ]
            if hub.is_dir()
            else []
        )
        respond(
            ok=True,
            detail="Prefetched F5TTS_v1_Base",
            size_bytes=sum(path_size(item) for item in model_roots),
        )
        return
    if request.get("operation") != "speech":
        raise ValueError("F5-TTS adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    F5TTS = load_f5tts_api()
    voice = request.get("voice")
    reference_text = str(request.get("reference_text") or "").strip()
    if voice and voice != "default":
        reference = Path(voice).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
    else:
        reference = Path(str(files("f5_tts").joinpath("infer/examples/basic/basic_ref_en.wav")))
        if not reference_text:
            reference_text = DEFAULT_REFERENCE_TEXT
    engine = F5TTS(model="F5TTS_v1_Base")
    _, sample_rate, _ = engine.infer(
        ref_file=str(reference),
        ref_text=reference_text,
        gen_text=text,
        file_wave=str(output_path),
        seed=0,
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
