# Takokit model and voice-profile smoke tests

This companion to [TESTING.md](TESTING.md) contains the heavy-model portion of the release gate. Run it only after the clean build, daemon, workspace, session and pull-reliability sections of the main guide pass.

Use the variables created by the main guide:

```powershell
$Tako
$Evidence
$ProjectA
$Audio
```

For every test, save the install log, model plan, command output, generated file size, elapsed time, hardware state and audible/transcript quality notes. A compiled adapter without a real inference result remains **executable path**, not verified.

## 1. Common model procedure

For each model:

```powershell
& $Tako pull <model>
& $Tako plan <model> --json | Tee-Object "$Evidence\plan-<model>.json"
```

Confirm:

- the required runner and adapter are installed automatically,
- no global Python packages are modified,
- the model is executable only after all required components are ready,
- a repeat pull is idempotent,
- the adapter has its own `venv` and `install.log`,
- failure does not mark unrelated adapters failed.

## 2. Kokoro

```powershell
& $Tako pull kokoro
Push-Location $ProjectA
& $Tako speak "Kokoro stability test from Takokit." --model kokoro |
  Tee-Object "$Evidence\kokoro.json"
Pop-Location
```

Pass criteria:

- output is a valid non-empty WAV,
- the event and output appear in the active `.tako` session,
- repeat generation creates a new file,
- CPU execution remains usable when GPU acceleration is unavailable.

## 3. Qwen3-TTS

```powershell
& $Tako pull qwen3-tts
Push-Location $ProjectA
& $Tako speak "Qwen three text to speech stability test." `
  --model qwen3-tts --voice Ryan |
  Tee-Object "$Evidence\qwen3-tts.json"
Pop-Location
```

Verify the isolated `qwen3_tts` environment, official package import, selected voice and output validation.

## 4. Chatterbox and F5-TTS

```powershell
& $Tako pull chatterbox
& $Tako pull f5-tts
Push-Location $ProjectA
& $Tako speak "Chatterbox base voice test." --model chatterbox |
  Tee-Object "$Evidence\chatterbox-base.txt"
& $Tako speak "F five base voice test." --model f5-tts |
  Tee-Object "$Evidence\f5-base.txt"
Pop-Location
```

Verify that base synthesis works before testing stored voice profiles.

## 5. Dia

```powershell
& $Tako pull dia
Push-Location $ProjectA
& $Tako speak "[S1] Takokit is ready. [S2] The dialogue model is responding." `
  --model dia | Tee-Object "$Evidence\dia.txt"
Pop-Location
```

Confirm both speaker markers are respected and a valid WAV is produced.

## 6. Bark, MMS and Coqui families

```powershell
$Models = @("bark-small", "mms-tts-eng", "xtts-v2", "yourtts")
foreach ($Model in $Models) {
    & $Tako pull $Model
    & $Tako plan $Model --json | Out-File "$Evidence\plan-$Model.json"
}
```

Run one short sentence per model. For XTTS v2 and YourTTS, complete the profile-creation test before profile-conditioned synthesis.

Check model-specific license warnings, especially MMS and Coqui-family assets.

## 7. Kyutai DSM TTS

```powershell
& $Tako pull kyutai-tts-1.6b
& $Tako plan kyutai-tts-1.6b --json |
  Tee-Object "$Evidence\plan-kyutai.json"
Push-Location $ProjectA
& $Tako speak "Kyutai delayed streams modeling is running locally." `
  --model kyutai-tts-1.6b |
  Tee-Object "$Evidence\kyutai-tts.json"
