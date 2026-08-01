"""Takokit adapter for Qwen2.5-Omni and Qwen3-Omni audio workflows."""

from __future__ import annotations

import ctypes
import json
import sys
from pathlib import Path


QWEN25_SYSTEM_PROMPT = (
    "You are Qwen, a virtual human developed by the Qwen Team, Alibaba Group, "
    "capable of perceiving auditory and visual inputs, as well as generating "
    "text and speech."
)
GIB = 1024**3


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def windows_available_commit_bytes() -> int | None:
    if sys.platform != "win32":
        return None

    class MemoryStatusEx(ctypes.Structure):
        _fields_ = [
            ("dwLength", ctypes.c_ulong),
            ("dwMemoryLoad", ctypes.c_ulong),
            ("ullTotalPhys", ctypes.c_ulonglong),
            ("ullAvailPhys", ctypes.c_ulonglong),
            ("ullTotalPageFile", ctypes.c_ulonglong),
            ("ullAvailPageFile", ctypes.c_ulonglong),
            ("ullTotalVirtual", ctypes.c_ulonglong),
            ("ullAvailVirtual", ctypes.c_ulonglong),
            ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
        ]

    status = MemoryStatusEx()
    status.dwLength = ctypes.sizeof(status)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        return None
    return int(status.ullAvailPageFile)


def require_commit_headroom(operation: str) -> None:
    available = windows_available_commit_bytes()
    if available is None:
        return
    required = 18 * GIB if operation == "speech" else 10 * GIB
    if available < required:
        raise RuntimeError(
            "Qwen2.5-Omni cannot start safely because Windows has only "
            f"{available / GIB:.1f} GiB of available committed memory; "
            f"this {operation} path requires at least {required / GIB:.0f} GiB. "
            "Close memory-heavy applications or increase the Windows paging file."
        )


def load_runtime(model_id: str, model_dir: Path, operation: str):
    try:
        import torchvision  # noqa: F401
    except ImportError as error:
        raise RuntimeError(
            "Qwen Omni requires torchvision for its multimodal processor"
        ) from error

    if model_id == "qwen2-5-omni":
        from transformers import (
            Qwen2_5OmniForConditionalGeneration,
            Qwen2_5OmniProcessor,
        )

        require_commit_headroom(operation)
        enable_audio_output = operation == "speech"
        model = Qwen2_5OmniForConditionalGeneration.from_pretrained(
            str(model_dir),
            device_map="auto",
            torch_dtype="auto",
            low_cpu_mem_usage=True,
            enable_audio_output=enable_audio_output,
        )
        processor = Qwen2_5OmniProcessor.from_pretrained(str(model_dir))
        return model, processor
    if model_id == "qwen3-omni":
        from transformers import (
            Qwen3OmniMoeForConditionalGeneration,
            Qwen3OmniMoeProcessor,
        )

        model = Qwen3OmniMoeForConditionalGeneration.from_pretrained(
            str(model_dir), device_map="auto", dtype="auto"
        )
        processor = Qwen3OmniMoeProcessor.from_pretrained(str(model_dir))
        return model, processor
    raise ValueError(f"unsupported Qwen Omni model id: {model_id}")


def build_inputs(model, processor, messages):
    from qwen_omni_utils import process_mm_info

    prompt = processor.apply_chat_template(
        messages, add_generation_prompt=True, tokenize=False
    )
    audios, images, videos = process_mm_info(messages, use_audio_in_video=True)
    inputs = processor(
        text=prompt,
        audio=audios,
        images=images,
        videos=videos,
        return_tensors="pt",
        padding=True,
        use_audio_in_video=True,
    )
    return inputs.to(model.device).to(model.dtype)


def generated_parts(result):
    if isinstance(result, tuple):
        text_ids = result[0]
        audio = result[1] if len(result) > 1 else None
        return text_ids, audio
    return result, None


def decode_text(processor, text_ids) -> str:
    text = processor.batch_decode(
        text_ids, skip_special_tokens=True, clean_up_tokenization_spaces=False
    )[0]
    return text.split("\n")[-1].strip()


def main() -> None:
    request = json.load(sys.stdin)
    operation = request.get("operation")
    model_id = str(request["model_id"])
    model_dir = Path(request["model_dir"]).expanduser().resolve()
    if not model_dir.is_dir():
        raise FileNotFoundError(f"Qwen Omni snapshot is missing: {model_dir}")
    model, processor = load_runtime(model_id, model_dir, operation)

    if operation == "transcribe":
        audio_path = Path(request["audio_path"]).expanduser().resolve()
        if not audio_path.is_file():
            raise FileNotFoundError(f"audio file does not exist: {audio_path}")
        messages = [
            {
                "role": "system",
                "content": [
                    {
                        "type": "text",
                        "text": "Transcribe the supplied audio exactly. Return only the transcript.",
                    }
                ],
            },
            {
                "role": "user",
                "content": [
                    {"type": "audio", "audio": str(audio_path)},
                    {"type": "text", "text": "Transcribe this audio."},
                ],
            },
        ]
        inputs = build_inputs(model, processor, messages)
        generated = model.generate(
            **inputs,
            use_audio_in_video=True,
            return_audio=False,
            max_new_tokens=256,
        )
        text_ids, _ = generated_parts(generated)
        transcript = decode_text(processor, text_ids)
        if not transcript:
            raise RuntimeError("Qwen Omni returned an empty transcript")
        respond(ok=True, text=transcript)
        return

    if operation == "speech":
        text = str(request.get("input") or "").strip()
        if not text:
            raise ValueError("speech input cannot be empty")
        voice = request.get("voice") or "Chelsie"
        instruction = request.get("instruction") or (
            "Speak the following text exactly and do not add other words: " + text
        )
        messages = [
            {
                "role": "system",
                "content": [{"type": "text", "text": QWEN25_SYSTEM_PROMPT}],
            },
            {
                "role": "user",
                "content": [{"type": "text", "text": instruction}],
            },
        ]
        inputs = build_inputs(model, processor, messages)
        generated = model.generate(
            **inputs,
            speaker=voice,
            use_audio_in_video=True,
            return_audio=True,
            max_new_tokens=96,
        )
        _, audio = generated_parts(generated)
        if audio is None:
            raise RuntimeError("Qwen Omni did not return speech audio")
        import numpy as np
        import soundfile as sf

        if hasattr(audio, "detach"):
            audio = audio.detach().float().cpu().numpy()
        audio = np.asarray(audio).squeeze()
        output_path = Path(request["output_path"]).expanduser().resolve()
        output_path.parent.mkdir(parents=True, exist_ok=True)
        sf.write(str(output_path), audio, 24000)
        if not output_path.is_file() or output_path.stat().st_size <= 44:
            raise RuntimeError(f"Qwen Omni did not create a valid WAV at {output_path}")
        respond(
            ok=True,
            output_path=str(output_path),
            bytes=output_path.stat().st_size,
            sample_rate=24000,
            voice=voice,
        )
        return

    raise ValueError(f"Qwen Omni does not support operation: {operation}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
