//! Storage inspection and safe cache cleanup for the Takokit home directory.

use crate::args::{StorageArgs, StorageCommand, StorageScope};
use serde::Serialize;
use std::{
    io::{self, Write},
    path::Path,
};
use takokit_package::{
    acquire_maintenance_lock, automatic_cleanup_state, clean_provider_storage, clean_uv_cache,
    migrate_legacy_provider_cache, provider_ownership_status, ProviderCleanupReport,
    ProviderOwnershipStatus,
};

#[path = "storage_cache.rs"]
mod storage_cache;
#[path = "storage_recovery.rs"]
mod storage_recovery;
#[path = "storage_report.rs"]
mod storage_report;

use storage_recovery::{begin_provider_cleanup, finish_provider_cleanup};
use storage_report::{format_bytes, print_storage_report};
pub(crate) use storage_report::{inspect_storage, StorageReport};

#[derive(Debug, Serialize)]
struct StorageEnvelope<'a> {
    storage: &'a StorageReport,
    provider_ownership: &'a ProviderOwnershipStatus,
}

pub(crate) fn run_storage_command(
    root: &Path,
    args: StorageArgs,
    json: bool,
) -> anyhow::Result<()> {
    match args.command {
        None => {
            let json_requested = args.json || json;
            if !json_requested {
                eprintln!("Scanning Takokit storage at {}...", root.display());
                io::stderr().flush()?;
            }
            let report = inspect_storage(root)?;
            let ownership = provider_ownership_status(root)?;
            if json_requested {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&StorageEnvelope {
                        storage: &report,
                        provider_ownership: &ownership,
                    })?
                );
            } else {
                print_storage_report(&report);
                print_provider_ownership(&ownership);
            }
        }
        Some(StorageCommand::Status) => {
            let state = automatic_cleanup_state(root)?;
            let ownership = provider_ownership_status(root)?;
            if args.json || json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "automatic_cleanup": state,
                        "provider_ownership": ownership,
                    }))?
                );
            } else {
                println!("Takokit automatic storage cleanup");
                println!(
                    "  enabled      {}",
                    if state.enabled { "yes" } else { "no" }
                );
                println!("  status       {}", state.status);
                println!("  reclaimed    {}", format_bytes(state.reclaimed_bytes));
                if let Some(reason) = state.skip_reason {
                    println!("  skipped      {reason}");
                }
                if let Some(error) = state.error {
                    println!("  error        {error}");
                }
                println!(
                    "  configuration TAKOKIT_AUTO_STORAGE_CLEANUP=0 disables background cleanup"
                );
                println!();
                print_provider_ownership(&ownership);
            }
        }
        Some(StorageCommand::Clean { dry_run, scope }) => {
            run_cleanup(root, scope, dry_run, args.json || json)?;
        }
    }
    Ok(())
}

fn run_cleanup(root: &Path, scope: StorageScope, dry_run: bool, json: bool) -> anyhow::Result<()> {
    match scope {
        StorageScope::Uv => {
            let report = clean_uv_cache(root, dry_run)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Takokit storage cleanup");
                println!("  scope        uv");
                println!("  target       {}", report.target.display());
                println!("  cache size   {}", format_bytes(report.reclaimed_bytes));
                println!(
                    "  mode         {}",
                    if report.dry_run { "dry-run" } else { "clean" }
                );
                println!(
                    "  removed      {}",
                    if report.removed { "yes" } else { "no" }
                );
            }
        }
        StorageScope::Downloads | StorageScope::Unused | StorageScope::AllSafe => {
            let _guard = acquire_maintenance_lock(root)?;
            let before = provider_ownership_status(root)?;
            let migration = if !dry_run
                && matches!(scope, StorageScope::Unused | StorageScope::AllSafe)
                && !before.legacy_models_pending_migration.is_empty()
            {
                Some(migrate_legacy_provider_cache(root)?)
            } else {
                None
            };
            let scope_name = match scope {
                StorageScope::Downloads => "downloads",
                StorageScope::Unused => "unused",
                StorageScope::AllSafe => "all-safe",
                StorageScope::Uv => unreachable!(),
            };
            let recovered_cleanup = if dry_run {
                false
            } else {
                begin_provider_cleanup(root, scope_name)?
            };
            let report = clean_provider_storage(root, scope_name, dry_run)?;
            if !dry_run {
                finish_provider_cleanup(root)?;
            }
            let after = provider_ownership_status(root)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "migration": migration,
                        "cleanup": report,
                        "cleanup_recovered_from_interrupted_run": recovered_cleanup,
                        "provider_ownership_before": before,
                        "provider_ownership_after": after,
                    }))?
                );
            } else {
                if dry_run && !before.legacy_models_pending_migration.is_empty() {
                    println!(
                        "Provider migration required before provider caches can become cleanable: {}",
                        before.legacy_models_pending_migration.join(", ")
                    );
                    println!(
                        "A real `--scope {scope_name}` cleanup performs the non-destructive durable migration first."
                    );
                    println!();
                }
                if let Some(migration) = migration.as_ref() {
                    println!("Takokit provider ownership migration");
                    println!("  journal        {}", migration.journal.display());
                    println!("  migrated       {}", migration.migrated_models.len());
                    println!("  already owned  {}", migration.already_owned_models.len());
                    println!(
                        "  provider bytes {}",
                        format_bytes(migration.provider_bytes)
                    );
                    println!();
                }
                if recovered_cleanup {
                    println!(
                        "Recovered an interrupted provider cleanup by recomputing its safe plan under the maintenance lock."
                    );
                    println!();
                }
                print_provider_cleanup(&report);
            }
        }
    }
    Ok(())
}

fn print_provider_cleanup(report: &ProviderCleanupReport) {
    println!("Takokit storage cleanup");
    println!("  scope        {}", report.scope);
    println!(
        "  mode         {}",
        if report.dry_run { "dry-run" } else { "clean" }
    );
    println!("  reclaimable  {}", format_bytes(report.reclaimed_bytes));
    println!("  remove paths {}", report.removed.len());
    println!("  retained     {}", report.retained.len());
    for item in &report.removed {
        println!(
            "  remove       {:>10}  {}  ({})",
            format_bytes(item.bytes),
            item.path.display(),
            item.reason
        );
    }
    for item in &report.retained {
        println!(
            "  retain       {:>10}  {}  ({})",
            format_bytes(item.bytes),
            item.path.display(),
            item.reason
        );
    }
}

fn print_provider_ownership(status: &ProviderOwnershipStatus) {
    println!("Provider checkpoint ownership");
    println!("  schema           {}", status.schema_version);
    println!(
        "  provider cache   {} across {} files",
        format_bytes(status.provider_cache_bytes),
        status.provider_cache_files
    );
    println!(
        "  durable blobs    {} across {} files",
        format_bytes(status.durable_blob_bytes),
        status.durable_blob_files
    );
    println!("  model ledgers    {}", status.model_ledgers);
    println!(
        "  fully owned      {}",
        if status.provider_cache_fully_owned {
            "yes"
        } else {
            "no"
        }
    );
    if !status.legacy_models_pending_migration.is_empty() {
        println!(
            "  migration pending {}",
            status.legacy_models_pending_migration.join(", ")
        );
    }
}