Pop-Location
```

Pass criteria:

- CUDA is detected,
- the official Moshi DSM checkpoint and voice assets resolve,
- output is a valid non-empty WAV,
- the voice is an official precomputed embedding,
- arbitrary local reference WAVs are not accepted as Kyutai cloning,
- out-of-memory errors are actionable and do not corrupt readiness records.

The first Takokit Kyutai adapter is batch-to-WAV even though the upstream model supports streaming. Do not advertise Takokit streaming until a streaming API test exists.

## 8. Whisper family

Whisper Tiny is the baseline locally verified STT path.

```powershell
$WhisperModels = @("whisper-tiny", "whisper-base", "whisper-small")
foreach ($Model in $WhisperModels) {
    & $Tako pull $Model
    & $Tako plan $Model --json | Out-File "$Evidence\plan-$Model.json"
    Push-Location $ProjectA
    & $Tako transcribe $Audio --model $Model *>&1 |
      Tee-Object "$Evidence\transcribe-$Model.txt"
    Pop-Location
}
```

Verify transcript quality, elapsed time, output persistence and idempotent runner installation.

## 9. Transformers and specialist STT

```powershell
$SttModels = @(
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
    & $Tako transcribe $Audio --model $Model *>&1 |
      Tee-Object "$Evidence\transcribe-$Model.txt"
    Pop-Location
}
```

Do not run Voxtral, Canary and Parakeet concurrently on an 8 GB GPU.

Pass criteria per model:

- dependency environment installs without mutating other adapters,
- the official upstream API loads,
- transcript is non-empty,
- session history identifies the correct model,
- model-specific errors and install logs are useful,
- GPU out-of-memory is distinguished from an adapter defect.

## 10. Consent rejection

Use only a voice you own or have permission to use.

```powershell
& $Tako clone $Audio --name "No Consent" --model chatterbox
```

Expected:

- command fails,
- no global voice profile is created,
- the failure is recorded in the active project session,
- API, TUI and GUI enforce the same consent boundary.

## 11. Voice profile creation

```powershell
& $Tako clone $Audio --name "Amaan Test Voice" `
  --model chatterbox --consent |
  Tee-Object "$Evidence\clone-profile.json"
& $Tako list voices | Tee-Object "$Evidence\voices-after-clone.json"
```

Verify:

- the reference file is copied under `~/.takokit/voices/amaan-test-voice/`,
- `profile.json` contains the model and consent metadata,
- duplicate profile IDs are rejected rather than overwritten,
- the active `.tako` session records the profile event,
- deleting or corrupting the copied sample produces a typed failure.

## 12. Profile reuse

```powershell
Push-Location $ProjectA
& $Tako speak "This sentence uses the stored profile." `
  --model chatterbox --voice amaan-test-voice |
  Tee-Object "$Evidence\chatterbox-profile.txt"
& $Tako speak "This sentence tests profile portability." `
  --model f5-tts --voice amaan-test-voice |
  Tee-Object "$Evidence\f5-profile.txt"
Pop-Location
```

Repeat with XTTS v2 and YourTTS when their base paths pass.

A model without the cloning capability must not treat a local file path as a voice profile.

## 13. Adapter isolation

```powershell
& $Tako pull qwen3-tts
& $Tako pull chatterbox
& $Tako pull sensevoice
Get-ChildItem "$env:TAKOKIT_HOME\runners\python-managed\adapters" -Directory
```

Pass criteria:

- every adapter owns its own `venv`, manifest and install log,
- installing or repairing one environment leaves the others unchanged,
- shared caches do not replace environment isolation,
- one failed adapter does not mark the shared runner or other models failed.

## 14. Failure matrix

For at least one lightweight and one heavy model, test:

- unavailable network,
- interrupted dependency install,
- interrupted model download,
- insufficient disk space,
- missing CUDA runtime,
- GPU out of memory,
- invalid voice/profile ID,
- missing input file,
- empty TTS input,
- unsupported capability,
- repeated concurrent pull.

Takokit must not emit a completed event or mark a model executable after a failed operation.

## 15. Evidence table

Create one row per tested model:

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
| Transcript/audio quality | |
| Retry/idempotency result | |
| Session/history result | |
| Status | Pass / Fail / Blocked |
| Tester and date | |

Promote a model to **locally verified** only after this record is complete and the output has been manually inspected.
