# Codex stability review prompt

Use the following prompt in Codex after the final branch is merged into `main` and before any website work begins.

---

You are reviewing the Takokit repository as a principal Rust systems engineer, release engineer and security-minded local-AI runtime maintainer.

Takokit is intended to provide an Ollama-level local experience for voice AI: simple model pull/install/remove, shared CLI/TUI/GUI/API state, local TTS/STT/voice-cloning workflows, and project-local `.tako` sessions and outputs. This is a release-blocking review, not a superficial code-style review.

## Operating rules

1. Work from the current `main` branch and record the starting commit SHA.
2. Read the repository root specification PDF, `README.md`, `docs/model-support.md`, `docs/project-sessions.md`, `docs/runners.md`, and `docs/TESTING.md` before judging implementation.
3. Inspect actual code and manifests. Do not trust README claims without tracing the execution path.
4. Run commands and tests yourself where the environment permits.
5. Do not label a registry-only model supported.
6. Distinguish:
   - locally verified,
   - executable path awaiting hardware smoke,
   - planned or blocked.
7. Never install model dependencies globally. Takokit must own all runtime environments.
8. Do not rewrite the project into a new architecture. Fix concrete defects while preserving the Rust-first modular design.
9. Do not add fake outputs, mocks in production paths, silent fallbacks or hardcoded success states.
10. Keep source files below the repository's enforced line limit.

## Primary review questions

Determine whether Takokit is stable enough to begin building its public website and model library.

The final answer must be an explicit **GO**, **CONDITIONAL GO**, or **NO-GO**.

## Repository and branch audit

- Confirm `main` contains every valid change from merged PRs and completion branches.
- List remaining remote branches and identify whether each is merged, obsolete or contains unique work.
- Confirm no temporary self-modifying GitHub Actions workflows remain.
- Confirm CI references the intended default branch.
- Check for generated logs, validation reports, build products, secrets or temporary files accidentally committed.
- Inspect `.gitignore` for model weights, `.tako`, local stores, Python environments and build outputs.

## Automated build and quality gates

