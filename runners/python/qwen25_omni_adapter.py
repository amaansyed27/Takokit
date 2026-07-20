"""Takokit local transcription adapter for Qwen2.5-Omni."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("operation") != "transcribe":
        raise ValueError("Qwen2.5-Omni adapter only supports transcription")
    audio = Path(str(request["audio_path"])).expanduser().resolve()
    model_dir = Path(str(request["model_dir"])).expanduser().resolve()
    if not audio.is_file() or not model_dir.is_dir():
        raise FileNotFoundError("Qwen2.5-Omni audio or local model snapshot is missing")

    from qwen_omni_utils import process_mm_info
    from transformers import Qwen2_5OmniForConditionalGeneration, Qwen2_5OmniProcessor

    model = Qwen2_5OmniForConditionalGeneration.from_pretrained(
        str(model_dir),
        torch_dtype="auto",
        device_map="auto",
        local_files_only=True,
    )
    model.disable_talker()
    processor = Qwen2_5OmniProcessor.from_pretrained(
        str(model_dir), local_files_only=True
    )
    conversation = [
        {
            "role": "user",
            "content": [
                {"type": "audio", "audio": str(audio)},
                {
                    "type": "text",
                    "text": "Transcribe this audio exactly. Output only the transcript.",
                },
            ],
        }
    ]
    prompt = processor.apply_chat_template(
        conversation, add_generation_prompt=True, tokenize=False
    )
    audios, images, videos = process_mm_info(conversation, use_audio_in_video=False)
    inputs = processor(
        text=prompt,
        audio=audios,
        images=images,
        videos=videos,
        return_tensors="pt",
        padding=True,
        use_audio_in_video=False,
    )
    inputs = inputs.to(model.device)
    generated = model.generate(
        **inputs,
        return_audio=False,
        use_audio_in_video=False,
        max_new_tokens=2048,
    )
    sequences = generated.sequences if hasattr(generated, "sequences") else generated
    prefix = inputs["input_ids"].shape[1]
    text = processor.batch_decode(
        sequences[:, prefix:],
        skip_special_tokens=True,
        clean_up_tokenization_spaces=False,
    )[0].strip()
    if not text:
        raise RuntimeError("Qwen2.5-Omni returned an empty transcript")
    respond(ok=True, text=text)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
