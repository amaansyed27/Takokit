# Takokit comprehensive stability and release test guide

This is the mandatory stability gate before website work or a public release. It covers the shared Rust backend, CLI, TUI, GUI, daemon ownership, project-local `.tako` sessions, pull reliability, security and packaging.

Heavy model and voice-profile commands are in [MODEL_SMOKE_TESTS.md](MODEL_SMOKE_TESTS.md). Both guides are required.

Do not promote a model to verified because its adapter compiles. Record real inference evidence.

## 1. Windows test environment

Primary machine:

- Windows 11 x64
- PowerShell 7 recommended
- RTX 5060 Laptop GPU, 8 GB VRAM
- Rust stable
- Node.js LTS and npm
- at least 80 GB free for the full heavy-model pass

Create evidence storage:

```powershell
$Evidence = "$HOME\takokit-test-evidence"
New-Item -ItemType Directory -Force $Evidence | Out-Null
Get-ComputerInfo | Out-File "$Evidence\computer-info.txt"
nvidia-smi | Out-File "$Evidence\nvidia-smi.txt"
rustc --version | Out-File "$Evidence\toolchain.txt"
cargo --version | Add-Content "$Evidence\toolchain.txt"
node --version | Add-Content "$Evidence\toolchain.txt"
npm --version | Add-Content "$Evidence\toolchain.txt"
git rev-parse HEAD | Out-File "$Evidence\commit.txt"
```

## 2. Stop existing processes

A running daemon can lock `target\release\takokit.exe`.

```powershell
if (Test-Path .\target\release\takokit.exe) {
    .\target\release\takokit.exe daemon stop
}
Get-Process takokit,tako -ErrorAction SilentlyContinue | Stop-Process -Force
Get-NetTCPConnection -LocalPort 5050 -ErrorAction SilentlyContinue
```

Expected: no Takokit listener remains.

## 3. Automated gates

From the repository root:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python .\scripts\check_file_sizes.py
```

GUI and release build:

```powershell
Push-Location .\apps\gui
npm ci
npm run build
Pop-Location
cargo build --release
Get-Item .\target\release\takokit.exe
Get-Item .\target\release\tako.exe
```

Save logs:

```powershell
cargo check --workspace *>&1 | Tee-Object "$Evidence\cargo-check.txt"
cargo test --workspace *>&1 | Tee-Object "$Evidence\cargo-test.txt"
```

Pass criteria:

- formatting, check and tests pass,
- GUI TypeScript/Vite build passes,
- file-size audit passes,
- both aliases exist,
- no production path emits an unreachable-code or dead-state warning.

## 4. Clean global runtime store

```powershell
$env:TAKOKIT_HOME = "$env:TEMP\takokit-release-test"
Remove-Item -Recurse -Force $env:TAKOKIT_HOME -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $env:TAKOKIT_HOME | Out-Null
$Tako = (Resolve-Path .\target\release\tako.exe).Path
$Takokit = (Resolve-Path .\target\release\takokit.exe).Path
& $Tako version
& $Takokit version
```

Expected: aliases report the same version and storage root.

## 5. Registry and baseline diagnostics

```powershell
& $Tako doctor --json | Tee-Object "$Evidence\doctor-clean.json"
& $Tako status
& $Tako capabilities
& $Tako models | Tee-Object "$Evidence\models-clean.json"
& $Tako runners | Tee-Object "$Evidence\runners-clean.json"
& $Tako library models | Tee-Object "$Evidence\library-models.json"
& $Tako library runners | Tee-Object "$Evidence\library-runners.json"
```

Verify:

- all 27 model IDs parse,
- every model names a known runner,
- planned models remain non-executable,
- executable-path models are not ready before installation,
- diagnostics provide actionable next steps,
- model status agrees with [model-support.md](model-support.md).

## 6. Daemon ownership and stale recovery

```powershell
& $Tako daemon start
& $Tako daemon status
& $Tako status
& $Tako daemon start
& $Tako daemon restart
& $Tako daemon logs
& $Tako daemon stop
& $Tako daemon status
```

Pass criteria:

- repeated start is idempotent,
- one managed daemon owns port 5050,
- restart preserves storage,
- stop terminates the owned daemon,
- logs and identity records are useful.

Crash recovery:

```powershell
& $Tako daemon start
Get-Process takokit | Stop-Process -Force
& $Tako daemon start
& $Tako daemon status
```

Expected: stale PID/lock ownership is recovered safely.

## 7. Two isolated project workspaces

```powershell
$ProjectA = "$env:TEMP\takokit-project-a"
$ProjectB = "$env:TEMP\takokit-project-b"
Remove-Item -Recurse -Force $ProjectA,$ProjectB -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $ProjectA,$ProjectB | Out-Null

