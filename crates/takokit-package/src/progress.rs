//! Structured, file-backed progress snapshots for long-running installs.

use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MONITOR_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallProgressState {
    Running,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallProgress {
    pub operation: String,
    pub id: String,
    pub stage: String,
    pub message: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub state: InstallProgressState,
    pub started_at_ms: u128,
    pub updated_at_ms: u128,
}

#[derive(Clone)]
pub struct InstallProgressReporter {
    path: Arc<PathBuf>,
    snapshot: Arc<Mutex<InstallProgress>>,
    persist_lock: Arc<Mutex<()>>,
}

impl InstallProgressReporter {
    pub fn model(takokit_root: &Path, model_id: &str) -> Self {
        let now = timestamp_ms();
        let reporter = Self {
            path: Arc::new(model_progress_path(takokit_root, model_id)),
            snapshot: Arc::new(Mutex::new(InstallProgress {
                operation: "model-pull".to_string(),
                id: model_id.to_string(),
                stage: "starting".to_string(),
                message: "Preparing model pull".to_string(),
                downloaded_bytes: 0,
                total_bytes: None,
                state: InstallProgressState::Running,
                started_at_ms: now,
                updated_at_ms: now,
            })),
            persist_lock: Arc::new(Mutex::new(())),
        };
        reporter.persist();
        reporter
    }

    pub fn update(
        &self,
        stage: impl Into<String>,
        message: impl Into<String>,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    ) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.stage = stage.into();
            snapshot.message = message.into();
            snapshot.downloaded_bytes = total_bytes
                .map(|total| downloaded_bytes.min(total))
                .unwrap_or(downloaded_bytes);
            snapshot.total_bytes = total_bytes;
            snapshot.state = InstallProgressState::Running;
            snapshot.updated_at_ms = timestamp_ms();
        }
        self.persist();
    }

    pub fn complete(&self, message: impl Into<String>) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.stage = "complete".to_string();
            snapshot.message = message.into();
            if let Some(total) = snapshot.total_bytes {
                snapshot.downloaded_bytes = total;
            }
            snapshot.state = InstallProgressState::Complete;
            snapshot.updated_at_ms = timestamp_ms();
        }
        self.persist();
    }

    pub fn fail(&self, message: impl Into<String>) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.stage = "failed".to_string();
            snapshot.message = message.into();
            snapshot.state = InstallProgressState::Failed;
            snapshot.updated_at_ms = timestamp_ms();
        }
        self.persist();
    }

    fn persist(&self) {
        let _persist_guard = match self.persist_lock.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let snapshot = match self.snapshot.lock() {
            Ok(snapshot) => snapshot.clone(),
            Err(_) => return,
        };
        let Some(parent) = self.path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec(&snapshot) else {
            return;
        };
        let temporary = self
            .path
            .with_extension(format!("json.tmp-{}", std::process::id()));
        if std::fs::write(&temporary, bytes).is_err() {
            return;
        }
        if self.path.exists() {
            let _ = std::fs::remove_file(self.path.as_ref());
        }
        if std::fs::rename(&temporary, self.path.as_ref()).is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
    }
}

pub struct InstallProgressMonitor {
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl InstallProgressMonitor {
    pub fn start(
        reporter: InstallProgressReporter,
        paths: Vec<PathBuf>,
        stage: impl Into<String>,
        message: impl Into<String>,
        total_bytes: Option<u64>,
    ) -> Self {
        let stage = stage.into();
        let message = message.into();
        let baseline = paths.iter().map(|path| path_size(path)).sum::<u64>();
        reporter.update(&stage, &message, 0, total_bytes);

        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = thread::spawn(move || {
            let mut maximum = 0_u64;
            while worker_running.load(Ordering::Relaxed) {
                let current = paths
                    .iter()
                    .map(|path| path_size(path))
                    .sum::<u64>()
                    .saturating_sub(baseline);
                maximum = maximum.max(current);
                reporter.update(&stage, &message, maximum, total_bytes);
                thread::sleep(MONITOR_INTERVAL);
            }
        });

        Self {
            running,
            worker: Some(worker),
        }
    }
}

impl Drop for InstallProgressMonitor {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn model_progress_path(takokit_root: &Path, model_id: &str) -> PathBuf {
    takokit_root
        .join("progress")
        .join(format!("model-{}.json", sanitize(model_id)))
}

pub fn read_model_progress(takokit_root: &Path, model_id: &str) -> Option<InstallProgress> {
    let bytes = std::fs::read(model_progress_path(takokit_root, model_id)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| path_size(&entry.path()))
        .sum()
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => character,
            _ => '_',
        })
        .collect()
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_progress_round_trips() {
        let root = tempfile::tempdir().expect("tempdir");
        let reporter = InstallProgressReporter::model(root.path(), "qwen3-tts");
        reporter.update("download", "Downloading", 50, Some(100));
        let snapshot = read_model_progress(root.path(), "qwen3-tts").expect("progress");
        assert_eq!(snapshot.downloaded_bytes, 50);
        assert_eq!(snapshot.total_bytes, Some(100));
        assert_eq!(snapshot.stage, "download");
    }
}
