# References

Date checked: 2026-07-09.

## Piper

- Piper voices source: https://huggingface.co/rhasspy/piper-voices
- Lessac medium path: https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/lessac/medium
- Lessac medium artifacts: `en_US-lessac-medium.onnx` and `en_US-lessac-medium.onnx.json`.
- Hugging Face lists the Piper voices repository as MIT licensed and describes the voices as ONNX files.
- `en_US-lessac-medium.onnx`: 63,201,294 bytes, SHA256 `5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f`.
- `en_US-lessac-medium.onnx.json`: 4,885 bytes, SHA256 `efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0`.
- The ONNX hash matches the Hugging Face LFS oid. Both files were downloaded from the `resolve/main` URLs and hashed locally.

## Runtime Licensing Notes

- Old Piper repository: https://github.com/rhasspy/piper
- The old `rhasspy/piper` repository is archived and points development to `OHF-Voice/piper1-gpl`.
- New Piper repository: https://github.com/OHF-Voice/piper1-gpl
- `OHF-Voice/piper1-gpl` is GPL-3.0 licensed.
- Takokit may use Piper voices as downloadable artifacts, but must not vendor GPL runtime code without an explicit licensing decision.
