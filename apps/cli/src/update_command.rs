use crate::{
    args::{UpdateChannelArg, UpdateCommand, UpdateConfigureArgs, UpdateSourceArgs},
    daemon, distribution,
};
use serde::Serialize;
use std::{fs, path::PathBuf, process::Command as ProcessCommand};
use takokit_core::RuntimeConfig;
use takokit_package::try_acquire_maintenance_lock;
use takokit_release::{
    parse_manifest, parse_signature, validate_artifact, validate_manifest, DistributionMetadata,
    ReleaseArtifact, ReleaseManifest, SignatureEnvelope, UpdateDecision,
};
use takokit_store::LocalStore;

#[path = "update_support.rs"]
mod update_support;
use update_support::{
    now, print_value, read_config, read_journal, read_source, refuse_active_runtime_operations,
    sibling_signature_source, stage_artifact, update_journal_path, validate_source_location,
    write_config,
};

#[derive(Debug, Clone, Serialize)]
struct UpdateStatusReport {
    current_version: String,
    available_version: Option<String>,
    downloaded_version: Option<String>,
    channel: String,
    distribution_mode: String,
    manifest_source: Option<String>,
    automatic_checks: bool,
    automatic_download: bool,
    last_check_unix: Option<u64>,
    last_error: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
struct UpdateDownloadReport {
    downloaded: bool,
    version: String,
    path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
struct AutomaticCheckReport {
    checked: bool,
    available_version: Option<String>,
    downloaded: bool,
}

struct VerifiedUpdate {
    manifest: ReleaseManifest,
    artifact: Option<ReleaseArtifact>,
    installer: Option<ReleaseArtifact>,
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
        UpdateCommand::Configure(args) => configure(store, args, json),
        UpdateCommand::Check(source) => {
            let verified = checked(store, source)?;
            print_value(&check_report(store, &verified)?, json)
        }
        UpdateCommand::Download(source) => download(store, source, json),
        UpdateCommand::Apply(source) => apply(store, config, source, json),
        UpdateCommand::AutoCheck => automatic_check(store, json),
    }
}

pub(crate) fn maybe_start_automatic_check(store: &LocalStore) {
    let Some(metadata) = distribution::distribution_metadata() else {
        return;
    };
    if metadata.mode != "installed" {
        return;
    }
    let mut settings = read_config(store, Some(&metadata));
    let timestamp = now();
    if !settings.automatic_check_due(timestamp)
        || metadata
            .manifest_url_for_channel(&settings.channel)
            .is_none()
    {
        return;
    }
    settings.last_check_attempt_unix = Some(timestamp);
    if write_config(store, &settings).is_err() {
        return;
    }
    let store = store.clone();
    std::thread::spawn(move || {
        let _ = automatic_check(&store, false);
    });
}

fn print_status(store: &LocalStore, json: bool) -> anyhow::Result<()> {
    let metadata = distribution::distribution_metadata();
    let settings = read_config(store, metadata.as_ref());
    let report = UpdateStatusReport {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        available_version: settings.last_available_version.clone(),
        downloaded_version: settings.last_downloaded_version.clone(),
        channel: settings.channel.clone(),
        distribution_mode: metadata
            .as_ref()
            .map(|value| value.mode.clone())
            .unwrap_or_else(|| "development".to_string()),
        manifest_source: metadata
            .as_ref()
            .and_then(|value| value.manifest_url_for_channel(&settings.channel)),
        automatic_checks: settings.automatic_checks,
        automatic_download: settings.automatic_download,
        last_check_unix: settings.last_check_unix,
        last_error: settings.last_error,
        journal: read_journal(store),
    };
    print_value(&report, json)
}

fn set_channel(store: &LocalStore, channel: UpdateChannelArg, json: bool) -> anyhow::Result<()> {
    let metadata = distribution::distribution_metadata();
    let mut config = read_config(store, metadata.as_ref());
    config.channel = channel.as_str().to_string();
    config.last_check_unix = None;
    config.last_check_attempt_unix = None;
    config.last_available_version = None;
    config.last_downloaded_version = None;
    config.last_error = None;
    write_config(store, &config)?;
    print_value(&config, json)
}

fn configure(store: &LocalStore, args: UpdateConfigureArgs, json: bool) -> anyhow::Result<()> {
    if args.automatic_checks.is_none() && args.automatic_download.is_none() {
        anyhow::bail!(
            "update configure requires --automatic-checks on|off and/or --automatic-download on|off"
        );
    }
    let metadata = distribution::distribution_metadata();
    let mut config = read_config(store, metadata.as_ref());
    if let Some(value) = args.automatic_checks {
        config.automatic_checks = value.enabled();
    }
    if let Some(value) = args.automatic_download {
        config.automatic_download = value.enabled();
    }
    write_config(store, &config)?;
    print_value(&config, json)
}

fn checked(store: &LocalStore, source: UpdateSourceArgs) -> anyhow::Result<VerifiedUpdate> {
    match check_unrecorded(store, source) {
        Ok(verified) => {
            let metadata = distribution::distribution_metadata();
            let mut config = read_config(store, metadata.as_ref());
            let timestamp = now();
            config.last_check_unix = Some(timestamp);
            config.last_check_attempt_unix = Some(timestamp);
            config.last_available_version = verified
                .artifact
                .as_ref()
                .map(|_| verified.manifest.version.clone());
            config.last_error = None;
            write_config(store, &config)?;
            Ok(verified)
        }
        Err(error) => {
            let metadata = distribution::distribution_metadata();
            let mut config = read_config(store, metadata.as_ref());
            let timestamp = now();
            config.last_check_unix = Some(timestamp);
            config.last_check_attempt_unix = Some(timestamp);
            config.last_error = Some(error.to_string());
            let _ = write_config(store, &config);
            Err(error)
        }
    }
}

fn check_unrecorded(
    store: &LocalStore,
    source: UpdateSourceArgs,
) -> anyhow::Result<VerifiedUpdate> {
    let metadata = distribution::distribution_metadata();
    let config = read_config(store, metadata.as_ref());
    let manifest_source = source
        .manifest
        .clone()
        .or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.manifest_url_for_channel(&config.channel))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no update manifest is configured for channel {}; pass --manifest for a private/test channel",
                config.channel
            )
        })?;
    validate_source_location(&manifest_source)?;
    let signature_source = source
        .signature
        .clone()
        .unwrap_or_else(|| sibling_signature_source(&manifest_source));
    validate_source_location(&signature_source)?;
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
    let installer = artifact.as_ref().and_then(|_| {
        manifest
            .artifacts
            .iter()
            .find(|candidate| candidate.role == "installer")
            .cloned()
    });
    Ok(VerifiedUpdate {
        manifest,
        artifact,
        installer,
        manifest_source,
    })
}

