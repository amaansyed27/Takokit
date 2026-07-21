# Interface scope

Takokit keeps model discovery separate from local runtime operation.

- The companion library site is the catalog and discovery surface.
- The GUI shows only models installed and verified on the current machine.
- The interactive TUI shows the same installed-model inventory.
- The CLI remains available for direct installation, automation, diagnostics, and scripting.
- Runners, voices, sessions, outputs, diagnostics, and settings remain local runtime concerns.

The TUI is task-first rather than tab-first. Home exposes Speak, Transcribe, Clone voice, Manage, Sessions, and Activity as visible numbered actions. Forms are single-column, management screens place details below the selected item, and the latest task output stays in a compact status bar until Activity is opened.

This boundary keeps the GUI and TUI lightweight while the library site can scale to a large model catalog without loading catalog data into every local session.
