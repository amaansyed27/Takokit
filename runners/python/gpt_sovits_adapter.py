"""Takokit adapter for official GPT-SoVITS inference and two-stage training."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def source_root() -> Path:
    return Path(__file__).resolve().parent / "source"


def model_paths(model_dir: Path) -> dict[str, Path]:
    return {
        "bert": model_dir / "chinese-roberta-wwm-ext-large",
        "hubert": model_dir / "chinese-hubert-base",
        "s1": model_dir
        / "gsv-v2final-pretrained"
        / "s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
        "s2g": model_dir / "gsv-v2final-pretrained" / "s2G2333k.pth",
        "s2d": model_dir / "gsv-v2final-pretrained" / "s2D2333k.pth",
    }


def require_paths(paths: dict[str, Path]) -> None:
    missing = [f"{name}: {path}" for name, path in paths.items() if not path.exists()]
    if missing:
        raise FileNotFoundError("GPT-SoVITS snapshot is incomplete: " + "; ".join(missing))


def run_checked(command: list[str], *, cwd: Path, env: dict[str, str], log) -> None:
    log.write("$ " + " ".join(command) + "\n")
    log.flush()
    process = subprocess.run(
        command,
        cwd=str(cwd),
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if process.returncode != 0:
        raise RuntimeError(f"command exited with {process.returncode}: {' '.join(command)}")


def run_speech(request: dict, model_dir: Path) -> None:
    source = source_root()
    sys.path.insert(0, str(source))
    sys.path.insert(0, str(source / "GPT_SoVITS"))
    os.chdir(source)
    paths = model_paths(model_dir)
    require_paths(paths)

    import soundfile as sf
    import torch
    from GPT_SoVITS.TTS_infer_pack.TTS import TTS, TTS_Config

    text = str(request.get("input") or "").strip()
    reference = request.get("voice")
    if not text:
        raise ValueError("speech input cannot be empty")
    if not reference:
        raise ValueError(
            "GPT-SoVITS requires --voice with a consent-backed reference sample"
        )
    reference_path = Path(reference).expanduser().resolve()
    if not reference_path.is_file():
        raise FileNotFoundError(f"reference audio does not exist: {reference_path}")

    config = {
        "custom": {
            "bert_base_path": str(paths["bert"]),
            "cnhuhbert_base_path": str(paths["hubert"]),
            "device": "cuda" if torch.cuda.is_available() else "cpu",
            "is_half": bool(torch.cuda.is_available()),
            "t2s_weights_path": str(paths["s1"]),
            "version": "v2",
            "vits_weights_path": str(paths["s2g"]),
        }
    }
    pipeline = TTS(TTS_Config(config))
    language = str(request.get("language") or "auto").lower()
    prompt_text = str(request.get("reference_text") or "").strip()
    tts_request = {
        "text": text,
        "text_lang": language,
        "ref_audio_path": str(reference_path),
        "prompt_text": prompt_text,
        "prompt_lang": language,
        "top_k": 15,
        "top_p": 1.0,
        "temperature": 1.0,
        "text_split_method": "cut5",
        "batch_size": 1,
        "speed_factor": 1.0,
        "streaming_mode": False,
        "return_fragment": False,
        "seed": -1,
    }
    chunks = list(pipeline.run(tts_request))
    if not chunks:
        raise RuntimeError("GPT-SoVITS returned no audio")
    sample_rate = int(chunks[0][0])
    import numpy as np

    audio = np.concatenate([chunk for _, chunk in chunks])
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output_path), audio, sample_rate)
    if not output_path.is_file() or output_path.stat().st_size <= 44:
        raise RuntimeError(f"GPT-SoVITS did not create a valid WAV at {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=sample_rate,
        voice=str(reference_path),
    )


def merge_parts(directory: Path, pattern: str, output: Path, header: str | None = None) -> None:
    parts = sorted(directory.glob(pattern))
    if not parts:
        raise FileNotFoundError(f"no generated dataset parts matched {pattern} in {directory}")
    lines: list[str] = []
    for part in parts:
        lines.extend(part.read_text(encoding="utf-8").splitlines())
    if header is not None:
        lines.insert(0, header)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_training(request: dict, model_dir: Path) -> None:
    source = source_root()
    paths = model_paths(model_dir)
    require_paths(paths)
    dataset = Path(request["dataset_path"]).expanduser().resolve()
    train_list = dataset / "train.list"
    wav_dir = dataset / "wavs"
    if not train_list.is_file() or not wav_dir.is_dir():
        raise FileNotFoundError(
            "GPT-SoVITS training requires dataset/train.list and dataset/wavs/. "
            "Each train.list row must be wav_path|speaker|language|transcript."
        )
    output_dir = Path(request["output_dir"]).expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    experiment = output_dir / "experiment"
    experiment.mkdir(parents=True, exist_ok=True)
    logs = output_dir / "train.log"
    epochs = max(1, int(request.get("epochs") or 5))
    name = str(request.get("name") or "takokit-voice").strip()

    env = os.environ.copy()
    env.update(
        {
            "inp_text": str(train_list),
            "inp_wav_dir": str(wav_dir),
            "exp_name": name,
            "i_part": "0",
            "all_parts": "1",
            "_CUDA_VISIBLE_DEVICES": "0",
            "opt_dir": str(experiment),
            "bert_pretrained_dir": str(paths["bert"]),
            "cnhubert_base_dir": str(paths["hubert"]),
            "pretrained_s2G": str(paths["s2g"]),
            "s2config_path": str(source / "GPT_SoVITS" / "configs" / "s2.json"),
            "is_half": "True",
            "version": "v2",
        }
    )
    python = sys.executable
    with logs.open("w", encoding="utf-8") as log:
        for script in [
            "GPT_SoVITS/prepare_datasets/1-get-text.py",
            "GPT_SoVITS/prepare_datasets/2-get-hubert-wav32k.py",
            "GPT_SoVITS/prepare_datasets/3-get-semantic.py",
        ]:
            run_checked([python, "-s", script], cwd=source, env=env, log=log)
        merge_parts(experiment, "2-name2text-*.txt", experiment / "2-name2text.txt")
        merge_parts(
            experiment,
            "6-name2semantic-*.tsv",
            experiment / "6-name2semantic.tsv",
            "item_name\tsemantic_audio",
        )

        s2_config = json.loads(
            (source / "GPT_SoVITS" / "configs" / "s2.json").read_text(encoding="utf-8")
        )
        s2_config["train"].update(
            {
                "batch_size": 1,
                "epochs": epochs,
                "pretrained_s2G": str(paths["s2g"]),
                "pretrained_s2D": str(paths["s2d"]),
                "if_save_latest": True,
                "if_save_every_weights": True,
                "save_every_epoch": 1,
                "gpu_numbers": "0",
                "fp16_run": True,
            }
        )
        s2_config["model"]["version"] = "v2"
        s2_config["data"]["exp_dir"] = str(experiment)
        s2_config["s2_ckpt_dir"] = str(experiment / "logs_s2_v2")
        s2_config["save_weight_dir"] = str(output_dir / "SoVITS_weights_v2")
        s2_config["name"] = name
        s2_config["version"] = "v2"
        s2_path = output_dir / "takokit-s2.json"
        s2_path.write_text(json.dumps(s2_config, indent=2), encoding="utf-8")
        run_checked(
            [python, "-s", "GPT_SoVITS/s2_train.py", "--config", str(s2_path)],
            cwd=source,
            env=env,
            log=log,
        )

        import yaml

        s1_config = yaml.safe_load(
            (source / "GPT_SoVITS" / "configs" / "s1longer-v2.yaml").read_text(
                encoding="utf-8"
            )
        )
        s1_config["train"].update(
            {
                "batch_size": 1,
                "epochs": epochs,
                "save_every_n_epoch": 1,
                "if_save_every_weights": True,
                "if_save_latest": True,
                "half_weights_save_dir": str(output_dir / "GPT_weights_v2"),
                "exp_name": name,
            }
        )
        s1_config["pretrained_s1"] = str(paths["s1"])
        s1_config["train_semantic_path"] = str(experiment / "6-name2semantic.tsv")
        s1_config["train_phoneme_path"] = str(experiment / "2-name2text.txt")
        s1_config["output_dir"] = str(experiment / "logs_s1_v2")
        s1_path = output_dir / "takokit-s1.yaml"
        s1_path.write_text(yaml.safe_dump(s1_config), encoding="utf-8")
        run_checked(
            [python, "-s", "GPT_SoVITS/s1_train.py", "--config_file", str(s1_path)],
            cwd=source,
            env=env,
            log=log,
        )

    marker = output_dir / "training-complete.json"
    marker.write_text(
        json.dumps(
            {
                "model": "gpt-sovits",
                "name": name,
                "epochs": epochs,
                "dataset": str(dataset),
            },
            indent=2,
        ),
        encoding="utf-8",
    )
    respond(
        ok=True,
        output_path=str(output_dir),
        status="completed",
        log_path=str(logs),
    )


def main() -> None:
    request = json.load(sys.stdin)
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    operation = request.get("operation")
    if operation == "speech":
        run_speech(request, model_dir)
    elif operation == "train":
        run_training(request, model_dir)
    else:
        raise ValueError(f"GPT-SoVITS does not support operation: {operation}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
