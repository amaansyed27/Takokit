use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::*;
use takokit_package::{resolve_execution_plan, InstalledRegistry, PackageRegistry};
use takokit_store::RvcVoiceStore;
use uuid::Uuid;

use crate::execute_voice_conversion;

mod artifacts;
mod jobs;
mod packages;

#[derive(Debug, Clone)]
pub struct RvcVoiceService {
    pub(super) root: PathBuf,
    pub(super) store: RvcVoiceStore,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkerInspection {
    duration_ms: Option<u64>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    codec: Option<String>,
    container: Option<String>,
    peak_dbfs: Option<f32>,
    rms_dbfs: Option<f32>,
    silence_ratio: Option<f32>,
    clipped_ratio: Option<f32>,
    #[serde(default)]
    warnings: Vec<RvcSampleWarning>,
    #[serde(default)]
    valid: bool,
}

impl RvcVoiceService {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            store: RvcVoiceStore::new(root.join("voices").join("rvc")),
            root,
        }
    }

    pub fn store(&self) -> &RvcVoiceStore {
        &self.store
    }

    pub fn presets(&self) -> Vec<RvcTrainingPresetInfo> {
        rvc_training_presets()
    }

    pub fn create(&self, request: CreateRvcVoiceRequest) -> TakokitResult<RvcVoiceProject> {
        self.store.create(
            &request.name,
            request.consent_affirmed,
            request.consent_note,
        )
    }

    pub fn list(&self) -> TakokitResult<Vec<RvcVoiceProject>> {
        self.reconcile_all()?;
        self.store.list()
    }

    pub fn show(&self, voice: &str) -> TakokitResult<RvcVoiceDetail> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let project = self.store.load_id(project.id)?;
        let voice_id = project.id.to_string();
        let target = self.conversion_target_id(project.id);
        Ok(RvcVoiceDetail {
            samples: self.store.samples_id(project.id)?,
            dataset: self.store.dataset_summary(&voice_id)?,
            managed: self.store.managed_voice(&voice_id).ok(),
            checkpoints: self.store.checkpoints(&voice_id)?,
            indexes: self.store.indexes(&voice_id)?,
            active_job: self.store.active_job(project.id)?,
            conversion_target: target.is_dir().then_some(target),
            project,
        })
    }

    pub fn add_samples(
        &self,
        voice: &str,
        request: AddRvcSamplesRequest,
    ) -> TakokitResult<Vec<RvcVoiceSample>> {
        self.ensure_idle(voice)?;
        self.store.add_samples(voice, &request.paths)
    }

    pub fn samples(&self, voice: &str) -> TakokitResult<Vec<RvcVoiceSample>> {
        self.store.samples(voice)
    }

    pub fn set_sample_included(
        &self,
        voice: &str,
        sample_id: Uuid,
        included: bool,
    ) -> TakokitResult<RvcVoiceSample> {
        self.ensure_idle(voice)?;
        self.store.set_sample_included(voice, sample_id, included)
    }

    pub fn remove_sample(&self, voice: &str, sample_id: Uuid) -> TakokitResult<()> {
        self.ensure_idle(voice)?;
        self.store.remove_sample(voice, sample_id)
    }

    pub fn clear_prepared(&self, voice: &str) -> TakokitResult<()> {
        self.ensure_idle(voice)?;
        self.store.clear_prepared_dataset(voice)
    }

    pub fn inspect_dataset(&self, voice: &str) -> TakokitResult<RvcDatasetInspection> {
        let project = self.store.load(voice)?;
        let samples = self.store.samples_id(project.id)?;
        if samples.is_empty() {
            return self.store.dataset_summary(voice);
        }
        for sample in samples {
            let request = json!({"operation": "inspect", "path": sample.managed_path});
            match self.run_worker(&request) {
                Ok(value) => {
                    let raw: WorkerInspection =
                        serde_json::from_value(value.get("inspection").cloned().ok_or_else(
                            || invalid("RVC inspection returned no inspection payload"),
                        )?)
                        .map_err(|error| {
                            invalid(format!("invalid RVC inspection response: {error}"))
                        })?;
                    self.store.save_sample_inspection(
                        sample,
                        RvcAudioInspection {
                            duration_ms: raw.duration_ms,
                            sample_rate: raw.sample_rate,
                            channels: raw.channels,
                            codec: raw.codec,
                            container: raw.container,
                            peak_dbfs: raw.peak_dbfs,
                            rms_dbfs: raw.rms_dbfs,
                            silence_ratio: raw.silence_ratio,
                            clipped_ratio: raw.clipped_ratio,
                        },
                        raw.warnings,
                        raw.valid,
                    )?;
                }
                Err(error) => {
                    self.store.save_sample_inspection(
                        sample,
                        RvcAudioInspection::default(),
                        vec![RvcSampleWarning {
                            code: "unreadable_audio".into(),
                            message: error.to_string(),
                        }],
                        false,
                    )?;
                }
            }
        }
        let summary = self.store.dataset_summary(voice)?;
        self.store.set_state(
            project.id,
            if summary.ready_for_preparation {
                RvcVoiceProjectState::ReadyForPreparation
            } else {
                RvcVoiceProjectState::CollectingSamples
            },
            None,
        )?;
        Ok(summary)
    }

    pub fn preflight(
        &self,
        voice: &str,
        config: RvcTrainingConfig,
    ) -> TakokitResult<RvcHardwarePreflight> {
        config.validate().map_err(invalid)?;
        let project = self.store.load(voice)?;
        let dataset = self.store.dataset_summary(voice)?;
        if !dataset.ready_for_preparation {
            return Err(invalid(
                "inspect the dataset and resolve invalid included samples before preflight",
            ));
        }
        let mut value = self.run_worker(&json!({
            "operation": "preflight",
            "voice_root": self.store.layout(project.id).root,
            "device": config.device,
            "precision": config.precision,
            "dataset_duration_ms": dataset.usable_duration_ms,
        }))?;
        let payload = value
            .get_mut("preflight")
            .ok_or_else(|| invalid("RVC preflight returned no payload"))?;
        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "selected_preset".into(),
                serde_json::to_value(config.preset).map_err(|error| invalid(error.to_string()))?,
            );
            if config.precision != RvcTrainingPrecision::Auto {
                object.insert(
                    "resolved_precision".into(),
                    serde_json::to_value(config.precision)
                        .map_err(|error| invalid(error.to_string()))?,
                );
            }
        }
        let result: RvcHardwarePreflight = serde_json::from_value(payload.clone())
            .map_err(|error| invalid(format!("invalid RVC preflight response: {error}")))?;
        if result.resolved_device == RvcTrainingDevice::Cpu
            && result.resolved_precision == RvcTrainingPrecision::Fp16
        {
            return Err(invalid("verified CPU training requires fp32 precision"));
        }
        Ok(result)
    }

    pub async fn test_voice(
        &self,
        voice: &str,
        input: PathBuf,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
        output_dir: &Path,
    ) -> TakokitResult<VoiceConversionResponse> {
        let project = self.store.load(voice)?;
        let target = self.conversion_target_id(project.id);
        if !target.join("rvc.json").is_file() {
            return Err(invalid(
                "voice is not ready; select a valid checkpoint/index first",
            ));
        }
        let plan = resolve_execution_plan(
            package_registry,
            installed_registry,
            "rvc",
            CapabilityKind::VoiceConversion,
        )
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
        execute_voice_conversion(
            &plan,
            VoiceConversionRequest {
                model: "rvc".into(),
                source_path: input,
                target_voice: target.to_string_lossy().into_owned(),
                f0_method: RvcF0Method::Rmvpe,
                pitch_shift: 0,
                index_rate: 0.75,
                rms_mix_rate: 0.25,
                protect: 0.33,
                filter_radius: 3,
                consent_affirmed: true,
            },
            output_dir,
        )
        .await
    }

    pub fn remove(&self, voice: &str, dry_run: bool) -> TakokitResult<RvcVoiceRemovalPreview> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() {
            return Err(invalid(
                "cancel the active voice job before removing this project",
            ));
        }
        let (files, bytes) = tree_stats(&self.store.layout(project.id).root)?;
        if !dry_run {
            self.store.remove(voice, false)?;
        }
        Ok(RvcVoiceRemovalPreview {
            voice_id: project.id,
            name: project.name,
            bytes,
            files,
            dry_run,
            removed: !dry_run,
        })
    }
}

pub(super) fn write_atomic_json(path: &Path, value: &Value) -> TakokitResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(storage)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(value).map_err(|error| invalid(error.to_string()))?,
    )
    .map_err(storage)?;
    fs::rename(temporary, path).map_err(storage)
}

fn tree_stats(root: &Path) -> TakokitResult<(usize, u64)> {
    fn walk(path: &Path, files: &mut usize, bytes: &mut u64) -> std::io::Result<()> {
        if path.is_file() {
            *files += 1;
            *bytes += path.metadata()?.len();
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                walk(&entry?.path(), files, bytes)?;
            }
        }
        Ok(())
    }
    let (mut files, mut bytes) = (0, 0);
    walk(root, &mut files, &mut bytes).map_err(storage)?;
    Ok((files, bytes))
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) fn invalid(message: impl Into<String>) -> TakokitError {
    TakokitError::InvalidRequest(message.into())
}

pub(super) fn storage(error: std::io::Error) -> TakokitError {
    TakokitError::Storage(error.to_string())
}

#[cfg(test)]
mod package_security_tests;
#[cfg(test)]
mod tests;
