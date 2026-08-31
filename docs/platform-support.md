# Platform support for the v0.3.0 candidate

| Platform | Architecture | Distribution | Candidate status |
| --- | --- | --- | --- |
| Windows 10/11 | x86_64 | Inno per-user installer, portable ZIP | Supported; regression target |
| Linux | x86_64 | Per-user install, portable tar.gz, freedesktop launcher | Supported; CI and manual acceptance target |
| macOS 12+ | arm64 | Per-user install, portable tar.gz, Takokit.app launcher | Supported; GitHub-hosted Apple Silicon acceptance target |
| macOS 12+ | x86_64 | Per-user install, portable tar.gz, Takokit.app launcher | Experimental; build-contract only until an x86_64 candidate is produced and exercised |
| Linux | arm64 | — | Unsupported in v0.3.0; not advertised or published |

The CLI, TUI, server, browser GUI, OpenAI-compatible audio routes, Takokit-native API, registry, workspace/session semantics, and release-signature validation are shared. Platform adapters only own installation, launch integration, package format, and executable replacement.

## Runner truth

| Runtime/model family | Windows x86_64 | Linux x86_64 | macOS arm64 |
| --- | --- | --- | --- |
| `takokit-onnx`, Kokoro, Piper | Supported | Supported; CPU smoke target | Supported on CPU; MPS is not used by ONNX |
| `takokit-whispercpp`, Whisper Tiny | Supported | Supported; CPU smoke target | Supported; CPU smoke target |
| `takokit-python-managed`, F5, Chatterbox, OpenVoice, Qwen TTS | Hardware/upstream dependent | Experimental; CPU/CUDA dependency resolution varies by adapter | Experimental; CPU/MPS support varies upstream |
| `takokit-nemo` | CUDA hardware dependent | CUDA hardware dependent | Unsupported where the selected upstream requires CUDA |
| RVC inference | Supported with prepared adapter | Experimental; CPU/CUDA dependent | Experimental CPU; no CUDA equivalence is claimed |
| RVC training | CUDA recommended | Hardware dependent; Linux CUDA is the intended GPU path | Unsupported for v0.3.0 acceptance; no MPS parity claim |

MPS and CUDA are accelerator backends, not distribution requirements. The packaged CLI/server run without either. Real large-model parity remains model- and upstream-specific and is not inferred from a successful package build.
