"""Takokit adapter for official RVC library voice conversion."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def find_model(target: Path) -> tuple[Path, Path | None]:
    if target.is_file() and target.suffix.lower() == ".pth":
        index = next(target.parent.glob("*.index"), None)
        return target, index
    if target.is_dir():
        model = next(target.rglob("*.pth"), None)
        if model is None:
            raise FileNotFoundError(f"no RVC .pth checkpoint found below {target}")
        return model, next(target.rglob("*.index"), None)
    raise FileNotFoundError(f"RVC target checkpoint does not exist: {target}")


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "convert":
        raise ValueError("RVC adapter only supports voice conversion")
    source_audio = Path(request["audio_path"]).expanduser().resolve()
    target = Path(request["target_voice"]).expanduser().resolve()
    output_path = Path(request["output_path"]).expanduser().resolve()
    if not source_audio.is_file():
        raise FileNotFoundError(f"source audio does not exist: {source_audio}")
    model_path, index_path = find_model(target)

    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    from scipy.io import wavfile
    from rvc.modules.vc.modules import VC

    converter = VC()
    converter.get_vc(str(model_path))
    sample_rate, audio, _, _ = converter.vc_inference(
        0,
        source_audio,
        f0_up_key=int(request.get("pitch_shift") or 0),
        f0_method="rmvpe",
        file_index=str(index_path) if index_path else "",
        index_rate=0.75 if index_path else 0.0,
        filter_radius=3,
        resample_sr=0,
        rms_mix_rate=0.25,
        protect=0.33,
    )
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
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
