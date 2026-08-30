//! Native Windows resident application hosted by the normal `tako.exe` binary.

use crate::daemon;
use std::{
    ffi::OsStr,
    io::Write,
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    ptr::{null, null_mut},
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

mod startup;
mod update;
use startup::*;
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use update::*;
use uuid::Uuid;
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
    },
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{
        Shell::{
            Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
            NIM_MODIFY, NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, GetSystemMetrics, LoadImageW,
            MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
            SetTimer, TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
            HMENU, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, MB_ICONINFORMATION, MB_OK,
            MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, SM_CXSMICON,
            SM_CYSMICON, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY,
            WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
        },
    },
};

const TRAY_MESSAGE: u32 = WM_APP + 1;
const STATE_CHANGED: u32 = WM_APP + 2;
const UPDATE_READY: u32 = WM_APP + 3;
const UPDATE_CHECKED: u32 = WM_APP + 4;
const TIMER_UPDATE: usize = 1;
const UPDATE_INTERVAL_MS: u32 = 6 * 60 * 60 * 1000;
const NIN_BALLOONUSERCLICK: u32 = 0x0405;
const ID_OPEN_GUI: usize = 100;
const ID_COPY_API: usize = 101;
const ID_START: usize = 102;
const ID_STOP: usize = 103;
const ID_CHECK_UPDATE: usize = 104;
const ID_STARTUP: usize = 105;
const ID_ABOUT: usize = 106;
const ID_QUIT: usize = 107;
const ID_INSTALL_UPDATE: usize = 108;

#[derive(Default)]
struct ResidentState {
    owned_instance: Option<Uuid>,
    update_version: Option<String>,
    update_check_running: bool,
    last_update_check_failed: bool,
}

static STATE: OnceLock<Mutex<ResidentState>> = OnceLock::new();

pub(crate) fn run() -> anyhow::Result<()> {
    let class_name = wide("TakokitResidentWindow");
    let arguments = std::env::args().collect::<Vec<_>>();
    if let Some(action) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--resident-action")
        .map(|pair| pair[1].as_str())
    {
        let hwnd = unsafe { FindWindowW(class_name.as_ptr(), null()) };
        if !hwnd.is_null() {
            let command = match action {
                "start" => ID_START,
                "stop" => ID_STOP,
                "open-gui" => ID_OPEN_GUI,
                "check-update" => ID_CHECK_UPDATE,
                _ => anyhow::bail!("unknown resident action: {action}"),
            };
            unsafe { PostMessageW(hwnd, WM_COMMAND, command, 0) };
        }
        return Ok(());
    }
    if arguments
        .iter()
        .any(|argument| argument == "--resident-quit")
    {
        let hwnd = unsafe { FindWindowW(class_name.as_ptr(), null()) };
        if !hwnd.is_null() {
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
        return Ok(());
    }

    STATE.get_or_init(|| Mutex::new(ResidentState::default()));
    let mutex_name = wide("Local\\TakokitResidentApplicationV1");
    let instance_guard = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
    if instance_guard.is_null() {
        anyhow::bail!("could not create Takokit resident single-instance lock");
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(instance_guard) };
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
        anyhow::bail!("could not register Takokit resident window class");
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
        anyhow::bail!("could not create Takokit resident window");
    }

    add_icon(hwnd)?;
    unsafe { SetTimer(hwnd, TIMER_UPDATE, UPDATE_INTERVAL_MS, None) };
    ensure_server_async(hwnd, false);
    check_update_async(hwnd, true);

    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    remove_icon(hwnd);
    unsafe { CloseHandle(instance_guard) };
    Ok(())
}

fn store_and_config() -> (LocalStore, RuntimeConfig) {
    let store = LocalStore::new(LocalStore::default_root());
    let _ = store.ensure_layout();
    let config = RuntimeConfig::local(store.root().to_path_buf());
    (store, config)
}

fn ensure_server_async(hwnd: HWND, open_gui: bool) {
    let hwnd = hwnd as isize;
    thread::spawn(move || {
        let (store, config) = store_and_config();
        let before = daemon::status(&store, &config).ok().flatten();
        if let Ok(info) = daemon::ensure_running(&store, &config) {
            if before.as_ref().map(|value| value.instance_id) != Some(info.instance_id) {
                STATE.get().unwrap().lock().unwrap().owned_instance = Some(info.instance_id);
            }
            if open_gui {
                let _ = open::that(config.gui_url());
            }
        }
        unsafe { PostMessageW(hwnd as HWND, STATE_CHANGED, 0, 0) };
    });
}

fn stop_server_async(hwnd: HWND, only_if_owned: bool, exit_after: bool) {
    let hwnd = hwnd as isize;
    thread::spawn(move || {
        let (store, config) = store_and_config();
        let owned = STATE.get().unwrap().lock().unwrap().owned_instance;
        let current = daemon::status(&store, &config).ok().flatten();
        let may_stop = !only_if_owned
            || should_stop_owned(current.as_ref().map(|info| info.instance_id), owned);
        if may_stop {
            let _ = daemon::stop(&store, &config);
        }
        if exit_after {
            unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 1, 0) };
        } else {
            STATE.get().unwrap().lock().unwrap().owned_instance = None;
            unsafe { PostMessageW(hwnd as HWND, STATE_CHANGED, 0, 0) };
        }
    });
}

