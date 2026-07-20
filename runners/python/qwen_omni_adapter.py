"""Takokit adapter for Qwen2.5-Omni and Qwen3-Omni audio workflows."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def respond(**payload: object) -> None:
    print(json.dumps(payload), flush=True)


def load_runtime(model_id: str, model_dir: Path):
    if model_id == "qwen2-5-omni":
        from transformers import (
            Qwen2_5OmniForConditionalGeneration,
            Qwen2_5OmniProcessor,
        )

        model = Qwen2_5OmniForConditionalGeneration.from_pretrained(
            str(model_dir), device_map="auto", torch_dtype="auto"
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
    model, processor = load_runtime(model_id, model_dir)

    if operation == "transcribe":
        audio_path = Path(request["audio_path"]).expanduser().resolve()
        if not audio_path.is_file():
            raise FileNotFoundError(f"audio file does not exist: {audio_path}")
        if hasattr(model, "disable_talker"):
            model.disable_talker()
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
        generated = model.generate(**inputs, use_audio_in_video=True)
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
            "Speak the following text exactly and do not add any other words: " + text
        )
        messages = [
            {
                "role": "user",
                "content": [{"type": "text", "text": instruction}],
            }
        ]
        inputs = build_inputs(model, processor, messages)
        generated = model.generate(
            **inputs, speaker=voice, use_audio_in_video=True
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
