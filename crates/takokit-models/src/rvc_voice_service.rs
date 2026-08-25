use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::*;
use takokit_package::{
    install_python_adapter, python_managed_runner_layout, resolve_execution_plan, InstalledRegistry,
    PackageRegistry,
};
use takokit_store::{sha256_file, RvcVoiceStore};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::execute_voice_conversion;

const MAX_PACKAGE_FILES: usize = 16;
const MAX_PACKAGE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct RvcVoiceService {
    root: PathBuf,
    store: RvcVoiceStore,
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
        self.store
            .create(&request.name, request.consent_affirmed, request.consent_note)
    }

    pub fn list(&self) -> TakokitResult<Vec<RvcVoiceProject>> {
        self.reconcile_all()?;
        self.store.list()
    }

    pub fn show(&self, voice: &str) -> TakokitResult<RvcVoiceDetail> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let project = self.store.load_id(project.id)?;
        let samples = self.store.samples_id(project.id)?;
        let dataset = self.store.dataset_summary(&project.id.to_string())?;
        let managed = self.store.managed_voice(&project.id.to_string()).ok();
        let checkpoints = self.store.checkpoints(&project.id.to_string())?;
        let indexes = self.store.indexes(&project.id.to_string())?;
        let active_job = self.store.active_job(project.id)?;
        let target = self.conversion_target_id(project.id);
        Ok(RvcVoiceDetail {
            project,
            samples,
            dataset,
            managed,
            checkpoints,
            indexes,
            active_job,
            conversion_target: target.is_dir().then_some(target),
        })
    }

    pub fn add_samples(&self, voice: &str, request: AddRvcSamplesRequest) -> TakokitResult<Vec<RvcVoiceSample>> {
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
        self.ensure_training_adapter()?;
        let project = self.store.load(voice)?;
        let samples = self.store.samples_id(project.id)?;
        if samples.is_empty() {
            return self.store.dataset_summary(voice);
        }
        for sample in samples {
            let request = json!({"operation":"inspect","path":sample.managed_path});
            match self.run_worker(&request) {
                Ok(value) => {
                    let raw: WorkerInspection = serde_json::from_value(
                        value.get("inspection").cloned().ok_or_else(|| invalid("RVC inspection returned no inspection payload"))?,
                    )
                    .map_err(|error| invalid(format!("invalid RVC inspection response: {error}")))?;
                    let inspection = RvcAudioInspection {
                        duration_ms: raw.duration_ms,
                        sample_rate: raw.sample_rate,
                        channels: raw.channels,
                        codec: raw.codec,
                        container: raw.container,
                        peak_dbfs: raw.peak_dbfs,
                        rms_dbfs: raw.rms_dbfs,
                        silence_ratio: raw.silence_ratio,
                        clipped_ratio: raw.clipped_ratio,
                    };
                    self.store.save_sample_inspection(sample, inspection, raw.warnings, raw.valid)?;
                }
                Err(error) => {
                    self.store.save_sample_inspection(
                        sample,
                        RvcAudioInspection::default(),
                        vec![RvcSampleWarning { code: "unreadable_audio".into(), message: error.to_string() }],
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

    pub fn preflight(&self, voice: &str, config: RvcTrainingConfig) -> TakokitResult<RvcHardwarePreflight> {
        config.validate().map_err(invalid)?;
        self.ensure_training_adapter()?;
        let project = self.store.load(voice)?;
        let dataset = self.store.dataset_summary(voice)?;
        if !dataset.ready_for_preparation {
            return Err(invalid("inspect the dataset and resolve invalid included samples before preflight"));
        }
        let request = json!({
            "operation":"preflight",
            "voice_root":self.store.layout(project.id).root,
            "device":config.device,
            "precision":config.precision,
            "dataset_duration_ms":dataset.usable_duration_ms,
        });
        let mut value = self.run_worker(&request)?;
        let payload = value.get_mut("preflight").ok_or_else(|| invalid("RVC preflight returned no payload"))?;
        if let Some(object) = payload.as_object_mut() {
            object.insert("selected_preset".into(), serde_json::to_value(config.preset).unwrap());
            if config.precision != RvcTrainingPrecision::Auto {
                object.insert("resolved_precision".into(), serde_json::to_value(config.precision).unwrap());
            }
        }
        let result: RvcHardwarePreflight = serde_json::from_value(payload.clone())
            .map_err(|error| invalid(format!("invalid RVC preflight response: {error}")))?;
        if result.class == RvcPreflightClass::Unsupported {
            return Ok(result);
        }
        if result.resolved_device == RvcTrainingDevice::Cpu && result.resolved_precision == RvcTrainingPrecision::Fp16 {
            return Err(invalid("verified CPU training requires fp32 precision"));
        }
        Ok(result)
    }

    pub fn prepare(&self, voice: &str, request: StartRvcTrainingRequest) -> TakokitResult<RvcTrainingJob> {
        let config = request.resolve().map_err(invalid)?;
        self.launch_job(voice, config, true)
    }

    pub fn start_training(&self, voice: &str, request: StartRvcTrainingRequest) -> TakokitResult<RvcTrainingJob> {
        let config = request.resolve().map_err(invalid)?;
        self.launch_job(voice, config, false)
    }

    pub fn recover_training(&self, voice: &str) -> TakokitResult<RvcTrainingJob> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let project = self.store.load_id(project.id)?;
        let latest = project.latest_job_id.ok_or_else(|| invalid("voice has no prior training job to recover"))?;
        let previous = self.store.load_job(project.id, latest)?;
        if matches!(previous.status, RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running) {
            return Err(invalid("the existing job is still running"));
        }
        self.launch_job(&project.id.to_string(), previous.config, false)
    }

    pub fn training_status(&self, voice: &str) -> TakokitResult<Option<RvcTrainingJob>> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let project = self.store.load_id(project.id)?;
        match project.latest_job_id {
            Some(id) => self.store.load_job(project.id, id).map(Some),
            None => Ok(None),
        }
    }

    pub fn training_logs(&self, voice: &str, max_bytes: usize) -> TakokitResult<String> {
        let job = self.training_status(voice)?.ok_or_else(|| invalid("voice has no training job"))?;
        let mut file = File::open(&job.log_path).map_err(storage)?;
        let length = file.metadata().map_err(storage)?.len();
        if length > max_bytes as u64 {
            use std::io::{Seek, SeekFrom};
            file.seek(SeekFrom::Start(length - max_bytes as u64)).map_err(storage)?;
        }
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(storage)?;
        Ok(text)
    }

    pub fn cancel_training(&self, voice: &str) -> TakokitResult<RvcTrainingJob> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let mut job = self.store.active_job(project.id)?.ok_or_else(|| invalid("voice has no running preparation/training job"))?;
        job.cancellation_requested = true;
        self.store.save_job(&job)?;
        let pid = job.owner_pid.ok_or_else(|| invalid("job has no recorded Takokit worker PID"))?;
        let request_path = self.job_request_path(project.id, job.id);
        if !process_matches_job(pid, &request_path) {
            job.status = RvcTrainingJobStatus::Stale;
            job.failure = Some("recorded PID is not the Takokit-owned RVC worker; no process was terminated".into());
            job.finished_at = Some(now());
            self.store.save_job(&job)?;
            return Err(invalid("refused to terminate a PID that no longer belongs to this Takokit RVC job"));
        }
        terminate_owned_tree(pid)?;
        job.status = RvcTrainingJobStatus::Cancelled;
        job.finished_at = Some(now());
        job.failure = None;
        self.store.save_job(&job)?;
        self.store.set_state(project.id, RvcVoiceProjectState::Cancelled, None)?;
        Ok(job)
    }

    pub fn checkpoints(&self, voice: &str) -> TakokitResult<Vec<RvcCheckpoint>> {
        self.refresh_completed_artifacts(voice)?;
        self.store.checkpoints(voice)
    }

    pub fn select_checkpoint(&self, voice: &str, request: SelectRvcCheckpointRequest) -> TakokitResult<ManagedRvcVoice> {
        let project = self.store.load(voice)?;
        let checkpoints = self.store.checkpoints(voice)?;
        let checkpoint = checkpoints.into_iter().find(|item| item.id == request.checkpoint_id)
            .ok_or_else(|| invalid("checkpoint does not belong to this voice"))?;
        if !checkpoint.valid_for_inference || !checkpoint.path.is_file() {
            return Err(invalid("selected checkpoint is missing or invalid for inference"));
        }
        let index = match request.index_id {
            Some(id) => Some(self.store.indexes(voice)?.into_iter().find(|item| item.id == id)
                .ok_or_else(|| invalid("index does not belong to this voice"))?),
            None => None,
        };
        if let Some(index) = index.as_ref() {
            if !index.valid || !index.path.is_file() {
                return Err(invalid("selected index is missing or invalid"));
            }
            if let Some(pair) = index.checkpoint_id {
                if pair != checkpoint.id {
                    return Err(invalid("selected index is paired with a different checkpoint"));
                }
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

    pub fn import_existing(&self, request: ImportRvcVoiceRequest) -> TakokitResult<RvcVoiceProject> {
        validate_artifact_path(&request.checkpoint, "pth")?;
        if let Some(index) = request.index.as_ref() {
            validate_artifact_path(index, "index")?;
        }
        let mut project = self.store.create(&request.name, request.consent_affirmed, request.consent_note)?;
        project.imported = true;
        self.store.save_project(&project)?;
        self.import_artifacts(project.id, &request.checkpoint, request.index.as_deref(), Some(json!({
            "source_checkpoint":request.checkpoint,
            "source_index":request.index,
            "imported_at":now()
        })))?;
        Ok(self.store.load_id(project.id)?)
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
            return Err(invalid("voice is not ready; select a valid checkpoint/index first"));
        }
        let plan = resolve_execution_plan(package_registry, installed_registry, "rvc", CapabilityKind::VoiceConversion)
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

    pub fn export_package(&self, voice: &str, request: ExportRvcVoiceRequest) -> TakokitResult<PathBuf> {
        let project = self.store.load(voice)?;
        let runtime = self.conversion_target_id(project.id);
        let checkpoint = runtime.join("checkpoint.pth");
        let index = runtime.join("model.index");
        if !checkpoint.is_file() || !runtime.join("rvc.json").is_file() {
            return Err(invalid("voice has no ready managed checkpoint to export"));
        }
        if request.output.extension().and_then(|v| v.to_str()).map(|v| v.eq_ignore_ascii_case("takovoice")) != Some(true) {
            return Err(invalid("voice package output must use .takovoice"));
        }
        if let Some(parent) = request.output.parent() { fs::create_dir_all(parent).map_err(storage)?; }
        let reference = if request.include_reference {
            first_reference(&self.store.layout(project.id).references)
        } else { None };
        let manifest = RvcPackageManifest {
            schema_version: TAKOVOICE_SCHEMA_VERSION,
            engine: "rvc".into(),
            voice_name: project.name.clone(),
            exported_at: now(),
            checkpoint: package_artifact("checkpoint.pth", &checkpoint)?,
            index: index.is_file().then(|| package_artifact("model.index", &index)).transpose()?,
            reference: reference.as_ref().map(|path| package_artifact("reference.wav", path)).transpose()?,
            consent_acknowledged: true,
            provenance_note: "Takokit package provenance records local artifact integrity. It does not prove speaker identity, legal ownership, consent authenticity, or perceptual similarity.".into(),
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| invalid(e.to_string()))?;
        let file = File::create(&request.output).map_err(storage)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options).map_err(zip_error)?;
        zip.write_all(&manifest_bytes).map_err(storage)?;
        add_zip_file(&mut zip, "checkpoint.pth", &checkpoint, options)?;
        if index.is_file() { add_zip_file(&mut zip, "model.index", &index, options)?; }
        if let Some(reference) = reference.as_ref() { add_zip_file(&mut zip, "reference.wav", reference, options)?; }
        if request.sign {
            let signature = self.sign_manifest(&manifest_bytes)?;
            zip.start_file("signature.json", options).map_err(zip_error)?;
            zip.write_all(&serde_json::to_vec_pretty(&signature).map_err(|e| invalid(e.to_string()))?).map_err(storage)?;
        }
        zip.finish().map_err(zip_error)?;
        Ok(request.output)
    }

    pub fn verify_package(&self, package: &Path) -> TakokitResult<RvcPackageVerification> {
        let mut archive = open_package(package)?;
        validate_archive_bounds(&mut archive)?;
        let manifest_bytes = read_small_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
        let manifest: RvcPackageManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| invalid(format!("invalid voice package manifest: {e}")))?;
        let mut errors = Vec::new();
        if manifest.schema_version != TAKOVOICE_SCHEMA_VERSION { errors.push(format!("unsupported package schema {}", manifest.schema_version)); }
        if manifest.engine != "rvc" { errors.push("package engine is not rvc".into()); }
        let mut hashes_valid = true;
        for artifact in [&manifest.checkpoint, manifest.index.as_ref().unwrap_or(&manifest.checkpoint)].into_iter().take(if manifest.index.is_some(){2}else{1}) {
            match hash_zip_entry(&mut archive, artifact) {
                Ok(true) => {},
                Ok(false) => { hashes_valid = false; errors.push(format!("artifact hash/size mismatch: {}", artifact.path)); },
                Err(error) => { hashes_valid = false; errors.push(error.to_string()); },
            }
        }
        if let Some(reference) = manifest.reference.as_ref() {
            match hash_zip_entry(&mut archive, reference) {
                Ok(true) => {},
                Ok(false) => { hashes_valid = false; errors.push(format!("artifact hash/size mismatch: {}", reference.path)); },
                Err(error) => { hashes_valid = false; errors.push(error.to_string()); },
            }
        }
        let signature_bytes = read_optional_small_entry(&mut archive, "signature.json", MAX_MANIFEST_BYTES)?;
        let (signed, signature_valid, fingerprint) = match signature_bytes {
            Some(bytes) => match serde_json::from_slice::<RvcPackageSignature>(&bytes) {
                Ok(signature) => {
                    let valid = verify_signature(&manifest_bytes, &signature).unwrap_or(false);
                    if !valid { errors.push("voice package signature is invalid".into()); }
                    (true, Some(valid), Some(signature.signer_fingerprint))
                }
                Err(error) => { errors.push(format!("invalid signature metadata: {error}")); (true, Some(false), None) }
            },
            None => (false, None, None),
        };
        Ok(RvcPackageVerification {
            schema_version: manifest.schema_version,
            package_path: package.to_path_buf(),
            signed,
            signature_valid,
            signer_fingerprint: fingerprint,
            hashes_valid,
            voice_name: Some(manifest.voice_name),
            errors,
        })
    }

    pub fn import_package(&self, request: ImportRvcPackageRequest) -> TakokitResult<RvcVoiceProject> {
        if !request.consent_affirmed { return Err(invalid("import requires permission/provenance acknowledgement")); }
        let verification = self.verify_package(&request.package)?;
        if !verification.hashes_valid || verification.signature_valid == Some(false) || !verification.errors.is_empty() {
            return Err(invalid(format!("voice package verification failed: {}", verification.errors.join("; "))));
        }
        let mut archive = open_package(&request.package)?;
        let manifest_bytes = read_small_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
        let manifest: RvcPackageManifest = serde_json::from_slice(&manifest_bytes).map_err(|e| invalid(e.to_string()))?;
        let name = request.name.unwrap_or(manifest.voice_name);
        let mut project = self.store.create(&name, true, request.consent_note)?;
        project.imported = true;
        self.store.save_project(&project)?;
        let layout = self.store.layout(project.id);
        let temporary = layout.packages.join(format!("import-{}", Uuid::new_v4()));
        fs::create_dir_all(&temporary).map_err(storage)?;
        let checkpoint = temporary.join("checkpoint.pth");
        extract_entry(&mut archive, &manifest.checkpoint, &checkpoint)?;
        let index = if let Some(meta) = manifest.index.as_ref() {
            let path = temporary.join("model.index");
            extract_entry(&mut archive, meta, &path)?;
            Some(path)
        } else { None };
        self.import_artifacts(project.id, &checkpoint, index.as_deref(), Some(json!({
            "package":request.package,
            "signed":verification.signed,
            "signer_fingerprint":verification.signer_fingerprint,
            "imported_at":now()
        })))?;
        let _ = fs::remove_dir_all(temporary);
        self.store.load_id(project.id)
    }

    pub fn remove(&self, voice: &str, dry_run: bool) -> TakokitResult<RvcVoiceRemovalPreview> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() { return Err(invalid("cancel the active voice job before removing this project")); }
        let (files, bytes) = tree_stats(&self.store.layout(project.id).root)?;
        if !dry_run { self.store.remove(voice, false)?; }
        Ok(RvcVoiceRemovalPreview { voice_id: project.id, name: project.name, bytes, files, dry_run, removed: !dry_run })
    }

    fn launch_job(&self, voice: &str, config: RvcTrainingConfig, prepare_only: bool) -> TakokitResult<RvcTrainingJob> {
        config.validate().map_err(invalid)?;
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() { return Err(invalid("this voice already has an active preparation/training job")); }
        let dataset = self.store.dataset_summary(voice)?;
        if !dataset.ready_for_preparation { return Err(invalid("inspect the dataset and resolve invalid included samples before starting")); }
        let preflight = self.preflight(voice, config.clone())?;
        if preflight.class == RvcPreflightClass::Unsupported { return Err(invalid(format!("RVC training preflight is unsupported: {}", preflight.reasons.join("; ")))); }
        let paths = self.training_paths()?;
        let job_id = Uuid::new_v4();
        let layout = self.store.layout(project.id);
        let log_path = layout.logs.join(format!("{job_id}.log"));
        let job_path = layout.jobs.join(format!("{job_id}.json"));
        let request_path = self.job_request_path(project.id, job_id);
        let mut job = RvcTrainingJob {
            id: job_id,
            voice_id: project.id,
            config: config.clone(),
            status: RvcTrainingJobStatus::Queued,
            stage: RvcTrainingStage::ValidateSamples,
            created_at: now(),
            started_at: None,
            finished_at: None,
            owner_pid: None,
            child_pid: None,
            log_path: log_path.clone(),
            checkpoint_ids: Vec::new(),
            failure: None,
            cancellation_requested: false,
        };
        self.store.save_job(&job)?;
        let samples = self.store.samples_id(project.id)?.into_iter().filter(|sample| sample.included && sample.state == RvcSampleState::Inspected)
            .map(|sample| json!({"path":sample.managed_path,"sha256":sample.sha256})).collect::<Vec<_>>();
        let payload = json!({
            "operation": if prepare_only {"prepare"} else {"train"},
            "prepare_only":prepare_only,
            "voice_id":project.id,
            "voice_root":layout.root,
            "trainer_root":paths.trainer_root,
            "asset_root":paths.asset_root,
            "job_path":job_path,
            "log_path":log_path,
            "config":config,
            "samples":samples,
            "resolved_device":preflight.resolved_device,
            "resolved_precision":preflight.resolved_precision,
        });
        write_atomic_json(&request_path, &payload)?;
        let mut command = Command::new(&paths.python);
        command.arg(&paths.script).arg("--job").arg(&request_path)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000 | 0x0000_0200);
        }
        let child = command.spawn().map_err(|error| invalid(format!("failed to start managed RVC worker: {error}")))?;
        job.owner_pid = Some(child.id());
        self.store.save_job(&job)?;
        let mut project = project;
        project.latest_job_id = Some(job_id);
        project.state = RvcVoiceProjectState::Preprocessing;
        project.last_error = None;
        self.store.save_project(&project)?;
        Ok(job)
    }

    fn reconcile_all(&self) -> TakokitResult<()> {
        for project in self.store.list()? { self.reconcile_job(project.id)?; }
        Ok(())
    }

    fn reconcile_job(&self, voice_id: Uuid) -> TakokitResult<()> {
        let project = self.store.load_id(voice_id)?;
        let Some(job_id) = project.latest_job_id else { return Ok(()); };
        let mut job = match self.store.load_job(voice_id, job_id) { Ok(job) => job, Err(_) => return Ok(()) };
        if matches!(job.status, RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running) {
            if let Some(pid) = job.owner_pid {
                if !process_is_running(pid) {
                    job.status = RvcTrainingJobStatus::Stale;
                    job.finished_at = Some(now());
                    job.failure = Some("Takokit restarted or the managed worker exited before recording a terminal result. Retained RVC checkpoints can be recovered with Recover training.".into());
                    self.store.save_job(&job)?;
                    self.store.set_state(voice_id, RvcVoiceProjectState::Failed, job.failure.clone())?;
                }
            }
        }
        let current = self.store.load_job(voice_id, job_id)?;
        match current.status {
            RvcTrainingJobStatus::Succeeded => {
                if current.stage == RvcTrainingStage::ReadyToTrain {
                    self.store.set_state(voice_id, RvcVoiceProjectState::ReadyToTrain, None)?;
                } else {
                    self.refresh_completed_artifacts(&voice_id.to_string())?;
                }
            }
            RvcTrainingJobStatus::Failed => { self.store.set_state(voice_id, RvcVoiceProjectState::Failed, current.failure.clone())?; }
            RvcTrainingJobStatus::Cancelled => { self.store.set_state(voice_id, RvcVoiceProjectState::Cancelled, None)?; }
            _ => {}
        }
        Ok(())
    }

    fn refresh_completed_artifacts(&self, voice: &str) -> TakokitResult<()> {
        let project = self.store.load(voice)?;
        let result_path = self.store.layout(project.id).jobs.join("latest-result.json");
        if !result_path.is_file() { return Ok(()); }
        let value: Value = serde_json::from_reader(File::open(&result_path).map_err(storage)?).map_err(|e| invalid(e.to_string()))?;
        let checkpoint_path = value.get("checkpoint").and_then(Value::as_str).map(PathBuf::from).ok_or_else(|| invalid("RVC worker result is missing checkpoint"))?;
        let index_path = value.get("index").and_then(Value::as_str).map(PathBuf::from);
        if !checkpoint_path.is_file() { return Err(invalid("RVC worker checkpoint is missing")); }
        let checkpoint_hash = sha256_file(&checkpoint_path)?;
        let existing = self.store.checkpoints(voice)?.into_iter().find(|item| item.sha256 == checkpoint_hash);
        let checkpoint = match existing {
            Some(item) => item,
            None => {
                let item = RvcCheckpoint { id: Uuid::new_v4(), voice_id: project.id, path: checkpoint_path.clone(), sha256: checkpoint_hash, bytes: fs::metadata(&checkpoint_path).map_err(storage)?.len(), epoch: None, sample_rate_hz: Some(40_000), model_version: Some("v2".into()), f0: Some(true), created_at: now(), valid_for_inference: true };
                self.store.save_checkpoint(&item)?;
                item
            }
        };
        let index = if let Some(path) = index_path.filter(|path| path.is_file()) {
            let hash = sha256_file(&path)?;
            match self.store.indexes(voice)?.into_iter().find(|item| item.sha256 == hash) {
                Some(item) => Some(item),
                None => {
                    let item = RvcIndexArtifact { id: Uuid::new_v4(), voice_id: project.id, path: path.clone(), sha256: hash, bytes: fs::metadata(&path).map_err(storage)?.len(), checkpoint_id: Some(checkpoint.id), created_at: now(), valid: true };
                    self.store.save_index(&item)?;
                    Some(item)
                }
            }
        } else { None };
        self.select_checkpoint(voice, SelectRvcCheckpointRequest { checkpoint_id: checkpoint.id, index_id: index.as_ref().map(|item| item.id) })?;
        Ok(())
    }

    fn import_artifacts(&self, voice_id: Uuid, checkpoint_source: &Path, index_source: Option<&Path>, provenance: Option<Value>) -> TakokitResult<()> {
        let project = self.store.load_id(voice_id)?;
        let layout = self.store.layout(voice_id);
        let checkpoint_id = Uuid::new_v4();
        let checkpoint_path = layout.checkpoints.join(format!("artifact-{checkpoint_id}.pth"));
        copy_or_link(checkpoint_source, &checkpoint_path)?;
        let checkpoint = RvcCheckpoint { id: checkpoint_id, voice_id, path: checkpoint_path.clone(), sha256: sha256_file(&checkpoint_path)?, bytes: fs::metadata(&checkpoint_path).map_err(storage)?.len(), epoch: None, sample_rate_hz: None, model_version: None, f0: None, created_at: now(), valid_for_inference: true };
        self.store.save_checkpoint(&checkpoint)?;
        let index = match index_source {
            Some(source) => {
                let id = Uuid::new_v4();
                let path = layout.indexes.join(format!("artifact-{id}.index"));
                copy_or_link(source, &path)?;
                let item = RvcIndexArtifact { id, voice_id, path: path.clone(), sha256: sha256_file(&path)?, bytes: fs::metadata(&path).map_err(storage)?.len(), checkpoint_id: Some(checkpoint_id), created_at: now(), valid: true };
                self.store.save_index(&item)?;
                Some(item)
            }
            None => None,
        };
        if let Some(provenance) = provenance { write_atomic_json(&layout.root.join("provenance.json"), &provenance)?; }
        self.select_checkpoint(&project.id.to_string(), SelectRvcCheckpointRequest { checkpoint_id, index_id: index.as_ref().map(|item| item.id) })?;
        Ok(())
    }

    fn materialize_runtime(&self, project: &RvcVoiceProject, checkpoint: &RvcCheckpoint, index: Option<&RvcIndexArtifact>) -> TakokitResult<()> {
        let runtime = self.conversion_target_id(project.id);
        let temporary = runtime.with_extension(format!("tmp-{}", Uuid::new_v4()));
        if temporary.exists() { fs::remove_dir_all(&temporary).map_err(storage)?; }
        fs::create_dir_all(&temporary).map_err(storage)?;
        copy_or_link(&checkpoint.path, &temporary.join("checkpoint.pth"))?;
        if let Some(index) = index { copy_or_link(&index.path, &temporary.join("model.index"))?; }
        let manifest = json!({
            "schema_version":1,
            "engine":"rvc",
            "checkpoint":"checkpoint.pth",
            "index": index.map(|_| "model.index"),
            "quality_baseline":false,
            "managed_voice_id":project.id,
            "note":"Artifacts validated for execution. Perceptual identity/similarity is not inferred from successful file generation."
        });
        write_atomic_json(&temporary.join("rvc.json"), &manifest)?;
        if runtime.exists() { fs::remove_dir_all(&runtime).map_err(storage)?; }
        fs::rename(&temporary, &runtime).map_err(storage)?;
        Ok(())
    }

    fn conversion_target_id(&self, id: Uuid) -> PathBuf {
        self.store.layout(id).root.join("runtime")
    }

    fn ensure_idle(&self, voice: &str) -> TakokitResult<()> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() { Err(invalid("voice has an active preparation/training job")) } else { Ok(()) }
    }

    fn ensure_training_adapter(&self) -> TakokitResult<()> {
        let layout = python_managed_runner_layout(&self.root);
        let adapter = layout.adapters.join("rvc_training");
        if !adapter.join("rvc_training.py").is_file() || !adapter_python(&adapter).is_file() {
            install_python_adapter(&self.root, "rvc_training").map_err(|error| TakokitError::Storage(error.to_string()))?;
        }
        Ok(())
    }

    fn training_paths(&self) -> TakokitResult<TrainingPaths> {
        self.ensure_training_adapter()?;
        let layout = python_managed_runner_layout(&self.root);
        let adapter = layout.adapters.join("rvc_training");
        let installed = InstalledRegistry::new(self.root.join("manifests"));
        let record = installed.installed_model_record("rvc").map_err(|_| invalid("RVC assets are not installed; run `tako pull rvc` before training"))?;
        let asset_root = record.snapshot.map(|snapshot| snapshot.local_path)
            .unwrap_or_else(|| self.root.join("models").join("rvc"));
        Ok(TrainingPaths { python: adapter_python(&adapter), script: adapter.join("rvc_training.py"), trainer_root: adapter.join("source"), asset_root })
    }

    fn run_worker(&self, request: &Value) -> TakokitResult<Value> {
        let paths = self.training_paths()?;
        let mut child = Command::new(paths.python)
            .arg(paths.script)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8")
            .spawn().map_err(|e| invalid(format!("failed to start RVC training adapter: {e}")))?;
        serde_json::to_writer(child.stdin.take().ok_or_else(|| invalid("RVC adapter stdin unavailable"))?, request).map_err(|e| invalid(e.to_string()))?;
        let output = child.wait_with_output().map_err(storage)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value: Value = serde_json::from_str(stdout.lines().last().unwrap_or("{}"))
            .map_err(|e| invalid(format!("invalid RVC adapter response: {e}; stderr: {}", String::from_utf8_lossy(&output.stderr))))?;
        if !output.status.success() || value.get("ok") == Some(&Value::Bool(false)) {
            return Err(invalid(value.get("error").and_then(Value::as_str).unwrap_or("RVC adapter failed")));
        }
        Ok(value)
    }

    fn job_request_path(&self, voice_id: Uuid, job_id: Uuid) -> PathBuf {
        self.store.layout(voice_id).jobs.join(format!("{job_id}.request.json"))
    }

    fn sign_manifest(&self, manifest: &[u8]) -> TakokitResult<RvcPackageSignature> {
        let directory = self.root.join("keys").join("voice-packages");
        fs::create_dir_all(&directory).map_err(storage)?;
        let path = directory.join("ed25519.key");
        let signing = if path.is_file() {
            let text = fs::read_to_string(&path).map_err(storage)?;
            let bytes = hex::decode(text.trim()).map_err(|e| invalid(format!("invalid voice signing key: {e}")))?;
            let array: [u8; 32] = bytes.try_into().map_err(|_| invalid("voice signing key has invalid length"))?;
            SigningKey::from_bytes(&array)
        } else {
            let key = SigningKey::generate(&mut OsRng);
            fs::write(&path, hex::encode(key.to_bytes())).map_err(storage)?;
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(storage)?;
            }
            key
        };
        let verifying = signing.verifying_key();
        let signature = signing.sign(manifest);
        let fingerprint = hex::encode(Sha256::digest(verifying.as_bytes()));
        Ok(RvcPackageSignature { algorithm: "Ed25519".into(), public_key_hex: hex::encode(verifying.as_bytes()), signature_hex: hex::encode(signature.to_bytes()), signer_fingerprint: fingerprint })
    }
}

