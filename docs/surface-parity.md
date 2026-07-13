# CLI, TUI, GUI surface parity

Takokit has three user-facing surfaces over one runtime:

- direct CLI: `takokit <command>` or `tako <command>`
- interactive Ratatui interface: `takokit`
- browser GUI: `takokit gui`

## Backend rule

The managed daemon and its `/v1` API are the default application backend.

- The GUI calls the daemon API.
- Normal direct CLI commands are routed through `daemon_commands.rs` to the same API.
- The TUI validates palette input with the exact Clap `Cli` command grammar, launches the current Takokit executable with those arguments, captures stdout/stderr, and refreshes shared state.
- `--direct` remains an explicit developer escape hatch for supported in-process execution.

The TUI must not implement separate model installation, runner installation, inference, diagnostics, or lifecycle semantics. Adding a new public CLI command automatically makes its grammar available to the TUI palette; the parity test in `tui/command.rs` must also be updated so omissions fail CI.

## TUI coverage

The Models and Runners tabs provide shortcuts for frequent lifecycle operations. The Operations and System tabs expose execution, testing, setup, adapters, daemon controls, diagnostics, catalogs, and GUI launch actions.

The `/` palette accepts the complete public CLI surface, including quoted text and Windows paths. Foreground `serve` and internal daemon-child flags are intentionally rejected because they would block or bypass the interactive session; use `daemon start` instead.

## Adding features

When adding a feature:

1. Define the public command in `apps/cli/src/args.rs`.
2. Implement it through the shared daemon/API or an explicitly local setup service.
3. Add or extend the corresponding server route for GUI access when applicable.
4. Add it to the TUI parity test and, when useful, the Operations or System tab.
5. Keep `--direct` behavior semantically equivalent where supported.

No surface should maintain its own model catalog, installed-state database, artifact installer, runner lifecycle, or inference implementation.
