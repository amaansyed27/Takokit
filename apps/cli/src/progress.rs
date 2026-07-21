use std::{
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

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
            const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
            let started = Instant::now();
            let mut frame = 0usize;
            while worker_running.load(Ordering::Relaxed) {
                eprint!(
                    "\r\x1b[2K{} {}  {:.1}s",
                    FRAMES[frame % FRAMES.len()],
                    label,
                    started.elapsed().as_secs_f32()
                );
                let _ = io::stderr().flush();
                frame += 1;
                thread::sleep(Duration::from_millis(120));
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
            eprint!("\r\x1b[2K");
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