#[derive(Debug)]
struct TrainingPaths { python: PathBuf, script: PathBuf, trainer_root: PathBuf, asset_root: PathBuf }

fn adapter_python(adapter: &Path) -> PathBuf {
    #[cfg(windows)] { adapter.join("venv").join("Scripts").join("python.exe") }
    #[cfg(not(windows))] { adapter.join("venv").join("bin").join("python") }
}

fn validate_artifact_path(path: &Path, extension: &str) -> TakokitResult<()> {
    if !path.is_file() { return Err(invalid(format!("artifact does not exist: {}", path.display()))); }
    if path.extension().and_then(|v| v.to_str()).map(|v| v.eq_ignore_ascii_case(extension)) != Some(true) {
        return Err(invalid(format!("expected .{extension} artifact: {}", path.display())));
    }
    if fs::metadata(path).map_err(storage)?.len() == 0 { return Err(invalid(format!("artifact is empty: {}", path.display()))); }
    Ok(())
}

fn copy_or_link(source: &Path, destination: &Path) -> TakokitResult<()> {
    if let Some(parent) = destination.parent() { fs::create_dir_all(parent).map_err(storage)?; }
    if destination.exists() { fs::remove_file(destination).map_err(storage)?; }
    if fs::hard_link(source, destination).is_err() { fs::copy(source, destination).map_err(storage)?; }
    Ok(())
}

