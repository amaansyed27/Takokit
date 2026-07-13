# Takokit comprehensive stability and release test guide

This guide is the mandatory stability gate before building the public website or publishing `v0.1.0`. It tests the shared Rust backend, CLI, TUI, GUI, daemon, project sessions, model installation, real inference, voice profiles, failure recovery and packaging assumptions.

Do not mark a model verified merely because CI compiles its adapter. Record real-device evidence for every promoted model.

## 1. Test environment

Primary Windows test machine:

- Windows 11 x64
- PowerShell 7 recommended
- NVIDIA RTX 5060 Laptop GPU with 8 GB VRAM
- Python 3.12 may exist globally, but Takokit must not depend on it
- Rust stable
- Node.js LTS and npm
- At least 80 GB free disk space for the complete heavy-model pass

Create an evidence folder outside the repository:

```powershell
$Evidence = "$HOME\takokit-test-evidence"
New-Item -ItemType Directory -Force $Evidence | Out-Null
```

Record system information:

```powershell
Get-ComputerInfo | Out-File "$Evidence\computer-info.txt"
nvidia-smi | Out-File "$Evidence\nvidia-smi.txt"
rustc --version | Out-File "$Evidence\toolchain.txt"
cargo --version | Add-Content "$Evidence\toolchain.txt"
node --version | Add-Content "$Evidence\toolchain.txt"
npm --version | Add-Content "$Evidence\toolchain.txt"
git rev-parse HEAD | Out-File "$Evidence\commit.txt"
```

## 2. Stop running processes before rebuilding

A running Windows daemon can lock `target\release\takokit.exe`.

```powershell
if (Test-Path .\target\release\takokit.exe) {
    .\target\release\takokit.exe daemon stop
}
Get-Process takokit,tako -ErrorAction SilentlyContinue | Stop-Process -Force
```

Confirm ports are free:

```powershell
Get-NetTCPConnection -LocalPort 5050 -ErrorAction SilentlyContinue
```

Expected: no remaining Takokit listener.

## 3. Clean automated gates

