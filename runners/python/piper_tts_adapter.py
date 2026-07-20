"""Takokit JSON adapter for Piper TTS."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Piper adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")

    model_dir = Path(request["model_dir"]).expanduser().resolve()
    model = model_dir / "en_US-lessac-medium.onnx"
    config = model_dir / "en_US-lessac-medium.onnx.json"
    for artifact in (model, config):
        if not artifact.is_file():
            raise FileNotFoundError(f"required Piper artifact is missing: {artifact}")

    output = Path(request["output_path"]).expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    command = [
        sys.executable,
        "-m",
        "piper",
        "--model",
        str(model),
        "--config",
        str(config),
        "--output_file",
        str(output),
    ]
    completed = subprocess.run(
        command,
        input=text,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())
    if not output.is_file() or output.stat().st_size <= 44:
        raise RuntimeError(f"Piper did not create a valid WAV at {output}")
    respond(
        ok=True,
        output_path=str(output),
        bytes=output.stat().st_size,
        sample_rate=22050,
        voice="lessac",
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