fn should_stop_owned(current: Option<Uuid>, owned: Option<Uuid>) -> bool {
    current.is_some() && current == owned
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
            ensure_server_async(hwnd, true);
            0
        }
        TRAY_MESSAGE if lparam as u32 == NIN_BALLOONUSERCLICK => {
            apply_update_async();
            0
        }
        WM_COMMAND => {
            handle_command(hwnd, wparam & 0xffff);
            0
        }
        WM_TIMER if wparam == TIMER_UPDATE => {
            check_update_async(hwnd, true);
            0
        }
        UPDATE_READY => {
            show_update_notification(hwnd);
            0
        }
        UPDATE_CHECKED => {
            show_update_check_result(hwnd);
            0
        }
        STATE_CHANGED => 0,
        WM_CLOSE if wparam == 1 => {
            remove_icon(hwnd);
            PostQuitMessage(0);
            0
        }
        WM_CLOSE => {
            stop_server_async(hwnd, true, true);
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
            "Configured port is in use",
            MF_STRING | MF_DISABLED | MF_GRAYED,
        );
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    if let Some(version) = STATE.get().unwrap().lock().unwrap().update_version.clone() {
        append(
            menu,
            ID_INSTALL_UPDATE,
            &format!("Update to v{version}"),
            MF_STRING,
        );
    }
    append(menu, ID_CHECK_UPDATE, "Check for Updates", MF_STRING);
    append(
        menu,
        ID_STARTUP,
        "Launch Takokit at startup",
        MF_STRING | if startup_enabled() { MF_CHECKED } else { 0 },
    );
    append(
        menu,
        ID_ABOUT,
        concat!("About Takokit v", env!("CARGO_PKG_VERSION")),
        MF_STRING,
    );
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
        ID_OPEN_GUI => ensure_server_async(hwnd, true),
        ID_COPY_API => copy_api_url(),
        ID_START => ensure_server_async(hwnd, false),
        ID_STOP => stop_server_async(hwnd, false, false),
        ID_CHECK_UPDATE => check_update_async(hwnd, false),
        ID_INSTALL_UPDATE => apply_update_async(),
        ID_STARTUP => set_startup(!startup_enabled()),
        ID_ABOUT => {
            MessageBoxW(hwnd, wide(concat!("Takokit ", env!("CARGO_PKG_VERSION"), "\nLocal voice AI runtime\n\nGUI: browser-based\nAPI: OpenAI-compatible audio + Takokit native")).as_ptr(), wide("About Takokit").as_ptr(), MB_OK | MB_ICONINFORMATION);
        }
        ID_QUIT => stop_server_async(hwnd, true, true),
        _ => {}
    }
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
            Self::Managed => "Running",
            Self::Foreground => "Running (foreground)",
            Self::Stopped => "Stopped",
            Self::Occupied => "Unavailable",
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
    let (_, config) = store_and_config();
    match ureq::get(&format!(
        "{}/api/v1/daemon/identity",
        config.local_base_url()
    ))
    .timeout(Duration::from_millis(350))
    .call()
    {
        Ok(response) => {
            let value: serde_json::Value = response.into_json().unwrap_or_default();
            match value
                .pointer("/identity/mode")
                .and_then(|value| value.as_str())
            {
                Some("managed") => ServerState::Managed,
                _ => ServerState::Foreground,
            }
        }
        Err(ureq::Error::Transport(error)) if error.kind() == ureq::ErrorKind::ConnectionFailed => {
            ServerState::Stopped
        }
        Err(_) => ServerState::Occupied,
    }
}

fn copy_api_url() {
    let (_, config) = store_and_config();
    if let Ok(mut child) = hidden_command("clip.exe").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(format!("{}/v1", config.local_base_url()).as_bytes());
        }
    }
}

fn add_icon(hwnd: HWND) -> anyhow::Result<()> {
    let mut data = notification_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = TRAY_MESSAGE;
    data.hIcon = load_brand_icon();
    if data.hIcon.is_null() {
        anyhow::bail!("Takokit icon resource could not be loaded");
    }
    copy_wide(
        &mut data.szTip,
        concat!("Takokit ", env!("CARGO_PKG_VERSION")),
    );
    if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
        anyhow::bail!("could not add Takokit notification icon");
    }
    Ok(())
}

fn notification_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data
}

fn load_brand_icon() -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    let path = icon_path();
    let wide_path = wide(path.as_os_str());
    unsafe {
        let width = GetSystemMetrics(SM_CXSMICON);
        let height = GetSystemMetrics(SM_CYSMICON);
        LoadImageW(
            null_mut(),
            wide_path.as_ptr(),
            IMAGE_ICON,
            width,
            height,
            LR_LOADFROMFILE
                | if width == 0 || height == 0 {
                    LR_DEFAULTSIZE
                } else {
                    0
                },
        ) as _
    }
}

fn icon_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()?
                .parent()
                .map(|root| root.join("resources/icons/takokit.ico"))
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/favicon/favicon.ico")
        })
}

fn remove_icon(hwnd: HWND) {
    let data = notification_data(hwnd);
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.creation_flags(0x0800_0000);
    command
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
    for (destination, source) in target.iter_mut().zip(wide(value)) {
        *destination = source;
    }
}

#[cfg(test)]
mod tests;