fn package_artifact(name: &str, path: &Path) -> TakokitResult<RvcPackageArtifact> {
    Ok(RvcPackageArtifact { path: name.into(), sha256: sha256_file(path)?, bytes: fs::metadata(path).map_err(storage)?.len() })
}

fn add_zip_file(zip: &mut ZipWriter<File>, name: &str, path: &Path, options: SimpleFileOptions) -> TakokitResult<()> {
    zip.start_file(name, options).map_err(zip_error)?;
    let mut source = File::open(path).map_err(storage)?;
    std::io::copy(&mut source, zip).map_err(storage)?;
    Ok(())
}

fn open_package(path: &Path) -> TakokitResult<ZipArchive<File>> {
    if path.extension().and_then(|v| v.to_str()).map(|v| v.eq_ignore_ascii_case("takovoice")) != Some(true) { return Err(invalid("voice package must use .takovoice")); }
    ZipArchive::new(File::open(path).map_err(storage)?).map_err(zip_error)
}

fn validate_archive_bounds(archive: &mut ZipArchive<File>) -> TakokitResult<()> {
    if archive.len() == 0 || archive.len() > MAX_PACKAGE_FILES { return Err(invalid("voice package contains an invalid number of files")); }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(zip_error)?;
        if file.enclosed_name().is_none() { return Err(invalid(format!("unsafe package path: {}", file.name()))); }
        total = total.saturating_add(file.size());
        if total > MAX_PACKAGE_BYTES { return Err(invalid("voice package exceeds the 10 GiB safety bound")); }
    }
    Ok(())
}

