use std::path::{Path, PathBuf};
use takokit_core::{TakokitError, TakokitResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".takokit")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn voices_dir(&self) -> PathBuf {
        self.root.join("voices")
    }

    pub fn datasets_dir(&self) -> PathBuf {
        self.root.join("datasets")
    }

    pub fn outputs_dir(&self) -> PathBuf {
        self.root.join("outputs")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn ensure_layout(&self) -> TakokitResult<()> {
        for path in [
            self.models_dir(),
            self.voices_dir(),
            self.datasets_dir(),
            self.outputs_dir(),
            self.cache_dir(),
            self.logs_dir(),
        ] {
            std::fs::create_dir_all(path)
                .map_err(|error| TakokitError::Storage(error.to_string()))?;
        }

        if !self.config_path().exists() {
            std::fs::write(self.config_path(), "host = \"127.0.0.1\"\nport = 5050\n")
                .map_err(|error| TakokitError::Storage(error.to_string()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_layout_creates_expected_directories() {
        let root = std::env::temp_dir().join("takokit-store-test");
        let store = LocalStore::new(root.clone());

        store.ensure_layout().expect("layout");

        assert!(root.join("models").is_dir());
        assert!(root.join("voices").is_dir());
        assert!(root.join("datasets").is_dir());
        assert!(root.join("outputs").is_dir());
        assert!(root.join("cache").is_dir());
        assert!(root.join("logs").is_dir());
        assert!(root.join("config.toml").is_file());

        let _ = std::fs::remove_dir_all(root);
    }
}
