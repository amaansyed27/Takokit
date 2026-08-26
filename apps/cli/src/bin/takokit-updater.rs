use std::{fs, io, path::Path};

#[path = "takokit_updater_support.rs"]
mod support;
use support::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    if !cfg!(windows) {
        return Err("takokit-updater is Windows-only in Slice 4".into());
    }
    wait_for_parent(args.parent_pid)?;
    validate_install_root(&args.install_root)?;

    let parent = args
        .install_root
        .parent()
        .ok_or("installation root has no parent")?;
    let nonce = format!("{}-{}", std::process::id(), now());
    let replacement = parent.join(format!("Takokit.update.{nonce}"));
    let backup = parent.join(format!("Takokit.rollback.{nonce}"));

    write_journal(
        &args.journal,
        &Journal {
            state: "extracting".into(),
            install_root: args.install_root.clone(),
            bundle: args.bundle.clone(),
            expected_version: args.expected_version.clone(),
            backup_root: Some(backup.clone()),
            message: "Extracting verified update bundle.".into(),
            updated_at: now(),
        },
    )?;

    remove_dir_if_exists(&replacement)?;
    remove_dir_if_exists(&backup)?;
    extract_bundle(&args.bundle, &replacement)?;
    validate_replacement(&replacement)?;

    write_state(
        &args,
        "replacing",
        Some(backup.clone()),
        "Replacing immutable application tree.",
    )?;

    let failpoint = configured_test_failpoint();
    replace_installation(
        &args,
        parent,
        &replacement,
        &backup,
        &nonce,
        failpoint.as_deref(),
        |root| verify_install(root, &args.expected_version),
    )
}

fn replace_installation<F>(
    args: &Args,
    parent: &Path,
    replacement: &Path,
    backup: &Path,
    nonce: &str,
    failpoint: Option<&str>,
    verify: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&Path) -> Result<(), Box<dyn std::error::Error>>,
{
    fs::rename(&args.install_root, backup).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "could not move current installation {} to rollback {}: {error}",
                args.install_root.display(),
                backup.display()
            ),
        )
    })?;

    if failpoint == Some("after_backup") {
        return rollback_before_replacement(
            args,
            backup,
            "test failpoint after moving the current installation to rollback storage",
        );
    }

    if let Err(error) = fs::rename(replacement, &args.install_root) {
        return rollback_before_replacement(
            args,
            backup,
            &format!("Replacement rename failed: {error}"),
        );
    }

    if failpoint == Some("after_replace") {
        return rollback_after_replacement(
            args,
            parent,
            backup,
            nonce,
            "test failpoint after replacing the installed application tree",
        );
    }

    let verification = verify(&args.install_root).and_then(|_| restart_daemon_if_requested(args));
    match verification {
        Ok(()) => {
            remove_dir_if_exists(backup)?;
            write_state(
                args,
                "completed",
                None,
                if args.restart_daemon {
                    "Update installed and verified; the owned Takokit daemon restarted successfully. Rollback tree removed."
                } else {
                    "Update installed and verified. Rollback tree removed."
                },
            )?;
            Ok(())
        }
        Err(error) => rollback_after_replacement(
            args,
            parent,
            backup,
            nonce,
            &format!("Post-update verification failed: {error}"),
        ),
    }
}

fn rollback_before_replacement(
    args: &Args,
    backup: &Path,
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let rollback_result = fs::rename(backup, &args.install_root);
    write_state(
        args,
        "rolled_back",
        None,
        &format!("{reason}; rollback result: {rollback_result:?}"),
    )?;
    if let Err(rollback_error) = rollback_result {
        return Err(format!("{reason}; rollback failed: {rollback_error}").into());
    }
    Err(reason.to_string().into())
}

fn rollback_after_replacement(
    args: &Args,
    parent: &Path,
    backup: &Path,
    nonce: &str,
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let failed = parent.join(format!("Takokit.failed.{nonce}"));
    let failed_result = fs::rename(&args.install_root, &failed);
    let rollback_result = fs::rename(backup, &args.install_root);
    let rollback_restart = if args.restart_daemon && rollback_result.is_ok() {
        restart_daemon(&args.install_root).map_err(|restart_error| restart_error.to_string())
    } else {
        Ok(())
    };
    write_state(
        args,
        "rolled_back",
        None,
        &format!(
            "{reason}; failed-tree result: {failed_result:?}; rollback result: {rollback_result:?}; rollback daemon restart: {rollback_restart:?}"
        ),
    )?;
    if let Err(rollback_error) = rollback_result {
        return Err(format!("{reason}; rollback failed: {rollback_error}").into());
    }
    if let Err(restart_error) = rollback_restart {
        return Err(format!("{reason}; rollback daemon restart failed: {restart_error}").into());
    }
    Err(reason.to_string().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_args(root: &Path) -> Args {
        Args {
            parent_pid: 0,
            install_root: root.join("Takokit"),
            bundle: root.join("update.zip"),
            expected_version: "0.0.2".to_string(),
            journal: root.join("update-journal.json"),
            restart_daemon: false,
        }
    }

    #[test]
    fn failpoint_after_backup_restores_original_installation() {
        let temp = tempfile::tempdir().unwrap();
        let args = fixture_args(temp.path());
        let replacement = temp.path().join("Takokit.update.fixture");
        let backup = temp.path().join("Takokit.rollback.fixture");
        fs::create_dir_all(&args.install_root).unwrap();
        fs::create_dir_all(&replacement).unwrap();
        fs::write(args.install_root.join("original.txt"), b"original").unwrap();
        fs::write(replacement.join("replacement.txt"), b"replacement").unwrap();

        let result = replace_installation(
            &args,
            temp.path(),
            &replacement,
            &backup,
            "fixture",
            Some("after_backup"),
            |_| Ok(()),
        );

        assert!(result.is_err());
        assert!(args.install_root.join("original.txt").is_file());
        assert!(!args.install_root.join("replacement.txt").exists());
        assert!(!backup.exists());
        assert!(replacement.join("replacement.txt").is_file());
        let journal: Journal =
            serde_json::from_slice(&fs::read(&args.journal).unwrap()).unwrap();
        assert_eq!(journal.state, "rolled_back");
    }

    #[test]
    fn failpoint_after_replace_restores_original_installation() {
        let temp = tempfile::tempdir().unwrap();
        let args = fixture_args(temp.path());
        let replacement = temp.path().join("Takokit.update.fixture");
        let backup = temp.path().join("Takokit.rollback.fixture");
        fs::create_dir_all(&args.install_root).unwrap();
        fs::create_dir_all(&replacement).unwrap();
        fs::write(args.install_root.join("original.txt"), b"original").unwrap();
        fs::write(replacement.join("replacement.txt"), b"replacement").unwrap();

        let result = replace_installation(
            &args,
            temp.path(),
            &replacement,
            &backup,
            "fixture",
            Some("after_replace"),
            |_| Ok(()),
        );

        assert!(result.is_err());
        assert!(args.install_root.join("original.txt").is_file());
        assert!(!args.install_root.join("replacement.txt").exists());
        assert!(!backup.exists());
        assert!(temp
            .path()
            .join("Takokit.failed.fixture")
            .join("replacement.txt")
            .is_file());
        let journal: Journal =
            serde_json::from_slice(&fs::read(&args.journal).unwrap()).unwrap();
        assert_eq!(journal.state, "rolled_back");
    }
}
