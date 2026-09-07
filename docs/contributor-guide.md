# Contributor guide

This guide describes how to extend Takokit without creating separate runtime semantics across CLI, TUI, browser GUI, local API, runners, and packaged applications.

## Shared-service flow

```text
CLI ─────┐
TUI ─────┼── shared Takokit runtime/services ── planner ── runner/adapter ── model
GUI ─────┤                    │
API ─────┘                    ├── ~/.takokit reusable state
                              └── <workspace>/.tako project state
```

Platform app integrations (Windows notification-area resident, macOS menu-bar resident, Linux desktop launcher) start/control the same local server. They do not contain another inference implementation.

## Add a model

1. Choose a canonical family name and task.
2. Add a versioned/pinned release in the canonical registry.
3. Set truthful aliases/default tag, source revision/digest, license, hardware, and support metadata.
4. Select an existing runner/adapter contract where possible.
5. Add registry/planning tests.
6. Pull/run on representative hardware before upgrading the model's advertised execution status.
7. Update generated/public presentation only where registry-derived fields cannot explain the workflow.

Never turn an upstream repository's arbitrary scripts into an implicit Takokit execution path.

## Add a runner or adapter

Runner contracts own broad execution environments; adapters own model-family integration inside those contracts. Keep model-family dependency sets isolated when they can conflict.

A runner/adapter change should define:

- install/bootstrap inputs,
- pinned/verified dependencies,
- process/JSON contracts,
- readiness/doctor behavior,
- output verification,
- cleanup/recovery behavior,
- platform/hardware boundaries.

Managed tooling must be self-contained and recoverable. Do not require a user to install Homebrew, a global Python package, or a system uv merely to pull a supported packaged model.

## Add a CLI command

The Clap command tree in `apps/cli/src/args.rs` and its scoped modules is authoritative.

For a new command:

1. Add typed args/subcommand definitions.
2. Route to shared service/runtime logic.
3. Keep `--output human` readable and `--output json` valid JSON stdout.
4. Add parsing/unit/integration tests.
5. Update public CLI docs and documentation drift checks.

## Add an API route

- OpenAI-compatible **audio** routes belong under `/v1` only when their semantics are intentionally compatible.
- Takokit-specific functionality belongs under `/api/v1`.
- Privileged GUI/control helpers should stay narrowly scoped and local.

Update the executable router/OpenAPI contract, API tests, `docs/api-route-inventory.md`, and user/developer docs together.

## Add TUI or GUI functionality

UI code should translate user intent into shared runtime actions. Do not add a TUI-only/GUI-only model installer, workspace implementation, or inference path.

The GUI is a browser application built from `apps/gui` and served by `takokit-server`. End users do not need Node/npm.

## State invariants

- `~/.takokit` is reusable global runtime/model/voice/log state.
- `<workspace>/.tako` is project session/output state.
- Immutable installed application files are separate from both.
- Normal uninstall preserves global/project state.
- Destructive reset is separately confirmed.
- Process ownership is verified: resident-owned, developer-owned foreground, and foreign processes are distinct states.

See [state-and-storage.md](state-and-storage.md).

## Testing strategy

Run deterministic/model-free checks on every change. Use representative lightweight real models in distribution acceptance rather than downloading the entire registry.

Required baseline:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
python scripts/audit_file_sizes.py
python scripts/check_documentation.py

cd apps/gui && npm ci && npm run build
cd ../../site && npm ci && npm run check
```

Distribution changes additionally require the appropriate Windows/Linux/macOS acceptance jobs. Support claims that depend on specific hardware require actual-machine evidence.

## Release changes

Do not publish release assets manually from random local builds. The tagged release workflow builds all supported platforms from the same source commit, verifies the production Takokit signing identity, assembles manifests/index/checksums/provenance, and publishes the release only after acceptance.

See [release-process.md](release-process.md).
