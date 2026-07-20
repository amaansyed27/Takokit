"""Takokit adapter for the pinned RVC library and CLI."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def run(command: list[str], cwd: Path, log: Path | None = None) -> None:
    if log is None:
        completed = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
        if completed.returncode != 0:
            raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())
        return
    with log.open("a", encoding="utf-8") as stream:
        completed = subprocess.run(
            command,
            cwd=cwd,
            stdout=stream,
            stderr=subprocess.STDOUT,
            check=False,
        )
    if completed.returncode != 0:
        raise RuntimeError(f"RVC command failed; see {log}")


def ensure_runtime(cache_dir: Path) -> Path:
    workspace = cache_dir / "rvc-runtime"
    workspace.mkdir(parents=True, exist_ok=True)
    if not (workspace / ".env").is_file():
        run([sys.executable, "-m", "rvc.cli", "init"], workspace)
    return workspace


def convert(request: dict[str, object], workspace: Path) -> dict[str, object]:
    source = Path(str(request["audio_path"])).expanduser().resolve()
    model = Path(str(request["target_voice"])).expanduser().resolve()
    output = Path(str(request["output_path"])).expanduser().resolve()
    if not source.is_file() or not model.is_file():
        raise FileNotFoundError("RVC requires input audio and a consent-backed .pth model")
    output.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "rvc",
        "infer",
        "-m",
        str(model),
        "-i",
        str(source),
        "-o",
        str(output),
        "-fu",
        str(int(request.get("pitch_shift") or 0)),
    ]
    run(command, workspace)
    if not output.is_file() or output.stat().st_size <= 44:
        raise RuntimeError(f"RVC did not create a valid WAV at {output}")
    return {
        "ok": True,
        "output_path": str(output),
        "bytes": output.stat().st_size,
        "sample_rate": None,
        "voice": str(model),
    }


def train(request: dict[str, object], workspace: Path) -> dict[str, object]:
    dataset = Path(str(request["dataset_path"])).expanduser().resolve()
    output = Path(str(request["output_dir"])).expanduser().resolve()
    name = str(request.get("name") or "takokit-rvc").strip()
    if not dataset.is_dir():
        raise FileNotFoundError(dataset)
    output.mkdir(parents=True, exist_ok=True)
    log = output / "training.log"
    epochs = int(request.get("epochs") or 100)
    run(
        [
            "rvc",
            "train",
            "-n",
            name,
            "-i",
            str(dataset),
            "-e",
            str(epochs),
        ],
        workspace,
        log,
    )
    candidates = list(workspace.rglob(f"{name}*.pth"))
    if not candidates:
        raise RuntimeError(f"RVC training produced no checkpoint; see {log}")
    for candidate in candidates:
        shutil.copy2(candidate, output / candidate.name)
    return {
        "ok": True,
        "output_path": str(output),
        "status": "completed",
        "log_path": str(log),
    }


def main() -> None:
    request = json.load(sys.stdin)
    workspace = ensure_runtime(Path(str(request["cache_dir"])).expanduser().resolve())
    operation = request.get("operation")
    if operation == "convert":
        respond(**convert(request, workspace))
    elif operation == "train":
        respond(**train(request, workspace))
    else:
        raise ValueError(f"RVC does not support operation: {operation}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
