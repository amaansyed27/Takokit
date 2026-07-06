use std::sync::Arc;
use takokit_core::{RuntimeConfig, RuntimeStatus};
use takokit_models::{MockTextToSpeechEngine, ModelRegistry};
use takokit_store::LocalStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: RuntimeConfig,
    pub store: LocalStore,
    pub registry: Arc<ModelRegistry>,
    pub tts: Arc<MockTextToSpeechEngine>,
}

impl AppState {
    pub fn new(config: RuntimeConfig, store: LocalStore) -> Self {
        Self {
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
                .registry
                .models()
                .iter()
                .filter(|model| model.installed)
                .count(),
            voices: self.registry.voices().len(),
        }
    }
}
