# State and storage

Takokit separates reusable machine-level runtime state, immutable installed application files, and project-local sessions/outputs. This separation is a product invariant across Windows, Linux, and macOS.

## Global runtime root

```text
~/.takokit/
```

Conceptually contains:

```text
models/       installed model records/artifacts
runners/      runner/adapter runtime state
blobs/        content-addressed reusable data
cache/        reconstructible caches
manifests/    model/runtime metadata
voices/       reusable voice profiles/projects
logs/         runtime/server/bootstrap logs
tools/        Takokit-managed tools such as pinned uv
```

Exact internal subdirectories can evolve; callers should use Takokit APIs/commands rather than hard-code private internal paths.

## Workspace root

Each project stores local work state in:

```text
<workspace>/.tako/
```

Typical structure:

```text
.tako/
├── active-session
├── version
└── sessions/
    └── <session-id>/
        ├── session.json
        ├── events.jsonl
        └── outputs/
```

The current working directory supplies the default CLI project context. Use `--workspace <path>` when the project must be explicit. Use `--session <uuid>` to resume a specific session.

## Interface consistency

CLI, TUI, browser GUI, and native app launchers all resolve the same Takokit home/workspace/session contracts. Selecting a workspace in the GUI does not create a GUI-private project database.

The GUI folder picker is a narrowly scoped local control action that returns an absolute selected directory. It is not a general arbitrary filesystem/command API.

## Application installation

Immutable packaged application files are stored separately from `~/.takokit`:

- Windows — per-user Takokit program installation directory,
- Linux — per-user application tree under `~/.local/share/takokit` by default,
- macOS — per-user runtime tree plus `~/Applications/Takokit.app` by default.

Portable archives run from their extracted tree and do not create desktop/startup/PATH integration simply by being extracted.

## Server identity and ownership

Takokit distinguishes:

1. **resident/managed-owned server** — can be stopped when the owning resident quits,
2. **developer-owned foreground `tako serve`** — must survive resident shutdown unless explicitly stopped,
3. **foreign process** — must never be killed merely because it uses Takokit's configured port.

Identity is based on Takokit instance/process metadata rather than name/port coincidence.

## Updates

Updates replace immutable application files only after release metadata/signature/checksum verification. Mutable global runtime data and workspace `.tako` data are not part of the application replacement set.

Unix update/install paths preserve executable permissions. Interrupted replacement uses staging/rollback contracts rather than modifying the active application tree in place without recovery.

## Uninstall versus reset

Normal uninstall removes the Takokit application/integration but preserves:

```text
~/.takokit
<workspace>/.tako
```

This means reinstalling/upgrading Takokit can reuse downloaded models, voices, and project history where compatible.

Destructive data reset is a separate command and requires explicit path acknowledgements. Project `.tako` deletion is separate again because project outputs/history may need to survive even when the global runtime is removed.

## Backups

For a complete user-data backup, include `~/.takokit` plus any projects whose `.tako` history/outputs matter. For a lightweight project-only backup, include the project directory including `.tako`; downloaded model runtimes can be reconstructed separately.
