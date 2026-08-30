#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("The Takokit tray controller is available on Windows only.");
}

#[cfg(windows)]
mod windows_tray {
    use std::{
        ffi::OsStr,
        io::Write,
        os::windows::ffi::OsStrExt,
        path::PathBuf,
        process::{Command, Stdio},
        ptr::{null, null_mut},
        time::Duration,
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
        },
        System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
        UI::{
            Shell::{
                Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
                NOTIFYICONDATAW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
                DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, LoadIconW, MessageBoxW,
                PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, TrackPopupMenu,
                TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HMENU, IDI_APPLICATION,
                MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_SEPARATOR,
                MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_APP, WM_CLOSE, WM_COMMAND,
                WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
            },
        },
    };

    const TRAY_MESSAGE: u32 = WM_APP + 1;
    const ID_OPEN_GUI: usize = 100;
    const ID_COPY_API: usize = 101;
    const ID_START: usize = 102;
    const ID_STOP: usize = 103;
    const ID_UPDATE: usize = 104;
    const ID_STARTUP: usize = 105;
    const ID_ABOUT: usize = 106;
    const ID_QUIT: usize = 107;
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = "TakokitTray";

    pub fn run() -> anyhow::Result<()> {
        let class_name = wide("TakokitTrayControllerWindow");
        let arguments = std::env::args().collect::<Vec<_>>();
        if arguments
            .iter()
            .any(|argument| argument == "--startup-enable")
        {
            set_startup(true);
            return Ok(());
        }
        if arguments
            .iter()
            .any(|argument| argument == "--startup-disable")
        {
            set_startup(false);
            return Ok(());
        }
        if arguments.iter().any(|argument| argument == "--quit") {
            let hwnd = unsafe { FindWindowW(class_name.as_ptr(), null()) };
            if !hwnd.is_null() {
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            }
            return Ok(());
        }
        let mutex_name = wide("Local\\TakokitTrayControllerV1");
        let mutex = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
        if mutex.is_null() {
            anyhow::bail!("could not create tray single-instance lock");
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(mutex) };
            return Ok(());
        }

        let instance = unsafe { GetModuleHandleW(null()) };
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..unsafe { std::mem::zeroed() }
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            anyhow::bail!("could not register tray window class");
        }
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide("Takokit").as_ptr(),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        };
        if hwnd.is_null() {
            anyhow::bail!("could not create tray controller window");
        }
        add_icon(hwnd)?;
        let mut message: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        remove_icon(hwnd);
        unsafe { CloseHandle(mutex) };
        Ok(())
    }

    fn add_icon(hwnd: HWND) -> anyhow::Result<()> {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = 1;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_MESSAGE;
        data.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
        copy_wide(&mut data.szTip, "Takokit 0.2.0");
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            anyhow::bail!("could not add Takokit notification icon");
        }
        Ok(())
    }

    fn remove_icon(hwnd: HWND) {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = 1;
        unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            TRAY_MESSAGE if lparam as u32 == WM_RBUTTONUP => {
                show_menu(hwnd);
                0
            }
            TRAY_MESSAGE if lparam as u32 == WM_LBUTTONDBLCLK => {
                run_tako(&["gui"]);
                0
            }
            WM_COMMAND => {
                handle_command(hwnd, wparam & 0xffff);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn show_menu(hwnd: HWND) {
        let menu = CreatePopupMenu();
        let status = server_status();
        append(
            menu,
            0,
            &format!("Server: {}", status.label()),
            MF_STRING | MF_DISABLED | MF_GRAYED,
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        append(menu, ID_OPEN_GUI, "Open GUI", MF_STRING);
        append(menu, ID_COPY_API, "Copy API URL", MF_STRING);
        if status.running() {
            append(menu, ID_STOP, "Stop Server", MF_STRING);
        } else if status.can_start() {
            append(menu, ID_START, "Start Server", MF_STRING);
        } else {
            append(
                menu,
                0,
                "Port 5050 is in use",
                MF_STRING | MF_DISABLED | MF_GRAYED,
            );
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        append(menu, ID_UPDATE, "Check for Updates", MF_STRING);
        append(
            menu,
            ID_STARTUP,
            "Launch Takokit at startup",
            MF_STRING | if startup_enabled() { MF_CHECKED } else { 0 },
        );
        append(menu, ID_ABOUT, "About Takokit v0.2.0", MF_STRING);
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        append(menu, ID_QUIT, "Quit Takokit", MF_STRING);
        let mut point: POINT = std::mem::zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            point.x,
            point.y,
            0,
            hwnd,
            null(),
        );
        DestroyMenu(menu);
    }

    unsafe fn append(menu: HMENU, id: usize, label: &str, flags: u32) {
        let label = wide(label);
        AppendMenuW(menu, flags, id, label.as_ptr());
    }

    unsafe fn handle_command(hwnd: HWND, command: usize) {
        match command {
            ID_OPEN_GUI => run_tako(&["gui"]),
            ID_COPY_API => copy_api_url(),
            ID_START => run_tako(&["server", "start"]),
            ID_STOP => run_tako(&["server", "stop"]),
            ID_UPDATE => show_update(hwnd),
            ID_STARTUP => set_startup(!startup_enabled()),
            ID_ABOUT => {
                MessageBoxW(
                    hwnd,
                    wide("Takokit 0.2.0\nLocal voice AI runtime\n\nGUI: browser-based\nAPI: OpenAI-compatible audio + Takokit native").as_ptr(),
                    wide("About Takokit").as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            ID_QUIT => {
                remove_icon(hwnd);
                PostQuitMessage(0);
            }
            _ => {}
        }
    }

    fn run_tako(args: &[&str]) {
        if let Some(tako) = tako_executable() {
            let mut command = Command::new(tako);
            command
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
            let _ = command.spawn();
        }
    }

    fn tako_executable() -> Option<PathBuf> {
        let sibling = std::env::current_exe().ok()?.with_file_name("tako.exe");
        sibling.is_file().then_some(sibling)
    }

    #[derive(Clone, Copy)]
    enum ServerState {
        Managed,
        Foreground,
        Stopped,
        Occupied,
    }

    impl ServerState {
        fn label(self) -> &'static str {
            match self {
                Self::Managed => "Running (managed)",
                Self::Foreground => "Running (foreground)",
                Self::Stopped => "Stopped",
                Self::Occupied => "Unavailable (foreign port owner)",
            }
        }
        fn running(self) -> bool {
            matches!(self, Self::Managed | Self::Foreground)
        }
        fn can_start(self) -> bool {
            matches!(self, Self::Stopped)
        }
    }

    fn server_status() -> ServerState {
        let url = "http://127.0.0.1:5050/api/v1/daemon/identity";
        match ureq::get(url).timeout(Duration::from_millis(500)).call() {
            Ok(response) => {
                let value: serde_json::Value = response.into_json().unwrap_or_default();
                match value.pointer("/identity/mode").and_then(|v| v.as_str()) {
                    Some("managed") => ServerState::Managed,
                    _ => ServerState::Foreground,
                }
            }
            Err(ureq::Error::Status(_, _)) => ServerState::Occupied,
            Err(ureq::Error::Transport(error))
                if error.kind() == ureq::ErrorKind::ConnectionFailed =>
            {
                ServerState::Stopped
            }
            Err(_) => ServerState::Occupied,
        }
    }

    fn copy_api_url() {
        if let Ok(mut child) = Command::new("clip.exe").stdin(Stdio::piped()).spawn() {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(b"http://127.0.0.1:5050/v1");
            }
        }
    }

    fn startup_enabled() -> bool {
        Command::new("reg.exe")
            .args(["query", RUN_KEY, "/v", RUN_VALUE])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn set_startup(enabled: bool) {
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let value = format!("\"{}\" --startup", exe.display());
                let _ = Command::new("reg.exe")
                    .args([
                        "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &value, "/f",
                    ])
                    .status();
            }
        } else {
            let _ = Command::new("reg.exe")
                .args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
                .status();
        }
    }

    unsafe fn show_update(hwnd: HWND) {
        let output = tako_executable().and_then(|tako| {
            Command::new(tako)
                .args(["--output", "json", "update", "check"])
                .output()
                .ok()
        });
        let message = match output {
            Some(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                if text.contains("update_available") && text.contains("true") {
                    "A Takokit update is available. Open a terminal and run `tako update apply` when current work is idle."
                } else {
                    "Takokit is up to date."
                }
            }
            _ => "Takokit could not check for updates. See `tako server logs` for diagnostics.",
        };
        MessageBoxW(
            hwnd,
            wide(message).as_ptr(),
            wide("Takokit Updates").as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
        for (destination, source) in target.iter_mut().zip(wide(value)) {
            *destination = source;
        }
    }
}

#[cfg(windows)]
fn main() {
    let _ = windows_tray::run();
}
