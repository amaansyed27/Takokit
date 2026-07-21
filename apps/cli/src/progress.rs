use std::{
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const DISPLAY_WIDTH: usize = 96;
const POLL_INTERVAL: Duration = Duration::from_millis(200);

pub(crate) struct Activity {
    running: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
}

impl Activity {
    pub(crate) fn start(label: impl Into<String>) -> Self {
        if !enabled() {
            return Self {
                running: None,
                worker: None,
            };
        }

        let label = label.into();
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = thread::spawn(move || {
            let started = Instant::now();
            let mut displayed_second = u64::MAX;
            while worker_running.load(Ordering::Relaxed) {
                let elapsed = started.elapsed().as_secs();
                if elapsed != displayed_second {
                    let line = format!("{label}  {elapsed}s");
                    eprint!("\r{line:<width$}", width = DISPLAY_WIDTH);
                    let _ = io::stderr().flush();
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

fn enabled() -> bool {
    io::stderr().is_terminal()
        && !std::env::var("TAKOKIT_OUTPUT")
            .map(|value| value.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
}
