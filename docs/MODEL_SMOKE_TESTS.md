# Takokit model and voice-runtime smoke tests

This guide is the hardware portion of the Takokit release gate. Run it only after the automated build, daemon, workspace, session and pull-reliability sections of [TESTING.md](TESTING.md) pass.

A model is not verified because its manifest parses or its adapter imports. Verification requires a real pull, real execution, a non-empty output or transcript, manual quality inspection, retry evidence and recorded hardware details.

## 1. Required inputs

Use voice and training material that you own or have explicit permission to use.

```powershell
$Audio = "C:\path\to\test01_20s.wav"
$ReferenceAudio = "C:\path\to\owned-reference.wav"
$ReferenceText = "The exact words spoken in the reference audio."
$TrainingSamples = "C:\path\to\gpt-sovits-dataset"
$RvcTarget = "C:\path\to\owned-rvc-checkpoint-or-directory"
```

GPT-SoVITS training input must contain:

```text
<dataset>/
├── train.list
└── wavs/
```

Each `train.list` row uses:

```text
wav_path|speaker|language|transcript
```

RVC conversion needs a user-owned or consent-backed `.pth` checkpoint and may use a matching `.index` file from the same directory.

## 2. Catalog integrity pass

After building the release binary:

```powershell
$Tako = (Resolve-Path .\target\release\tako.exe).Path
& $Tako test --suite launch --json |
  Tee-Object "$Evidence\launch-catalog.json"
```

Pass criteria:

- all **31** bundled model IDs appear,
- every model resolves to a known runner,
- each model reports its artifact, runner-runtime and executable state,
- no model disappears because it was omitted from a hard-coded launch list,
- missing requirements include a usable next command.

Category filters:

```powershell
& $Tako test --suite launch --category tts
& $Tako test --suite launch --category stt
& $Tako test --suite launch --category clone
& $Tako test --suite launch --category convert
& $Tako test --suite launch --category train
```

## 3. Automated catalog-wide hardware runner

The repository includes a PowerShell runner that pulls, plans and exercises all models with suitable inputs while preserving one log per phase.

```powershell
.\scripts\run_all_model_smokes.ps1 `
  -Audio $Audio `
  -ReferenceAudio $ReferenceAudio `
  -ReferenceText $ReferenceText `
  -TrainingSamples $TrainingSamples `
  -RvcTarget $RvcTarget
```

Useful narrower passes:

```powershell
.\scripts\run_all_model_smokes.ps1 -Audio $Audio -Category stt
.\scripts\run_all_model_smokes.ps1 -Audio $Audio -ReferenceAudio $ReferenceAudio -Category tts
.\scripts\run_all_model_smokes.ps1 -Audio $Audio -PlanOnly
```

The script writes:

```text
~/takokit-test-evidence/all-models-<timestamp>/
├── commit.txt
├── nvidia-smi.txt
├── results.json
├── results.csv
├── summary.json
└── <model>-<phase>.log
```

Statuses are deliberately distinct:

- `passed` — command exited successfully and the runtime validated its result,
- `failed` — installation or execution failed,
- `blocked-input` — a required consent-backed checkpoint or dataset was not supplied,
- `blocked-hardware` — the current device cannot meet the declared model requirement.

Do not convert a blocked result into a pass.

## 4. Fast baseline before heavy models

```powershell
& $Tako pull kokoro
& $Tako pull whisper-tiny
& $Tako pull whisper-base
& $Tako pull qwen3-tts
& $Tako test --suite fast --run --json |
  Tee-Object "$Evidence\fast-suite.json"
```

Pass criteria:

- Kokoro and Qwen3-TTS create non-empty WAV files,
- the generated Kokoro sample is used for Whisper rather than silence,
- Whisper Tiny/Base produce non-empty transcripts,
- repeat execution creates unique output files,
- failures return a non-zero exit code.

## 5. Lightweight and general TTS

Models:

```text
piper-lessac
kokoro
mms-tts-eng
bark-small
dia
```

Example commands:

```powershell
& $Tako pull piper-lessac
& $Tako speak "Piper smoke test." --model piper-lessac

