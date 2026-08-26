use super::*;
use serde_json::json;
use std::{fs, path::Path};
use takokit_package::python_managed_runner_layout;

impl RvcVoiceService {
    pub(super) fn finalize_successful_training(
        &self,
        project: &RvcVoiceProject,
        checkpoint: &RvcCheckpoint,
        index: Option<&RvcIndexArtifact>,
    ) -> TakokitResult<()> {
        let Some(job_id) = project.latest_job_id else {
            return Ok(());
        };
        let mut job = self.store.load_job(project.id, job_id)?;
        if job.status != RvcTrainingJobStatus::Succeeded || job.stage != RvcTrainingStage::Complete
        {
            return Ok(());
        }

        if job.checkpoint_ids != vec![checkpoint.id] {
            job.checkpoint_ids = vec![checkpoint.id];
            self.store.save_job(&job)?;
        }

        let layout = self.store.layout(project.id);
        let cleanup_marker = layout.jobs.join("cleanup-complete.json");
        if cleanup_marker.is_file() {
            return Ok(());
        }
        let warning_path = layout.jobs.join("cleanup-warning.txt");
        match self.cleanup_successful_training(project.id, &checkpoint.path) {
            Ok(()) => {
                if warning_path.is_file() {
                    let _ = fs::remove_file(&warning_path);
                }
                write_atomic_json(
                    &cleanup_marker,
                    &json!({
                        "cleaned_at": now(),
                        "checkpoint_id": checkpoint.id,
                        "index_id": index.map(|item| item.id),
                        "policy": "successful-training-intermediates-v1"
                    }),
                )?;
            }
            Err(message) => {
                let _ = fs::write(
                    &warning_path,
                    format!(
                        "The final RVC voice is ready, but Takokit could not remove all training intermediates.\n{message}\n"
                    ),
                );
            }
        }
        Ok(())
    }

    fn cleanup_successful_training(
        &self,
        voice_id: Uuid,
        active_checkpoint: &Path,
    ) -> Result<(), String> {
        let layout = self.store.layout(voice_id);
        let experiment = format!("takokit_{}", voice_id.simple());
        let trainer_root = python_managed_runner_layout(&self.root)
            .adapters
            .join("rvc_training")
            .join("source");
        let mut errors = Vec::new();

        let logs_link = trainer_root.join("logs").join(&experiment);
        if fs::symlink_metadata(&logs_link).is_ok() {
            #[cfg(windows)]
            record_cleanup_result(
                fs::remove_dir(&logs_link),
                &logs_link,
                "remove trainer experiment junction",
                &mut errors,
            );
            #[cfg(not(windows))]
            record_cleanup_result(
                fs::remove_file(&logs_link),
                &logs_link,
                "remove trainer experiment link",
                &mut errors,
            );
        }

        for path in [
            layout.root.join("dataset").join("experiment"),
            layout.root.join("dataset").join("inputs"),
            layout.dataset_segments.clone(),
            layout.dataset_f0.clone(),
            layout.dataset_features.clone(),
        ] {
            remove_dir_tree(&path, &mut errors);
        }
        remove_file_if_present(
            &layout.root.join("dataset").join(".prepare-key"),
            &mut errors,
        );
        remove_file_if_present(&layout.jobs.join("latest-preparation.json"), &mut errors);

        remove_matching_files(
            &trainer_root.join("assets").join("weights"),
            &experiment,
            None,
            &mut errors,
        );
        remove_matching_files(
            &layout.checkpoints,
            &experiment,
            Some(active_checkpoint),
            &mut errors,
        );

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

fn remove_matching_files(root: &Path, prefix: &str, keep: Option<&Path>, errors: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if keep.is_some_and(|candidate| candidate == path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with(prefix)
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("pth"))
        {
            record_cleanup_result(
                fs::remove_file(&path),
                &path,
                "remove intermediate checkpoint",
                errors,
            );
        }
    }
}

fn remove_dir_tree(path: &Path, errors: &mut Vec<String>) {
    if path.exists() {
        record_cleanup_result(
            fs::remove_dir_all(path),
            path,
            "remove training scratch",
            errors,
        );
    }
}

fn remove_file_if_present(path: &Path, errors: &mut Vec<String>) {
    if path.is_file() {
        record_cleanup_result(
            fs::remove_file(path),
            path,
            "remove training marker",
            errors,
        );
    }
}

fn record_cleanup_result(
    result: std::io::Result<()>,
    path: &Path,
    action: &str,
    errors: &mut Vec<String>,
) {
    if let Err(error) = result {
        errors.push(format!("{action}: {}: {error}", path.display()));
    }
}
