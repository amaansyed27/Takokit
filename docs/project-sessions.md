# Project-local sessions and outputs

Takokit keeps reusable models, runners, adapters, and caches globally under `~/.takokit`. User work belongs to the project that launched Takokit.

Running CLI, TUI, or GUI from a project creates:

```text
<project>/.tako/
├── active-session
├── version
└── sessions/
    └── <uuid>/
        ├── session.json
        ├── events.jsonl
        └── outputs/
```

## CLI

Start or resume work in the current project:

```powershell
takokit
takokit speak "Hello" --model kokoro
takokit transcribe .\sample.wav --model whisper-tiny
```

Use another project or resume an exact session:

```powershell
takokit --workspace C:\work\voice-demo sessions list
takokit --workspace C:\work\voice-demo --session <uuid> speak "Resume this session" --model kokoro
```

Session commands:

```powershell
takokit sessions list
takokit sessions list --query whisper
takokit sessions new --title "Narration tests"
takokit sessions show <uuid>
takokit sessions open <uuid>
takokit sessions rm <uuid>
```

## TUI

Run `takokit` without a subcommand. Use `/sessions` from navigation screens to open the Sessions view, `/new` to create a new session, or the dedicated Sessions tab.

All TUI tasks remain in one Ratatui process. Switching sessions changes where subsequent outputs and events are saved.

## GUI

Launch with:

```powershell
takokit gui
```

The GUI launch URL includes the project path and active session. The History page supports:

- full-text search across session titles, model IDs, input text, transcripts, errors, and messages
- opening a previous session
- creating a new session
- deleting inactive sessions
- reopening generated audio and transcript files through protected local API routes

## Voice profiles

Create a reusable consent-backed profile:

```powershell
takokit clone .\reference.wav --name "My voice" --model chatterbox --consent --consent-note "I recorded and own this voice"
```

The reference audio is stored globally under `~/.takokit/voices` so it can be reused across projects. The creation event and profile JSON artifact are also stored in the active `.tako` session.

Use the profile ID with a cloning-capable model:

```powershell
takokit speak "This uses my saved profile" --model chatterbox --voice my-voice
```
