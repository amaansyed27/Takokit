use crate::{
    args::{UpdateChannelArg, UpdateCommand, UpdateSourceArgs},
    daemon, distribution,
};
use serde::Serialize;
use std::{fs, process::Command as ProcessCommand};
use takokit_core::RuntimeConfig;
use takokit_package::try_acquire_maintenance_lock;
use takokit_release::{
    parse_manifest, parse_signature, validate_manifest, DistributionMetadata, ReleaseArtifact,
    ReleaseManifest, SignatureEnvelope, UpdateDecision,
};
use takokit_store::LocalStore;

#[path = "update_support.rs"]
mod update_support;
use update_support::{
    config_path, now, print_value, read_config, read_journal, read_source,
    refuse_active_runtime_operations, sibling_signature_source, stage_artifact, update_journal_path,
    validate_source_location, write_json_atomic, UpdateConfig,
};

#[derive(Debug, Clone, Serialize)]
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

fn set_channel(
    store: &LocalStore,
    channel: UpdateChannelArg,
    json: bool,
) -> anyhow::Result<()> {
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
        .or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.update_manifest_url.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no update manifest is configured for this distribution; pass --manifest for a private/test channel"
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
                .ok_or_else(|| {
                    anyhow::anyhow!("signed manifest has no update_bundle artifact")
                })?,
        ),
    };
    Ok(VerifiedUpdate {
        manifest,
        artifact,
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

    let maintenance = try_acquire_maintenance_lock(store.root())?.ok_or_else(|| {
        anyhow::anyhow!(
            "update pending: a model pull, runner install, or adapter install is active; retry after it finishes"
        )
    })?;
    refuse_active_runtime_operations(store, config)?;

    let staging_root = store
        .root()
        .join("runtime")
        .join("updates")
        .join(&verified.manifest.version);
    fs::create_dir_all(&staging_root)?;
    let staged_bundle = staging_root.join(&artifact.name);
    stage_artifact(&verified.manifest_source, artifact, &staged_bundle)?;

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
