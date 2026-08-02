# RVC execution and perceptual-quality gate

Takokit treats RVC compatibility and RVC voice quality as two separate results.

A conversion process that exits successfully and writes a valid WAV proves only that the adapter, model, index, HuBERT assets, pitch extractor, audio decoder and output path worked together. It does not prove that the output is natural or resembles the intended target speaker.

## Expected result

RVC transfers vocal timbre while retaining most of the source performance. A good conversion should preserve:

- the same spoken words
- approximately the same timing and pauses
- the source speaker's delivery and prosody
- intelligibility

It should materially change the apparent speaker identity toward the supplied target reference.

RVC does not invent the target speaker's cadence from unrelated source delivery. To reproduce a distinctive cadence, the source performance must contain a similar rhythm and expression.

## Package format

A quality-testable RVC package should be a directory containing a checkpoint, its matching retrieval index, a target reference recording, and `rvc.json`.

```text
my-rvc-voice/
  voice.pth
  added_voice.index
  target-reference.wav
  rvc.json
```

Example `rvc.json`:

```json
{
  "checkpoint": "voice.pth",
  "index": "added_voice.index",
  "target_reference": "target-reference.wav",
  "quality_baseline": true,
  "license": "SPDX-or-package-license-identifier"
}
```

Takokit refuses ambiguous directories containing multiple checkpoints or indexes unless `rvc.json` selects the intended files. A single automatically discovered index is reported as unverified rather than silently treated as a confirmed pair.

`quality_baseline_ready` becomes true only when the package explicitly declares a quality baseline, supplies license metadata, contains a target reference, and has a sufficiently established checkpoint/index pairing.

## CLI

```powershell
& $Tako --direct convert `
    "C:\path\to\source.wav" `
    --target-voice "C:\path\to\my-rvc-voice" `
    --model rvc `
    --f0-method rmvpe `
    --pitch-shift 0 `
    --index-rate 0.75 `
    --rms-mix-rate 0.25 `
    --protect 0.33 `
    --filter-radius 3 `
    --consent
```

Validated controls:

| Control | Range | Default |
| --- | ---: | ---: |
| F0 method | `rmvpe`, `harvest`, `crepe`, `pm` | `rmvpe` |
| Pitch shift | -24 to 24 semitones | 0 |
| Index rate | 0.0 to 1.0 | 0.75 |
| RMS mix rate | 0.0 to 1.0 | 0.25 |
| Protect | 0.0 to 0.5 | 0.33 |
| Filter radius | 0 to 7 | 3 |

The response records the effective settings, checkpoint and index paths, SHA-256 hashes, byte sizes, pairing state, target reference and quality-baseline readiness.

## Result interpretation

A successful runtime response starts with:

```text
execution            passed
perceptual quality   not evaluated
listening review     required
```

Do not rewrite `not evaluated` to `passed` merely because the output WAV exists.

## Human listening checklist

Compare the source audio, target reference and converted output. Record a perceptual pass only when all conditions hold:

1. The same words remain intelligible and unchanged.
2. Vocal timbre changed materially from the source.
3. The output resembles the supplied target reference.
4. There are no severe robotic, metallic, tearing, octave-jump, dropout or timing artefacts.
5. The package reports `quality_baseline_ready: true`.

A generic, unfinished or undocumented checkpoint may be used to test execution, but it cannot serve as a launch-quality baseline.

## Evidence record

For every actual-machine quality test, retain:

- Takokit commit and version
- source-audio path and hash
- checkpoint and index hashes
- target-reference path and hash
- effective RVC settings
- output path and byte size
- execution result
- each human checklist result
- final perceptual result
- brief notes describing artefacts or target similarity

CI verifies schemas, routing, validation and package-selection behavior. Target-speaker similarity and naturalness remain actual-machine human tests.
