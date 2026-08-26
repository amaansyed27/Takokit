use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use takokit_core::{
    ManagedRvcVoice, RvcCheckpoint, RvcIndexArtifact, RvcTrainingJob, RvcTrainingJobStatus,
    RvcVoiceConsent, RvcVoiceProject, RvcVoiceProjectState, TakokitError, TakokitResult,
    RVC_VOICE_SCHEMA_VERSION,
};
use uuid::Uuid;

mod layout;
mod metadata;
mod samples;
use metadata::*;
pub use layout::RvcVoiceLayout;

#[derive(Debug, Clone)]
pub struct RvcVoiceStore {
    root: PathBuf,
}

impl RvcVoiceStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn layout(&self, voice_id: Uuid) -> RvcVoiceLayout {
        RvcVoiceLayout::new(self.root.join(voice_id.to_string()))
    }

    pub fn create(
        &self,
        name: &str,
        consent_affirmed: bool,
        consent_note: Option<String>,
    ) -> TakokitResult<RvcVoiceProject> {
        let name = validate_name(name)?;
        if !consent_affirmed {
            return Err(TakokitError::InvalidRequest(
                "advanced voice creation requires permission acknowledgement".into(),
            ));
        }
        fs::create_dir_all(&self.root).map_err(storage_error)?;
        let id = Uuid::new_v4();
        let layout = self.layout(id);
        layout.ensure()?;
        write_recovery_name(&layout, &name)?;
        let now = now_secs();
        let project = RvcVoiceProject {
            schema_version: RVC_VOICE_SCHEMA_VERSION,
            id,
            name,
            engine: "rvc".into(),
            state: RvcVoiceProjectState::Created,
            imported: false,
            created_at: now,
            updated_at: now,
            latest_job_id: None,
            active_checkpoint_id: None,
            active_index_id: None,
            last_error: None,
        };
        let consent = RvcVoiceConsent {
            voice_id: id,
            affirmed: true,
            note: consent_note,
            recorded_at: now,
            statement: "I own this voice or have explicit permission to use these recordings. This acknowledgement is metadata, not legal verification.".into(),
        };
        atomic_json(&layout.project, &project)?;
        atomic_json(&layout.consent, &consent)?;
        Ok(project)
    }

    pub fn load(&self, voice: &str) -> TakokitResult<RvcVoiceProject> {
        read_json(&self.layout(self.resolve_id(voice)?).project)
    }

    pub fn load_id(&self, id: Uuid) -> TakokitResult<RvcVoiceProject> {
        read_json(&self.layout(id).project)
    }

    pub fn list(&self) -> TakokitResult<Vec<RvcVoiceProject>> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut projects = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(storage_error)? {
            let directory = entry.map_err(storage_error)?.path();
            let path = directory.join("project.json");
            if !path.is_file() {
                if let Some(id) = directory
                    .file_name()
                    .and_then(|value| value.to_str())
                    .and_then(|value| Uuid::parse_str(value).ok())
                {
                    if let Some(project) = self.recover_project(id)? {
                        projects.push(project);
                    }
                }
                continue;
            }
            if let Ok(project) = read_json::<RvcVoiceProject>(&path) {
                projects.push(project);
            }
        }
        projects.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(projects)
    }

    pub fn resolve_id(&self, voice: &str) -> TakokitResult<Uuid> {
        if let Ok(id) = Uuid::parse_str(voice) {
            if self.layout(id).project.is_file() {
                return Ok(id);
            }
            if self.recover_project(id)?.is_some() {
                return Ok(id);
            }
        }
        let matches = self
            .list()?
            .into_iter()
            .filter(|project| project.name.eq_ignore_ascii_case(voice))
            .map(|project| project.id)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [id] => Ok(*id),
            [] => Err(TakokitError::Storage(format!(
                "RVC voice not found: {voice}"
            ))),
            _ => Err(TakokitError::InvalidRequest(format!(
                "more than one RVC voice is named {voice:?}; use its voice ID"
            ))),
        }
    }

    pub fn save_project(&self, project: &RvcVoiceProject) -> TakokitResult<()> {
        let mut project = project.clone();
        project.updated_at = now_secs();
        write_recovery_name(&self.layout(project.id), &project.name)?;
        atomic_json(&self.layout(project.id).project, &project)
    }

    fn recover_project(&self, id: Uuid) -> TakokitResult<Option<RvcVoiceProject>> {
        let layout = self.layout(id);
        if !layout.root.is_dir() || !layout.consent.is_file() {
            return Ok(None);
        }
        let consent: RvcVoiceConsent = read_json(&layout.consent)?;
        if consent.voice_id != id || !consent.affirmed {
            return Ok(None);
        }
        let samples = self.samples_id(id)?;
        let name = read_recovery_name(&layout)
            .unwrap_or_else(|| format!("Recovered RVC voice {}", &id.to_string()[..8]));
        let mut jobs = Vec::new();
        if layout.jobs.is_dir() {
            for entry in fs::read_dir(&layout.jobs).map_err(storage_error)? {
                let path = entry.map_err(storage_error)?.path();
                if is_uuid_json_record(&path) {
                    if let Ok(job) = read_json::<RvcTrainingJob>(&path) {
                        if job.voice_id == id {
                            jobs.push(job);
                        }
                    }
                }
            }
        }
        jobs.sort_by_key(|job| job.created_at);
        let managed = read_json::<ManagedRvcVoice>(&layout.voice).ok();
        let ready_for_preparation = !samples.is_empty()
            && samples
                .iter()
                .filter(|sample| sample.included)
                .all(|sample| {
                    sample.state == takokit_core::RvcSampleState::Inspected
                        && sample
                            .inspection
                            .as_ref()
                            .and_then(|inspection| inspection.duration_ms)
                            .unwrap_or_default()
                            > 0
                });
        let created_at = samples
            .iter()
            .map(|sample| sample.imported_at)
            .chain(std::iter::once(consent.recorded_at))
            .min()
            .unwrap_or(consent.recorded_at);
        let project = RvcVoiceProject {
            schema_version: RVC_VOICE_SCHEMA_VERSION,
            id,
            name,
            engine: "rvc".into(),
            state: if managed.is_some() {
                RvcVoiceProjectState::Ready
            } else if ready_for_preparation {
                RvcVoiceProjectState::ReadyForPreparation
            } else {
                RvcVoiceProjectState::CollectingSamples
            },
            imported: false,
            created_at,
            updated_at: now_secs(),
            latest_job_id: jobs.last().map(|job| job.id),
            active_checkpoint_id: managed.as_ref().map(|voice| voice.checkpoint_id),
            active_index_id: managed.as_ref().and_then(|voice| voice.index_id),
            last_error: None,
        };
        write_recovery_name(&layout, &project.name)?;
        atomic_json(&layout.project, &project)?;
        Ok(Some(project))
    }

    pub fn set_state(
        &self,
        voice_id: Uuid,
        state: RvcVoiceProjectState,
        error: Option<String>,
    ) -> TakokitResult<RvcVoiceProject> {
        let mut project = self.load_id(voice_id)?;
        project.state = state;
        project.last_error = error;
        project.updated_at = now_secs();
        atomic_json(&self.layout(voice_id).project, &project)?;
        Ok(project)
    }

    pub fn save_job(&self, job: &RvcTrainingJob) -> TakokitResult<()> {
        let layout = self.layout(job.voice_id);
        layout.ensure()?;
        atomic_json(&layout.jobs.join(format!("{}.json", job.id)), job)
    }

    pub fn load_job(&self, voice_id: Uuid, job_id: Uuid) -> TakokitResult<RvcTrainingJob> {
        read_json(&self.layout(voice_id).jobs.join(format!("{job_id}.json")))
    }

    pub fn active_job(&self, voice_id: Uuid) -> TakokitResult<Option<RvcTrainingJob>> {
        let dir = self.layout(voice_id).jobs;
        if !dir.is_dir() {
            return Ok(None);
        }
        let mut jobs = Vec::new();
        for entry in fs::read_dir(dir).map_err(storage_error)? {
            let path = entry.map_err(storage_error)?.path();
            if !is_uuid_json_record(&path) {
                continue;
            }
            let job: RvcTrainingJob = read_json(&path)?;
            if matches!(
                job.status,
                RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running
            ) {
                jobs.push(job);
            }
        }
        jobs.sort_by_key(|job| job.created_at);
        Ok(jobs.pop())
    }

    pub fn save_checkpoint(&self, checkpoint: &RvcCheckpoint) -> TakokitResult<()> {
        atomic_json(
            &self
                .layout(checkpoint.voice_id)
                .checkpoints
                .join(format!("{}.json", checkpoint.id)),
            checkpoint,
        )
    }

    pub fn checkpoints(&self, voice: &str) -> TakokitResult<Vec<RvcCheckpoint>> {
        let id = self.resolve_id(voice)?;
        read_metadata_dir(&self.layout(id).checkpoints)
    }

    pub fn save_index(&self, index: &RvcIndexArtifact) -> TakokitResult<()> {
        atomic_json(
            &self
                .layout(index.voice_id)
                .indexes
                .join(format!("{}.json", index.id)),
            index,
        )
    }

    pub fn indexes(&self, voice: &str) -> TakokitResult<Vec<RvcIndexArtifact>> {
        let id = self.resolve_id(voice)?;
        read_metadata_dir(&self.layout(id).indexes)
    }

    pub fn save_managed_voice(&self, voice: &ManagedRvcVoice) -> TakokitResult<()> {
        atomic_json(&self.layout(voice.project_id).voice, voice)
    }

    pub fn managed_voice(&self, voice: &str) -> TakokitResult<ManagedRvcVoice> {
        read_json(&self.layout(self.resolve_id(voice)?).voice)
    }

    pub fn remove(&self, voice: &str, dry_run: bool) -> TakokitResult<Vec<PathBuf>> {
        let id = self.resolve_id(voice)?;
        if self.active_job(id)?.is_some() {
            return Err(TakokitError::InvalidRequest(
                "cannot remove a voice while a training job is active; cancel it first".into(),
            ));
        }
        let root = self.layout(id).root;
        let files = if root.is_dir() {
            fs::read_dir(&root)
                .map_err(storage_error)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        } else {
            Vec::new()
        };
        if !dry_run && root.exists() {
            fs::remove_dir_all(root).map_err(storage_error)?;
        }
        Ok(files)
    }
}

pub fn sha256_file(path: &Path) -> TakokitResult<String> {
    let mut file = File::open(path).map_err(storage_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(storage_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests;
