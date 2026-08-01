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


def install_soundfile_torchaudio_io():
    import numpy as np
    import soundfile as sf
    import torch
    import torchaudio

    def load(
        path,
        frame_offset=0,
        num_frames=-1,
        normalize=True,
        channels_first=True,
        **_kwargs,
    ):
        del normalize
        audio, sample_rate = sf.read(str(path), dtype="float32", always_2d=True)
        start = max(0, int(frame_offset))
        stop = None if int(num_frames) < 0 else start + int(num_frames)
        audio = audio[start:stop]
        tensor = torch.from_numpy(np.ascontiguousarray(audio))
        if channels_first:
            tensor = tensor.transpose(0, 1)
        return tensor, int(sample_rate)

    def save(path, source, sample_rate, channels_first=True, **_kwargs):
        if hasattr(source, "detach"):
            source = source.detach().cpu().float().numpy()
        audio = np.asarray(source)
        if channels_first and audio.ndim == 2:
            audio = audio.T
        sf.write(str(path), audio, int(sample_rate))

    torchaudio.load = load
    torchaudio.save = save


def load_f5tts_api():
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


def cuda_error_allows_cpu_retry(error):
    message = f"{type(error).__name__}: {error}".lower()
    return any(
        marker in message
        for marker in (
            "device(s) is/are busy or unavailable",
            "cuda-capable device",
            "cudaerrordevicesunavailable",
            "no kernel image is available",
            "out of memory",
        )
    )


def create_engine(F5TTS):
    import torch

    if torch.cuda.is_available():
        try:
            return F5TTS(model="F5TTS_v1_Base", device="cuda"), "cuda"
        except Exception as error:
            if not cuda_error_allows_cpu_retry(error):
                raise
            print(
                "F5-TTS is retrying on CPU because CUDA could not initialize: "
                f"{type(error).__name__}: {error}",
                file=sys.stderr,
                flush=True,
            )
            try:
                torch.cuda.empty_cache()
            except RuntimeError:
                pass
    return F5TTS(model="F5TTS_v1_Base", device="cpu"), "cpu"


def main():
    request = json.load(sys.stdin)
    install_soundfile_torchaudio_io()
    if request.get("operation") == "prefetch":
        F5TTS = load_f5tts_api()
        # Prefetch only needs to materialize checkpoints. CPU initialization
        # avoids making model installation depend on transient GPU availability.
        F5TTS(model="F5TTS_v1_Base", device="cpu")
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

    engine, device = create_engine(F5TTS)
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
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
