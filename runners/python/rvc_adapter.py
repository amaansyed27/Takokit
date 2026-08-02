"""Takokit adapter for official RVC library voice conversion."""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path
from typing import Any

F0_METHODS = {"rmvpe", "harvest", "crepe", "pm"}
REFERENCE_NAMES = ("reference.wav", "target.wav", "sample.wav", "preview.wav")


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def load_package_manifest(root: Path) -> dict[str, Any]:
    path = root / "rvc.json"
    if not path.is_file():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"RVC package manifest must contain a JSON object: {path}")
    return payload


def resolve_manifest_path(root: Path, value: object, label: str) -> Path | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"RVC package {label} must be a non-empty relative path")
    candidate = (root / value).resolve()
    if root != candidate and root not in candidate.parents:
        raise ValueError(f"RVC package {label} escapes the target directory: {value}")
    if not candidate.is_file():
        raise FileNotFoundError(f"RVC package {label} does not exist: {candidate}")
    return candidate


def find_model(target: Path) -> tuple[Path, Path | None, str, Path | None, bool]:
    root = target.parent if target.is_file() else target
    if not root.is_dir():
        raise FileNotFoundError(f"RVC target checkpoint does not exist: {target}")
    manifest = load_package_manifest(root)

    if target.is_file():
        if target.suffix.lower() != ".pth":
            raise ValueError(f"RVC target file must be a .pth checkpoint: {target}")
        model = target
    elif manifest.get("checkpoint") is not None:
        model = resolve_manifest_path(root, manifest.get("checkpoint"), "checkpoint")
        assert model is not None
        if model.suffix.lower() != ".pth":
            raise ValueError(f"RVC package checkpoint must use .pth: {model}")
    else:
        models = sorted(root.rglob("*.pth"))
        if not models:
            raise FileNotFoundError(f"no RVC .pth checkpoint found below {root}")
        if len(models) != 1:
            names = ", ".join(path.name for path in models)
            raise ValueError(
                "multiple RVC checkpoints were found; add rvc.json with an explicit "
                f"checkpoint field: {names}"
            )
        model = models[0]

    index, pairing_status = find_index(root, model, manifest)
    reference = find_reference(root, manifest)
    quality_ready = bool(
        manifest.get("quality_baseline") is True
        and isinstance(manifest.get("license"), str)
        and manifest.get("license", "").strip()
        and reference is not None
        and pairing_status != "single_index_unverified"
    )
    return model.resolve(), index, pairing_status, reference, quality_ready


def find_index(
    root: Path, model: Path, manifest: dict[str, Any]
) -> tuple[Path | None, str]:
    explicit = resolve_manifest_path(root, manifest.get("index"), "index")
    if explicit is not None:
        if explicit.suffix.lower() != ".index":
            raise ValueError(f"RVC package index must use .index: {explicit}")
        return explicit.resolve(), "manifest_verified"

    indexes = sorted(root.rglob("*.index"))
    if not indexes:
        return None, "no_index"
    model_name = model.stem.lower()
    matches = [path for path in indexes if model_name in path.stem.lower()]
    if len(matches) == 1:
        return matches[0].resolve(), "matched_by_name"
    if len(indexes) == 1:
        return indexes[0].resolve(), "single_index_unverified"
    names = ", ".join(path.name for path in indexes)
    raise ValueError(
        "multiple RVC indexes were found and none matched the checkpoint name; "
        f"add rvc.json with an explicit index field: {names}"
    )


def find_reference(root: Path, manifest: dict[str, Any]) -> Path | None:
    explicit = resolve_manifest_path(root, manifest.get("target_reference"), "target_reference")
    if explicit is not None:
        return explicit.resolve()
    for name in REFERENCE_NAMES:
        candidate = root / name
        if candidate.is_file():
            return candidate.resolve()
    wavs = sorted(root.glob("*.wav"))
    return wavs[0].resolve() if len(wavs) == 1 else None


def configure_rvc_roots(model_path: Path, index_path: Path | None) -> None:
    """Configure the directory variables required by the pinned RVC library."""
    os.environ["weight_root"] = str(model_path.parent)
    os.environ["index_root"] = str(index_path.parent if index_path else model_path.parent)


def install_pyav_mode_compat() -> None:
    """Normalize legacy binary mode strings rejected by current PyAV releases."""
    import av

    original_open = av.open
    if getattr(original_open, "_takokit_mode_compat", False):
        return

    def open_compat(file, mode=None, *args, **kwargs):
        if mode == "rb":
            mode = "r"
        elif mode == "wb":
            mode = "w"
        return original_open(file, mode, *args, **kwargs)

    open_compat._takokit_mode_compat = True
    av.open = open_compat


def checkpoint_candidate(file: object) -> Path | None:
    """Resolve a torch.load path from a path-like value or an opened file object."""
    value = file if isinstance(file, (str, os.PathLike)) else getattr(file, "name", None)
    if not isinstance(value, (str, os.PathLike)):
        return None
    try:
        return Path(value).expanduser().resolve()
    except (OSError, RuntimeError, TypeError, ValueError):
        return None