fn read_small_entry(archive: &mut ZipArchive<File>, name: &str, max: u64) -> TakokitResult<Vec<u8>> {
    let mut file = archive.by_name(name).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() > max { return Err(invalid(format!("invalid package entry: {name}"))); }
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).map_err(storage)?;
    Ok(bytes)
}

fn read_optional_small_entry(archive: &mut ZipArchive<File>, name: &str, max: u64) -> TakokitResult<Option<Vec<u8>>> {
    match archive.by_name(name) {
        Ok(mut file) => {
            if file.enclosed_name().is_none() || file.size() > max { return Err(invalid(format!("invalid package entry: {name}"))); }
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes).map_err(storage)?;
            Ok(Some(bytes))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(error) => Err(zip_error(error)),
    }
}

fn hash_zip_entry(archive: &mut ZipArchive<File>, artifact: &RvcPackageArtifact) -> TakokitResult<bool> {
    let mut file = archive.by_name(&artifact.path).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() != artifact.bytes { return Ok(false); }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(storage)?;
        if count == 0 { break; }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()) == artifact.sha256)
}

fn extract_entry(archive: &mut ZipArchive<File>, artifact: &RvcPackageArtifact, output: &Path) -> TakokitResult<()> {
    let mut file = archive.by_name(&artifact.path).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() != artifact.bytes { return Err(invalid("package artifact metadata mismatch")); }
    if let Some(parent) = output.parent() { fs::create_dir_all(parent).map_err(storage)?; }
    let mut target = File::create(output).map_err(storage)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(storage)?;
        if count == 0 { break; }
        hash.update(&buffer[..count]);
        target.write_all(&buffer[..count]).map_err(storage)?;
    }
    if hex::encode(hash.finalize()) != artifact.sha256 { let _ = fs::remove_file(output); return Err(invalid("package artifact hash changed during extraction")); }
    Ok(())
}

