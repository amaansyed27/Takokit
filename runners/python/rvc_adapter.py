"""Takokit adapter for official RVC library voice conversion."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import types
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


def install_fairseq_import_stub() -> None:
    """Satisfy the pinned RVC utils import without compiling fairseq extensions.

    Takokit supplies a local Transformers HuBERT model before RVC inference starts,
    so the legacy fairseq checkpoint loader must never be executed.
    """
    if "fairseq" in sys.modules:
        return
    fairseq = types.ModuleType("fairseq")
    checkpoint_utils = types.ModuleType("fairseq.checkpoint_utils")

    def unsupported_loader(*_args, **_kwargs):
        raise RuntimeError(
            "Takokit RVC uses the managed Transformers HuBERT runtime; "
            "the legacy fairseq checkpoint loader is disabled"
        )

    checkpoint_utils.load_model_ensemble_and_task = unsupported_loader
    fairseq.checkpoint_utils = checkpoint_utils
    sys.modules["fairseq"] = fairseq
    sys.modules["fairseq.checkpoint_utils"] = checkpoint_utils


def load_transformers_hubert(model_root: Path, device: object, is_half: bool):
    """Expose Transformers HuBERT with the small API expected by the pinned RVC pipeline."""
    import torch
    import torch.nn.functional as F
    from torch import nn
    from transformers import AutoFeatureExtractor, HubertModel

    class HubertModelWithFinalProj(HubertModel):
        def __init__(self, config):
            super().__init__(config)
            self.final_proj = nn.Linear(config.hidden_size, config.classifier_proj_size)

    class HubertCompat:
        def __init__(self):
            self.device = device
            self.normalize_audio = bool(
                AutoFeatureExtractor.from_pretrained(
                    str(model_root), local_files_only=True
                ).do_normalize
            )
            dtype = torch.float16 if is_half and "cpu" not in str(device) else torch.float32
            self.model = HubertModelWithFinalProj.from_pretrained(
                str(model_root),
                local_files_only=True,
                torch_dtype=dtype,
            ).to(device)
            self.model.eval()
            self.final_proj = self.model.final_proj

        def extract_features(
            self,
            source,
            padding_mask=None,
            output_layer: int = 12,
        ):
            if self.normalize_audio:
                source = F.layer_norm(source, source.shape[1:])
            attention_mask = None
            if padding_mask is not None and bool(torch.any(padding_mask).item()):
                attention_mask = (~padding_mask.bool()).long().to(source.device)
            layer = int(output_layer)
            if layer == 9:
                outputs = self.model(
                    input_values=source,
                    attention_mask=attention_mask,
                    output_hidden_states=True,
                    return_dict=True,
                )
                features = outputs.hidden_states[9]
            elif layer == 12:
                features = self.model(
                    input_values=source,
                    attention_mask=attention_mask,
                    output_hidden_states=False,
                    return_dict=True,
                ).last_hidden_state
            else:
                raise ValueError(f"unsupported RVC HuBERT output layer: {layer}")
            return features, padding_mask

    return HubertCompat()


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
    hubert_root = model_dir / "hubert_base"
    rmvpe_path = model_dir / "rmvpe.pt"
    if not source_audio.is_file():
        raise FileNotFoundError(f"source audio does not exist: {source_audio}")
    required_hubert = [
        hubert_root / "config.json",
        hubert_root / "preprocessor_config.json",
        hubert_root / "pytorch_model.bin",
    ]
    if not all(path.is_file() for path in required_hubert) or not rmvpe_path.is_file():
        raise FileNotFoundError(
            f"RVC base assets are incomplete below {model_dir}; run `tako pull rvc`"
        )
    model_path, index_path, pairing_status, reference, quality_ready = find_model(target)
    settings = effective_settings(request, index_path is not None)
    configure_rvc_roots(model_path, index_path)

    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    os.environ["rmvpe_root"] = str(model_dir)
    install_pyav_mode_compat()
    install_fairseq_import_stub()
    from scipy.io import wavfile
    from rvc.modules.vc.modules import VC

    converter = VC()
    converter.get_vc(model_path.name)
    converter.hubert_model = load_transformers_hubert(
        hubert_root, converter.config.device, converter.config.is_half
    )
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
        hubert_path=str(hubert_root),
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