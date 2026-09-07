# Contributing to Takokit

Takokit is a Rust-first local voice runtime with shared semantics across CLI, TUI, browser GUI, local APIs, model runners, and platform distributions. Changes should extend that shared runtime rather than create parallel behavior in one interface.

## Release maturity and branch policy

- Every `0.x.x` release is **Beta**.
- `1.0.0` is the first Stable release target.
- Do feature work on a dedicated branch.
- Keep a slice/feature on one branch through fixes and acceptance.
- Do not push implementation directly to `main` unless the repository owner explicitly chooses an emergency/direct-main workflow.
- Create a PR only after the branch's required automated and manual gates are satisfied.
- Release tags are created from reviewed `main`, never from an unmerged feature branch.

## Repository map

```text
apps/cli                 CLI, TUI, installed application entrypoints/updater
apps/gui                 React/Vite browser GUI
crates/takokit-core      shared types, requests, capabilities, sessions/errors
crates/takokit-store     global state, voices, workspace/session storage
crates/takokit-package   registry, planning, pulling, managed runtimes
crates/takokit-models    model execution adapters
crates/takokit-server    Axum server, local APIs, GUI hosting/control helpers
crates/takokit-audio     audio utilities
crates/takokit-release   release manifests/signing/index contracts
registry                 canonical model registry
runners                  packaged runner/adapter resources
scripts                  validation, model smoke, release/install tooling
packaging                platform application/installer integration
site                     public companion site, docs, model library, release API
```

See [docs/architecture.md](docs/architecture.md) and [docs/contributor-guide.md](docs/contributor-guide.md).

## Development setup

Install Rust stable and Node.js 20+ with npm. Platform packaging can require additional native build tools; ordinary core/GUI/site work should not require model downloads.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
python scripts/audit_file_sizes.py
```

GUI:

```bash
cd apps/gui
npm ci
npm run build
cd ../..
```

Companion site and public docs:

```bash
cd site
npm ci
npm run check
cd ..
```

Do not reintroduce `apps/desktop`, Tauri, Electron, or an embedded WebView. Takokit's GUI is served by the local server and opened in the user's browser.

## Core design rules

- Keep model-specific behavior behind runner/adapter contracts.
- Keep CLI/TUI/GUI/API actions routed through shared application/runtime services.
- Do not put independent model semantics in UI components or HTTP handlers.
- Prefer typed errors and explicit unsupported/hardware-blocked states.
- Never kill a process solely because its name or port resembles Takokit; use verified process identity/ownership.
- Separate immutable application files from mutable `~/.takokit` and workspace `.tako` state.
- Destructive operations require previews/confirmation where the current product contract demands them.
- Model and release downloads must remain pinned/integrity-verified according to their trust contract.

## Add a model or variant

1. Choose the correct task/family and canonical registry name.
2. Add immutable tag/source metadata, aliases/defaults, license, hardware, runner, and adapter fields.
3. Reuse an existing generic runner where possible.
4. Do not add arbitrary repository-script execution as a shortcut.
5. Add deterministic planning/registry tests.
6. Run a real machine smoke when changing advertised executable support.
7. Update public documentation only from the canonical registry facts where practical.

See [docs/registry.md](docs/registry.md), [docs/runners.md](docs/runners.md), and [docs/model-support.md](docs/model-support.md).

## Add or update a runner/adapter

Keep runner lifecycle and adapter dependencies isolated. Managed Python adapters must not silently mutate another model family's environment. Bootstrap/install steps should be deterministic, recoverable, logged, and testable without requiring global Python packages.

For managed tooling such as uv:

- use the Takokit-owned pinned resolver,
- verify platform/architecture and integrity,
- keep explicit environment overrides real and validated,
- keep managed state self-repairable,
- do not adopt arbitrary Homebrew/system tools into a managed location and then reject them.

## Add a CLI command

Define it in the Clap command tree under `apps/cli/src/args.rs` (or its scoped modules), route it through shared services, add parsing/behavior tests, update machine/human output contracts, then update the canonical CLI documentation. `--output json` must remain valid JSON stdout.

## Add an API endpoint

Prefer `/api/v1` for Takokit-native functionality. The `/v1` namespace is reserved for the intentionally small OpenAI-compatible **audio** subset. Do not expand compatibility claims without implementing/testing the corresponding semantics.

Update:

- router and typed schemas,
- `/openapi.json`,
- API contract tests,
- `docs/api-route-inventory.md`,
- public docs/examples where release-facing.

Privileged local-control helpers (picker/open/shutdown/update/destructive maintenance) should remain narrowly scoped and must not become generic arbitrary-command/filesystem APIs.

## Add TUI/GUI functionality

The UI is a client of shared runtime semantics. A new screen/action should map to an existing or deliberately added shared service/API behavior.

GUI source lives in `apps/gui`; production assets are packaged and served by `takokit-server`. Do not require Node/npm on end-user machines.

## Tests and CI

Normal changes should run the core gates above. Distribution/runtime changes must additionally run the relevant package acceptance workflow and preserve cross-platform regression contracts.

Heavy model downloads do not belong in every CI run. Use small representative models for package smoke tests and deterministic fixtures/contracts for larger GPU-dependent paths. Real hardware evidence is required before upgrading an advertised support status.

See [docs/TESTING.md](docs/TESTING.md) and [docs/release-process.md](docs/release-process.md).

## Documentation expectations

Update documentation in the same change when behavior, commands, routes, install paths, support status, or safety boundaries change. Avoid duplicating manually maintained model tables when registry-generated facts can be used.

Repository documentation lives under `docs/`; public user/API docs are rendered by the companion site. README should remain an entry point rather than a second full manual.

## Security and voice safety

Read [SECURITY.md](SECURITY.md). Do not weaken path/archive safety, release signature/checksum validation, non-loopback authentication, process identity, destructive-operation confirmation, or voice consent boundaries to make a test pass.