Run from the repository root:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python .\scripts\check_file_sizes.py
```

GUI:

```powershell
Push-Location .\apps\gui
npm ci
npm run build
Pop-Location
```

Release build:

```powershell
cargo build --release
```

Required binaries:

```powershell
Get-Item .\target\release\takokit.exe
Get-Item .\target\release\tako.exe
```

Save logs:

```powershell
cargo check --workspace *>&1 | Tee-Object "$Evidence\cargo-check.txt"
cargo test --workspace *>&1 | Tee-Object "$Evidence\cargo-test.txt"
```

Pass criteria:

- formatting succeeds,
- all Rust crates compile,
- all Rust tests pass,
- GUI TypeScript and Vite build pass,
- no tracked source file exceeds the repository line limit,
- both executable aliases exist.

## 4. Isolated global Takokit home

Do not use the normal model store during the clean-room pass.

```powershell
$env:TAKOKIT_HOME = "$env:TEMP\takokit-release-test"
Remove-Item -Recurse -Force $env:TAKOKIT_HOME -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $env:TAKOKIT_HOME | Out-Null
$Tako = (Resolve-Path .\target\release\tako.exe).Path
$Takokit = (Resolve-Path .\target\release\takokit.exe).Path
```

The two aliases must expose the same version and storage location:

```powershell
& $Tako version
& $Takokit version
```

Pass criteria: versions and global storage roots match.

## 5. Baseline CLI and registry

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

- the registry parses without panic,
- all 27 model IDs are visible,
- every model names a known runner,
- planned models remain non-executable,
- executable-path models are not ready before their dependencies are installed,
- diagnostics provide actionable next commands.

## 6. Daemon ownership and recovery

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
- only one managed daemon owns port 5050,
- restart changes the daemon instance but preserves storage,
- stop terminates the owned daemon,
- stale PID or lock files are recovered safely,
- log paths exist and contain useful errors.

Also test abnormal termination:

```powershell
& $Tako daemon start
Get-Process takokit | Stop-Process -Force
& $Tako daemon start
& $Tako daemon status
```

Expected: Takokit detects stale ownership and starts cleanly.

## 7. Project-local `.tako` workspace

Create two independent projects:

```powershell
$ProjectA = "$env:TEMP\takokit-project-a"
$ProjectB = "$env:TEMP\takokit-project-b"
Remove-Item -Recurse -Force $ProjectA,$ProjectB -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $ProjectA,$ProjectB | Out-Null
```

In project A:

```powershell
Push-Location $ProjectA
& $Tako sessions new --title "Project A baseline" | Tee-Object "$Evidence\session-a-new.json"
& $Tako sessions list | Tee-Object "$Evidence\session-a-list.json"
Get-ChildItem -Recurse -Force .\.tako | Out-File "$Evidence\session-a-tree.txt"
Pop-Location
```

In project B:

```powershell
Push-Location $ProjectB
& $Tako sessions new --title "Project B baseline"
& $Tako sessions list | Tee-Object "$Evidence\session-b-list.json"
Pop-Location
```

Pass criteria:

- each launch directory receives its own `.tako`,
- sessions from A do not appear in B,
- model and runner files remain in `$env:TAKOKIT_HOME`,
- no generated user output silently falls back to `~/.takokit/outputs`.

## 8. Session lifecycle and search

Run in project A:

```powershell
Push-Location $ProjectA
$NewSession = & $Tako sessions new --title "Searchable interview" | ConvertFrom-Json
$SessionId = $NewSession.summary.id
& $Tako sessions show $SessionId
& $Tako sessions open $SessionId
& $Tako sessions list --query "interview"
& $Tako sessions list --query "does-not-exist"
Pop-Location
```

Later, after inference, search for prompt text, transcript text, model ID and failure message.

Test deletion with a disposable inactive session:

```powershell
Push-Location $ProjectA
$Disposable = & $Tako sessions new --title "Delete me" | ConvertFrom-Json
& $Tako sessions new --title "Keep me" | Out-Null
& $Tako sessions rm $Disposable.summary.id
& $Tako sessions list --query "Delete me"
Pop-Location
```

Pass criteria:

- open resumes the exact session,
- search includes event content,
- deleting an inactive session removes its directory,
- active-session state never points to a deleted session.

## 9. Mock output and session recording

```powershell
Push-Location $ProjectA
& $Tako speak "Mock session persistence test" --model mock-tts | Tee-Object "$Evidence\mock-speech.json"
& $Tako sessions list --query "Mock session persistence test"
Get-ChildItem -Recurse -Force .\.tako\sessions | Out-File "$Evidence\mock-session-tree.txt"
Pop-Location
```

Pass criteria:

- a non-empty WAV is written under the active session's `outputs` directory,
- `events.jsonl` contains a completed TTS event,
- `session.json` counts the event and output,
- the output path never escapes the project `.tako` tree.

## 10. Whisper Tiny verified STT path

Create sample audio when needed:

```powershell
& $Tako samples create
```

Prepare Whisper:

```powershell
& $Tako pull whisper-tiny | Tee-Object "$Evidence\pull-whisper-tiny.json"
& $Tako plan whisper-tiny --json | Tee-Object "$Evidence\plan-whisper-tiny.json"
```

Run transcription from project A using a real WAV:

```powershell
$Audio = "C:\path\to\test01_20s.wav"
Push-Location $ProjectA
& $Tako transcribe $Audio --model whisper-tiny | Tee-Object "$Evidence\whisper-tiny-transcript.json"
& $Tako sessions list --query "whisper-tiny"
Pop-Location
```

Validate:

- transcript is intelligible,
- transcript text file exists inside the active session,
- source audio was not copied or modified unexpectedly,
- elapsed time is printed,
- a repeated run creates a new output without overwriting the first.

## 11. Pull reliability and idempotency

For Whisper Tiny and Kokoro:

```powershell
& $Tako pull whisper-tiny
& $Tako pull whisper-tiny
& $Tako pull kokoro
& $Tako pull kokoro
```

Pass criteria:

- second pull reuses verified artifacts,
- ready installations are not downgraded,
- checksums are revalidated,
- no duplicate blob is created for the same digest.

Interrupted download test:

1. Start a large pull.
2. Terminate Takokit during download.
3. Run the same pull again.
4. Confirm recovery or clean restart without treating a partial file as ready.

Checksum rollback test should use a test fixture or isolated copied cache, never corrupt the normal model store.

## 12. TTS model smoke tests

Run each only after `plan` says executable. Record install duration, inference duration, output size and audible quality.

### Kokoro

```powershell
& $Tako pull kokoro
Push-Location $ProjectA
& $Tako speak "Kokoro stability test from Takokit." --model kokoro | Tee-Object "$Evidence\kokoro.json"
Pop-Location
```

### Qwen3-TTS

```powershell
& $Tako pull qwen3-tts
Push-Location $ProjectA
& $Tako speak "Qwen three text to speech stability test." --model qwen3-tts --voice Ryan | Tee-Object "$Evidence\qwen3-tts.json"
Pop-Location
```

### Chatterbox and F5-TTS

```powershell
& $Tako pull chatterbox
& $Tako pull f5-tts
Push-Location $ProjectA
& $Tako speak "Chatterbox base voice test." --model chatterbox
& $Tako speak "F five base voice test." --model f5-tts
Pop-Location
```

### Dia

```powershell
& $Tako pull dia
Push-Location $ProjectA
& $Tako speak "[S1] Takokit is ready. [S2] The dialogue model is responding." --model dia
Pop-Location
```

### Bark, MMS and Coqui families

```powershell
& $Tako pull bark-small
& $Tako pull mms-tts-eng
& $Tako pull xtts-v2
& $Tako pull yourtts
```

Run one short sentence per model. For cloning-capable models, complete the voice-profile test first.

### Kyutai DSM TTS

```powershell
& $Tako pull kyutai-tts-1.6b
& $Tako plan kyutai-tts-1.6b --json | Tee-Object "$Evidence\plan-kyutai.json"
Push-Location $ProjectA
& $Tako speak "Kyutai delayed streams modeling is running locally." --model kyutai-tts-1.6b | Tee-Object "$Evidence\kyutai-tts.json"
Pop-Location
```

Kyutai pass criteria:

- CUDA is detected,
- official model and voice assets resolve,
- output is a valid non-empty WAV,
- no arbitrary local reference WAV is accepted as a Kyutai voice embedding,
- out-of-memory failure is clear and does not mark the model broken globally.

## 13. Additional STT adapter tests

Use the same clean 20-second WAV for comparison:

```powershell
$SttModels = @(
  "whisper-base",
  "whisper-small",
  "distil-whisper-large-v3",
  "wav2vec2-base-960h",
  "sensevoice",
  "voxtral",
  "canary",
  "parakeet"
)
foreach ($Model in $SttModels) {
    & $Tako pull $Model
    & $Tako plan $Model --json | Out-File "$Evidence\plan-$Model.json"
    Push-Location $ProjectA
    & $Tako transcribe $Audio --model $Model *>&1 | Tee-Object "$Evidence\transcribe-$Model.txt"
    Pop-Location
}
```

Do not run Voxtral, Canary or Parakeet concurrently on an 8 GB GPU. Record out-of-memory behaviour separately from functional adapter errors.

Pass criteria per model:

- dependency environment installs without altering another adapter,
- model resolves through the expected official API,
- transcript is non-empty,
- session history identifies the correct model,
- failure leaves useful adapter logs.

## 14. Voice profile and cloning tests

Use only a voice you own or have explicit permission to use.

Consent rejection:

```powershell
& $Tako clone $Audio --name "No Consent" --model chatterbox
```

Expected: command fails and no profile is created.

Profile creation:

```powershell
& $Tako clone $Audio --name "Amaan Test Voice" --model chatterbox --consent | Tee-Object "$Evidence\clone-profile.json"
& $Tako list voices | Tee-Object "$Evidence\voices-after-clone.json"
```

Profile reuse:

```powershell
Push-Location $ProjectA
& $Tako speak "This sentence uses the stored profile." --model chatterbox --voice amaan-test-voice
& $Tako speak "This sentence tests profile portability." --model f5-tts --voice amaan-test-voice
Pop-Location
```

Pass criteria:

- profile source is copied into global local-only voice storage,
- the profile JSON records model and consent,
- the clone event appears in the active project session,
- compatible models resolve the profile ID,
- missing or deleted profile audio produces a typed error,
- duplicate profile IDs are rejected rather than overwritten.

## 15. TUI test

Start from project A:

```powershell
Push-Location $ProjectA
& $Tako
```

Manually verify:

- arrows switch simple task screens,
- Models Enter either prepares a model or opens its matching task,
- Speak accepts text and generates output without CLI syntax,
- Transcribe accepts a Windows path and returns visible transcript output,
- Clone requires name, sample and consent,
- `/sessions` opens history,
- `/new` creates and activates a session,
- previous sessions can be resumed,
- running tasks show progress and final output,
- the terminal does not flash, tear down or lose input focus,
- Ctrl+C does not detach a running installer unsafely.

After exit, inspect `.tako` and confirm TUI outputs used the active session.

## 16. GUI test

```powershell
Push-Location $ProjectA
& $Tako gui
```

Verify:

- browser URL carries workspace and session context,
- model and runner status matches CLI `plan`,
- pulling from GUI changes the same installed records as CLI,
- Speak and Transcribe use the shared API and active session,
- Voices lists custom profiles,
- creating a profile requires consent,
- History searches title, model, prompt, transcript and error text,
- opening an older session redirects new outputs to it,
- generated WAV files play in History,
- transcripts reopen correctly,
- inactive sessions can be deleted,
- path traversal attempts cannot retrieve files outside session outputs,
- refreshing the browser preserves workspace/session context.

Test at desktop width and below 900 px.

## 17. CLI/TUI/GUI parity

For one TTS model and one STT model:

1. Pull with CLI and verify ready in TUI/GUI.
2. Generate from TUI and verify in GUI History and CLI session output.
3. Transcribe from GUI and verify in TUI Sessions and `sessions show`.
4. Remove a model with CLI and verify both UIs become non-executable.
5. Re-pull from GUI and verify CLI `plan` becomes executable.

No surface may maintain a second model database, output directory or session implementation.

## 18. Isolation and dependency conflict test

Install at least three adapters with conflicting Python stacks:

```powershell
& $Tako pull qwen3-tts
& $Tako pull chatterbox
& $Tako pull sensevoice
```

Inspect:

```powershell
Get-ChildItem "$env:TAKOKIT_HOME\runners\python-managed\adapters" -Directory
```

Pass criteria:

- each adapter owns its own `venv`, manifest and install log,
- upgrading one adapter does not alter another environment,
- failure in one adapter does not mark the shared runner or other adapters failed.

## 19. Negative and safety tests

Test and record clear failures for:

- unknown model ID,
- model used for unsupported capability,
- missing audio file,
- empty speech input,
- invalid session UUID,
- deleted session,
- output filename containing `..`, `/` or `\`,
- workspace path with spaces and non-ASCII characters,
- read-only workspace,
- insufficient disk space,
- interrupted runner install,
- unavailable network,
- GPU out of memory,
- missing CUDA runtime,
- voice cloning without consent,
- voice profile path outside the profile store,
- two simultaneous pulls of the same model.

Takokit must never report success without a validated artifact or output.

## 20. Persistence and upgrade simulation

1. Complete several sessions and model installs.
2. Build a second release binary from the next commit.
3. Run it against the same isolated `TAKOKIT_HOME` and projects.
4. Confirm sessions, outputs, voices and installed records remain readable.
5. Confirm atomic replacement leaves no active `.tmp-*` or `.bak-*` files.

## 21. Packaging smoke gate

Before a public release, test every produced artifact in a clean VM:

- Windows MSI and portable ZIP,
- macOS package on Intel and Apple Silicon where available,
- Linux `.deb`, `.rpm` and AppImage/install script where available.

Verify:

- `tako` and `takokit` are available on PATH,
- uninstall does not delete `~/.takokit` or project `.tako` without explicit consent,
- upgrade preserves models and sessions,
- no large model is bundled into the core installer,
- checksums and version metadata match the release.

## 22. Evidence record

For every model tested, add a row to the final sign-off report:

| Field | Value |
|---|---|
| Model ID | |
| Takokit commit | |
| OS / architecture | |
| CPU / RAM | |
| GPU / VRAM / driver | |
| Pull result and duration | |
| Adapter/runner result | |
| Inference command | |
| Output path and bytes | |
| Transcript/audio quality notes | |
| Retry/idempotency result | |
| Session/history result | |
| Status | Pass / Fail / Blocked |
| Tester and date | |

## 23. Release decision

Website implementation and public release work may begin only when:

- permanent CI is green on Linux and Windows,
- the Codex review reports no unresolved P0 or P1 findings,
- core workspace/session/TUI/GUI parity tests pass,
- Whisper Tiny, one additional STT model, Kokoro, one managed TTS model and one cloning model pass real-device tests,
- every other catalog model is accurately labelled executable-path or planned,
- installer scope and known limitations are documented,
- no test relies on manually editing Takokit-managed environments.

A failed heavy model does not require deleting its catalog entry. It must be downgraded to experimental or planned with the exact blocker before release.
