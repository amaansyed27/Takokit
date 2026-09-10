use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 5050;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
    pub storage_root: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StoredRuntimeConfig {
    host: Option<String>,
    port: Option<u16>,
}

impl RuntimeConfig {
    pub fn local(storage_root: PathBuf) -> Self {
        let stored = load_stored_runtime_config(&storage_root).unwrap_or_default();
        let host = resolve_host(std::env::var("TAKOKIT_HOST").ok(), stored.host);
        let port = resolve_port(std::env::var("TAKOKIT_PORT").ok(), stored.port);
        Self {
            host,
            port,
            storage_root,
        }
    }

    pub fn bind_addr(&self) -> String {
        format_host_port(&self.host, self.port)
    }

    pub fn local_base_url(&self) -> String {
        format!("http://{}", self.bind_addr())
    }

    pub fn gui_url(&self) -> String {
        format!("{}/gui", self.local_base_url())
    }
}

fn load_stored_runtime_config(storage_root: &Path) -> Option<StoredRuntimeConfig> {
    let source = std::fs::read_to_string(storage_root.join("config.toml")).ok()?;
    Some(parse_stored_runtime_config(&source))
}

fn parse_stored_runtime_config(source: &str) -> StoredRuntimeConfig {
    let mut config = StoredRuntimeConfig::default();
    for raw_line in source.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = raw_value.trim();
        match key {
            "host" => {
                if let Some(host) = parse_quoted_string(value).and_then(nonempty) {
                    config.host = Some(host);
                }
            }
            "port" => {
                if let Ok(port) = value.parse::<u16>() {
                    config.port = Some(port);
                }
            }
            _ => {}
        }
    }
    config
}

fn parse_quoted_string(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let quote = bytes[0];
    if !matches!(quote, b'\'' | b'"') || bytes[bytes.len() - 1] != quote {
        return None;
    }
    Some(value[1..value.len() - 1].to_string())
}

fn resolve_host(environment: Option<String>, stored: Option<String>) -> String {
    environment
        .and_then(nonempty)
        .or_else(|| stored.and_then(nonempty))
        .unwrap_or_else(|| DEFAULT_HOST.to_string())
}

fn resolve_port(environment: Option<String>, stored: Option<u16>) -> u16 {
    environment
        .and_then(|value| value.trim().parse::<u16>().ok())
        .or(stored)
        .unwrap_or(DEFAULT_PORT)
}

fn nonempty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn format_host_port(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn stored_runtime_config_is_loaded() {
        let root = std::env::temp_dir().join(format!("takokit-config-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("config.toml"),
            "host = \"localhost\"\nport = 6060\n",
        )
        .unwrap();

        let stored = load_stored_runtime_config(&root).unwrap();
        assert_eq!(stored.host.as_deref(), Some("localhost"));
        assert_eq!(stored.port, Some(6060));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn config_parser_ignores_unknown_and_invalid_values() {
        let stored = parse_stored_runtime_config(
            "unknown = true\nhost = localhost\nport = nope\nhost = '::1' # local\nport = 6060\n",
        );
        assert_eq!(stored.host.as_deref(), Some("::1"));
        assert_eq!(stored.port, Some(6060));
    }

    #[test]
    fn environment_values_override_stored_values() {
        assert_eq!(
            resolve_host(Some(" 127.0.0.2 ".into()), Some("localhost".into())),
            "127.0.0.2"
        );
        assert_eq!(resolve_port(Some("7070".into()), Some(6060)), 7070);
    }

    #[test]
    fn blank_or_invalid_environment_values_fall_back_safely() {
        assert_eq!(
            resolve_host(Some("   ".into()), Some("localhost".into())),
            "localhost"
        );
        assert_eq!(resolve_port(Some("not-a-port".into()), Some(6060)), 6060);
    }

    #[test]
    fn ipv6_bind_addresses_are_bracketed() {
        let config = RuntimeConfig {
            host: "::1".into(),
            port: 5050,
            storage_root: PathBuf::from("."),
        };
        assert_eq!(config.bind_addr(), "[::1]:5050");
        assert_eq!(config.local_base_url(), "http://[::1]:5050");
    }
}
