use super::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use takokit_store::sha256_file;

impl RvcVoiceService {
    pub fn checkpoints(&self, voice: &str) -> TakokitResult<Vec<RvcCheckpoint>> {
        self.refresh_completed_artifacts(voice)?;
        self.store.checkpoints(voice)
    }

    pub fn indexes(&self, voice: &str) -> TakokitResult<Vec<RvcIndexArtifact>> {
        self.refresh_completed_artifacts(voice)?;
        self.store.indexes(voice)
    }

    pub fn select_checkpoint(
        &self,
        voice: &str,
        request: SelectRvcCheckpointRequest,
    ) -> TakokitResult<ManagedRvcVoice> {
        let project = self.store.load(voice)?;
        let checkpoint = self
            .store
            .checkpoints(voice)?
            .into_iter()
            .find(|item| item.id == request.checkpoint_id)
            .ok_or_else(|| invalid("checkpoint does not belong to this voice"))?;
        if !checkpoint.valid_for_inference || !checkpoint.path.is_file() {
            return Err(invalid(
                "selected checkpoint is missing or invalid for inference",
            ));
        }
        let index = match request.index_id {
            Some(id) => Some(
                self.store
                    .indexes(voice)?
                    .into_iter()
                    .find(|item| item.id == id)
                    .ok_or_else(|| invalid("index does not belong to this voice"))?,
            ),
            None => None,
        };
        if let Some(index) = index.as_ref() {
            if !index.valid || !index.path.is_file() {
                return Err(invalid("selected index is missing or invalid"));
            }
            if index
                .checkpoint_id
                .is_some_and(|pair| pair != checkpoint.id)
            {
                return Err(invalid(
                    "selected index is paired with a different checkpoint",
                ));
            }
        }
        self.materialize_runtime(&project, &checkpoint, index.as_ref())?;
        let managed = ManagedRvcVoice {
            id: project.id,
            project_id: project.id,
            name: project.name.clone(),
            checkpoint_id: checkpoint.id,
            index_id: index.as_ref().map(|item| item.id),
            ready_at: now(),
        };
        self.store.save_managed_voice(&managed)?;
        let mut project = project;
        project.active_checkpoint_id = Some(checkpoint.id);
        project.active_index_id = index.as_ref().map(|item| item.id);
        project.state = RvcVoiceProjectState::Ready;
        project.last_error = None;
        self.store.save_project(&project)?;
        Ok(managed)
    }

    pub fn import_existing(
        &self,
        request: ImportRvcVoiceRequest,
    ) -> TakokitResult<RvcVoiceProject> {
        validate_artifact_path(&request.checkpoint, "pth")?;
        if let Some(index) = request.index.as_ref() {
            validate_artifact_path(index, "index")?;
        }
        let mut project = self.store.create(
            &request.name,
            request.consent_affirmed,
            request.consent_note,
        )?;
        project.imported = true;
        self.store.save_project(&project)?;
        self.import_artifacts(
            project.id,
            &request.checkpoint,
            request.index.as_deref(),
            Some(json!({
                "source_checkpoint": request.checkpoint,
                "source_index": request.index,
                "imported_at": now()
            })),
        )?;
        self.store.load_id(project.id)
    }

    pub fn resolve_conversion_target(&self, voice_or_path: &str) -> TakokitResult<String> {
        let candidate = Path::new(voice_or_path);
        if candidate.exists() {
            return Ok(voice_or_path.to_string());
        }
        let project = self.store.load(voice_or_path)?;
        let target = self.conversion_target_id(project.id);
        if !target.join("rvc.json").is_file() {
            return Err(invalid("managed RVC voice is not ready for conversion"));
        }
        Ok(target.to_string_lossy().into_owned())
    }