fn verify_signature(manifest: &[u8], signature: &RvcPackageSignature) -> Result<bool, String> {
    if signature.algorithm != "Ed25519" { return Ok(false); }
    let public = hex::decode(&signature.public_key_hex).map_err(|e| e.to_string())?;
    let public: [u8; 32] = public.try_into().map_err(|_| "invalid public key length".to_string())?;
    let verifying = VerifyingKey::from_bytes(&public).map_err(|e| e.to_string())?;
    let signature_bytes = hex::decode(&signature.signature_hex).map_err(|e| e.to_string())?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|e| e.to_string())?;
    let fingerprint = hex::encode(Sha256::digest(verifying.as_bytes()));
    Ok(fingerprint == signature.signer_fingerprint && verifying.verify(manifest, &signature).is_ok())
}

fn first_reference(root: &Path) -> Option<PathBuf> {
    fs::read_dir(root).ok()?.filter_map(Result::ok).map(|entry| entry.path()).find(|path| path.is_file())
}

fn write_atomic_json(path: &Path, value: &Value) -> TakokitResult<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(storage)?; }
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    fs::write(&temporary, serde_json::to_vec_pretty(value).map_err(|e| invalid(e.to_string()))?).map_err(storage)?;
    fs::rename(temporary, path).map_err(storage)
}

