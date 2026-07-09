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

    pub fn runners_dir(&self) -> PathBuf {
        self.root.join("runners")
    }

    pub fn python_managed_runner_dir(&self) -> PathBuf {
        self.runners_dir().join("python-managed")
    }

    pub fn python_managed_runtime_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("runtime")
    }

    pub fn python_managed_env_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("env")
    }

    pub fn python_managed_packages_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("packages")
    }

    pub fn python_managed_wheels_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("wheels")
    }

    pub fn python_managed_logs_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("logs")
    }

    pub fn python_managed_manifests_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("manifests")
    }

    pub fn python_managed_cache_dir(&self) -> PathBuf {
        self.python_managed_runner_dir().join("cache")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.root.join("blobs")
    }

    pub fn sha256_blobs_dir(&self) -> PathBuf {
        self.blobs_dir().join("sha256")
    }

    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("manifests")
    }

    pub fn model_manifests_dir(&self) -> PathBuf {
        self.manifests_dir().join("models")
    }

    pub fn runner_manifests_dir(&self) -> PathBuf {
        self.manifests_dir().join("runners")
    }

    pub fn installed_model_records_dir(&self) -> PathBuf {
        self.manifests_dir().join("installed-models")
    }

    pub fn installed_runner_records_dir(&self) -> PathBuf {
        self.manifests_dir().join("installed-runners")
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

    pub fn downloads_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("downloads")
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
            self.runners_dir(),
            self.python_managed_runner_dir(),
            self.python_managed_runtime_dir(),
            self.python_managed_env_dir(),
            self.python_managed_packages_dir(),
            self.python_managed_wheels_dir(),
            self.python_managed_logs_dir(),
            self.python_managed_manifests_dir(),
            self.python_managed_cache_dir(),
            self.blobs_dir(),
            self.sha256_blobs_dir(),
            self.manifests_dir(),
            self.model_manifests_dir(),
            self.runner_manifests_dir(),
            self.installed_model_records_dir(),
            self.installed_runner_records_dir(),
            self.voices_dir(),
            self.datasets_dir(),
            self.outputs_dir(),
            self.cache_dir(),
            self.downloads_cache_dir(),
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
        assert!(root.join("runners").is_dir());
        assert!(root.join("blobs").is_dir());
        assert!(root.join("blobs").join("sha256").is_dir());
        assert!(root.join("manifests").is_dir());
        assert!(root.join("manifests").join("models").is_dir());
        assert!(root.join("manifests").join("runners").is_dir());
        assert!(root.join("manifests").join("installed-models").is_dir());
        assert!(root.join("manifests").join("installed-runners").is_dir());
        assert!(root.join("voices").is_dir());
        assert!(root.join("datasets").is_dir());
        assert!(root.join("outputs").is_dir());
        assert!(root.join("cache").is_dir());
        assert!(root.join("cache").join("downloads").is_dir());
        assert!(root.join("logs").is_dir());
        assert!(root.join("config.toml").is_file());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_layout_creates_python_managed_runner_directories() {
        let root = std::env::temp_dir().join("takokit-store-python-managed-test");
        let store = LocalStore::new(root.clone());

        store.ensure_layout().expect("layout");

        let runner_root = root.join("runners").join("python-managed");
        for child in [
            "runtime",
            "env",
            "packages",
            "wheels",
            "logs",
            "manifests",
            "cache",
        ] {
            assert!(runner_root.join(child).is_dir(), "missing {child}");
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
