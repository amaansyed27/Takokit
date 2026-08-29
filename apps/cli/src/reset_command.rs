use crate::{args::ResetArgs, daemon, distribution};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResetTarget {
    category: String,
    path: PathBuf,
    exists: bool,
    bytes: u64,
    action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResetPlan {
    takokit_home: PathBuf,
    installed_application: Option<PathBuf>,
    installed_application_action: String,
    daemon_action: String,
    path_action: String,
    targets: Vec<ResetTarget>,
    project_target: Option<ResetTarget>,
    project_requires_separate_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResetJournal {
    state: String,
    takokit_home: PathBuf,
    project_target: Option<PathBuf>,
    completed: Vec<PathBuf>,
    message: String,
    updated_at: u64,
}

pub(crate) fn run_reset_command(
    store: &LocalStore,
    config: &RuntimeConfig,
    args: ResetArgs,
    json: bool,
) -> anyhow::Result<()> {
    let root = absolute_lexical(store.root())?;
    validate_global_root(&root)?;
    let project = args
        .project_data
        .as_ref()
        .map(|path| resolve_project_tako(path))
        .transpose()?;
    if let Some(project) = project.as_ref() {
        validate_project_root(project, &root)?;
    }
    let plan = build_plan(&root, project.as_deref());

    if args.dry_run || !args.all {
        print_plan(&plan, json)?;
        if !args.dry_run && !args.all && !json {
            println!();
            println!("No data was removed. Use --all with --confirm <exact resolved Takokit home> after reviewing this plan.");
        }
        return Ok(());
    }

    let confirm = args.confirm.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "full reset requires --confirm with the exact resolved Takokit home: {}",
            root.display()
        )
    })?;
    if absolute_lexical(confirm)? != root {
        anyhow::bail!(
            "reset acknowledgement does not exactly match resolved Takokit home {}",
            root.display()
        );
    }

    if let Some(project) = project.as_ref() {
        let confirm = args.confirm_project_data.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "project data requires separate --confirm-project-data with exact path {}",
                project.display()
            )
        })?;
        if absolute_lexical(confirm)? != *project {
            anyhow::bail!(
                "project acknowledgement does not exactly match {}",
                project.display()
            );
        }
    } else if args.confirm_project_data.is_some() {
        anyhow::bail!("--confirm-project-data requires --project-data");
    }

    let journal_path = reset_journal_path();
    let mut journal = ResetJournal {
        state: "starting".into(),
        takokit_home: root.clone(),
        project_target: project.clone(),
        completed: Vec::new(),
        message: "Reset confirmed; stopping owned daemon before deleting user data.".into(),
        updated_at: now(),
    };
    write_journal(&journal_path, &journal)?;

    let _ = daemon::stop(store, config)?;
    journal.state = "deleting-global-data".into();
    journal.message = "Owned daemon stopped. Removing confirmed Takokit global data root.".into();
    journal.updated_at = now();
    write_journal(&journal_path, &journal)?;

    remove_confirmed_tree(&root)?;
    journal.completed.push(root.clone());
    write_journal(&journal_path, &journal)?;

    if let Some(project) = project {
        journal.state = "deleting-project-data".into();
        journal.message = "Removing separately confirmed project .tako data.".into();
        journal.updated_at = now();
        write_journal(&journal_path, &journal)?;
        remove_confirmed_tree(&project)?;
        journal.completed.push(project);
    }

    journal.state = "completed".into();
    journal.message = "Confirmed reset completed. Installed application, PATH registration, and other workspaces were preserved.".into();
    journal.updated_at = now();
    write_journal(&journal_path, &journal)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&journal)?);
    } else {
        println!("Takokit reset completed.");
        println!("Journal: {}", journal_path.display());
        println!("Installed application and PATH registration were preserved; use the Windows uninstaller to remove them.");
    }
    Ok(())
}