fn tree_stats(root: &Path) -> TakokitResult<(usize, u64)> {
    fn walk(path: &Path, files: &mut usize, bytes: &mut u64) -> std::io::Result<()> {
        if path.is_file() { *files += 1; *bytes += path.metadata()?.len(); return Ok(()); }
        if path.is_dir() { for entry in fs::read_dir(path)? { walk(&entry?.path(), files, bytes)?; } }
        Ok(())
    }
    let (mut files, mut bytes) = (0, 0);
    walk(root, &mut files, &mut bytes).map_err(storage)?;
    Ok((files, bytes))
}

fn process_is_running(pid: u32) -> bool {
    #[cfg(windows)] {
        Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"]).output()
            .ok().is_some_and(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
    }
    #[cfg(not(windows))] {
        PathBuf::from(format!("/proc/{pid}")).exists() || Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
    }
}

fn process_matches_job(pid: u32, request_path: &Path) -> bool {
    if !process_is_running(pid) { return false; }
    #[cfg(windows)] {
        let command = format!("(Get-CimInstance Win32_Process -Filter \"ProcessId = {pid}\").CommandLine");
        return Command::new("powershell.exe").args(["-NoProfile", "-Command", &command]).output().ok()
            .is_some_and(|out| String::from_utf8_lossy(&out.stdout).contains(&request_path.to_string_lossy().to_string()) && String::from_utf8_lossy(&out.stdout).contains("rvc_training.py"));
    }
    #[cfg(not(windows))] {
        fs::read(format!("/proc/{pid}/cmdline")).ok().is_some_and(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            text.contains("rvc_training.py") && text.contains(&request_path.to_string_lossy().to_string())
        })
    }
}