def install_trusted_torch_checkpoint_compat(trusted_checkpoint: Path) -> None:
    """Permit Fairseq to deserialize Takokit's pinned HuBERT checkpoint on PyTorch 2.6+."""
    import torch

    original_load = torch.load
    if getattr(original_load, "_takokit_rvc_checkpoint_compat", False):
        return

    trusted_checkpoint = trusted_checkpoint.resolve()

    def load_compat(file, *args, **kwargs):
        candidate = checkpoint_candidate(file)
        if candidate == trusted_checkpoint and "weights_only" not in kwargs:
            kwargs["weights_only"] = False
        return original_load(file, *args, **kwargs)

    load_compat._takokit_rvc_checkpoint_compat = True
    torch.load = load_compat


def number(request: dict[str, Any], key: str, default: float) -> float:
    value = request.get(key, default)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"RVC {key} must be numeric")
    return float(value)


def effective_settings(request: dict[str, Any], has_index: bool) -> dict[str, Any]:
    f0_method = str(request.get("f0_method") or "rmvpe").lower()
    pitch_shift = int(number(request, "pitch_shift", 0))
    index_rate = number(request, "index_rate", 0.75) if has_index else 0.0
    rms_mix_rate = number(request, "rms_mix_rate", 0.25)
    protect = number(request, "protect", 0.33)
    filter_radius = int(number(request, "filter_radius", 3))

    if f0_method not in F0_METHODS:
        raise ValueError(f"unsupported RVC f0_method: {f0_method}")
    if not -24 <= pitch_shift <= 24:
        raise ValueError("RVC pitch_shift must be between -24 and 24")
    if not 0.0 <= index_rate <= 1.0:
        raise ValueError("RVC index_rate must be between 0.0 and 1.0")
    if not 0.0 <= rms_mix_rate <= 1.0:
        raise ValueError("RVC rms_mix_rate must be between 0.0 and 1.0")
    if not 0.0 <= protect <= 0.5:
        raise ValueError("RVC protect must be between 0.0 and 0.5")
    if not 0 <= filter_radius <= 7:
        raise ValueError("RVC filter_radius must be between 0 and 7")

    return {
        "f0_method": f0_method,
        "pitch_shift": pitch_shift,
        "index_rate": index_rate,
        "rms_mix_rate": rms_mix_rate,
        "protect": protect,
        "filter_radius": filter_radius,
    }


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def checkpoint_metadata(
    model: Path,
    index: Path | None,
    pairing_status: str,
    reference: Path | None,
    quality_ready: bool,
) -> dict[str, Any]:
    return {
        "checkpoint_path": str(model),
        "checkpoint_sha256": sha256(model),
        "checkpoint_bytes": model.stat().st_size,
        "index_path": str(index) if index else None,
        "index_sha256": sha256(index) if index else None,
        "index_bytes": index.stat().st_size if index else None,
        "pairing_status": pairing_status,
        "target_reference_path": str(reference) if reference else None,
        "quality_baseline_ready": quality_ready,
    }


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "convert":
        raise ValueError("RVC adapter only supports voice conversion")
    source_audio = Path(request["audio_path"]).expanduser().resolve()
    target = Path(request["target_voice"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    hubert_path = model_dir / "hubert_base.pt"
    rmvpe_path = model_dir / "rmvpe.pt"
    if not source_audio.is_file():
        raise FileNotFoundError(f"source audio does not exist: {source_audio}")
    if not hubert_path.is_file() or not rmvpe_path.is_file():
        raise FileNotFoundError(
            f"RVC base assets are incomplete below {model_dir}; run `tako pull rvc`"
        )
    model_path, index_path, pairing_status, reference, quality_ready = find_model(target)
    settings = effective_settings(request, index_path is not None)
    configure_rvc_roots(model_path, index_path)

    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    os.environ["rmvpe_root"] = str(model_dir)
    os.environ["hubert_path"] = str(hubert_path)
    install_pyav_mode_compat()
    install_trusted_torch_checkpoint_compat(hubert_path)
    from scipy.io import wavfile
    from rvc.modules.vc.modules import VC

    converter = VC()
    converter.get_vc(model_path.name)
    sample_rate, audio, _, error = converter.vc_inference(
        0,
        str(source_audio),
        f0_up_key=settings["pitch_shift"],
        f0_method=settings["f0_method"],
        index_file=str(index_path) if index_path else "",
        index_rate=settings["index_rate"],
        filter_radius=settings["filter_radius"],
        resample_sr=0,
        rms_mix_rate=settings["rms_mix_rate"],
        protect=settings["protect"],
        hubert_path=str(hubert_path),
    )
    if error or sample_rate is None or audio is None:
        raise RuntimeError(error or "RVC returned no converted audio")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    wavfile.write(str(output_path), int(sample_rate), audio)
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"RVC did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=int(sample_rate),
        voice=str(model_path),
        effective_settings=settings,
        checkpoint=checkpoint_metadata(
            model_path, index_path, pairing_status, reference, quality_ready
        ),
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
