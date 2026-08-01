import importlib.util
import json
import sys
import types
from pathlib import Path


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


def install_pkg_resources_compat():
    try:
        import pkg_resources  # noqa: F401

        return
    except ModuleNotFoundError:
        pass

    module = types.ModuleType("pkg_resources")

    def resource_filename(package_or_requirement, resource_name):
        package = str(package_or_requirement)
        spec = importlib.util.find_spec(package)
        if spec is None:
            raise ModuleNotFoundError(f"could not resolve package resources for {package}")
        locations = list(spec.submodule_search_locations or [])
        if locations:
            root = Path(locations[0])
        elif spec.origin:
            root = Path(spec.origin).resolve().parent
        else:
            raise ModuleNotFoundError(f"package {package} has no resource location")
        return str((root / resource_name).resolve())

    module.resource_filename = resource_filename
    sys.modules["pkg_resources"] = module


def ensure_perth_watermarker():
    install_pkg_resources_compat()
    import perth

    if getattr(perth, "PerthImplicitWatermarker", None) is not None:
        return
    try:
        from perth.perth_net.perth_net_implicit.perth_watermarker import (
            PerthImplicitWatermarker,
        )
    except Exception as error:
        raise RuntimeError(
            "Chatterbox watermark runtime could not be imported: "
            f"{type(error).__name__}: {error}"
        ) from error
    perth.PerthImplicitWatermarker = PerthImplicitWatermarker


def load_chatterbox_tts():
    adapter_dir = Path(__file__).resolve().parent
    original_path = list(sys.path)
    try:
        sys.path[:] = [
            entry
            for entry in sys.path
            if Path(entry or ".").resolve() != adapter_dir
        ]
        ensure_perth_watermarker()
        from chatterbox.tts import ChatterboxTTS

        return ChatterboxTTS
    finally:
        sys.path[:] = original_path


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Chatterbox adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    if not model_dir.is_dir():
        raise FileNotFoundError(f"Chatterbox snapshot is missing: {model_dir}")

    import torch

    install_soundfile_torchaudio_io()
    ChatterboxTTS = load_chatterbox_tts()

    if torch.cuda.is_available():
        device = "cuda"
    elif torch.backends.mps.is_available():
        device = "mps"
    else:
        device = "cpu"

    model = ChatterboxTTS.from_local(model_dir, device=device)
    voice = request.get("voice")
    options = {}
    if voice and voice != "default":
        reference = Path(voice).expanduser().resolve()
        if not reference.is_file():
            raise FileNotFoundError(f"voice reference does not exist: {reference}")
        options["audio_prompt_path"] = str(reference)
    waveform = model.generate(text, **options)

    import soundfile as sf

    audio = waveform.detach().cpu().float().numpy()
    if audio.ndim == 2:
        audio = audio.T
    sf.write(str(output_path), audio, int(model.sr))
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"Chatterbox did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(model.sr),
        voice=voice or "default",
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
