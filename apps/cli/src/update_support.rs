use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use takokit_core::RuntimeConfig;
use takokit_release::{validate_artifact, DistributionMetadata, ReleaseArtifact};
use takokit_store::LocalStore;

pub(super) const AUTO_CHECK_INTERVAL_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct UpdateConfig {
    #[serde(default = "default_channel")]
    pub(super) channel: String,
    #[serde(default = "default_automatic_checks")]
    pub(super) automatic_checks: bool,
    #[serde(default)]
    pub(super) automatic_download: bool,
    #[serde(default)]
    pub(super) last_check_unix: Option<u64>,
    #[serde(default)]
    pub(super) last_check_attempt_unix: Option<u64>,
    #[serde(default)]
    pub(super) last_available_version: Option<String>,
    #[serde(default)]
    pub(super) last_downloaded_version: Option<String>,
    #[serde(default)]
    pub(super) last_error: Option<String>,
}

impl UpdateConfig {
    pub(super) fn new(channel: String) -> Self {
        Self {
            channel,
            automatic_checks: true,
            automatic_download: false,
            last_check_unix: None,
            last_check_attempt_unix: None,
            last_available_version: None,
            last_downloaded_version: None,
            last_error: None,
        }
    }

    pub(super) fn automatic_check_due(&self, timestamp: u64) -> bool {
        if !self.automatic_checks {
            return false;
        }
        self.last_check_attempt_unix
            .or(self.last_check_unix)
            .is_none_or(|previous| {
                timestamp.saturating_sub(previous) >= AUTO_CHECK_INTERVAL_SECONDS
            })
    }
}

fn default_channel() -> String {
    "stable".to_string()
}

fn default_automatic_checks() -> bool {
    true
}

pub(super) fn refuse_active_runtime_operations(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<()> {
    if let Ok(response) = ureq::get(&format!("{}/api/v1/ps", config.local_base_url()))
        .timeout(Duration::from_millis(500))
        .call()
    {
        if let Ok(value) = response.into_json::<serde_json::Value>() {
            if value.as_array().is_some_and(|items| !items.is_empty()) {
                anyhow::bail!(
                    "update pending: active inference/conversion work is running; retry after it finishes"
                );
            }
        }
    }
    if contains_active_rvc_job(&store.voices_dir()) {
        anyhow::bail!(
            "update pending: an RVC preparation/training job is active; retry after it finishes"
        );
    }
    Ok(())
}

fn contains_active_rvc_job(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return false;
    }
    if metadata.is_file() {
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return false;
        }
        let Ok(text) = fs::read_to_string(path) else {
            return false;
        };
        let compact = text.replace(char::is_whitespace, "").to_ascii_lowercase();
        return [
            "\"state\":\"running\"",
            "\"state\":\"preparing\"",
            "\"state\":\"training\"",
            "\"status\":\"running\"",
            "\"status\":\"preparing\"",
            "\"status\":\"training\"",
        ]
        .iter()
        .any(|needle| compact.contains(needle));
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| contains_active_rvc_job(&entry.path()))
}

pub(super) fn stage_artifact(
    manifest_source: &str,
    artifact: &ReleaseArtifact,
    destination: &Path,
) -> anyhow::Result<()> {
    if !takokit_release::safe_artifact_name(&artifact.name) {
        anyhow::bail!(
            "signed manifest contains unsafe artifact name {}",
            artifact.name
        );
    }
    let source = artifact
        .url
        .clone()
        .or_else(|| local_sibling_artifact(manifest_source, &artifact.name))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "artifact {} has no download URL and the manifest is not a local file",
                artifact.name
            )
        })?;
    validate_source_location(&source)?;
    let bytes = read_source(&source, usize::MAX)?;
    validate_artifact(artifact, &bytes)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, &bytes)?;
    let _ = fs::remove_file(destination);
    fs::rename(temporary, destination)?;
    Ok(())
}

fn local_sibling_artifact(manifest_source: &str, name: &str) -> Option<String> {
    if is_remote_source(manifest_source) {
        return None;
    }
    let manifest = PathBuf::from(manifest_source);
    Some(manifest.parent()?.join(name).to_string_lossy().into_owned())
}

