use std::sync::Arc;
use takokit_core::{RuntimeConfig, RuntimeStatus};
use takokit_models::{MockTextToSpeechEngine, ModelRegistry};
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: RuntimeConfig,
    pub store: LocalStore,
    pub registry: Arc<ModelRegistry>,
    pub package_registry: Arc<PackageRegistry>,
    pub installed_registry: Arc<InstalledRegistry>,
    pub tts: Arc<MockTextToSpeechEngine>,
}

impl AppState {
    pub fn new(config: RuntimeConfig, store: LocalStore) -> Self {
        Self {
            package_registry: Arc::new(PackageRegistry::bundled()),
            installed_registry: Arc::new(InstalledRegistry::new(store.manifests_dir())),
            config,
            store,
            registry: Arc::new(ModelRegistry::default()),
            tts: Arc::new(MockTextToSpeechEngine),
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
