# Takokit comprehensive stability and release test guide

This is the mandatory gate before website work, packaging or a public release. It covers the Rust workspace, GUI, daemon lifecycle, global model storage, project-local `.tako` sessions, all 31 model manifests, runner installation, security and hardware smoke evidence.

Run [MODEL_SMOKE_TESTS.md](MODEL_SMOKE_TESTS.md) after the core gate. A compiled adapter or parsed manifest is not a model pass.

## 1. Primary Windows environment

Target machine:

- Windows 11 x64
- PowerShell 7 recommended
- RTX 5060 Laptop GPU, 8 GB VRAM
- Rust stable
- Node.js LTS and npm
- at least 100 GB free for broad model testing

Create an evidence directory:

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

## 2. Stop stale processes

```powershell
if (Test-Path .\target\release\tako.exe) {
  .\target\release\tako.exe daemon stop
}
Get-Process takokit,tako -ErrorAction SilentlyContinue | Stop-Process -Force
Get-NetTCPConnection -LocalPort 5050 -ErrorAction SilentlyContinue
```

Expected: no Takokit listener remains on port 5050.

## 3. Automated repository gates

From the repository root:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python .\scripts\audit_file_sizes.py

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

- formatting, workspace check and workspace tests pass,
- GUI TypeScript/Vite build passes,
- file-size audit passes,
- both executable aliases exist,
- no production path emits an unreachable-code or dead-state warning,
- the catalog invariant test reports exactly 31 model manifests.

## 4. Isolated runtime home

```powershell
$env:TAKOKIT_HOME = "$env:TEMP\takokit-release-test"
Remove-Item -Recurse -Force $env:TAKOKIT_HOME -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $env:TAKOKIT_HOME | Out-Null
$Tako = (Resolve-Path .\target\release\tako.exe).Path
$Takokit = (Resolve-Path .\target\release\takokit.exe).Path
& $Tako version
& $Takokit version
```

Expected: both aliases report the same version and storage root.

## 5. Registry and launch-catalog diagnostics

```powershell
& $Tako doctor --json | Tee-Object "$Evidence\doctor-clean.json"
& $Tako status
& $Tako capabilities
& $Tako list
& $Tako models | Tee-Object "$Evidence\models-clean.json"
& $Tako runners | Tee-Object "$Evidence\runners-clean.json"
& $Tako library models | Tee-Object "$Evidence\library-models.json"
& $Tako library runners | Tee-Object "$Evidence\library-runners.json"
& $Tako test --suite launch --json |
  Tee-Object "$Evidence\launch-catalog.json"
```

Verify:

- all 31 model IDs parse,
- every model names an existing runner,
- each model reports a lifecycle, artifact and runner-runtime state,
- executable-path models are not ready before installation,
- missing state has an actionable next command,
- model output agrees with [model-support.md](model-support.md),
- Qwen3-Omni is represented but not falsely marked verified on the 8 GB machine.

## 6. Daemon ownership and recovery

```powershell
& $Tako daemon start
$Daemon = & $Tako daemon status | ConvertFrom-Json
$Daemon
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
- a daemon started through `tako.exe` records `takokit.exe` as its canonical executable,
- restart preserves storage,
- stop terminates the owned daemon,
- logs and identity records are useful.

Crash recovery must terminate the exact managed PID from the active `TAKOKIT_HOME`; do not rely on a process name:

```powershell
& $Tako daemon start | Out-Null
$Before = & $Tako daemon status | ConvertFrom-Json

if ([IO.Path]::GetFileName($Before.executable) -ne "takokit.exe") {
  throw "Managed daemon used the wrong executable: $($Before.executable)"
}

Stop-Process -Id $Before.pid -Force
Start-Sleep -Milliseconds 500

if (Get-Process -Id $Before.pid -ErrorAction SilentlyContinue) {
  throw "Managed daemon PID $($Before.pid) did not terminate"
}
if (Get-NetTCPConnection -LocalPort 5050 -State Listen -ErrorAction SilentlyContinue) {
  throw "Port 5050 is still occupied after the simulated crash"
}

