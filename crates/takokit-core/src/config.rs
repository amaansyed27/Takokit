use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
    pub storage_root: PathBuf,
}

impl RuntimeConfig {
    pub fn local(storage_root: PathBuf) -> Self {
        Self {
            host: std::env::var("TAKOKIT_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            port: std::env::var("TAKOKIT_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5050),
            storage_root,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn local_base_url(&self) -> String {
        format!("http://{}", self.bind_addr())
    }

    pub fn gui_url(&self) -> String {
        format!("{}/gui", self.local_base_url())
    }
}
