"""Takokit managed worker for official RVC dataset preparation and training.

This adapter deliberately owns only the training pipeline. Inference remains in
rvc_adapter.py so imported and Takokit-trained checkpoints use the same converter.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any

GIB = 1024**3
MIN_RVC_RAM = 8 * GIB
MIN_RECOMMENDED_VRAM = 4 * GIB


def respond(**payload: object) -> None:
    print(json.dumps(payload, ensure_ascii=False), flush=True)


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False), encoding="utf-8")
    os.replace(temporary, path)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def update_job(job_path: Path, **updates: object) -> dict[str, Any]:
    payload = json.loads(job_path.read_text(encoding="utf-8"))
    payload.update(updates)
    atomic_json(job_path, payload)
    return payload


def append_log(log_path: Path, message: str) -> None:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("a", encoding="utf-8", errors="replace") as stream:
        stream.write(message.rstrip() + "\n")


def run_stage(command: list[str], cwd: Path, log_path: Path, env: dict[str, str]) -> None:
    append_log(log_path, "$ " + subprocess.list2cmdline(command))
    with log_path.open("a", encoding="utf-8", errors="replace") as stream:
        process = subprocess.Popen(
            command,
            cwd=str(cwd),
            env=env,
            stdout=stream,
            stderr=subprocess.STDOUT,
            text=True,
        )
        return_code = process.wait()
    if return_code != 0:
        raise RuntimeError(f"RVC stage exited with code {return_code}; see {log_path}")


def link_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        try:
            if source.samefile(destination):
                return
        except OSError:
            pass
        destination.unlink()
    try:
        os.link(source, destination)
    except OSError:
        shutil.copy2(source, destination)


def link_directory(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() or destination.is_symlink():
        try:
            if source.samefile(destination):
                return
        except OSError:
            pass
        if destination.is_symlink():
            destination.unlink()
        elif destination.is_dir():
            shutil.rmtree(destination)
        else:
            destination.unlink()
    if os.name == "nt":
        result = subprocess.run(
            ["cmd", "/d", "/c", "mklink", "/J", str(destination), str(source)],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Could not create the managed RVC experiment junction: "
                + (result.stderr or result.stdout).strip()
            )
    else:
        destination.symlink_to(source, target_is_directory=True)


def inspect_audio(path: Path) -> dict[str, Any]:
    import librosa
    import numpy as np
    import soundfile as sf

    if not path.is_file():
        raise FileNotFoundError(f"audio sample does not exist: {path}")
    try:
        info = sf.info(str(path))
        audio, sample_rate = librosa.load(str(path), sr=None, mono=False)
    except Exception as error:
        raise ValueError(f"audio sample is unreadable or unsupported: {path}: {error}") from error
    audio = np.asarray(audio, dtype=np.float32)
    if audio.size == 0 or sample_rate <= 0:
        raise ValueError(f"audio sample is empty: {path}")
    if audio.ndim > 1:
        mono = audio.mean(axis=0)
        channels = int(audio.shape[0])
    else:
        mono = audio
        channels = int(getattr(info, "channels", 1) or 1)
    duration_ms = int(round((mono.shape[-1] / float(sample_rate)) * 1000.0))
    absolute = np.abs(mono)
    peak = float(absolute.max(initial=0.0))
    rms = float(np.sqrt(np.mean(np.square(mono), dtype=np.float64)))
    silence_ratio = float(np.mean(absolute < 10 ** (-50.0 / 20.0)))
    clipped_ratio = float(np.mean(absolute >= 0.999))
    peak_dbfs = 20.0 * math.log10(max(peak, 1e-12))
    rms_dbfs = 20.0 * math.log10(max(rms, 1e-12))
    warnings: list[dict[str, str]] = []
    if duration_ms < 1000:
        warnings.append({"code": "too_short", "message": "Recording is under one second and is unlikely to produce useful RVC segments."})
    if silence_ratio >= 0.50:
        warnings.append({"code": "heavy_silence", "message": "At least half of this recording is near-silent; trim long silent sections or exclude it."})
    if clipped_ratio >= 0.001:
        warnings.append({"code": "clipping", "message": "Clipped samples were detected; a cleaner recording is preferable."})
    if sample_rate < 16000:
        warnings.append({"code": "low_sample_rate", "message": "Source sample rate is below 16 kHz; preprocessing can resample it but detail is already limited."})
    if channels > 2:
        warnings.append({"code": "many_channels", "message": "More than two channels were detected; RVC preprocessing will downmix the recording."})
    return {
        "duration_ms": duration_ms,
        "sample_rate": int(sample_rate),
        "channels": channels,
        "codec": str(getattr(info, "subtype", "") or "") or None,
        "container": str(getattr(info, "format", "") or path.suffix.lstrip(".")) or None,
        "peak_dbfs": peak_dbfs,
        "rms_dbfs": rms_dbfs,
        "silence_ratio": silence_ratio,
        "clipped_ratio": clipped_ratio,
        "warnings": warnings,
        "valid": duration_ms > 0,
    }


def system_ram_bytes() -> int | None:
    if os.name == "nt":
        try:
            import ctypes

            class MemoryStatus(ctypes.Structure):
                _fields_ = [
                    ("length", ctypes.c_ulong),
                    ("memory_load", ctypes.c_ulong),
                    ("total_phys", ctypes.c_ulonglong),
                    ("avail_phys", ctypes.c_ulonglong),
                    ("total_page", ctypes.c_ulonglong),
                    ("avail_page", ctypes.c_ulonglong),
                    ("total_virtual", ctypes.c_ulonglong),
                    ("avail_virtual", ctypes.c_ulonglong),
                    ("avail_extended_virtual", ctypes.c_ulonglong),
                ]

            status = MemoryStatus()
            status.length = ctypes.sizeof(MemoryStatus)
            if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
                return int(status.total_phys)
        except Exception:
            return None
    try:
        return int(os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES"))
    except (AttributeError, OSError, ValueError):
        return None


def preflight(request: dict[str, Any]) -> dict[str, Any]:
    import torch

    voice_root = Path(request["voice_root"]).resolve()
    requested_device = str(request.get("device") or "auto").lower()
    ram = system_ram_bytes()
    disk = shutil.disk_usage(voice_root).free
    gpu = None
    vram = None
    cuda = bool(torch.cuda.is_available())
    if cuda:
        gpu = torch.cuda.get_device_name(0)
        vram = int(torch.cuda.get_device_properties(0).total_memory)
    reasons: list[str] = []
    if requested_device == "cuda" and not cuda:
        classification = "unsupported"
        resolved_device = "cuda"
        reasons.append("CUDA was explicitly selected but PyTorch cannot see an NVIDIA CUDA device.")
    elif requested_device == "cpu":
        resolved_device = "cpu"
        classification = "possible" if ram is None or ram >= MIN_RVC_RAM else "unsupported"
        reasons.append("The pinned RVC trainer supports CPU fallback, but training can be substantially slower than CUDA.")
    elif cuda:
        resolved_device = "cuda"
        classification = "recommended" if vram is not None and vram >= MIN_RECOMMENDED_VRAM else "possible"
        if vram is not None and vram < MIN_RECOMMENDED_VRAM:
            reasons.append("Detected VRAM is below Takokit's 4 GiB RVC model requirement; reduce batch size if CUDA runs out of memory.")
    else:
        resolved_device = "cpu"
        classification = "possible" if ram is None or ram >= MIN_RVC_RAM else "unsupported"
        reasons.append("No CUDA device was detected; the upstream trainer will use its real CPU fallback.")
    if ram is not None and ram < MIN_RVC_RAM:
        classification = "unsupported"
        reasons.append("System RAM is below Takokit's 8 GiB RVC model requirement.")
    return {
        "class": classification,
        "cpu": platform.processor() or platform.machine() or "unknown",
        "gpu": gpu,
        "backend": "cuda" if cuda else "cpu",
        "vram_bytes": vram,
        "system_ram_bytes": ram,
        "available_disk_bytes": int(disk),
        "dataset_duration_ms": int(request.get("dataset_duration_ms") or 0),
        "resolved_device": resolved_device,
        "resolved_precision": "fp16" if resolved_device == "cuda" else "fp32",
        "resource_category": "gpu_training" if resolved_device == "cuda" else "cpu_training",
        "reasons": reasons,
    }


def ensure_assets(trainer_root: Path, asset_root: Path) -> tuple[Path, Path]:
    hubert_source = asset_root / "hubert_base"
    rmvpe_source = asset_root / "rmvpe.pt"
    pretrain_root = asset_root / "pretrained_v2"
    required = [
        hubert_source / "config.json",
        hubert_source / "preprocessor_config.json",
        hubert_source / "pytorch_model.bin",
        rmvpe_source,
        pretrain_root / "f0G40k.pth",
        pretrain_root / "f0D40k.pth",
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise FileNotFoundError("RVC training assets are incomplete; run `tako pull rvc`. Missing: " + ", ".join(missing))
    for source in hubert_source.iterdir():
        if source.is_file():
            link_file(source, trainer_root / "assets" / "hubert_base" / source.name)
    link_file(rmvpe_source, trainer_root / "assets" / "rmvpe" / "rmvpe.pt")
    generator = trainer_root / "assets" / "pretrained_v2" / "f0G40k.pth"
    discriminator = trainer_root / "assets" / "pretrained_v2" / "f0D40k.pth"
    link_file(pretrain_root / "f0G40k.pth", generator)
    link_file(pretrain_root / "f0D40k.pth", discriminator)
    return generator, discriminator


def prepare_key(samples: list[dict[str, Any]], config: dict[str, Any]) -> str:
    digest = hashlib.sha256()
    for sample in sorted(samples, key=lambda value: str(value.get("sha256") or value.get("path"))):
        digest.update(str(sample.get("sha256") or "").encode())
        digest.update(str(sample.get("path") or "").encode("utf-8"))
    for key in ("sample_rate_hz", "model_version", "f0_enabled", "f0_method"):
        digest.update(f"{key}={config.get(key)}".encode())
    return digest.hexdigest()


def stage_inputs(samples: list[dict[str, Any]], input_root: Path) -> None:
    if input_root.exists():
        shutil.rmtree(input_root)
    input_root.mkdir(parents=True)
    if not samples:
        raise ValueError("RVC training has no included samples")
    for index, sample in enumerate(samples):
        source = Path(sample["path"]).resolve()
        if not source.is_file():
            raise FileNotFoundError(f"included sample is missing: {source}")
        suffix = source.suffix.lower() or ".wav"
        link_file(source, input_root / f"sample_{index:04d}{suffix}")


def build_filelist(exp_dir: Path, f0_enabled: bool, version: str) -> int:
    gt = exp_dir / "0_gt_wavs"
    features = exp_dir / ("3_feature256" if version == "v1" else "3_feature768")
    f0 = exp_dir / "2a_f0"
    f0nsf = exp_dir / "2b-f0nsf"
    lines: list[str] = []
    if not gt.is_dir() or not features.is_dir():
        raise RuntimeError("RVC preprocessing/features are incomplete")
    for wav in sorted(gt.glob("*.wav")):
        stem = wav.stem
        feature = features / f"{stem}.npy"
        if not feature.is_file():
            continue
        if f0_enabled:
            coarse = f0 / f"{stem}.wav.npy"
            continuous = f0nsf / f"{stem}.wav.npy"
            if not coarse.is_file() or not continuous.is_file():
                coarse = f0 / f"{stem}.npy"
                continuous = f0nsf / f"{stem}.npy"
            if not coarse.is_file() or not continuous.is_file():
                continue
            lines.append(f"{wav}|{feature}|{coarse}|{continuous}|0")
        else:
            lines.append(f"{wav}|{feature}|0")
    if not lines:
        raise RuntimeError("RVC produced no trainable segments after preprocessing and feature extraction")
    (exp_dir / "filelist.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return len(lines)


def discover_artifacts(voice_root: Path, trainer_root: Path, experiment: str) -> dict[str, Any]:
    checkpoint_root = voice_root / "checkpoints"
    index_root = voice_root / "indexes"
    checkpoint_root.mkdir(parents=True, exist_ok=True)
    index_root.mkdir(parents=True, exist_ok=True)
    exported = sorted((trainer_root / "assets" / "weights").glob(f"{experiment}*.pth"), key=lambda path: path.stat().st_mtime)
    if not exported:
        raise RuntimeError("RVC training completed without producing an inference .pth checkpoint")
    checkpoint_paths: list[Path] = []
    for source in exported:
        destination = checkpoint_root / source.name
        shutil.copy2(source, destination)
        checkpoint_paths.append(destination)
    indexes = sorted(index_root.glob("*.index"), key=lambda path: path.stat().st_mtime)
    if not indexes:
        raise RuntimeError("RVC index generation completed without producing an .index artifact")
    active_checkpoint = checkpoint_paths[-1]
    active_index = indexes[-1]
    manifest = {
        "schema_version": 1,
        "engine": "rvc",
        "checkpoint": active_checkpoint.name,
        "index": os.path.relpath(active_index, checkpoint_root).replace("\\", "/"),
        "quality_baseline": False,
        "note": "Artifact generation succeeded. Takokit does not infer perceptual voice similarity from file creation.",
    }
    atomic_json(checkpoint_root / "rvc.json", manifest)
    artifacts = {
        "checkpoint": str(active_checkpoint),
        "checkpoint_sha256": sha256(active_checkpoint),
        "checkpoint_bytes": active_checkpoint.stat().st_size,
        "index": str(active_index),
        "index_sha256": sha256(active_index),
        "index_bytes": active_index.stat().st_size,
        "all_checkpoints": [str(path) for path in checkpoint_paths],
    }
    atomic_json(voice_root / "jobs" / "latest-artifacts.json", artifacts)
    return artifacts


def run_training(request: dict[str, Any]) -> None:
    voice_root = Path(request["voice_root"]).resolve()
    trainer_root = Path(request["trainer_root"]).resolve()
    asset_root = Path(request["asset_root"]).resolve()
    job_path = Path(request["job_path"]).resolve()
    log_path = Path(request["log_path"]).resolve()
    config = dict(request["config"])
    samples = list(request.get("samples") or [])
    if not trainer_root.joinpath("train", "train.py").is_file():
        raise FileNotFoundError(f"managed RVC trainer is missing: {trainer_root}")
    if config.get("sample_rate_hz") != 40000 or config.get("model_version") != "v2":
        raise ValueError("Slice 3 currently supports the pinned RVC v2 40 kHz training path only")
    if str(config.get("f0_method") or "rmvpe").lower() != "rmvpe":
        raise ValueError("Slice 3 RVC training currently supports RMVPE F0 extraction only")
    generator, discriminator = ensure_assets(trainer_root, asset_root)
    experiment = "takokit_" + str(request["voice_id"]).replace("-", "")
    exp_dir = voice_root / "dataset" / "experiment"
    input_root = voice_root / "samples" / "managed"
    logs_link = trainer_root / "logs" / experiment
    config_source = trainer_root / "configs" / "v2" / "40k.json"
    if not config_source.is_file():
        raise FileNotFoundError(f"RVC v2 40k config is missing: {config_source}")
    env = os.environ.copy()
    resolved_device = str(request.get("resolved_device") or "auto")
    if resolved_device == "cpu":
        env["CUDA_VISIBLE_DEVICES"] = ""
    prepare_hash = prepare_key(samples, config)
    marker = voice_root / "dataset" / ".prepare-key"
    can_reuse = marker.is_file() and marker.read_text(encoding="utf-8").strip() == prepare_hash and (exp_dir / "filelist.txt").is_file()
    if not can_reuse:
        if exp_dir.exists():
            shutil.rmtree(exp_dir)
        exp_dir.mkdir(parents=True)
        stage_inputs(samples, input_root)
        link_directory(exp_dir, logs_link)
        update_job(job_path, status="running", stage="preprocess", started_at=int(time.time()), child_pid=os.getpid())
        run_stage([sys.executable, "train/preprocess.py", str(input_root), "40000", str(max(1, min(os.cpu_count() or 1, 8))), str(exp_dir), "False", "3.7"], trainer_root, log_path, env)
        if bool(config.get("f0_enabled", True)):
            update_job(job_path, stage="extract_f0")
            if resolved_device == "cuda":
                run_stage([sys.executable, "train/dataset/extract_f0.py", "cuda", "1", "0", "0", str(exp_dir), "true"], trainer_root, log_path, env)
            else:
                run_stage([sys.executable, "train/dataset/extract_f0.py", "cpu", str(exp_dir), str(max(1, min(os.cpu_count() or 1, 8))), "rmvpe"], trainer_root, log_path, env)
        update_job(job_path, stage="extract_features")
        run_stage([sys.executable, "train/dataset/extract_hubert_feature.py", "cuda" if resolved_device == "cuda" else "cpu", "1", "0", str(exp_dir), "v2", "true" if resolved_device == "cuda" else "false"], trainer_root, log_path, env)
        segment_count = build_filelist(exp_dir, bool(config.get("f0_enabled", True)), "v2")
        shutil.copy2(config_source, exp_dir / "config.json")
        marker.write_text(prepare_hash, encoding="utf-8")
        append_log(log_path, f"Prepared {segment_count} real RVC training segments.")
    else:
        link_directory(exp_dir, logs_link)
        append_log(log_path, "Reusing deterministic prepared dataset; input/config fingerprint is unchanged.")
    update_job(job_path, stage="train", status="running")
    train_command = [
        sys.executable, "train/train.py",
        "-e", experiment,
        "-sr", "40k",
        "-f0", "1" if config.get("f0_enabled", True) else "0",
        "-bs", str(config["batch_size"]),
        "-te", str(config["epochs"]),
        "-se", str(config["save_every_epochs"]),
        "-pg", str(generator),
        "-pd", str(discriminator),
        "-l", "0",
        "-c", "1" if config.get("cache_dataset_on_gpu") else "0",
        "-sw", "1",
        "-v", "v2",
        "-g", "0",
    ]
    run_stage(train_command, trainer_root, log_path, env)
    update_job(job_path, stage="build_index")
    run_stage([sys.executable, "train/train_index.py", experiment, "v2", str(voice_root / "indexes"), str(max(1, min(os.cpu_count() or 1, 8))), "single"], trainer_root, log_path, env)
    update_job(job_path, stage="validate_artifacts")
    artifacts = discover_artifacts(voice_root, trainer_root, experiment)
    update_job(job_path, stage="complete", status="succeeded", finished_at=int(time.time()), failure=None)
    append_log(log_path, "RVC execution completed and artifacts validated. Listen to a conversion preview to judge perceptual results.")
    atomic_json(voice_root / "jobs" / "latest-result.json", artifacts)


def handle_request(request: dict[str, Any]) -> dict[str, Any]:
    operation = request.get("operation")
    if operation == "inspect":
        return {"ok": True, "inspection": inspect_audio(Path(request["path"]).expanduser().resolve())}
    if operation == "preflight":
        return {"ok": True, "preflight": preflight(request)}
    if operation == "train":
        run_training(request)
        return {"ok": True}
    raise ValueError(f"unsupported RVC training operation: {operation}")


def main() -> None:
    if len(sys.argv) == 3 and sys.argv[1] == "--job":
        request_path = Path(sys.argv[2]).resolve()
        request = json.loads(request_path.read_text(encoding="utf-8"))
        job_path = Path(request["job_path"]).resolve()
        log_path = Path(request["log_path"]).resolve()
        try:
            run_training(request)
        except BaseException as error:
            append_log(log_path, f"{type(error).__name__}: {error}")
            try:
                update_job(job_path, status="failed", finished_at=int(time.time()), failure=f"{type(error).__name__}: {error}")
            except Exception:
                pass
            raise
        return
    request = json.load(sys.stdin)
    result = handle_request(request)
    respond(**result)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        if not (len(sys.argv) == 3 and sys.argv[1] == "--job"):
            respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