& $Tako pull kokoro
& $Tako speak "Kokoro smoke test." --model kokoro

& $Tako pull mms-tts-eng
& $Tako speak "MMS smoke test." --model mms-tts-eng

& $Tako pull bark-small
& $Tako speak "Bark smoke test." --model bark-small

& $Tako pull dia
& $Tako speak "[S1] Takokit is ready. [S2] The second speaker is responding." --model dia
```

Verify valid WAV structure, audible speech, expected speaker behavior for Dia, session persistence and idempotent repeat pulls.

## 6. Qwen3-TTS checkpoint matrix

### CustomVoice checkpoints

```powershell
& $Tako pull qwen3-tts
& $Tako speak "Legacy custom voice test." --model qwen3-tts --voice Ryan

& $Tako pull qwen3-tts-1.7b-custom
& $Tako speak "One point seven billion custom voice test." `
  --model qwen3-tts-1.7b-custom `
  --voice Ryan `
  --instruction "Clear, calm narration with a natural pace."
```

### Base cloning checkpoints

```powershell
& $Tako pull qwen3-tts-0.6b-base
& $Tako speak "Zero-shot cloning test." `
  --model qwen3-tts-0.6b-base `
  --voice $ReferenceAudio `
  --reference-text $ReferenceText

& $Tako pull qwen3-tts-1.7b-base
& $Tako speak "Larger zero-shot cloning test." `
  --model qwen3-tts-1.7b-base `
  --voice $ReferenceAudio `
  --reference-text $ReferenceText
```

### VoiceDesign checkpoint

```powershell
& $Tako pull qwen3-tts-1.7b-voice-design
& $Tako speak "A newly designed voice is speaking." `
  --model qwen3-tts-1.7b-voice-design `
  --instruction "Warm, mature, confident documentary narration."
```

Pass criteria:

- each ID loads its own pinned checkpoint directory,
- CustomVoice uses supported preset speakers,
- Base checkpoints reject missing reference audio,
- VoiceDesign rejects a missing instruction,
- 0.6B and 1.7B results are not accidentally served by the legacy model directory,
- all outputs are non-empty WAV files.

## 7. Reference-conditioned TTS and cloning

Models:

```text
chatterbox
f5-tts
xtts-v2
yourtts
cosyvoice2
fish-speech
openvoice
gpt-sovits
```

Base speech examples:

```powershell
& $Tako speak "Chatterbox reference test." --model chatterbox --voice $ReferenceAudio
& $Tako speak "F5 reference test." --model f5-tts --voice $ReferenceAudio
& $Tako speak "XTTS reference test." --model xtts-v2 --voice $ReferenceAudio
& $Tako speak "YourTTS reference test." --model yourtts --voice $ReferenceAudio
& $Tako speak "CosyVoice reference test." --model cosyvoice2 --voice $ReferenceAudio
& $Tako speak "Fish Speech reference test." --model fish-speech --voice $ReferenceAudio
& $Tako speak "OpenVoice reference test." --model openvoice --voice $ReferenceAudio
& $Tako speak "GPT-SoVITS reference test." `
  --model gpt-sovits `
  --voice $ReferenceAudio `
  --reference-text $ReferenceText
```

Profile creation must be tested separately:

```powershell
& $Tako clone $ReferenceAudio --name "Amaan Test Voice" --model chatterbox --consent
& $Tako list voices
```

Also test the rejection path without `--consent`. No profile or completed event may be created.

## 8. Whisper family

```powershell
$WhisperModels = @("whisper-tiny", "whisper-base", "whisper-small")
foreach ($Model in $WhisperModels) {
  & $Tako pull $Model
  & $Tako transcribe $Audio --model $Model *>&1 |
    Tee-Object "$Evidence\transcribe-$Model.txt"
}
```

Verify non-empty intelligible transcripts, output persistence, elapsed time, repeated pull reuse and distinct model IDs in session history.

## 9. Specialist STT

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
  & $Tako transcribe $Audio --model $Model *>&1 |
    Tee-Object "$Evidence\transcribe-$Model.txt"
}
```