$After = & $Tako daemon start | ConvertFrom-Json
& $Tako daemon status

if ($After.pid -eq $Before.pid) {
  throw "Crash recovery reused the stale PID"
}
if ($After.instance_id -eq $Before.instance_id) {
  throw "Crash recovery reused the stale instance identity"
}
if ([IO.Path]::GetFileName($After.executable) -ne "takokit.exe") {
  throw "Recovered daemon used the wrong executable: $($After.executable)"
}
```

Expected: stale PID, identity and lock ownership are recovered safely; the replacement daemon has a new PID and instance ID and runs through the canonical `takokit.exe` binary.

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

- each project owns its own `.tako`,
- sessions from A never appear in B,
- global models and runners remain under `$env:TAKOKIT_HOME`,
- generated outputs never fall back to a global output folder.

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

## 9. Output persistence

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

- non-empty WAV exists in the active session `outputs/`,
- a completed event exists in `events.jsonl`,
- summary counts are accurate,
- no output path escapes the session.

## 10. Pull idempotency and recovery

```powershell
& $Tako pull whisper-tiny
& $Tako pull whisper-tiny
& $Tako pull kokoro
& $Tako pull kokoro
& $Tako list
```

Pass criteria:

- verified artifacts and snapshots are reused,
- ready state is not downgraded,
- digests or pinned source markers are rechecked,
- duplicate content-addressed blobs are not created,
- `tako list` contains only fully installed, currently verified models; catalog-only metadata is excluded.

Interrupt one large pull and restart it. A partial snapshot must never be treated as ready.

## 11. Fast real-inference baseline

```powershell
& $Tako pull kokoro
& $Tako pull whisper-tiny
& $Tako pull whisper-base
& $Tako pull qwen3-tts
& $Tako test --suite fast --run --json |
  Tee-Object "$Evidence\fast-suite.json"
```

The suite must synthesize actual speech and use that speech for transcription. Silence is not a valid STT pass.

## 12. Complete model and voice-runtime pass

Follow [MODEL_SMOKE_TESTS.md](MODEL_SMOKE_TESTS.md). Prepare the input folder at
`$HOME\Downloads\takokit-smoke-inputs`, then perform Section 12 in two stages from the
repository root. Both commands must use the same isolated storage directory.

### 12A. Pull and verify every model first

```powershell
$SmokeStorage = "$env:TEMP\takokit-all-model-smoke"

.\scripts\pull_takokit_all_models.ps1 `
  -StorageRoot $SmokeStorage
```

The prefetch helper pulls models sequentially, verifies each model with `tako plan`, saves
per-model logs under `$HOME\takokit-test-evidence`, and continues after an individual
failure so the final report shows every model. It is safe to rerun: already verified
artifacts are reused and failed or interrupted pulls are retried.

On the primary RTX 5060 Laptop GPU machine, Qwen3-Omni is recorded as
`blocked-hardware` and is not downloaded. Only on a workstation with suitable memory,
run the prefetch with:

```powershell
.\scripts\pull_takokit_all_models.ps1 `
  -StorageRoot $SmokeStorage `
  -IncludeWorkstation
```

Do not delete or change `$SmokeStorage` after the pull completes.

### 12B. Run all smoke tests from the prefetched cache

```powershell
.\scripts\run_takokit_all_smokes.ps1 `
  -StorageRoot $SmokeStorage `
  -SkipPull
```

If the workstation-only model was prefetched, include it in the smoke pass as well:

```powershell
.\scripts\run_takokit_all_smokes.ps1 `
  -StorageRoot $SmokeStorage `
  -SkipPull `
  -IncludeWorkstation
