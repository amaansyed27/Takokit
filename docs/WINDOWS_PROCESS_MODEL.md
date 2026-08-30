# Windows process model

Takokit has one consumer application and one command-line interface:

- `Takokit.exe` is the native GUI-subsystem resident application. It owns the notification icon, update timer, and managed-server lifecycle. A normal launch opens the browser GUI; `--background` is reserved for login startup and does not open a browser.
- `tako.exe` is the console CLI and TUI. It can ensure the resident application exists, but the TUI process never owns the resident lifetime.
- `takokit-server.exe` is the internal server runtime used for managed background service. A direct `tako serve` runs the same server implementation in the foreground.
- `takokit-updater.exe` is the existing signed update helper.

`takokit-tray.exe` is not built or shipped. Server control is identity-gated using the Takokit build, instance, configured endpoint, storage root, and executable location. A process merely occupying the configured port is never claimed or killed.

| Scenario | Expected processes and state |
| --- | --- |
| Nothing running | No resident or server process; configured port is free. |
| User clicks Takokit | One `Takokit.exe`, one managed `takokit-server.exe`, browser GUI open, no console window. |
| User opens TUI | Existing resident/server are reused; one console `tako.exe` hosts the TUI. |
| User exits TUI | TUI `tako.exe` exits; resident and server remain. |
| User runs `tako serve` with no server | One foreground `tako.exe` serves until Ctrl+C or a verified shutdown request. |
| User runs `tako serve` with a verified server | No new server; CLI reports the existing URL and exits cleanly. |
| User runs `tako stop` | The verified server stops and the port releases; a resident, if present, remains and displays Stopped. |
| User clicks Stop Server | The verified server stops; resident remains. |
| User clicks Start Server | Exactly one managed `takokit-server.exe` starts; resident remains. |
| User clicks Quit Takokit | Verified server stops, port releases, timers and icon are removed, then resident exits. |
| Server crashes while resident | Resident remains, restores/keeps its icon, displays Stopped, and offers Start Server. |

The resident re-adds its notification icon after the Windows `TaskbarCreated` broadcast so an Explorer restart does not leave a live invisible resident.
