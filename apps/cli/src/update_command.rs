use crate::{
    args::{UpdateChannelArg, UpdateCommand, UpdateSourceArgs},
    daemon, distribution,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use takokit_core::RuntimeConfig;
use takokit_package::try_acquire_maintenance_lock;
use takokit_release::{
    parse_manifest, parse_signature, validate_artifact, validate_manifest, DistributionMetadata,
    ReleaseArtifact, ReleaseManifest, SignatureEnvelope, UpdateDecision,
};
use takokit_store::LocalStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateConfig {
    channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateStatusReport {
    current_version: String,
    channel: String,
    distribution_mode: String,
    manifest_source: Option<String>,
    journal: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateCheckReport {
    current_version: String,
    offered_version: String,
    channel: String,
    available: bool,
    manifest_source: String,
    signing_key_id: String,
    artifact: Option<String>,
    test_fixture: bool,
}

struct VerifiedUpdate {
    manifest: ReleaseManifest,
    artifact: Option<ReleaseArtifact>,
    manifest_source: String,
}

pub(crate) fn run_update_command(
    store: &LocalStore,
    config: &RuntimeConfig,
    command: UpdateCommand,
    json: bool,
) -> anyhow::Result<()> {
    match command {
        UpdateCommand::Status => print_status(store, json),
        UpdateCommand::Channel { channel } => set_channel(store, channel, json),
        UpdateCommand::Check(source) => {
            let verified = check(store, source)?;
            let report = check_report(store, &verified)?;
            print_value(&report, json)?;
            Ok(())
        }
        UpdateCommand::Apply(source) => apply(store, config, source, json),
    }
}

fn print_status(store: &LocalStore, json: bool) -> anyhow::Result<()> {
    let metadata = distribution::distribution_metadata();
    let settings = read_config(store, metadata.as_ref());
    let report = UpdateStatusReport {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        channel: settings.channel,
        distribution_mode: metadata
            .as_ref()
            .map(|value| value.mode.clone())
            .unwrap_or_else(|| "development".to_string()),
        manifest_source: metadata.and_then(|value| value.update_manifest_url),
        journal: read_journal(store),
    };
    print_value(&report, json)
}

fn set_channel(store: &LocalStore, channel: UpdateChannelArg, json: bool) -> anyhow::Result<()> {
    let config = UpdateConfig {
        channel: channel.as_str().to_string(),
    };
    write_json_atomic(&config_path(store), &config)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("Takokit update channel: {}", config.channel);
    }
    Ok(())
}

fn check(store: &LocalStore, source: UpdateSourceArgs) -> anyhow::Result<VerifiedUpdate> {
    let metadata = distribution::distribution_metadata();
    let config = read_config(store, metadata.as_ref());
    let manifest_source = source
        .manifest
        .clone()
        .or_else(|| metadata.as_ref().and_then(|value| value.update_manifest_url.clone()))
        .ok_or_else(|| anyhow::anyhow!(
            "no update manifest is configured for this distribution; pass --manifest for a private/test channel"
        ))?;
    let signature_source = source
        .signature
        .clone()
        .unwrap_or_else(|| sibling_signature_source(&manifest_source));
    let manifest_bytes = read_source(&manifest_source, 4 * 1024 * 1024)?;
    let signature_bytes = read_source(&signature_source, 64 * 1024)?;
    let manifest = parse_manifest(&manifest_bytes)?;
    let signature: SignatureEnvelope = parse_signature(&signature_bytes)?;
    takokit_release::verify_signature(&manifest_bytes, &signature, source.allow_test)?;
    if signature.key_id != manifest.signing_key_id {
        anyhow::bail!(
            "release manifest key id {} does not match detached signature key id {}",
            manifest.signing_key_id,
            signature.key_id
        );
    }
    let decision = validate_manifest(
        &manifest,
        env!("CARGO_PKG_VERSION"),
        &config.channel,
        source.allow_test,
    )?;
    let artifact = match decision {
        UpdateDecision::Current => None,
        UpdateDecision::UpdateAvailable { .. } => Some(
            manifest
                .artifacts
                .iter()
                .find(|artifact| artifact.role == "update_bundle")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("signed manifest has no update_bundle artifact"))?,
        ),
    };
    Ok(VerifiedUpdate {
        manifest,
        artifact,
        manifest_source,
    })
}

fn check_report(store: &LocalStore, verified: &VerifiedUpdate) -> anyhow::Result<UpdateCheckReport> {
    let metadata = distribution::distribution_metadata();
    let config = read_config(store, metadata.as_ref());
    Ok(UpdateCheckReport {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        offered_version: verified.manifest.version.clone(),
        channel: config.channel,
        available: verified.artifact.is_some(),
        manifest_source: verified.manifest_source.clone(),
        signing_key_id: verified.manifest.signing_key_id.clone(),
        artifact: verified.artifact.as_ref().map(|value| value.name.clone()),
        test_fixture: verified.manifest.test_fixture,
    })
}

fn apply(
    store: &LocalStore,
    config: &RuntimeConfig,
    source: UpdateSourceArgs,
    json: bool,
) -> anyhow::Result<()> {
    let metadata = require_installed_distribution()?;
    let verified = check(store, source)?;
    let Some(artifact) = verified.artifact.as_ref() else {
        if json {
            println!(
                "{}",
                serde_json::json!({"updated": false, "reason": "already-current"})
            );
        } else {
            println!("Takokit {} is already current.", env!("CARGO_PKG_VERSION"));
        }
        return Ok(());
    };

    let maintenance = try_acquire_maintenance_lock(store.root())?
        .ok_or_else(|| anyhow::anyhow!(
            "update pending: a model pull, runner install, or adapter install is active; retry after it finishes"
        ))?;
    refuse_active_runtime_operations(store, config)?;

    let staging_root = store
        .root()
        .join("runtime")
        .join("updates")
        .join(&verified.manifest.version);
    fs::create_dir_all(&staging_root)?;
    let staged_bundle = staging_root.join(&artifact.name);
    stage_artifact(
        &verified.manifest_source,
        artifact,
        &staged_bundle,
    )?;

    let daemon_was_running = daemon::status(store, config)?.is_some();
    if daemon_was_running {
        daemon::stop(store, config)?;
    }
    drop(maintenance);

    let install_root = distribution::application_root()
        .ok_or_else(|| anyhow::anyhow!("could not resolve Takokit installation root"))?;
    let helper = distribution::updater_executable()
        .ok_or_else(|| anyhow::anyhow!("installed updater helper is missing"))?;
    let temporary_helper = std::env::temp_dir().join(format!(
        "TakokitUpdater-{}-{}.exe",
        std::process::id(),
        now()
    ));
    fs::copy(&helper, &temporary_helper)?;
    let journal = update_journal_path(store);
    let mut command = ProcessCommand::new(&temporary_helper);
    command
        .arg("--parent-pid")
        .arg(std::process::id().to_string())
        .arg("--install-root")
        .arg(&install_root)
        .arg("--bundle")
        .arg(&staged_bundle)
        .arg("--expected-version")
        .arg(&verified.manifest.version)
        .arg("--journal")
        .arg(&journal);
    if daemon_was_running {
        command.arg("--restart-daemon").arg("true");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command.spawn()?;

    let report = serde_json::json!({
        "updated": true,
        "state": "staged",
        "from": env!("CARGO_PKG_VERSION"),
        "to": verified.manifest.version,
        "helper": temporary_helper,
        "journal": journal,
        "restart_daemon": daemon_was_running,
        "distribution_mode": metadata.mode,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Verified Takokit {} and staged the update. The external updater will replace the installed application after this process exits.",
            verified.manifest.version
        );
        println!("Update journal: {}", update_journal_path(store).display());
    }
    Ok(())
}

fn require_installed_distribution() -> anyhow::Result<DistributionMetadata> {
    let metadata = distribution::distribution_metadata().ok_or_else(|| {
        anyhow::anyhow!("self-update is disabled for development/repository builds")
    })?;
    if metadata.mode != "installed" {
        anyhow::bail!(
            "self-update is disabled for {} distributions; use the matching distribution workflow instead",
            metadata.mode
        );
    }
    Ok(metadata)
}

fn refuse_active_runtime_operations(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    if let Ok(response) = ureq::get(&format!("{}/v1/ps", config.local_base_url()))
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

fn stage_artifact(
    manifest_source: &str,
    artifact: &ReleaseArtifact,
    destination: &Path,
) -> anyhow::Result<()> {
    if !takokit_release::safe_artifact_name(&artifact.name) {
        anyhow::bail!("signed manifest contains unsafe artifact name {}", artifact.name);
    }
    let source = artifact
        .url
        .clone()
        .or_else(|| local_sibling_artifact(manifest_source, &artifact.name))
        .ok_or_else(|| anyhow::anyhow!(
            "artifact {} has no download URL and the manifest is not a local file",
            artifact.name
        ))?;
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
    if is_http(manifest_source) {
        return None;
    }
    let manifest = PathBuf::from(manifest_source);
    Some(manifest.parent()?.join(name).to_string_lossy().into_owned())
}

fn sibling_signature_source(manifest_source: &str) -> String {
    if is_http(manifest_source) {
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

fn read_source(source: &str, maximum: usize) -> anyhow::Result<Vec<u8>> {
    if is_http(source) {
        let response = ureq::get(source)
            .timeout(Duration::from_secs(30))
            .call()?;
        let mut reader = response.into_reader();
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

fn is_http(source: &str) -> bool {
    source.starts_with("https://") || source.starts_with("http://")
}

fn read_config(store: &LocalStore, metadata: Option<&DistributionMetadata>) -> UpdateConfig {
    fs::read(config_path(store))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| UpdateConfig {
            channel: metadata
                .map(|value| value.default_channel.clone())
                .filter(|value| matches!(value.as_str(), "stable" | "preview"))
                .unwrap_or_else(|| "stable".to_string()),
        })
}

fn config_path(store: &LocalStore) -> PathBuf {
    store.root().join("runtime").join("update-config.json")
}

fn update_journal_path(store: &LocalStore) -> PathBuf {
    store.root().join("runtime").join("update-journal.json")
}

fn read_journal(store: &LocalStore) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(update_journal_path(store)).ok()?).ok()
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
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

fn print_value(value: &impl Serialize, json: bool) -> anyhow::Result<()> {
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

fn now() -> u64 {
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
        assert!(sibling_signature_source(manifest.to_str().unwrap()).ends_with("release-manifest.sig"));
        assert!(local_sibling_artifact(manifest.to_str().unwrap(), "update.zip")
            .unwrap()
            .ends_with("update.zip"));
    }

    #[test]
    fn active_job_detection_is_state_specific() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("done.json"), r#"{"status":"completed"}"#).unwrap();
        assert!(!contains_active_rvc_job(temp.path()));
        fs::write(temp.path().join("live.json"), r#"{"status":"training"}"#).unwrap();
        assert!(contains_active_rvc_job(temp.path()));
    }
}