fn check_report(
    store: &LocalStore,
    verified: &VerifiedUpdate,
) -> anyhow::Result<UpdateCheckReport> {
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

fn download(store: &LocalStore, source: UpdateSourceArgs, json: bool) -> anyhow::Result<()> {
    require_installed_distribution()?;
    let verified = checked(store, source)?;
    let path = download_verified(store, &verified)?;
    print_value(
        &UpdateDownloadReport {
            downloaded: path.is_some(),
            version: verified.manifest.version.clone(),
            path,
        },
        json,
    )
}

fn download_verified(
    store: &LocalStore,
    verified: &VerifiedUpdate,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(artifact) = verified.artifact.as_ref() else {
        return Ok(None);
    };
    let _maintenance = try_acquire_maintenance_lock(store.root())?.ok_or_else(|| {
        anyhow::anyhow!(
            "update download deferred: a model pull, runner install, or adapter install is active"
        )
    })?;
    let path = ensure_staged_artifact(store, verified, artifact)?;
    let metadata = distribution::distribution_metadata();
    let mut config = read_config(store, metadata.as_ref());
    config.last_downloaded_version = Some(verified.manifest.version.clone());
    config.last_error = None;
    write_config(store, &config)?;
    Ok(Some(path))
}

fn ensure_staged_artifact(
    store: &LocalStore,
    verified: &VerifiedUpdate,
    artifact: &ReleaseArtifact,
) -> anyhow::Result<PathBuf> {
    let staging_root = store
        .root()
        .join("runtime")
        .join("updates")
        .join(&verified.manifest.version);
    fs::create_dir_all(&staging_root)?;
    let staged_bundle = staging_root.join(&artifact.name);
    let already_valid = fs::read(&staged_bundle)
        .ok()
        .is_some_and(|bytes| validate_artifact(artifact, &bytes).is_ok());
    if !already_valid {
        stage_artifact(&verified.manifest_source, artifact, &staged_bundle)?;
    }
    Ok(staged_bundle)
}

fn automatic_check(store: &LocalStore, json: bool) -> anyhow::Result<()> {
    let metadata = distribution::distribution_metadata();
    let config = read_config(store, metadata.as_ref());
    if !config.automatic_checks {
        return print_value(
            &AutomaticCheckReport {
                checked: false,
                available_version: config.last_available_version,
                downloaded: false,
            },
            json,
        );
    }
    let verified = checked(store, UpdateSourceArgs::default())?;
    let available_version = verified
        .artifact
        .as_ref()
        .map(|_| verified.manifest.version.clone());
    let downloaded = if config.automatic_download && verified.artifact.is_some() {
        match download_verified(store, &verified) {
            Ok(path) => path.is_some(),
            Err(error) => {
                let metadata = distribution::distribution_metadata();
                let mut latest = read_config(store, metadata.as_ref());
                latest.last_error = Some(error.to_string());
                let _ = write_config(store, &latest);
                false
            }
        }
    } else {
        false
    };
    print_value(
        &AutomaticCheckReport {
            checked: true,
            available_version,
            downloaded,
        },
        json,
    )
}

fn apply(
    store: &LocalStore,
    config: &RuntimeConfig,
    source: UpdateSourceArgs,
    json: bool,
) -> anyhow::Result<()> {
    let metadata = require_installed_distribution()?;
    let verified = checked(store, source)?;
    let Some(artifact) = verified.artifact.as_ref() else {
        return print_value(
            &serde_json::json!({"updated": false, "reason": "already-current"}),
            json,
        );
    };

    let maintenance = try_acquire_maintenance_lock(store.root())?.ok_or_else(|| {
        anyhow::anyhow!(
            "update pending: a model pull, runner install, or adapter install is active; retry after it finishes"
        )
    })?;
    let staged_bundle = ensure_staged_artifact(store, &verified, artifact)?;
    let staged_installer = verified
        .installer
        .as_ref()
        .map(|installer| ensure_staged_artifact(store, &verified, installer))
        .transpose()?;
    if cfg!(windows) && staged_installer.is_none() {
        anyhow::bail!("signed Windows manifest has no installer artifact");
    }
    refuse_active_runtime_operations(store, config)?;

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
        "TakokitUpdater-{}-{}{}",
        std::process::id(),
        now(),
        std::env::consts::EXE_SUFFIX,
    ));
    fs::copy(&helper, &temporary_helper)?;
    distribution::make_executable(&temporary_helper)?;
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
    if let Some(installer) = staged_installer.as_ref() {
        command.arg("--installer").arg(installer);
    }
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
    print_value(&report, json)
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