Do not run Voxtral, Canary and Parakeet concurrently on an 8 GB GPU. Distinguish GPU out-of-memory from adapter defects.

## 10. Qwen Omni

```powershell
& $Tako pull qwen2-5-omni
& $Tako transcribe $Audio --model qwen2-5-omni
& $Tako speak "Qwen two point five Omni speech test." --model qwen2-5-omni
```

`qwen3-omni` declares workstation-class requirements of roughly 64 GB system RAM and 40 GB VRAM. On the RTX 5060 Laptop GPU, record it as `blocked-hardware`; do not attempt to force a misleading pass.

Only run it on suitable hardware:

```powershell
.\scripts\run_all_model_smokes.ps1 `
  -Audio $Audio `
  -Category omni `
  -IncludeWorkstation
```

## 11. OpenVoice conversion

```powershell
& $Tako pull openvoice
& $Tako convert $Audio `
  --target-voice $ReferenceAudio `
  --model openvoice `
  --consent
```

Verify source and target embeddings are created from the supplied files, the converter writes a non-empty WAV, and missing consent or target audio fails cleanly.

## 12. RVC conversion

```powershell
& $Tako pull rvc
& $Tako convert $Audio `
  --target-voice $RvcTarget `
  --model rvc `
  --consent
```

Verify HuBERT and RMVPE assets were pulled, the supplied `.pth` checkpoint is loaded, an optional `.index` is discovered, and no target checkpoint is downloaded or invented by Takokit.

RVC training is not currently advertised. Do not mark RVC training as passed.

## 13. GPT-SoVITS training

```powershell
& $Tako pull gpt-sovits
& $Tako train $TrainingSamples `
  --name "amaan-smoke" `
  --model gpt-sovits `
  --epochs 1 `
  --consent
```

Pass criteria:

- dataset validation requires `train.list` and `wavs/`,
- preparation scripts complete,
- SoVITS and GPT training commands complete,
- output directories and `training-complete.json` exist,
- `train.log` is retained,
- missing consent fails before training starts.

A one-epoch smoke confirms orchestration only; it is not a quality training benchmark.

## 14. Adapter isolation

After several families are installed:

```powershell
Get-ChildItem "$env:TAKOKIT_HOME\runners\python-managed\adapters" -Directory
```

Each adapter must own its environment, source revision marker, manifest and install log. Repairing one adapter must not mutate unrelated environments.

## 15. Failure matrix

Test at least one lightweight and one heavy model for:

- unavailable network,
- interrupted package installation,
- interrupted model snapshot download,
- insufficient disk,
- missing CUDA,
- GPU out-of-memory,
- invalid reference audio,
- invalid voice profile,
- missing RVC checkpoint,
- malformed GPT-SoVITS dataset,
- empty TTS input,
- empty transcript,
- unsupported capability,
- repeated concurrent pull.

A failed operation must never produce a completed event or executable readiness record.

## 16. Evidence row

Create one row per model and capability tested:

| Field | Value |
|---|---|
| Model ID | |
| Capability | TTS / STT / clone / convert / train |
| Takokit commit | |
| OS / architecture | |
| CPU / RAM | |
| GPU / VRAM / driver | |
| Pull result and duration | |
| Runner/adapter result | |
| Command | |
| Output path and bytes | |
| Transcript/audio quality | |
| Retry/idempotency | |
| Session/history result | |
| Status | Pass / Fail / Blocked input / Blocked hardware |
| Tester and date | |

Promote a model to **Locally verified** only after the relevant row is complete and the output has been manually inspected.
