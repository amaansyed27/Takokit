import json
import sys
from pathlib import Path


MODELS = {
    "xtts-v2": "tts_models/multilingual/multi-dataset/xtts_v2",
    "yourtts": "tts_models/multilingual/multi-dataset/your_tts",
}
CPML_DIGEST = "sha256:3dbb31aa8875793cde77882e71dbb5f80fe31b818ecca4a4a5812a430f7209c7"
CPML_URL = "https://coqui.ai/cpml.txt"


def path_size(path):
    root = Path(path)
    try:
        if root.is_file():
            return root.stat().st_size
        if not root.is_dir():
            return 0
    except OSError:
        return 0

    total = 0
    for item in root.rglob("*"):
        try:
            if item.is_file():
                total += item.stat().st_size
        except OSError:
            continue
    return total


def coqui_model_root(cache_dir, checkpoint):
    model_directory = checkpoint.replace("/", "--")
    coqui_home = Path(cache_dir) / "coqui"
    candidates = [
        coqui_home / "tts" / model_directory,
        coqui_home / model_directory,
    ]
    for candidate in candidates:
        if candidate.is_dir():
            return candidate
    searched = ", ".join(str(candidate) for candidate in candidates)
    raise RuntimeError(
        f"Coqui loaded {checkpoint}, but its checkpoint directory was not found; "
        f"searched: {searched}"
    )


def respond(**payload):
    print(json.dumps(payload), flush=True)


def ensure_compatible_transformers():
    import transformers

    version = str(getattr(transformers, "__version__", "0"))
    try:
        major = int(version.split(".", 1)[0])
    except ValueError:
        major = 0
    if major >= 5:
        raise RuntimeError(
            "Coqui TTS requires Transformers 4.x; Takokit should install "
            f"transformers==4.57.6 in this adapter overlay, but found {version}"
        )


def valid_xtts_license_receipt(request):
    takokit_root = Path(request["cache_dir"]).expanduser().resolve().parent
    receipt_path = (
        takokit_root
        / "licenses"
        / "receipts"
        / "CPML"
        / "xtts-v2.json"
    )
    try:
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    return (
        receipt.get("license_id") == "CPML"
        and receipt.get("license_version") == "1.0.0"
        and receipt.get("license_digest") == CPML_DIGEST
        and receipt.get("license_url") == CPML_URL
        and receipt.get("model_id") == "xtts-v2"
    )


def ensure_xtts_terms_accepted(model_id, request):
    if model_id != "xtts-v2":
        return
    if valid_xtts_license_receipt(request):
        return
    raise RuntimeError(
        "XTTS v2 requires a valid Coqui Public Model License receipt. "
        "Run `tako pull xtts-v2`, review the CPML, and accept the prompt."
    )


def main():
    request = json.load(sys.stdin)
    model_id = request.get("model_id")
    checkpoint = MODELS.get(model_id)
    if not checkpoint:
        raise ValueError(f"unsupported Coqui model: {model_id}")
    ensure_xtts_terms_accepted(model_id, request)
    if request.get("operation") == "prefetch":
        ensure_compatible_transformers()
        from TTS.api import TTS

        TTS(checkpoint)
        model_root = coqui_model_root(request["cache_dir"], checkpoint)
        size_bytes = path_size(model_root)
        if size_bytes <= 0:
            raise RuntimeError(f"Coqui checkpoint directory is empty: {model_root}")
        respond(
            ok=True,
            detail=f"Prefetched {checkpoint}",
            size_bytes=size_bytes,
        )
        return
    if request.get("operation") != "speech":
        raise ValueError("Coqui adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    voice = request.get("voice")
    if not voice:
        raise ValueError(f"{model_id} requires a cloned voice profile or reference audio path")
    reference = Path(voice).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(f"voice reference does not exist: {reference}")
    output_path = Path(request["output_path"]).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    ensure_compatible_transformers()
    import torch
    from TTS.api import TTS

    device = "cuda" if torch.cuda.is_available() else "cpu"
    engine = TTS(checkpoint).to(device)
    engine.tts_to_file(
        text=text,
        speaker_wav=str(reference),
        language="en",
        file_path=str(output_path),
    )
    if not output_path.is_file():
        raise RuntimeError(f"Coqui did not create {output_path}")
    respond(
        ok=True,
        output_path=str(output_path),
        bytes=output_path.stat().st_size,
        sample_rate=None,
        voice=str(reference),
    )


if __name__ == "__main__":
    try:
        main()
    except SystemExit as error:
        respond(ok=False, error=f"SystemExit: {error}")
        raise
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