Push-Location $ProjectA
& $Tako sessions new --title "Project A baseline"
& $Tako sessions list | Tee-Object "$Evidence\session-a-list.json"
Get-ChildItem -Recurse -Force .\.tako | Out-File "$Evidence\session-a-tree.txt"
Pop-Location

Push-Location $ProjectB
& $Tako sessions new --title "Project B baseline"
& $Tako sessions list | Tee-Object "$Evidence\session-b-list.json"
Pop-Location
```

Pass criteria:

- each directory owns its own `.tako`,
- A sessions never appear in B,
- models/runners remain under `$env:TAKOKIT_HOME`,
- outputs do not fall back to global `outputs/`.

## 8. Session lifecycle and search

```powershell
Push-Location $ProjectA
$Session = & $Tako sessions new --title "Searchable interview" | ConvertFrom-Json
$SessionId = $Session.summary.id
& $Tako sessions show $SessionId
& $Tako sessions open $SessionId
& $Tako sessions list --query "interview"
& $Tako sessions list --query "does-not-exist"

$Disposable = & $Tako sessions new --title "Delete me" | ConvertFrom-Json
& $Tako sessions new --title "Keep me" | Out-Null
& $Tako sessions rm $Disposable.summary.id
& $Tako sessions list --query "Delete me"
Pop-Location
```

Verify exact resume, content search, inactive deletion and valid active-session state.

## 9. Session output persistence

```powershell
Push-Location $ProjectA
& $Tako speak "Mock session persistence test" --model mock-tts |
  Tee-Object "$Evidence\mock-speech.json"
& $Tako sessions list --query "Mock session persistence test"
Get-ChildItem -Recurse -Force .\.tako\sessions |
  Out-File "$Evidence\mock-session-tree.txt"
Pop-Location
```

Pass criteria:

- non-empty WAV in active session `outputs/`,
- completed event in `events.jsonl`,
- accurate summary counts,
- no output path escape.

## 10. Whisper Tiny baseline

```powershell
$Audio = "C:\path\to\test01_20s.wav"
& $Tako pull whisper-tiny
& $Tako plan whisper-tiny --json |
  Tee-Object "$Evidence\plan-whisper-tiny.json"
Push-Location $ProjectA
& $Tako transcribe $Audio --model whisper-tiny |
  Tee-Object "$Evidence\whisper-tiny-transcript.json"