Run and capture results:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python .\scripts\check_file_sizes.py
Push-Location .\apps\gui
npm ci
npm run build
Pop-Location
cargo build --release
```

On non-Windows systems use equivalent shell syntax.

Report every warning that signals dead production paths, unreachable branches, unused lifecycle code or duplicated implementations. Do not fail the review for harmless dependency warnings unless they affect security or release reliability.

## Architecture audit

Trace these end-to-end:

1. `tako pull <model>`
2. `tako speak ...`
3. `tako transcribe ...`
4. CLI voice profile creation
5. TUI speech, transcription, cloning and session actions
6. GUI speech, transcription, cloning and History
7. API speech, transcription, sessions and voice profiles

Confirm all surfaces use the same:

- package registry,
- installed model and runner records,
- execution planner,
- runner adapters,
- workspace/session store,
- output locations,
- error types,
- lifecycle states.

Flag any surface that invokes a separate implementation, writes to a different output directory, maintains duplicated model state, or bypasses capability/readiness validation.

## Daemon and process ownership

Audit:

- managed versus direct server mode,
- PID and lock ownership,
- stale process recovery,
- repeated start idempotency,
- restart and shutdown authorization,
- executable locking on Windows,
- active execution tracking,
- cancellation limitations,
- log retention and diagnostic usefulness.

Attempt race conditions where practical: simultaneous daemon starts, simultaneous pulls and shutdown during inference.

## `.tako` workspace and session audit

Verify the launch-directory rule for CLI, TUI and GUI.

Inspect:

- workspace path normalization,
- global `~/.takokit` versus project-local `.tako` separation,
- active-session handling,
- session creation/resume/delete,
- append-only JSONL event integrity,
- atomic summary replacement,
- Windows replacement semantics,
- locking and concurrent writers,
- output count/event count consistency,
- session search accuracy,
- history retention across upgrades,
- behaviour after deleting the active session,
- workspace paths with spaces and Unicode.

Security test all output-serving routes for:

- `..` traversal,
- encoded separators,
- absolute paths,
- symlink/junction escape,
- cross-session access,
- cross-workspace access,
- arbitrary local file disclosure.

Treat any arbitrary file-read or output escape as P0.

## Model and runner truthfulness audit

Generate a table from the registry containing:

- model ID,
- family,
- capability flags,
- required runner,
- required adapter,
- metadata-only flag,
- declared hardware,
- license string,
- planner state on a clean store,
- planner state after mocked/fixture installation,
- code path implementing execution,
- actual smoke evidence available.

Compare it with `docs/model-support.md`.

Flag:

- unknown runners,
- missing adapters,
- duplicate or conflicting adapter IDs,
- families that map to the wrong adapter,
- models reported executable without an implementation,
- weights that are downloaded implicitly but presented as checksum-verified,
- inaccurate CPU/GPU claims,
- incorrect model or code licenses,
- live/streaming capabilities that only perform batch inference.

The catalog target is 20–30 total model IDs, not 20–30 per category.

## Pull/install reliability audit

Inspect model, runner and adapter installation for:

- resumable or safely restartable downloads,
- checksum enforcement where checksums are available,
- content-addressed blob reuse,
- atomic install records,
- rollback after failed verification,
- idempotent repeat pulls,
- repair of incomplete runtime installs,
- concurrent pull locking,
- disk-space failures,
- network interruption,
- partial archive extraction,
- archive path traversal,
- environment isolation,
- pinned dependency versions,
- meaningful install logs.

Confirm `tako pull <model>` owns the complete required setup and does not instruct the user to manually install Python packages.

## Managed Python adapter audit

Review every adapter below `runners/python/` and its matching spec.

For each adapter verify:

- isolated virtual environment,
- supported Python version,
- dependency set,
- official upstream API usage,
- typed JSON stdin/stdout contract,
- operation validation,
- model ID validation,
- input file validation,
- output path validation,
- non-empty output verification,
- stderr/error propagation,
- CUDA/CPU/MPS device behaviour,
- profile/reference-voice semantics,
- no arbitrary shell command construction,
- no network server or Gradio launch,
- no writing outside managed cache/output paths.

Pay particular attention to:

- Qwen3-TTS,
- Chatterbox,
- F5-TTS,
- Coqui XTTS/YourTTS,
- Dia,
- Bark/MMS/Transformers ASR,
- SenseVoice,
- Voxtral,
- Canary/Parakeet NeMo,
- Kyutai DSM TTS.

For Kyutai verify that Takokit uses the official Moshi DSM API, the intended checkpoint, precomputed voice embeddings and CUDA readiness checks. Do not call arbitrary reference-audio input Kyutai voice cloning.

## Voice safety and profile audit

Verify:

- explicit consent is mandatory in CLI, TUI and GUI,
- API cannot bypass consent,
- profile IDs are sanitized,
- duplicate profile handling is safe,
- source audio is copied rather than referenced from an unstable external path,
- profile metadata cannot escape the voice store,
- compatible models resolve profile IDs consistently,
- deleted or corrupt profile audio produces a typed failure,
- no model marked as non-cloning accepts a local reference file as cloning.

A consent bypass is P0 or P1 depending on impact.

## TUI review

Run the TUI in a real terminal and verify:

- it is task-oriented rather than a CLI command editor,
- all screens are reachable,
- text editing works with Windows paths,
- Enter performs the visible primary action,
- model readiness changes refresh correctly,
- long-running tasks do not tear down the terminal,
- progress and errors remain visible,
- `/sessions`, `/new` and `/clone` behave correctly,
- session switching redirects subsequent output,
- Ctrl+C cannot silently orphan an installer,
- small terminal sizes do not panic.

Flag confusing double-Enter behaviour, invisible focus, unreachable actions or raw command syntax in normal workflows.

## GUI review

Run the daemon-served production GUI and verify:

- workspace and session context survives routing and refresh,
- every API request that writes output carries context,
- History search is correct and bounded,
- audio object URLs are revoked,
- output loading cannot disclose arbitrary files,
- voice-profile consent and readiness gates work,
- model status matches CLI and TUI,
- errors are actionable,
- no stale mock data masks API failures,
- responsive layout remains usable,
- accessibility labels and keyboard controls are reasonable.

## API review

Audit API compatibility and safety:

- request/response schemas,
- typed errors and status codes,
- local-only binding,
- workspace header encoding/decoding,
- session ownership,
- output content types,
- maximum input considerations,
- path handling,
- concurrent requests,
- server panic resistance.

Identify any endpoint that trusts arbitrary workspace paths without an explicit local threat-model decision.

## Training and conversion honesty

Inspect GPT-SoVITS, RVC and OpenVoice declarations.

If full training/conversion orchestration is not implemented, ensure:

- they remain planned or blocked,
- GUI/TUI do not imply readiness,
- commands return typed actionable errors,
- documentation does not include them in verified counts.

Do not invent a fake training workflow to satisfy the roadmap.

## Documentation review

Check that README and docs agree on:

- global and project-local storage,
- daemon behaviour,
- model tiers,
- runner installation,
- voice consent,
- known platform limitations,
- installer status,
- website readiness criteria.

Update stale statements discovered during the review.

## Fixing policy

Create a dedicated review branch.

You may directly fix:

- compile/test failures,
- incorrect status reporting,
- path validation bugs,
- session consistency bugs,
- missing tests,
- documentation drift,
- small adapter contract defects,
- obvious UI state errors.

Do not silently redesign architecture, change public command syntax or enable an untested heavy model. Document larger changes as blockers.

For each patch:

- add or update a regression test,
- keep files within the line limit,
- run the relevant focused tests,
- rerun the full gates before final reporting.

## Severity system

- **P0** — data loss, arbitrary file access/write, consent bypass with serious impact, command execution, corrupted global model store, release-blocking security defect.
- **P1** — common workflow broken, false executable/support claim, daemon ownership failure, session/output misrouting, installer corruption, cross-platform compile failure.
- **P2** — confusing UX, incomplete diagnostics, weak edge-case handling, documentation drift, maintainability issue.
- **P3** — cosmetic or optional improvement.

## Required output

Produce:

1. Starting and ending commit SHAs.
2. Commands run and pass/fail results.
3. Architecture summary based on actual call paths.
4. Findings ordered by severity with file and line references.
5. Patches made and regression tests added.
6. Model support table with truthful tiers.
7. Runner/adapter table and isolation assessment.
8. Workspace/session security assessment.
9. CLI/TUI/GUI/API parity assessment.
10. Windows-specific risks.
11. Known heavy-model tests that still require Amaan's RTX 5060 machine.
12. Remaining blockers before website work.
13. Final verdict: GO, CONDITIONAL GO or NO-GO.

A GO requires:

- all permanent CI gates green,
- no unresolved P0/P1 findings,
- no false model-support claims,
- shared workspace/session/output behaviour across all surfaces,
- at least the required core hardware smoke tests recorded in `docs/TESTING.md`,
- no temporary mutation workflows or unmerged unique branches.

Do not recommend beginning the website merely because the code compiles.

---