fn build_plan(root: &Path, project: Option<&Path>) -> ResetPlan {
    let categories = [
        (
            "models",
            "installed model snapshots and materialized artifacts",
        ),
        ("blobs", "Takokit-owned content-addressed model blobs"),
        ("runners", "managed runner runtimes"),
        ("tools", "Takokit-managed tools and shared Python bases"),
        ("cache", "provider, UV, and download caches"),
        (
            "manifests",
            "installed package manifests and ownership records",
        ),
        (
            "voices",
            "instant voices, trained RVC projects, checkpoints, and package keys",
        ),
        ("datasets", "global prepared datasets"),
        ("logs", "global logs"),
        ("licenses", "license acceptance receipts"),
        ("runtime", "daemon, updater, cleanup, and runtime metadata"),
        ("settings", "global settings if present"),
    ];
    let targets = categories
        .into_iter()
        .map(|(name, description)| {
            let path = root.join(name);
            ResetTarget {
                category: name.to_string(),
                exists: path.exists(),
                bytes: path_size(&path),
                path,
                action: format!("remove {description}"),
            }
        })
        .collect();
    let project_target = project.map(|path| ResetTarget {
        category: "project-data".to_string(),
        exists: path.exists(),
        bytes: path_size(path),
        path: path.to_path_buf(),
        action: "remove this project's .tako sessions, files, and outputs only".to_string(),
    });
    ResetPlan {
        takokit_home: root.to_path_buf(),
        installed_application: distribution::application_root(),
        installed_application_action:
            "preserve; Windows uninstall owns application files, Start Menu, and PATH".into(),
        daemon_action: "stop only the verified managed Takokit daemon for this storage root".into(),
        path_action: "preserve; reset does not mutate Windows PATH".into(),
        targets,
        project_target,
        project_requires_separate_confirmation: true,
    }
}

fn print_plan(plan: &ResetPlan, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    }
    println!("Takokit reset dry-run");
    println!("  global root       {}", plan.takokit_home.display());
    if let Some(path) = plan.installed_application.as_ref() {
        println!("  application       {} (preserved)", path.display());
    }
    println!("  daemon            {}", plan.daemon_action);
    println!("  Windows PATH      {}", plan.path_action);
    println!();
    for target in &plan.targets {
        println!(
            "  {:<12} {:>12}  {}  {}",
            target.category,
            format_bytes(target.bytes),
            if target.exists { "present" } else { "absent " },
            target.path.display()
        );
    }
    if let Some(project) = plan.project_target.as_ref() {
        println!();
        println!("  project .tako     {}", project.path.display());
        println!("  project deletion  requires separate exact-path acknowledgement");
    } else {
        println!();
        println!("Project .tako data is not part of this reset plan and will be preserved.");
    }
    Ok(())
}

fn validate_global_root(root: &Path) -> anyhow::Result<()> {
    reject_unsafe_path(root, "Takokit home")?;
    if let Some(user) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        if absolute_lexical(&user)? == root {
            anyhow::bail!("Takokit home resolves to the user profile; refusing reset");
        }
    }
    if let Some(install) = distribution::application_root() {
        if absolute_lexical(&install)? == root {
            anyhow::bail!(
                "Takokit home resolves to the application installation root; refusing reset"
            );
        }
    }
    if root.join(".git").exists() || root.join("Cargo.toml").is_file() {
        anyhow::bail!("Takokit home looks like a repository or project root; refusing reset");
    }
    Ok(())
}

fn validate_project_root(project: &Path, global_root: &Path) -> anyhow::Result<()> {
    reject_unsafe_path(project, "project .tako")?;
    if project.file_name().and_then(|name| name.to_str()) != Some(".tako") {
        anyhow::bail!("project reset target must resolve to a .tako directory");
    }
    if project == global_root || project.starts_with(global_root) {
        anyhow::bail!("project .tako target must be separate from Takokit global storage");
    }
    Ok(())
}

fn resolve_project_tako(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = absolute_lexical(path)?;
    Ok(
        if absolute.file_name().and_then(|name| name.to_str()) == Some(".tako") {
            absolute
        } else {
            absolute.join(".tako")
        },
    )
}

fn reject_unsafe_path(path: &Path, label: &str) -> anyhow::Result<()> {
    if !path.is_absolute() {
        anyhow::bail!("{label} must resolve to an absolute path");
    }
    let components = path.components().collect::<Vec<_>>();
    if components.len() <= 1
        || components
            .iter()
            .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("{label} resolves to an unsafe path: {}", path.display());
    }
    #[cfg(windows)]
    if components.len() <= 2 {
        anyhow::bail!("{label} cannot be a drive root: {}", path.display());
    }
    Ok(())
}

fn remove_confirmed_tree(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!("refusing to recursively delete symlink {}", path.display())
        }
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => anyhow::bail!("reset target is not a directory: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn absolute_lexical(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn reset_journal_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Takokit")
        .join("reset-journal.json")
}

fn write_journal(path: &Path, journal: &ResetJournal) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(journal)?)?;
    let _ = fs::remove_file(path);
    fs::rename(temporary, path)?;
    Ok(())
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| path_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
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
    fn workspace_argument_targets_only_dot_tako() {
        let root = std::env::temp_dir().join("Takokit reset unicode ü");
        let project = resolve_project_tako(&root).unwrap();
        assert_eq!(project, root.join(".tako"));
    }
    #[test]
    fn global_root_refuses_repository_like_directory() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[workspace]").unwrap();
        assert!(validate_global_root(temp.path()).is_err());
    }
}