    pub(super) fn refresh_completed_artifacts(&self, voice: &str) -> TakokitResult<()> {
        let project = self.store.load(voice)?;
        let result_path = self
            .store
            .layout(project.id)
            .jobs
            .join("latest-result.json");
        if !result_path.is_file() {
            return Ok(());
        }
        let value: Value =
            serde_json::from_reader(std::fs::File::open(&result_path).map_err(storage)?)
                .map_err(|error| invalid(error.to_string()))?;
        let checkpoint_path = value
            .get("checkpoint")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .ok_or_else(|| invalid("RVC worker result is missing checkpoint"))?;
        let index_path = value
            .get("index")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let layout = self.store.layout(project.id);
        ensure_managed_artifact(&checkpoint_path, &layout.checkpoints, "checkpoint")?;
        if let Some(index_path) = index_path.as_ref() {
            ensure_managed_artifact(index_path, &layout.indexes, "index")?;
        }
        if !checkpoint_path.is_file() {
            return Err(invalid("RVC worker checkpoint is missing"));
        }
        let checkpoint_hash = sha256_file(&checkpoint_path)?;
        let checkpoint = match self
            .store
            .checkpoints(voice)?
            .into_iter()
            .find(|item| item.sha256 == checkpoint_hash)
        {
            Some(item) => item,
            None => {
                let item = RvcCheckpoint {
                    id: Uuid::new_v4(),
                    voice_id: project.id,
                    path: checkpoint_path.clone(),
                    sha256: checkpoint_hash,
                    bytes: fs::metadata(&checkpoint_path).map_err(storage)?.len(),
                    epoch: value.get("epoch").and_then(Value::as_u64).map(|v| v as u32),
                    sample_rate_hz: Some(40_000),
                    model_version: Some("v2".into()),
                    f0: Some(true),
                    created_at: now(),
                    valid_for_inference: true,
                };
                self.store.save_checkpoint(&item)?;
                item
            }
        };
        let index = if let Some(path) = index_path.filter(|path| path.is_file()) {
            let hash = sha256_file(&path)?;
            match self
                .store
                .indexes(voice)?
                .into_iter()
                .find(|item| item.sha256 == hash && item.checkpoint_id == Some(checkpoint.id))
            {
                Some(item) => Some(item),
                None => {
                    let item = RvcIndexArtifact {
                        id: Uuid::new_v4(),
                        voice_id: project.id,
                        path: path.clone(),
                        sha256: hash,
                        bytes: fs::metadata(&path).map_err(storage)?.len(),
                        checkpoint_id: Some(checkpoint.id),
                        created_at: now(),
                        valid: true,
                    };
                    self.store.save_index(&item)?;
                    Some(item)
                }
            }
        } else {
            None
        };
        self.select_checkpoint(
            voice,
            SelectRvcCheckpointRequest {
                checkpoint_id: checkpoint.id,
                index_id: index.as_ref().map(|item| item.id),
            },
        )?;
        self.finalize_successful_training(&project, &checkpoint, index.as_ref())?;
        Ok(())
    }

