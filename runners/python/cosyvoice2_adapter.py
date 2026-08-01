"""Takokit adapter for the official CosyVoice2 repository API."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def install_soundfile_torchaudio_io() -> None:
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


def cuda_runtime_is_compatible(torch) -> bool:
    if not torch.cuda.is_available():
        return False
    major, minor = torch.cuda.get_device_capability(0)
    required = f"sm_{major}{minor}"
    try:
        supported = set(torch.cuda.get_arch_list())
    except (AttributeError, RuntimeError):
        return True
    return not supported or required in supported


def paging_file_error(error: BaseException) -> bool:
    return getattr(error, "winerror", None) == 1455 or "paging file is too small" in str(error).lower()


def load_model(model_dir: Path):
    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    sys.path.insert(0, str(source / "third_party" / "Matcha-TTS"))
    import pkg_resources  # noqa: F401
    import torch

    install_soundfile_torchaudio_io()
    compatible_cuda = cuda_runtime_is_compatible(torch)
    if torch.cuda.is_available() and not compatible_cuda:
        capability = torch.cuda.get_device_capability(0)
        supported = ", ".join(torch.cuda.get_arch_list()) or "unknown"
        print(
            "CosyVoice is falling back to CPU because this Torch build does not "
            f"support CUDA capability sm_{capability[0]}{capability[1]} "
            f"(supported: {supported}; torch: {torch.__version__}).",
            file=sys.stderr,
            flush=True,
        )
        torch.cuda.is_available = lambda: False

    from cosyvoice.cli.cosyvoice import AutoModel

    try:
        model = AutoModel(model_dir=str(model_dir))
    except OSError as error:
        if paging_file_error(error):
            raise RuntimeError(
                "CosyVoice exhausted Windows committed memory while loading. "
                "Close memory-heavy applications or increase the Windows paging file. "
                f"The adapter was using {'CUDA' if compatible_cuda else 'CPU fallback'} "
                f"with torch {torch.__version__}."
            ) from error
        raise
    return model, "cuda" if compatible_cuda else "cpu"


def collect_audio(chunks):
    import torch

    tensors = [chunk["tts_speech"].detach().cpu() for chunk in chunks]
    if not tensors:
        raise RuntimeError("CosyVoice2 returned no audio chunks")
    return torch.cat(tensors, dim=1)


def main() -> None:
    request = json.load(sys.stdin)
    operation = request.get("operation")
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    if not model_dir.is_dir():
        raise FileNotFoundError(f"CosyVoice2 model directory is missing: {model_dir}")
    model, device = load_model(model_dir)

    if operation == "speech":
        text = str(request.get("input") or "").strip()
        if not text:
            raise ValueError("speech input cannot be empty")
        reference = request.get("voice")
        if not reference:
            raise ValueError(
                "CosyVoice2 requires --voice with a consent-backed reference sample"
            )
        reference_path = Path(reference).expanduser().resolve()
        if not reference_path.is_file():
            raise FileNotFoundError(f"reference audio does not exist: {reference_path}")
        instruction = str(request.get("instruction") or "").strip()
        reference_text = str(request.get("reference_text") or "").strip()
        if instruction:
            prompt = instruction
            if "<|endofprompt|>" not in prompt:
                prompt += "<|endofprompt|>"
            chunks = model.inference_instruct2(
                text,
                prompt,
                str(reference_path),
                stream=False,
                text_frontend=False,
            )
        elif reference_text:
            chunks = model.inference_zero_shot(
                text,
                reference_text,
                str(reference_path),
                stream=False,
                text_frontend=False,
            )
        else:
            chunks = model.inference_cross_lingual(
                text,
                str(reference_path),
                stream=False,
                text_frontend=False,
            )
    elif operation == "convert":
        source_audio = Path(request["audio_path"]).expanduser().resolve()
        target_voice = Path(request["target_voice"]).expanduser().resolve()
        if not source_audio.is_file() or not target_voice.is_file():
            raise FileNotFoundError("CosyVoice2 conversion requires source and target WAV files")
        chunks = model.inference_vc(str(source_audio), str(target_voice), stream=False)
    else:
        raise ValueError(f"CosyVoice2 does not support operation: {operation}")

    import soundfile as sf

    audio = collect_audio(chunks).numpy()
    if audio.ndim == 2:
        audio = audio.T
    output_path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output_path), audio, int(model.sample_rate))
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"CosyVoice2 did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(model.sample_rate),
        voice=request.get("voice") or request.get("target_voice"),
        device=device,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