pub(super) fn sibling_signature_source(manifest_source: &str) -> String {
    if is_remote_source(manifest_source) {
        if let Some(prefix) = manifest_source.strip_suffix("release-manifest.json") {
            return format!("{prefix}release-manifest.sig");
        }
        return format!("{manifest_source}.sig");
    }
    PathBuf::from(manifest_source)
        .with_file_name("release-manifest.sig")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn validate_source_location(source: &str) -> anyhow::Result<()> {
    if source.starts_with("https://") {
        return Ok(());
    }
    if source.contains("://") {
        anyhow::bail!(
            "untrusted update source scheme; remote update metadata and artifacts require HTTPS"
        );
    }
    let path = Path::new(source);
    if path.as_os_str().is_empty() {
        anyhow::bail!("update source path is empty");
    }
    Ok(())
}

pub(super) fn read_source(source: &str, maximum: usize) -> anyhow::Result<Vec<u8>> {
    validate_source_location(source)?;
    if is_remote_source(source) {
        let response = ureq::get(source).timeout(Duration::from_secs(5)).call()?;
        let reader = response.into_reader();
        let mut bytes = Vec::new();
        let limit = maximum.saturating_add(1) as u64;
        reader.take(limit).read_to_end(&mut bytes)?;
        if maximum != usize::MAX && bytes.len() > maximum {
            anyhow::bail!("update metadata at {source} exceeded the safety limit");
        }
        Ok(bytes)
    } else {
        let bytes = fs::read(source)?;
        if maximum != usize::MAX && bytes.len() > maximum {
            anyhow::bail!("update metadata at {source} exceeded the safety limit");
        }
        Ok(bytes)
    }
}

fn is_remote_source(source: &str) -> bool {
    source.starts_with("https://")
}

pub(super) fn read_config(
    store: &LocalStore,
    metadata: Option<&DistributionMetadata>,
) -> UpdateConfig {
    let fallback_channel = metadata
        .map(|value| value.default_channel.clone())
        .filter(|value| matches!(value.as_str(), "stable" | "preview"))
        .unwrap_or_else(default_channel);
    let mut config = fs::read(config_path(store))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<UpdateConfig>(&bytes).ok())
        .unwrap_or_else(|| UpdateConfig::new(fallback_channel));
    if !matches!(config.channel.as_str(), "stable" | "preview") {
        config.channel = "stable".to_string();
    }
    config
}

pub(super) fn write_config(store: &LocalStore, config: &UpdateConfig) -> anyhow::Result<()> {
    write_json_atomic(&config_path(store), config)
}

pub(super) fn config_path(store: &LocalStore) -> PathBuf {
    store.root().join("runtime").join("update-config.json")
}

pub(super) fn update_journal_path(store: &LocalStore) -> PathBuf {
    store.root().join("runtime").join("update-journal.json")
}

pub(super) fn read_journal(store: &LocalStore) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(update_journal_path(store)).ok()?).ok()
}

pub(super) fn write_json_atomic(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    let _ = fs::remove_file(path);
    fs::rename(temporary, path)?;
    Ok(())
}

pub(super) fn print_value(value: &impl Serialize, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        let value = serde_json::to_value(value)?;
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                println!("{key}: {}", display_json(value));
            }
        } else {
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn display_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_manifest_resolves_sibling_signature_and_artifact() {
        let manifest = Path::new("C:/Temp/Takokit/release-manifest.json");
        assert!(
            sibling_signature_source(manifest.to_str().unwrap()).ends_with("release-manifest.sig")
        );
        assert!(
            local_sibling_artifact(manifest.to_str().unwrap(), "update.zip")
                .unwrap()
                .ends_with("update.zip")
        );
    }

    #[test]
    fn active_job_detection_is_state_specific() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("done.json"), r#"{"status":"completed"}"#).unwrap();
        assert!(!contains_active_rvc_job(temp.path()));
        fs::write(temp.path().join("live.json"), r#"{"status":"training"}"#).unwrap();
        assert!(contains_active_rvc_job(temp.path()));
    }

    #[test]
    fn remote_update_sources_require_https() {
        assert!(
            validate_source_location("https://updates.example.test/release-manifest.json").is_ok()
        );
        assert!(
            validate_source_location("http://updates.example.test/release-manifest.json").is_err()
        );
        assert!(
            validate_source_location("ftp://updates.example.test/release-manifest.json").is_err()
        );
        assert!(validate_source_location(r"C:\Temp\Takokit\release-manifest.json").is_ok());
    }

    #[test]
    fn automatic_checks_default_on_but_background_download_defaults_off() {
        let config: UpdateConfig = serde_json::from_str(r#"{"channel":"stable"}"#).unwrap();
        assert!(config.automatic_checks);
        assert!(!config.automatic_download);
        assert!(config.automatic_check_due(AUTO_CHECK_INTERVAL_SECONDS));
    }
}