```

The smoke runner covers all 31 model IDs and displays the current model, phase, step
count, elapsed time, latest child-install activity and exact log path. Results and
progress are saved incrementally, including child adapter/download logs and free-space
snapshots.

It records separate `passed`, `failed`, `skipped-dependency`, `blocked-input` and
`blocked-hardware` states. A model that failed during prefetch should fail its cached
plan and skip dependent inference instead of producing misleading cascade failures.
RVC is expected to be `blocked-input` unless a consent-backed checkpoint is supplied.
Do not run multiple large GPU models concurrently.

After preserving the evidence, preview and remove the isolated smoke storage with:

```powershell
.\scripts\clear_takokit_smoke_storage.ps1 `
  -StorageRoot $SmokeStorage

.\scripts\clear_takokit_smoke_storage.ps1 `
  -StorageRoot $SmokeStorage `
  -Force
```

## 13. TUI

```powershell
Push-Location $ProjectA
& $Tako
```

Verify:

- Home exposes Speak, Transcribe, Clone voice, Manage, Sessions and Activity,
- Manage exposes installed Models, Runners and System without a catalog browser,
- number keys open the visible Home and Manage actions,
- Windows and Unicode paths edit correctly,
- `Ctrl+Enter` submits task forms and `Esc` consistently returns to the previous screen,
- model state refreshes after pulls,
- actions save to the active session,
- clone requires name, sample and consent,
- failed tasks automatically open Activity with the actual error,
- progress and errors remain visible,
- Ctrl+C does not orphan installation work,
- small terminals do not panic.

## 14. GUI

```powershell
Push-Location $ProjectA
& $Tako gui
```

Verify:

- URL carries workspace and session context,
- refresh retains that context,
- Models and Runners agree with CLI planning,
- pulls update the same installed records,
- Speak and Transcribe write to active History,
- Voices enforces consent,
- History searches titles, prompts, models, transcripts and errors,
- old sessions can be resumed,
- WAV files play and transcripts reopen,
- inactive sessions can be deleted,
- output routes cannot read outside session outputs,
- layout works below 900 px.

## 15. Cross-interface parity

For one TTS and one STT model:

1. Pull in CLI and confirm ready in TUI/GUI.
2. Generate in TUI and confirm in GUI History and CLI session JSON.
3. Transcribe in GUI and confirm in TUI Sessions and `sessions show`.
4. Remove with CLI and confirm both UIs become non-executable.
5. Re-pull in GUI and confirm CLI planning becomes executable.

No surface may own a second model database, output root or session implementation.

## 16. Negative and security matrix

Test:

- unknown model and unsupported capability,
- missing or empty input,
- invalid or deleted session,
- `..`, `/`, `\` and encoded separators in output names,
- absolute output paths,
- symlink or Windows junction escape,
- cross-session and cross-workspace output access,
- paths with spaces and Unicode,
- read-only workspace,
- unavailable network and insufficient disk,
- interrupted runner or adapter installation,
- missing CUDA and GPU out-of-memory,
- cloning, conversion or training without consent,
- corrupt voice profile,
- malformed GPT-SoVITS dataset,
- missing RVC target checkpoint,
- simultaneous pulls and daemon starts.

Any arbitrary file access, output escape, consent bypass or corrupted global store is release-blocking.

## 17. Persistence and upgrade simulation

1. Create sessions, outputs, voices and installed records.
2. Build a binary from a later commit.
3. Run it against the same isolated home and projects.
4. Confirm all state remains readable.
5. Confirm no temporary or backup metadata remains active.

## 18. Packaging gate

In clean VMs, test each available artifact:

- Windows MSI and portable ZIP,
- macOS Intel and Apple Silicon package,
- Linux `.deb`, `.rpm`, AppImage or install script.

Verify PATH aliases, upgrade preservation, uninstall safety, checksums and version metadata. Installers must not bundle large models.

## 19. Sign-off

Before website work or release:

- automated gates are green on Linux and Windows,
- all 31 models have a recorded status,
- every claimed verified model has real-device evidence,
- hardware-blocked models are labelled honestly,
- all P0/P1 review findings are resolved,
- no planned capability is represented by fake output or a success placeholder.
