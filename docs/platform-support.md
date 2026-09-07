# Platform support

Takokit `0.x.x` releases are Beta. The v0.3.0 release target extends the public runtime from Windows to Linux x86_64 and macOS Apple Silicon while keeping shared CLI/TUI/server/GUI/API/storage semantics.

| Platform | Architecture | Distribution | v0.3.0 status |
| --- | --- | --- | --- |
| Windows 10/11 | x86_64 | Inno per-user installer, portable ZIP, PowerShell bootstrap | Supported Beta |
| Linux | x86_64 | Per-user install, portable tar.gz, freedesktop launcher, POSIX bootstrap | Supported Beta |
| macOS 12+ | arm64 | Per-user install, portable tar.gz, `Takokit.app`, POSIX bootstrap | Supported Beta |
| macOS 12+ | x86_64 | Contract/build path only | Experimental; not a stable v0.3.0 download target |
| Linux | arm64 | — | Unsupported in v0.3.0; not advertised/published |

The CLI, TUI, server, browser GUI, OpenAI-compatible audio routes, Takokit-native API, registry, workspace/session semantics, and release-signature validation are shared. Platform adapters only own installation, launch/resident integration, package format, and executable replacement.

## Runner/model truth

| Runtime/model family | Windows x86_64 | Linux x86_64 | macOS arm64 |
| --- | --- | --- | --- |
| `takokit-onnx`, Kokoro, Piper | Supported | Supported; CPU acceptance | Supported on CPU; MPS is not used by ONNX acceptance |
| `takokit-whispercpp`, Whisper Tiny | Supported | Supported; CPU acceptance | Supported; CPU acceptance |
| `takokit-python-managed`, F5, Chatterbox, OpenVoice, Qwen TTS | Hardware/upstream dependent | Experimental; CPU/CUDA dependency resolution varies by adapter | Experimental; CPU/MPS support varies upstream |
| `takokit-nemo` | CUDA hardware dependent | CUDA hardware dependent | Unsupported where selected upstream requires CUDA |
| RVC inference | Supported with prepared adapter | Experimental; CPU/CUDA dependent | Experimental CPU; no CUDA equivalence claimed |
| RVC training | CUDA recommended | Hardware dependent; Linux CUDA is intended GPU path | Unsupported as a v0.3.0 acceptance target; no MPS parity claim |

MPS and CUDA are accelerator backends, not distribution requirements. The packaged CLI/server run without either. Real large-model parity remains model/upstream-specific and is never inferred from a successful package build.

## macOS signing

Takokit's release notes/evidence must report the real Apple signing/notarization state. Takokit release-manifest signing (`takokit-release-v1`) verifies Takokit artifacts independently of Apple Developer ID signing. If a beta is only ad-hoc signed, Gatekeeper-facing behavior remains a documented beta limitation rather than something the installer bypasses.
