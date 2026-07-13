import json
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Dia adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    if "[S1]" not in text and "[S2]" not in text:
        text = f"[S1] {text}"
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    import torch
    from transformers import AutoProcessor, DiaForConditionalGeneration

    if torch.cuda.is_available():
        device = "cuda"
    elif torch.backends.mps.is_available():
        device = "mps"
    else:
        device = "cpu"
    checkpoint = "nari-labs/Dia-1.6B-0626"
    processor = AutoProcessor.from_pretrained(checkpoint)
    inputs = processor(text=[text], padding=True, return_tensors="pt").to(device)
    model = DiaForConditionalGeneration.from_pretrained(checkpoint).to(device)
    outputs = model.generate(
        **inputs,
        max_new_tokens=3072,
        guidance_scale=3.0,
        temperature=1.8,
        top_p=0.90,
        top_k=45,
    )
    decoded = processor.batch_decode(outputs)
    processor.save_audio(decoded, str(output_path))
    if not output_path.is_file():
        raise RuntimeError(f"Dia did not create {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=None,
        voice=request.get("voice") or "default",
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
