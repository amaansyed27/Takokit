use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{
    ManagedRvcVoice, RvcAudioInspection, RvcCheckpoint, RvcDatasetInspection, RvcIndexArtifact,
    RvcSampleState, RvcSampleWarning, RvcTrainingJob, RvcTrainingJobStatus, RvcVoiceConsent,
    RvcVoiceProject, RvcVoiceProjectState, RvcVoiceSample, TakokitError, TakokitResult,
    RVC_VOICE_SCHEMA_VERSION,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RvcVoiceLayout {
    pub root: PathBuf,
    pub project: PathBuf,
    pub voice: PathBuf,
    pub consent: PathBuf,
    pub samples_originals: PathBuf,
    pub samples_managed: PathBuf,
    pub sample_metadata: PathBuf,
    pub dataset_segments: PathBuf,
    pub dataset_f0: PathBuf,
    pub dataset_features: PathBuf,
    pub checkpoints: PathBuf,
    pub indexes: PathBuf,
    pub references: PathBuf,
    pub logs: PathBuf,
    pub jobs: PathBuf,
    pub packages: PathBuf,
}

impl RvcVoiceLayout {
    pub fn new(root: PathBuf) -> Self {
        Self {
            project: root.join("project.json"),
            voice: root.join("voice.json"),
            consent: root.join("consent.json"),
            samples_originals: root.join("samples").join("originals"),
            samples_managed: root.join("samples").join("managed"),
            sample_metadata: root.join("samples").join("metadata"),
            dataset_segments: root.join("dataset").join("segments"),
            dataset_f0: root.join("dataset").join("f0"),
            dataset_features: root.join("dataset").join("features"),
            checkpoints: root.join("checkpoints"),
            indexes: root.join("indexes"),
            references: root.join("references"),
            logs: root.join("logs"),
            jobs: root.join("jobs"),
            packages: root.join("packages"),
            root,
        }
    }

    fn ensure(&self) -> TakokitResult<()> {
        for path in [
            &self.root,
            &self.samples_originals,
            &self.samples_managed,
            &self.sample_metadata,
            &self.dataset_segments,
            &self.dataset_f0,
            &self.dataset_features,
            &self.checkpoints,
            &self.indexes,
            &self.references,
            &self.logs,
            &self.jobs,
            &self.packages,
        ] {
            fs::create_dir_all(path).map_err(storage_error)?;
        }
        Ok(())
    }
}

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
                "advanced voice creation requires permission acknowledgement".to_string(),
            ));
        }
        fs::create_dir_all(&self.root).map_err(storage_error)?;
        let id = Uuid::new_v4();
        let layout = self.layout(id);
        layout.ensure()?;
        let now = now_secs();
        let project = RvcVoiceProject {
            schema_version: RVC_VOICE_SCHEMA_VERSION,
            id,
            name,
            engine: "rvc".to_string(),
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
            statement: "I own this voice or have explicit permission to use these recordings. This acknowledgement is metadata, not legal verification.".to_string(),
        };
        atomic_json(&layout.project, &project)?;
        atomic_json(&layout.consent, &consent)?;
        Ok(project)
    }

    pub fn load(&self, voice: &str) -> TakokitResult<RvcVoiceProject> {
        let id = self.resolve_id(voice)?;
        read_json(&self.layout(id).project)
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
            let entry = entry.map_err(storage_error)?;
            let path = entry.path().join("project.json");
            if path.is_file() {
                if let Ok(project) = read_json::<RvcVoiceProject>(&path) {
                    projects.push(project);
                }
            }
        }
        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| a.name.cmp(&b.name)));
        Ok(projects)
    }

    pub fn resolve_id(&self, voice: &str) -> TakokitResult<Uuid> {
        if let Ok(id) = Uuid::parse_str(voice) {
            if self.layout(id).project.is_file() {
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
            [] => Err(TakokitError::Storage(format!("RVC voice not found: {voice}"))),
            _ => Err(TakokitError::InvalidRequest(format!(
                "more than one RVC voice is named {voice:?}; use its voice ID"
            ))),
        }
    }

    pub fn save_project(&self, project: &RvcVoiceProject) -> TakokitResult<()> {
        let mut project = project.clone();
        project.updated_at = now_secs();
        atomic_json(&self.layout(project.id).project, &project)
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

    pub fn add_samples(&self, voice: &str, paths: &[PathBuf]) -> TakokitResult<Vec<RvcVoiceSample>> {
        if paths.is_empty() {
            return Err(TakokitError::InvalidRequest("at least one audio file is required".to_string()));
        }
        let id = self.resolve_id(voice)?;
        let layout = self.layout(id);
        layout.ensure()?;
        let existing = self.samples_id(id)?;
        let mut added = Vec::new();
        for input in paths {
            let source = input.canonicalize().map_err(|error| {
                TakokitError::InvalidRequest(format!("audio file {} cannot be read: {error}", input.display()))
            })?;
            if !source.is_file() {
                return Err(TakokitError::InvalidRequest(format!("audio sample is not a file: {}", source.display())));
            }
            let hash = sha256_file(&source)?;
            if existing.iter().chain(added.iter()).any(|sample: &RvcVoiceSample| sample.sha256 == hash) {
                continue;
            }
            let sample_id = Uuid::new_v4();
            let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("audio");
            let managed = layout.samples_originals.join(format!("{sample_id}.{extension}"));
            fs::copy(&source, &managed).map_err(storage_error)?;
            let bytes = fs::metadata(&managed).map_err(storage_error)?.len();
            let sample = RvcVoiceSample {
                id: sample_id,
                voice_id: id,
                display_name: source.file_name().and_then(|value| value.to_str()).unwrap_or("sample").to_string(),
                source_path: source,
                managed_path: managed,
                sha256: hash,
                bytes,
                imported_at: now_secs(),
                included: true,
                state: RvcSampleState::Imported,
                inspection: None,
                warnings: Vec::new(),
            };
            atomic_json(&layout.sample_metadata.join(format!("{sample_id}.json")), &sample)?;
            added.push(sample);
        }
        if !added.is_empty() {
            self.set_state(id, RvcVoiceProjectState::CollectingSamples, None)?;
        }
        Ok(added)
    }

    pub fn samples(&self, voice: &str) -> TakokitResult<Vec<RvcVoiceSample>> {
        let id = self.resolve_id(voice)?;
        self.samples_id(id)
    }

    pub fn samples_id(&self, id: Uuid) -> TakokitResult<Vec<RvcVoiceSample>> {
        let dir = self.layout(id).sample_metadata;
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut samples = Vec::new();
        for entry in fs::read_dir(dir).map_err(storage_error)? {
            let path = entry.map_err(storage_error)?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("json") {
                samples.push(read_json(&path)?);
            }
        }
        samples.sort_by(|a: &RvcVoiceSample, b| a.imported_at.cmp(&b.imported_at));
        Ok(samples)
    }

    pub fn save_sample_inspection(
        &self,
        mut sample: RvcVoiceSample,
        inspection: RvcAudioInspection,
        warnings: Vec<RvcSampleWarning>,
        valid: bool,
    ) -> TakokitResult<RvcVoiceSample> {
        sample.inspection = Some(inspection);
        sample.warnings = warnings;
        sample.state = if valid { RvcSampleState::Inspected } else { RvcSampleState::Invalid };
        let path = self.layout(sample.voice_id).sample_metadata.join(format!("{}.json", sample.id));
        atomic_json(&path, &sample)?;
        Ok(sample)
    }

    pub fn set_sample_included(&self, voice: &str, sample_id: Uuid, included: bool) -> TakokitResult<RvcVoiceSample> {
        let voice_id = self.resolve_id(voice)?;
        let path = self.layout(voice_id).sample_metadata.join(format!("{sample_id}.json"));
        let mut sample: RvcVoiceSample = read_json(&path)?;
        sample.included = included;
        atomic_json(&path, &sample)?;
        Ok(sample)
    }

    pub fn remove_sample(&self, voice: &str, sample_id: Uuid) -> TakokitResult<()> {
        let voice_id = self.resolve_id(voice)?;
        let layout = self.layout(voice_id);
        let metadata = layout.sample_metadata.join(format!("{sample_id}.json"));
        let sample: RvcVoiceSample = read_json(&metadata)?;
        if sample.managed_path.starts_with(&layout.root) && sample.managed_path.is_file() {
            fs::remove_file(&sample.managed_path).map_err(storage_error)?;
        }
        fs::remove_file(metadata).map_err(storage_error)?;
        Ok(())
    }

    pub fn dataset_summary(&self, voice: &str) -> TakokitResult<RvcDatasetInspection> {
        let voice_id = self.resolve_id(voice)?;
        let samples = self.samples_id(voice_id)?;
        let included = samples.iter().filter(|sample| sample.included).collect::<Vec<_>>();
        let usable_duration_ms = included.iter().filter_map(|sample| sample.inspection.as_ref()?.duration_ms).sum();
        let mut warnings = included.iter().flat_map(|sample| sample.warnings.clone()).collect::<Vec<_>>();
        if included.is_empty() {
            warnings.push(RvcSampleWarning { code: "empty_dataset".into(), message: "Add at least one usable recording before preparing the dataset.".into() });
        }
        if usable_duration_ms > 0 && usable_duration_ms < 60_000 {
            warnings.push(RvcSampleWarning { code: "very_short_dataset".into(), message: "Usable speech is under one minute. RVC upstream guidance does not recommend datasets below one minute.".into() });
        }
        let ready = !included.is_empty()
            && included.iter().all(|sample| sample.state == RvcSampleState::Inspected)
            && included.iter().all(|sample| sample.inspection.as_ref().and_then(|value| value.duration_ms).unwrap_or(0) > 0);
        Ok(RvcDatasetInspection {
            voice_id,
            sample_count: samples.len(),
            included_sample_count: included.len(),
            usable_duration_ms,
            warning_count: warnings.len(),
            duplicate_count: 0,
            ready_for_preparation: ready,
            warnings,
            inspected_at: now_secs(),
        })
    }

    pub fn clear_prepared_dataset(&self, voice: &str) -> TakokitResult<()> {
        let id = self.resolve_id(voice)?;
        let layout = self.layout(id);
        for path in [&layout.samples_managed, &layout.dataset_segments, &layout.dataset_f0, &layout.dataset_features] {
            if path.exists() {
                fs::remove_dir_all(path).map_err(storage_error)?;
            }
            fs::create_dir_all(path).map_err(storage_error)?;
        }
        self.set_state(id, RvcVoiceProjectState::CollectingSamples, None)?;
        Ok(())
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
            if path.extension().and_then(|v| v.to_str()) == Some("json") {
                let job: RvcTrainingJob = read_json(&path)?;
                if matches!(job.status, RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running) {
                    jobs.push(job);
                }
            }
        }
        jobs.sort_by_key(|job| job.created_at);
        Ok(jobs.pop())
    }

    pub fn save_checkpoint(&self, checkpoint: &RvcCheckpoint) -> TakokitResult<()> {
        let path = self.layout(checkpoint.voice_id).checkpoints.join(format!("{}.json", checkpoint.id));
        atomic_json(&path, checkpoint)
    }

    pub fn checkpoints(&self, voice: &str) -> TakokitResult<Vec<RvcCheckpoint>> {
        let id = self.resolve_id(voice)?;
        read_metadata_dir(&self.layout(id).checkpoints)
    }

    pub fn save_index(&self, index: &RvcIndexArtifact) -> TakokitResult<()> {
        let path = self.layout(index.voice_id).indexes.join(format!("{}.json", index.id));
        atomic_json(&path, index)
    }

    pub fn indexes(&self, voice: &str) -> TakokitResult<Vec<RvcIndexArtifact>> {
        let id = self.resolve_id(voice)?;
        read_metadata_dir(&self.layout(id).indexes)
    }

    pub fn save_managed_voice(&self, voice: &ManagedRvcVoice) -> TakokitResult<()> {
        atomic_json(&self.layout(voice.project_id).voice, voice)
    }

    pub fn managed_voice(&self, voice: &str) -> TakokitResult<ManagedRvcVoice> {
        let id = self.resolve_id(voice)?;
        read_json(&self.layout(id).voice)
    }

    pub fn remove(&self, voice: &str, dry_run: bool) -> TakokitResult<Vec<PathBuf>> {
        let id = self.resolve_id(voice)?;
        if self.active_job(id)?.is_some() {
            return Err(TakokitError::InvalidRequest("cannot remove a voice while a training job is active; cancel it first".into()));
        }
        let root = self.layout(id).root;
        let files = if root.is_dir() {
            fs::read_dir(&root).map_err(storage_error)?.filter_map(Result::ok).map(|entry| entry.path()).collect()
        } else { Vec::new() };
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
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_name(name: &str) -> TakokitResult<String> {
    let value = name.trim();
    if value.is_empty() {
        return Err(TakokitError::InvalidRequest("voice name cannot be empty".to_string()));
    }
    if value.chars().count() > 120 {
        return Err(TakokitError::InvalidRequest("voice name cannot exceed 120 characters".to_string()));
    }
    Ok(value.to_string())
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn read_json<T: DeserializeOwned>(path: &Path) -> TakokitResult<T> {
    let bytes = fs::read(path).map_err(storage_error)?;
    serde_json::from_slice(&bytes).map_err(|error| TakokitError::Storage(format!("invalid metadata {}: {error}", path.display())))
}

fn atomic_json<T: Serialize>(path: &Path, value: &T) -> TakokitResult<()> {
    let parent = path.parent().ok_or_else(|| TakokitError::Storage(format!("metadata path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(storage_error)?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| TakokitError::Storage(error.to_string()))?;
    let mut file = File::create(&temporary).map_err(storage_error)?;
    file.write_all(&bytes).map_err(storage_error)?;
    file.sync_all().map_err(storage_error)?;
    if path.exists() {
        fs::remove_file(path).map_err(storage_error)?;
    }
    fs::rename(temporary, path).map_err(storage_error)
}

fn read_metadata_dir<T: DeserializeOwned>(dir: &Path) -> TakokitResult<Vec<T>> {
    if !dir.is_dir() { return Ok(Vec::new()); }
    let mut values = Vec::new();
    for entry in fs::read_dir(dir).map_err(storage_error)? {
        let path = entry.map_err(storage_error)?.path();
        if path.extension().and_then(|v| v.to_str()) == Some("json") {
            values.push(read_json(&path)?);
        }
    }
    Ok(values)
}

fn storage_error(error: std::io::Error) -> TakokitError {
    TakokitError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_unicode_voice_and_persists_layout() {
        let temp = TempDir::new().unwrap();
        let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
        let project = store.create("Voice ü 日本語", true, None).unwrap();
        let loaded = store.load(&project.id.to_string()).unwrap();
        assert_eq!(loaded.name, "Voice ü 日本語");
        assert!(store.layout(project.id).samples_originals.is_dir());
        assert!(store.layout(project.id).jobs.is_dir());
    }

    #[test]
    fn duplicate_names_are_allowed_but_ambiguous_by_name() {
        let temp = TempDir::new().unwrap();
        let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
        store.create("Same", true, None).unwrap();
        store.create("Same", true, None).unwrap();
        assert!(store.load("Same").is_err());
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn sample_import_deduplicates_by_hash_and_preserves_source() {
        let temp = TempDir::new().unwrap();
        let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
        let project = store.create("Voice", true, None).unwrap();
        let source = temp.path().join("sample ü.wav");
        fs::write(&source, b"not-real-audio-yet").unwrap();
        let added = store.add_samples(&project.id.to_string(), &[source.clone(), source.clone()]).unwrap();
        assert_eq!(added.len(), 1);
        assert!(source.is_file());
        assert!(added[0].managed_path.is_file());
        store.remove_sample(&project.id.to_string(), added[0].id).unwrap();
        assert!(source.is_file());
    }

    #[test]
    fn removing_voice_is_blocked_by_active_job() {
        let temp = TempDir::new().unwrap();
        let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
        let project = store.create("Voice", true, None).unwrap();
        let config = takokit_core::RvcTrainingConfig::preset(takokit_core::RvcTrainingPreset::Quick).unwrap();
        let job = RvcTrainingJob {
            id: Uuid::new_v4(), voice_id: project.id, config, status: RvcTrainingJobStatus::Running,
            stage: takokit_core::RvcTrainingStage::Train, created_at: now_secs(), started_at: Some(now_secs()),
            finished_at: None, owner_pid: None, child_pid: None, log_path: PathBuf::new(), checkpoint_ids: vec![],
            failure: None, cancellation_requested: false,
        };
        store.save_job(&job).unwrap();
        assert!(store.remove(&project.id.to_string(), false).is_err());
    }
}
