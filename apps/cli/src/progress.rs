use crate::daemon_client::Client;
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use takokit_package::InstallProgress;

const DISPLAY_WIDTH: usize = 132;
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const REDRAW_INTERVAL: Duration = Duration::from_secs(1);
const BAR_WIDTH: usize = 22;

pub(crate) struct Activity {
    running: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
}

impl Activity {
    pub(crate) fn start(label: impl Into<String>) -> Self {
        Self::spawn_timer(label.into())
    }

    pub(crate) fn start_model_pull(
        label: impl Into<String>,
        client: Client,
        model_id: impl Into<String>,
    ) -> Self {
        if !enabled() {
            return Self::disabled();
        }

        let label = label.into();
        let model_id = model_id.into();
        let activity_started_ms = timestamp_ms();
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = thread::spawn(move || {
            let started = Instant::now();
            let mut last_redraw = Instant::now() - REDRAW_INTERVAL;
            let mut last_line = String::new();
            let mut previous_bytes = 0_u64;
            let mut previous_sample = Instant::now();
            let mut previous_stage = String::new();
            let progress_path = format!("/v1/models/{model_id}/progress");

            while worker_running.load(Ordering::Relaxed) {
                if last_redraw.elapsed() >= REDRAW_INTERVAL {
                    let snapshot = client
                        .get::<InstallProgress>(&progress_path)
                        .ok()
                        .filter(|progress| progress.started_at_ms >= activity_started_ms);
                    let now = Instant::now();
                    let line = match snapshot {
                        Some(progress) => {
                            if progress.stage != previous_stage
                                || progress.downloaded_bytes < previous_bytes
                            {
                                previous_bytes = progress.downloaded_bytes;
                                previous_sample = now;
                                previous_stage = progress.stage.clone();
                            }
                            let sample_seconds = now.duration_since(previous_sample).as_secs_f64();
                            let speed = if sample_seconds >= 0.5 {
                                progress.downloaded_bytes.saturating_sub(previous_bytes) as f64
                                    / sample_seconds
                            } else {
                                0.0
                            };
                            if sample_seconds >= 0.5 {
                                previous_bytes = progress.downloaded_bytes;
                                previous_sample = now;
                            }
                            format_progress_line(&label, &progress, speed)
                        }
                        None => format!("{label}  {}", format_duration(started.elapsed())),
                    };
                    if line != last_line {
                        draw_line(&line);
                        last_line = line;
                    }
                    last_redraw = now;
                }
                thread::sleep(POLL_INTERVAL);
            }
        });

        Self {
            running: Some(running),
            worker: Some(worker),
        }
    }

    fn spawn_timer(label: String) -> Self {
        if !enabled() {
            return Self::disabled();
        }

        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = thread::spawn(move || {
            let started = Instant::now();
            let mut displayed_second = u64::MAX;
            while worker_running.load(Ordering::Relaxed) {
                let elapsed = started.elapsed().as_secs();
                if elapsed != displayed_second {
                    draw_line(&format!("{label}  {}", format_duration(started.elapsed())));
                    displayed_second = elapsed;
                }
                thread::sleep(POLL_INTERVAL);
            }
        });

        Self {
            running: Some(running),
            worker: Some(worker),
        }
    }

    fn disabled() -> Self {
        Self {
            running: None,
            worker: None,
        }
    }

    fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.store(false, Ordering::Relaxed);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if enabled() {
            eprint!("\r{blank:<width$}\r", blank = "", width = DISPLAY_WIDTH);
            let _ = io::stderr().flush();
        }
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        self.stop();
    }
}

fn format_progress_line(label: &str, progress: &InstallProgress, speed: f64) -> String {
    let elapsed = timestamp_ms().saturating_sub(progress.started_at_ms);
    let elapsed = Duration::from_millis(elapsed.min(u64::MAX as u128) as u64);
    let stage = compact_message(&progress.message, 34);
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let downloaded = progress.downloaded_bytes.min(total);
            let ratio = downloaded as f64 / total as f64;
            let filled = ((ratio * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
            let bar = format!("{}{}", "#".repeat(filled), "-".repeat(BAR_WIDTH - filled));
            let percent = (ratio * 100.0).round() as u64;
            let eta = if speed > 1.0 && downloaded < total {
                Some(Duration::from_secs_f64((total - downloaded) as f64 / speed))
            } else {
                None
            };
            format!(
                "{label} [{bar}] {}/{} {percent:>3}%  {}/s  ETA {}  {stage}",
                format_bytes(downloaded),
                format_bytes(total),
                format_bytes(speed.max(0.0) as u64),
                eta.map(format_duration).unwrap_or_else(|| "--".to_string())
            )
        }
        None => format!(
            "{label} [size pending] {}  {}/s  {}  {stage}",
            format_bytes(progress.downloaded_bytes),
            format_bytes(speed.max(0.0) as u64),
            format_duration(elapsed)
        ),
    }
}

fn draw_line(line: &str) {
    let line = compact_message(line, DISPLAY_WIDTH);
    eprint!("\r{line:<width$}", width = DISPLAY_WIDTH);
    let _ = io::stderr().flush();
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0_usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn compact_message(value: &str, maximum: usize) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if output.chars().count() >= maximum {
            break;
        }
        output.push(if character.is_control() { ' ' } else { character });
    }
    if value.chars().count() > maximum && maximum >= 3 {
        output.truncate(output.len().saturating_sub(3));
        output.push_str("...");
    }
    output
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn enabled() -> bool {
    io::stderr().is_terminal()
        && !std::env::var("TAKOKIT_OUTPUT")
            .map(|value| value.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use takokit_package::InstallProgressState;

    #[test]
    fn known_total_line_contains_size_percent_and_eta() {
        let progress = InstallProgress {
            operation: "model-pull".into(),
            id: "qwen3-tts".into(),
            stage: "model-download".into(),
            message: "Downloading model files".into(),
            downloaded_bytes: 500,
            total_bytes: Some(1_000),
            state: InstallProgressState::Running,
            started_at_ms: timestamp_ms(),
            updated_at_ms: timestamp_ms(),
        };
        let line = format_progress_line("Pulling qwen3-tts", &progress, 100.0);
        assert!(line.contains("50%"));
        assert!(line.contains("ETA"));
        assert!(line.contains("500 B/1000 B"));
    }
}
