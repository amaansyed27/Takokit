use std::{
    env,
    io::{self, Read},
    process::{Child, Command, Output},
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
#[cfg(unix)]
const FORCE_KILL_WAIT: Duration = Duration::from_secs(2);

pub(crate) fn configure_owned_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
}

pub(crate) fn timeout_from_env(name: &str, default: Duration) -> Duration {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or(default)
}

pub(crate) fn wait_with_output_timeout(mut child: Child, timeout: Duration) -> io::Result<Output> {
    let pid = child.id();
    drop(child.stdin.take());
    let stdout = child.stdout.take().map(spawn_reader);
    let stderr = child.stderr.take().map(spawn_reader);
    let deadline = Instant::now() + timeout;

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            terminate_child_tree(&mut child, pid, Duration::from_secs(2))?;
            let _ = collect_reader(stdout);
            let _ = collect_reader(stderr);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("child process {pid} exceeded {} seconds", timeout.as_secs()),
            ));
        }
        thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
    };

    Ok(Output {
        status,
        stdout: collect_reader(stdout)?,
        stderr: collect_reader(stderr)?,
    })
}

pub(crate) fn detach_and_reap(mut child: Child) -> u32 {
    let pid = child.id();
    thread::spawn(move || {
        let _ = child.wait();
    });
    pid
}

pub(crate) fn terminate_owned_process_tree(pid: u32, grace: Duration) -> io::Result<()> {
    #[cfg(unix)]
    {
        let target = unix_signal_target(pid)?;
        send_unix_signal(target, SIGTERM)?;
        if wait_for_unix_target_exit(target, grace) {
            return Ok(());
        }
        send_unix_signal(target, SIGKILL)?;
        if wait_for_unix_target_exit(target, FORCE_KILL_WAIT) {
            return Ok(());
        }
        return Err(io::Error::other(format!(
            "process target {target} did not exit after SIGKILL"
        )));
    }

    #[cfg(windows)]
    {
        let _ = grace;
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/T", "/F"]);
        hide_windows_console(&mut command);
        let status = command.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "taskkill could not terminate process tree {pid}"
            )))
        }
    }
}

fn terminate_child_tree(child: &mut Child, pid: u32, grace: Duration) -> io::Result<()> {
    #[cfg(unix)]
    {
        let target = unix_signal_target(pid)?;
        send_unix_signal(target, SIGTERM)?;
        let deadline = Instant::now() + grace;
        loop {
            if child.try_wait()?.is_some() && !unix_target_exists(target) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(POLL_INTERVAL);
        }
        let _ = send_unix_signal(target, SIGKILL);
        if child.try_wait()?.is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
        return Ok(());
    }

    #[cfg(windows)]
    {
        if terminate_owned_process_tree(pid, grace).is_err() && child.try_wait()?.is_none() {
            child.kill()?;
        }
        if child.try_wait()?.is_none() {
            let _ = child.wait();
        }
        Ok(())
    }
}

fn spawn_reader<R>(mut reader: R) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn collect_reader(reader: Option<thread::JoinHandle<io::Result<Vec<u8>>>>) -> io::Result<Vec<u8>> {
    let Some(reader) = reader else {
        return Ok(Vec::new());
    };
    reader
        .join()
        .map_err(|_| io::Error::other("child output reader thread panicked"))?
}

#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const EPERM: i32 = 1;
#[cfg(unix)]
const ESRCH: i32 = 3;

#[cfg(unix)]
unsafe extern "C" {
    fn getpgid(pid: i32) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
fn unix_signal_target(pid: u32) -> io::Result<i32> {
    let pid = i32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "PID exceeds i32"))?;
    let process_group = unsafe { getpgid(pid) };
    if process_group < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(if process_group == pid { -pid } else { pid })
}

#[cfg(unix)]
fn send_unix_signal(target: i32, signal: i32) -> io::Result<()> {
    if unsafe { kill(target, signal) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(unix)]
fn unix_target_exists(target: i32) -> bool {
    if unsafe { kill(target, 0) } == 0 {
        return true;
    }
    io::Error::last_os_error().raw_os_error() == Some(EPERM)
}

#[cfg(unix)]
fn wait_for_unix_target_exit(target: i32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !unix_target_exists(target) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(windows)]
fn hide_windows_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn timeout_terminates_and_reaps_owned_child() {
        let mut command = sleeper_command();
        configure_owned_process_group(&mut command);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = command.spawn().expect("spawn sleeper");
        let started = Instant::now();
        let error = wait_with_output_timeout(child, Duration::from_millis(100))
            .expect_err("sleeper must time out");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(unix)]
    fn sleeper_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10"]);
        command
    }

    #[cfg(windows)]
    fn sleeper_command() -> Command {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"]);
        command
    }
}
