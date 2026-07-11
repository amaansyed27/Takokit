use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{DaemonIdentity, DaemonMode, ProcessInfo, RuntimeConfig, RuntimeStatus};
use takokit_models::{MockTextToSpeechEngine, ModelRegistry};
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: RuntimeConfig,
    pub store: LocalStore,
    pub registry: Arc<ModelRegistry>,
    pub package_registry: Arc<PackageRegistry>,
    pub installed_registry: Arc<InstalledRegistry>,
    pub tts: Arc<MockTextToSpeechEngine>,
    pub daemon_identity: DaemonIdentity,
    pub shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub executions: Arc<Mutex<HashMap<Uuid, ProcessInfo>>>,
}
impl AppState {
    pub fn new(config: RuntimeConfig, store: LocalStore) -> Self {
        Self {
            package_registry: Arc::new(PackageRegistry::bundled()),
            installed_registry: Arc::new(InstalledRegistry::new(store.manifests_dir())),
            daemon_identity: DaemonIdentity {
                instance_id: None,
                mode: DaemonMode::Direct,
                pid: std::process::id(),
                executable: std::env::current_exe().unwrap_or_else(|_| PathBuf::new()),
                storage_root: store.root().to_path_buf(),
                host: config.host.clone(),
                port: config.port,
                started_at: now(),
                log_path: None,
            },
            config,
            store,
            registry: Arc::new(ModelRegistry::default()),
            tts: Arc::new(MockTextToSpeechEngine),
            shutdown: Arc::new(Mutex::new(None)),
            executions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn managed(mut self, identity: DaemonIdentity, shutdown: oneshot::Sender<()>) -> Self {
        self.daemon_identity = identity;
        self.shutdown = Arc::new(Mutex::new(Some(shutdown)));
        self
    }
    /// Test-only callers can supply a fixture package registry and installed
    /// registry, keeping route tests entirely local and deterministic.
    pub fn with_package_registries(
        mut self,
        package_registry: PackageRegistry,
        installed_registry: InstalledRegistry,
    ) -> Self {
        self.package_registry = Arc::new(package_registry);
        self.installed_registry = Arc::new(installed_registry);
        self
    }
    pub async fn register_execution(&self, model: String, task: &str) -> ExecutionGuard {
        let id = Uuid::new_v4();
        self.executions.lock().await.insert(
            id,
            ProcessInfo {
                execution_id: id,
                model,
                task: task.to_string(),
                started_at: now(),
                state: "running".to_string(),
            },
        );
        ExecutionGuard {
            id,
            executions: self.executions.clone(),
        }
    }
    pub fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            service: "takokit".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            server: self.config.bind_addr(),
            storage_root: self.store.root().to_path_buf(),
            installed_models: self
                .package_registry
                .models()
                .map(|models| {
                    models
                        .iter()
                        .filter(|model| self.installed_registry.is_model_installed(&model.id))
                        .count()
                })
                .unwrap_or(0),
            voices: self.registry.voices().len(),
        }
    }
}
pub struct ExecutionGuard {
    id: Uuid,
    executions: Arc<Mutex<HashMap<Uuid, ProcessInfo>>>,
}
impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        let executions = self.executions.clone();
        let id = self.id;
        tokio::spawn(async move {
            executions.lock().await.remove(&id);
        });
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
