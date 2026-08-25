use super::*;
use takokit_core::{RvcAudioInspection, RvcDatasetInspection, RvcSampleState, RvcSampleWarning, RvcVoiceSample};

impl RvcVoiceStore {
    pub fn add_samples(
        &self,
        voice: &str,
        paths: &[PathBuf],
    ) -> TakokitResult<Vec<RvcVoiceSample>> {
        if paths.is_empty() {
            return Err(TakokitError::InvalidRequest(
                "at least one audio file is required".into(),
            ));
        }
        let id = self.resolve_id(voice)?;
        let layout = self.layout(id);
        layout.ensure()?;
        let existing = self.samples_id(id)?;
        let mut added = Vec::new();
        for input in paths {
            let source = input.canonicalize().map_err(|error| {
                TakokitError::InvalidRequest(format!(
                    "audio file {} cannot be read: {error}",
                    input.display()
                ))
            })?;
            if !source.is_file() {
                return Err(TakokitError::InvalidRequest(format!(
                    "audio sample is not a file: {}",
                    source.display()
                )));
            }
            let hash = sha256_file(&source)?;
            if existing
                .iter()
                .chain(added.iter())
                .any(|sample: &RvcVoiceSample| sample.sha256 == hash)
            {
                continue;
            }
            let sample_id = Uuid::new_v4();
            let extension = source
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("audio");
            let managed = layout
                .samples_originals
                .join(format!("{sample_id}.{extension}"));
            fs::copy(&source, &managed).map_err(storage_error)?;
            let sample = RvcVoiceSample {
                id: sample_id,
                voice_id: id,
                display_name: source
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("sample")
                    .to_string(),
                source_path: source,
                managed_path: managed.clone(),
                sha256: hash,
                bytes: fs::metadata(&managed).map_err(storage_error)?.len(),
                imported_at: now_secs(),
                included: true,
                state: RvcSampleState::Imported,
                inspection: None,
                warnings: Vec::new(),
            };
            atomic_json(
                &layout.sample_metadata.join(format!("{sample_id}.json")),
                &sample,
            )?;
            added.push(sample);
        }
        if !added.is_empty() {
            self.set_state(id, RvcVoiceProjectState::CollectingSamples, None)?;
        }
        Ok(added)
    }

    pub fn samples(&self, voice: &str) -> TakokitResult<Vec<RvcVoiceSample>> {
        self.samples_id(self.resolve_id(voice)?)
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
        samples.sort_by_key(|sample: &RvcVoiceSample| sample.imported_at);
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
        sample.state = if valid {
            RvcSampleState::Inspected
        } else {
            RvcSampleState::Invalid
        };
        atomic_json(
            &self
                .layout(sample.voice_id)
                .sample_metadata
                .join(format!("{}.json", sample.id)),
            &sample,
        )?;
        Ok(sample)
    }

    pub fn set_sample_included(
        &self,
        voice: &str,
        sample_id: Uuid,
        included: bool,
    ) -> TakokitResult<RvcVoiceSample> {
        let voice_id = self.resolve_id(voice)?;
        let path = self
            .layout(voice_id)
            .sample_metadata
            .join(format!("{sample_id}.json"));
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
        fs::remove_file(metadata).map_err(storage_error)
    }

    pub fn dataset_summary(&self, voice: &str) -> TakokitResult<RvcDatasetInspection> {
        let voice_id = self.resolve_id(voice)?;
        let samples = self.samples_id(voice_id)?;
        let included = samples
            .iter()
            .filter(|sample| sample.included)
            .collect::<Vec<_>>();
        let usable_duration_ms = included
            .iter()
            .filter_map(|sample| sample.inspection.as_ref()?.duration_ms)
            .sum();
        let mut warnings = included
            .iter()
            .flat_map(|sample| sample.warnings.clone())
            .collect::<Vec<_>>();
        if included.is_empty() {
            warnings.push(RvcSampleWarning {
                code: "empty_dataset".into(),
                message: "Add at least one usable recording before preparing the dataset.".into(),
            });
        }
        if usable_duration_ms > 0 && usable_duration_ms < 60_000 {
            warnings.push(RvcSampleWarning {
                code: "very_short_dataset".into(),
                message: "Usable speech is under one minute. RVC upstream guidance does not recommend datasets below one minute.".into(),
            });
        }
        let ready = !included.is_empty()
            && included
                .iter()
                .all(|sample| sample.state == RvcSampleState::Inspected)
            && included.iter().all(|sample| {
                sample
                    .inspection
                    .as_ref()
                    .and_then(|value| value.duration_ms)
                    .unwrap_or(0)
                    > 0
            });
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
        for path in [
            &layout.samples_managed,
            &layout.dataset_segments,
            &layout.dataset_f0,
            &layout.dataset_features,
        ] {
            if path.exists() {
                fs::remove_dir_all(path).map_err(storage_error)?;
            }
            fs::create_dir_all(path).map_err(storage_error)?;
        }
        self.set_state(id, RvcVoiceProjectState::CollectingSamples, None)?;
        Ok(())
    }
}