    pub(super) fn import_artifacts(
        &self,
        voice_id: Uuid,
        checkpoint_source: &Path,
        index_source: Option<&Path>,
        provenance: Option<Value>,
    ) -> TakokitResult<()> {
        let project = self.store.load_id(voice_id)?;
        let layout = self.store.layout(voice_id);
        let checkpoint_id = Uuid::new_v4();
        let checkpoint_path = layout
            .checkpoints
            .join(format!("artifact-{checkpoint_id}.pth"));
        copy_or_link(checkpoint_source, &checkpoint_path)?;
        let checkpoint = RvcCheckpoint {
            id: checkpoint_id,
            voice_id,
            path: checkpoint_path.clone(),
            sha256: sha256_file(&checkpoint_path)?,
            bytes: fs::metadata(&checkpoint_path).map_err(storage)?.len(),
            epoch: None,
            sample_rate_hz: None,
            model_version: None,
            f0: None,
            created_at: now(),
            valid_for_inference: true,
        };
        self.store.save_checkpoint(&checkpoint)?;
        let index = match index_source {
            Some(source) => {
                let id = Uuid::new_v4();
                let path = layout.indexes.join(format!("artifact-{id}.index"));
                copy_or_link(source, &path)?;
                let item = RvcIndexArtifact {
                    id,
                    voice_id,
                    path: path.clone(),
                    sha256: sha256_file(&path)?,
                    bytes: fs::metadata(&path).map_err(storage)?.len(),
                    checkpoint_id: Some(checkpoint_id),
                    created_at: now(),
                    valid: true,
                };
                self.store.save_index(&item)?;
                Some(item)
            }
            None => None,
        };
        if let Some(provenance) = provenance {
            write_atomic_json(&layout.root.join("provenance.json"), &provenance)?;
        }
        self.select_checkpoint(
            &project.id.to_string(),
            SelectRvcCheckpointRequest {
                checkpoint_id,
                index_id: index.as_ref().map(|item| item.id),
            },
        )?;
        Ok(())
    }

    pub(super) fn materialize_runtime(
        &self,
        project: &RvcVoiceProject,
        checkpoint: &RvcCheckpoint,
        index: Option<&RvcIndexArtifact>,
    ) -> TakokitResult<()> {
        let runtime = self.conversion_target_id(project.id);
        let temporary = runtime.with_extension(format!("tmp-{}", Uuid::new_v4()));
        if temporary.exists() {
            fs::remove_dir_all(&temporary).map_err(storage)?;
        }
        fs::create_dir_all(&temporary).map_err(storage)?;
        copy_or_link(&checkpoint.path, &temporary.join("checkpoint.pth"))?;
        if let Some(index) = index {
            copy_or_link(&index.path, &temporary.join("model.index"))?;
        }
        write_atomic_json(
            &temporary.join("rvc.json"),
            &json!({
                "schema_version": 1,
                "engine": "rvc",
                "checkpoint": "checkpoint.pth",
                "index": index.map(|_| "model.index"),
                "quality_baseline": false,
                "managed_voice_id": project.id,
                "note": "Artifacts validated for execution. Perceptual identity/similarity is not inferred from successful file generation."
            }),
        )?;
        if runtime.exists() {
            fs::remove_dir_all(&runtime).map_err(storage)?;
        }
        fs::rename(&temporary, &runtime).map_err(storage)?;
        Ok(())
    }

    pub(super) fn conversion_target_id(&self, id: Uuid) -> PathBuf {
        self.store.layout(id).root.join("runtime")
    }
}

fn validate_artifact_path(path: &Path, extension: &str) -> TakokitResult<()> {
    if !path.is_file() {
        return Err(invalid(format!(
            "artifact does not exist: {}",
            path.display()
        )));
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        != Some(true)
    {
        return Err(invalid(format!(
            "expected .{extension} artifact: {}",
            path.display()
        )));
    }
    if fs::metadata(path).map_err(storage)?.len() == 0 {
        return Err(invalid(format!("artifact is empty: {}", path.display())));
    }
    Ok(())
}

fn copy_or_link(source: &Path, destination: &Path) -> TakokitResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(storage)?;
    }
    if destination.exists() {
        fs::remove_file(destination).map_err(storage)?;
    }
    if fs::hard_link(source, destination).is_err() {
        fs::copy(source, destination).map_err(storage)?;
    }
    Ok(())
}

fn ensure_managed_artifact(path: &Path, expected_root: &Path, kind: &str) -> TakokitResult<()> {
    let canonical = path.canonicalize().map_err(storage)?;
    let canonical_root = expected_root.canonicalize().map_err(storage)?;
    if !canonical.starts_with(&canonical_root) {
        return Err(invalid(format!(
            "RVC worker {kind} does not belong to the selected voice project: {}",
            path.display()
        )));
    }
    Ok(())
}