fn terminate_owned_tree(pid: u32) -> TakokitResult<()> {
    #[cfg(windows)] {
        let status = Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).status().map_err(storage)?;
        if !status.success() { return Err(invalid(format!("taskkill could not terminate Takokit RVC job PID {pid}"))); }
    }
    #[cfg(not(windows))] {
        let status = Command::new("kill").args(["-TERM", &pid.to_string()]).status().map_err(storage)?;
        if !status.success() { return Err(invalid(format!("could not terminate Takokit RVC job PID {pid}"))); }
    }
    Ok(())
}

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
fn invalid(message: impl Into<String>) -> TakokitError { TakokitError::InvalidRequest(message.into()) }
fn storage(error: std::io::Error) -> TakokitError { TakokitError::Storage(error.to_string()) }
fn zip_error(error: zip::result::ZipError) -> TakokitError { TakokitError::Storage(error.to_string()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_manifest_never_escapes_conversion_root() {
        let temp = tempfile::tempdir().unwrap();
        let service = RvcVoiceService::new(temp.path());
        let project = service.create(CreateRvcVoiceRequest { name: "Voice ü".into(), consent_affirmed: true, consent_note: None }).unwrap();
        let layout = service.store.layout(project.id);
        let checkpoint_path = layout.checkpoints.join("a.pth");
        let index_path = layout.indexes.join("a.index");
        fs::write(&checkpoint_path, b"checkpoint").unwrap();
        fs::write(&index_path, b"index").unwrap();
        let checkpoint = RvcCheckpoint { id: Uuid::new_v4(), voice_id: project.id, path: checkpoint_path, sha256: "x".into(), bytes: 10, epoch: None, sample_rate_hz: Some(40_000), model_version: Some("v2".into()), f0: Some(true), created_at: now(), valid_for_inference: true };
        let index = RvcIndexArtifact { id: Uuid::new_v4(), voice_id: project.id, path: index_path, sha256: "y".into(), bytes: 5, checkpoint_id: Some(checkpoint.id), created_at: now(), valid: true };
        service.store.save_checkpoint(&checkpoint).unwrap();
        service.store.save_index(&index).unwrap();
        service.select_checkpoint(&project.id.to_string(), SelectRvcCheckpointRequest { checkpoint_id: checkpoint.id, index_id: Some(index.id) }).unwrap();
        let manifest: Value = serde_json::from_reader(File::open(service.conversion_target_id(project.id).join("rvc.json")).unwrap()).unwrap();
        assert_eq!(manifest["checkpoint"], "checkpoint.pth");
        assert_eq!(manifest["index"], "model.index");
        assert!(!manifest.to_string().contains(".."));
    }

    #[test]
    fn unsigned_package_roundtrip_verifies_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let service = RvcVoiceService::new(temp.path());
        let source = temp.path().join("model.pth");
        fs::write(&source, b"model").unwrap();
        let project = service.import_existing(ImportRvcVoiceRequest { checkpoint: source, index: None, name: "Pack".into(), consent_affirmed: true, consent_note: None }).unwrap();
        let package = temp.path().join("voice.takovoice");
        service.export_package(&project.id.to_string(), ExportRvcVoiceRequest { output: package.clone(), sign: false, include_reference: false }).unwrap();
        let report = service.verify_package(&package).unwrap();
        assert!(report.hashes_valid);
        assert!(!report.signed);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn signed_package_detects_tampering_through_hash_or_signature() {
        let temp = tempfile::tempdir().unwrap();
        let service = RvcVoiceService::new(temp.path());
        let source = temp.path().join("model.pth");
        fs::write(&source, b"model").unwrap();
        let project = service.import_existing(ImportRvcVoiceRequest { checkpoint: source, index: None, name: "Signed".into(), consent_affirmed: true, consent_note: None }).unwrap();
        let package = temp.path().join("voice.takovoice");
        service.export_package(&project.id.to_string(), ExportRvcVoiceRequest { output: package.clone(), sign: true, include_reference: false }).unwrap();
        let report = service.verify_package(&package).unwrap();
        assert_eq!(report.signature_valid, Some(true));
    }
}
