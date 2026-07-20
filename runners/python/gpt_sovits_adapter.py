"""Takokit adapter for pinned GPT-SoVITS inference and training commands."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from contextlib import contextmanager
from pathlib import Path

import numpy as np
import soundfile as sf


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


@contextmanager
def working_directory(path: Path):
    previous = Path.cwd()
    os.chdir(path)
    try:
        yield
    finally:
        os.chdir(previous)


def find_inference_config(model_dir: Path, source: Path) -> Path:
    candidates = list(model_dir.rglob("tts_infer.yaml"))
    candidates.extend(source.rglob("tts_infer.yaml"))
    if not candidates:
        raise FileNotFoundError("GPT-SoVITS tts_infer.yaml was not found")
    return candidates[0]


def speech(request: dict[str, object], source: Path, model_dir: Path) -> dict[str, object]:
    text = str(request.get("input") or "").strip()
    voice = request.get("voice")
    if not text or not voice:
        raise ValueError("GPT-SoVITS requires text and a consent-backed reference voice")
    reference = Path(str(voice)).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(reference)

    sys.path.insert(0, str(source))
    sys.path.insert(0, str(source / "GPT_SoVITS"))
    with working_directory(source):
        from GPT_SoVITS.TTS_infer_pack.TTS import TTS, TTS_Config

        config = TTS_Config(str(find_inference_config(model_dir, source)))
        engine = TTS(config)
        pieces = list(
            engine.run(
                {
                    "text": text,
                    "text_lang": str(request.get("language") or "en"),
                    "ref_audio_path": str(reference),
                    "prompt_text": str(request.get("reference_text") or ""),
                    "prompt_lang": str(request.get("language") or "en"),
                    "text_split_method": "cut5",
                    "batch_size": 1,
                    "media_type": "wav",
                    "streaming_mode": False,
                }
            )
        )
    if not pieces:
        raise RuntimeError("GPT-SoVITS returned no audio")
    sample_rate, audio = pieces[-1]
    output = Path(str(request["output_path"])).expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output), np.asarray(audio), int(sample_rate))
    if not output.is_file() or output.stat().st_size <= 44:
        raise RuntimeError(f"GPT-SoVITS did not create a valid WAV at {output}")
    return {
        "ok": True,
        "output_path": str(output),
        "bytes": output.stat().st_size,
        "sample_rate": int(sample_rate),
        "voice": str(reference),
    }


def training(request: dict[str, object], source: Path) -> dict[str, object]:
    dataset = Path(str(request["dataset_path"])).expanduser().resolve()
    output = Path(str(request["output_dir"])).expanduser().resolve()
    name = str(request.get("name") or "takokit-voice").strip()
    if not dataset.is_dir():
        raise FileNotFoundError(dataset)
    output.mkdir(parents=True, exist_ok=True)
    log = output / "training.log"

    command = [
        sys.executable,
        str(source / "GPT_SoVITS" / "s2_train.py"),
        "--config",
        str(dataset / "s2.json"),
    ]
    if not (dataset / "s2.json").is_file():
        raise ValueError(
            "GPT-SoVITS training requires a prepared dataset containing s2.json; "
            "use the Takokit dataset preparation step before training"
        )
    with log.open("w", encoding="utf-8") as stream:
        completed = subprocess.run(
            command,
            cwd=source,
            stdout=stream,
            stderr=subprocess.STDOUT,
            check=False,
        )
    if completed.returncode != 0:
        raise RuntimeError(f"GPT-SoVITS training failed; see {log}")
    for checkpoint in source.rglob("*.pth"):
        if checkpoint.stat().st_mtime >= log.stat().st_mtime:
            shutil.copy2(checkpoint, output / f"{name}-{checkpoint.name}")
    if not any(output.glob("*.pth")):
        raise RuntimeError("GPT-SoVITS training produced no checkpoint")
    return {
        "ok": True,
        "output_path": str(output),
        "status": "completed",
        "log_path": str(log),
    }


def main() -> None:
    request = json.load(sys.stdin)
    source = Path(__file__).resolve().parent / "source"
    model_dir = Path(str(request["model_dir"])).expanduser().resolve()
    if not source.is_dir():
        raise FileNotFoundError("GPT-SoVITS source checkout is missing")
    operation = request.get("operation")
    if operation == "speech":
        respond(**speech(request, source, model_dir))
    elif operation == "train":
        respond(**training(request, source))
    else:
        raise ValueError(f"GPT-SoVITS does not support operation: {operation}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