& $Tako sessions list --query "whisper-tiny"
Pop-Location
```

Verify intelligible text, transcript output, elapsed time and unique repeat outputs.

## 11. Pull idempotency and recovery

```powershell
& $Tako pull whisper-tiny
& $Tako pull whisper-tiny
& $Tako pull kokoro
& $Tako pull kokoro
```

Pass criteria:

- verified artifacts are reused,
- ready state is not downgraded,
- digests are rechecked,
- duplicate content-addressed blobs are not created.

Interrupt one large pull and restart it. A partial file must never be treated as ready. Test checksum rollback only in the isolated store.

## 12. Model and cloning pass

Follow [MODEL_SMOKE_TESTS.md](MODEL_SMOKE_TESTS.md) for:

- Kokoro and Qwen3-TTS,
- Chatterbox and F5-TTS,
- Dia, Bark, MMS, XTTS and YourTTS,
- Kyutai DSM TTS,
- Whisper Base/Small,
- Distil-Whisper, Wav2Vec2, SenseVoice, Voxtral, Canary and Parakeet,
- consent rejection,
- global profile creation and profile reuse,
- adapter isolation and heavy-model failure cases.

Do not execute several large GPU models concurrently on an 8 GB GPU.

## 13. TUI

From project A:

```powershell
Push-Location $ProjectA
& $Tako
```

Verify:

- simple task screens are reachable,
- text and Windows paths edit correctly,
- Enter runs the visible action,
- model state refreshes after pulls,
- Speak and Transcribe save to the active session,
- Clone requires name, sample and consent,
- `/sessions`, `/new` and `/clone` work,
- previous sessions can be resumed,
- progress/errors remain visible,
- no terminal teardown, flashing or hidden double-Enter flow,
- Ctrl+C does not orphan installation work,
- small terminal sizes do not panic.

Inspect `.tako` after exit.

## 14. GUI

```powershell
Push-Location $ProjectA
& $Tako gui
```

Verify:

- URL carries workspace/session context,
- routing and refresh retain that context,
- Models and Runners agree with CLI `plan`,
- pulls update the same installed records,
- Speak and Transcribe write to active history,
- Voices lists custom profiles and enforces consent,
- History searches titles, prompts, models, transcripts and errors,
- old sessions can be resumed,
- WAV files play and transcripts reopen,
- inactive sessions can be deleted,
- output requests cannot read outside session outputs,
- responsive layout works below 900 px.

## 15. Cross-interface parity

For one TTS and one STT model:

1. Pull in CLI; confirm ready in TUI/GUI.
2. Generate in TUI; confirm in GUI History and CLI session JSON.
3. Transcribe in GUI; confirm in TUI Sessions and `sessions show`.
4. Remove with CLI; confirm both UIs become non-executable.
5. Re-pull in GUI; confirm CLI `plan` becomes executable.

No surface may own a second model database, output root or session implementation.

## 16. Negative and security matrix

Test:

- unknown model,
- unsupported capability,
- missing/empty input,
- invalid/deleted session,
- `..`, `/`, `\` and encoded separator output names,
- absolute output paths,
- symlink or Windows junction escape,
- cross-session and cross-workspace output access,
- paths with spaces and Unicode,
- read-only workspace,
- unavailable network,
- insufficient disk,
- interrupted runner/adapter install,
- GPU out of memory or missing CUDA,
- cloning without consent,
- corrupt/missing voice profile,
- simultaneous pulls of one model,
- simultaneous daemon starts.

Any arbitrary file read/write, output escape, consent bypass or corrupted global store is a release-blocking failure.

## 17. Persistence and upgrade simulation

1. Create sessions, outputs, voices and installed records.
2. Build a binary from a later commit.
3. Run it against the same isolated home and projects.
4. Confirm all state remains readable.
5. Confirm no active temporary/backup metadata files remain.

## 18. Packaging smoke gate

In clean VMs, test available release artifacts:

- Windows MSI and portable ZIP,
- macOS Intel/Apple Silicon package,
- Linux `.deb`, `.rpm`, AppImage or install script.

Verify PATH aliases, upgrade preservation, uninstall safety, checksums and version metadata. The core installer must not bundle large models.

## 19. Evidence and sign-off

For each promoted model record:

| Field | Value |
|---|---|
| Model ID | |
| Takokit commit | |
| OS / architecture | |
| CPU / RAM | |
| GPU / VRAM / driver | |
| Pull result and duration | |
| Runner/adapter result | |
| Inference command | |
| Output path and bytes | |
| Quality notes | |
| Retry/idempotency | |
| Session/history | |
| Status | Pass / Fail / Blocked |
| Tester/date | |

Website work may begin only when:

- permanent CI is green on Linux and Windows,
- the Codex review has no unresolved P0/P1 issues,
- workspace/session and interface-parity tests pass,
- Whisper Tiny, another STT, Kokoro, a managed TTS and a cloning model pass real-device testing,
- all other models have truthful tiers,
- known limitations and packaging state are documented.

A failed heavy model can remain catalogued only after being downgraded to experimental or planned with the exact blocker.
