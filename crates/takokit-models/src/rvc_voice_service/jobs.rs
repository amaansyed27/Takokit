use super::*;
use serde_json::{json, Value};
#[cfg(not(windows))]
use std::fs;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use takokit_package::{
    install_python_adapter, python_adapter_is_current, python_managed_runner_layout,
    InstalledRegistry,
};

impl RvcVoiceService {
    pub fn prepare(
        &self,
        voice: &str,
        request: StartRvcTrainingRequest,
    ) -> TakokitResult<RvcTrainingJob> {
        self.launch_job(voice, request.resolve().map_err(invalid)?, true)
    }

    pub fn start_training(
        &self,
        voice: &str,
        request: StartRvcTrainingRequest,
    ) -> TakokitResult<RvcTrainingJob> {
        self.launch_job(voice, request.resolve().map_err(invalid)?, false)
    }

    pub fn recover_training(&self, voice: &str) -> TakokitResult<RvcTrainingJob> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let project = self.store.load_id(project.id)?;
        let latest = project
            .latest_job_id
            .ok_or_else(|| invalid("voice has no prior training job to recover"))?;
        let previous = self.store.load_job(project.id, latest)?;
        if matches!(
            previous.status,
            RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running
        ) {
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
        let job = self
            .training_status(voice)?
            .ok_or_else(|| invalid("voice has no training job"))?;
        let mut file = File::open(&job.log_path).map_err(storage)?;
        let length = file.metadata().map_err(storage)?.len();
        if length > max_bytes as u64 {
            use std::io::{Seek, SeekFrom};
            file.seek(SeekFrom::Start(length - max_bytes as u64))
                .map_err(storage)?;
        }
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(storage)?;
        Ok(text)
    }

    pub fn cancel_training(&self, voice: &str) -> TakokitResult<RvcTrainingJob> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        let mut job = self
            .store
            .active_job(project.id)?
            .ok_or_else(|| invalid("voice has no running preparation/training job"))?;
        job.cancellation_requested = true;
        self.store.save_job(&job)?;
        let pid = job
            .owner_pid
            .ok_or_else(|| invalid("job has no recorded Takokit worker PID"))?;
        let request_path = self.job_request_path(project.id, job.id);
        if !process_matches_job(pid, &request_path) {
            job.status = RvcTrainingJobStatus::Stale;
            job.failure = Some(
                "recorded PID is not the Takokit-owned RVC worker; no process was terminated"
                    .into(),
            );
            job.finished_at = Some(now());
            self.store.save_job(&job)?;
            return Err(invalid(
                "refused to terminate a PID that no longer belongs to this Takokit RVC job",
            ));
        }
        terminate_owned_tree(pid)?;
        job.status = RvcTrainingJobStatus::Cancelled;
        job.finished_at = Some(now());
        job.failure = None;
        self.store.save_job(&job)?;
        self.store
            .set_state(project.id, RvcVoiceProjectState::Cancelled, None)?;
        Ok(job)
    }

    pub(super) fn ensure_idle(&self, voice: &str) -> TakokitResult<()> {
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() {
            Err(invalid("voice has an active preparation/training job"))
        } else {
            Ok(())
        }
    }

    pub(super) fn reconcile_all(&self) -> TakokitResult<()> {
        for project in self.store.list()? {
            self.reconcile_job(project.id)?;
        }
        Ok(())
    }

    pub(super) fn reconcile_job(&self, voice_id: Uuid) -> TakokitResult<()> {
        let project = self.store.load_id(voice_id)?;
        let Some(job_id) = project.latest_job_id else {
            return Ok(());
        };
        let mut job = match self.store.load_job(voice_id, job_id) {
            Ok(job) => job,
            Err(_) => return Ok(()),
        };
        if matches!(
            job.status,
            RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running
        ) {
            match job.owner_pid {
                Some(pid) if process_is_running(pid) => {}
                Some(_) => {
                    job.status = RvcTrainingJobStatus::Stale;
                    job.finished_at = Some(now());
                    job.failure = Some("The Takokit-managed RVC worker exited before recording a terminal result. Retained upstream G/D checkpoints can be recovered with Recover training.".into());
                    self.store.save_job(&job)?;
                    self.store.set_state(
                        voice_id,
                        RvcVoiceProjectState::Failed,
                        job.failure.clone(),
                    )?;
                }
                None if job.status == RvcTrainingJobStatus::Queued => {}
                None => {
                    job.status = RvcTrainingJobStatus::Stale;
                    job.finished_at = Some(now());
                    job.failure = Some("running job has no Takokit worker PID".into());
                    self.store.save_job(&job)?;
                    self.store.set_state(
                        voice_id,
                        RvcVoiceProjectState::Failed,
                        job.failure.clone(),
                    )?;
                }
            }
        }
        let current = self.store.load_job(voice_id, job_id)?;
        match current.status {
            RvcTrainingJobStatus::Succeeded => {
                if current.stage == RvcTrainingStage::ReadyToTrain {
                    self.store
                        .set_state(voice_id, RvcVoiceProjectState::ReadyToTrain, None)?;
                } else {
                    self.refresh_completed_artifacts(&voice_id.to_string())?;
                }
            }
            RvcTrainingJobStatus::Failed | RvcTrainingJobStatus::Stale => {
                self.store.set_state(
                    voice_id,
                    RvcVoiceProjectState::Failed,
                    current.failure.clone(),
                )?;
            }
            RvcTrainingJobStatus::Cancelled => {
                self.store
                    .set_state(voice_id, RvcVoiceProjectState::Cancelled, None)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn run_worker(&self, request: &Value) -> TakokitResult<Value> {
        let paths = self.training_paths()?;
        let mut child = Command::new(paths.python)
            .arg(paths.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .spawn()
            .map_err(|error| execution(format!("failed to start RVC training adapter: {error}")))?;
        serde_json::to_writer(
            child
                .stdin
                .take()
                .ok_or_else(|| execution("RVC adapter stdin unavailable"))?,
            request,
        )
        .map_err(|error| execution(format!("failed to send RVC adapter request: {error}")))?;
        let output = child.wait_with_output().map_err(storage)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value: Value =
            serde_json::from_str(stdout.lines().last().unwrap_or("{}")).map_err(|error| {
                execution(format!(
                    "invalid RVC adapter response: {error}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            })?;
        if !output.status.success() || value.get("ok") == Some(&Value::Bool(false)) {
            let message = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("RVC adapter failed");
            return if value.get("error_kind").and_then(Value::as_str) == Some("audio_inspection") {
                Err(TakokitError::Audio(message.into()))
            } else {
                Err(execution(message))
            };
        }
        Ok(value)
    }

    fn launch_job(
        &self,
        voice: &str,
        config: RvcTrainingConfig,
        prepare_only: bool,
    ) -> TakokitResult<RvcTrainingJob> {
        config.validate().map_err(invalid)?;
        let project = self.store.load(voice)?;
        self.reconcile_job(project.id)?;
        if self.store.active_job(project.id)?.is_some() {
            return Err(invalid(
                "this voice already has an active preparation/training job",
            ));
        }
        let dataset = self.store.dataset_summary(voice)?;
        if !dataset.ready_for_preparation {
            return Err(invalid(
                "inspect the dataset and resolve invalid included samples before starting",
            ));
        }
        let preflight = self.preflight(voice, config.clone())?;
        if preflight.class == RvcPreflightClass::Unsupported {
            return Err(invalid(format!(
                "RVC training preflight is unsupported: {}",
                preflight.reasons.join("; ")
            )));
        }
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
        let samples = self
            .store
            .samples_id(project.id)?
            .into_iter()
            .filter(|sample| sample.included && sample.state == RvcSampleState::Inspected)
            .map(|sample| json!({"path": sample.managed_path, "sha256": sample.sha256}))
            .collect::<Vec<_>>();
        write_atomic_json(
            &request_path,
            &json!({
                "operation": if prepare_only { "prepare" } else { "train" },
                "prepare_only": prepare_only,
                "voice_id": project.id,
                "voice_root": layout.root,
                "trainer_root": paths.trainer_root,
                "asset_root": paths.asset_root,
                "job_path": job_path,
                "log_path": log_path,
                "config": config,
                "samples": samples,
                "resolved_device": preflight.resolved_device,
                "resolved_precision": preflight.resolved_precision,
            }),
        )?;
        let mut command = Command::new(&paths.python);
        command
            .arg(&paths.script)
            .arg("--job")
            .arg(&request_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000 | 0x0000_0200);
        }
        let child = command
            .spawn()
            .map_err(|error| invalid(format!("failed to start managed RVC worker: {error}")))?;
        job.owner_pid = Some(child.id());
        self.store.save_job(&job)?;
        let mut project = project;
        project.latest_job_id = Some(job_id);
        project.state = RvcVoiceProjectState::Preprocessing;
        project.last_error = None;
        self.store.save_project(&project)?;
        Ok(job)
    }

    fn ensure_training_adapter(&self) -> TakokitResult<()> {
        if python_adapter_is_current(&self.root, "rvc_training") {
            return Ok(());
        }
        install_python_adapter(&self.root, "rvc_training")
            .map_err(|error| TakokitError::Storage(error.to_string()))?;
        if python_adapter_is_current(&self.root, "rvc_training") {
            Ok(())
        } else {
            Err(TakokitError::Storage(
                "RVC training adapter installation finished without passing the complete readiness check"
                    .into(),
            ))
        }
    }

    fn training_paths(&self) -> TakokitResult<TrainingPaths> {
        self.ensure_training_adapter()?;
        let layout = python_managed_runner_layout(&self.root);
        let adapter = layout.adapters.join("rvc_training");
        let installed = InstalledRegistry::new(self.root.join("manifests"));
        let record = installed.installed_model_record("rvc").map_err(|_| {
            invalid("RVC assets are not installed; run `tako pull rvc` before training")
        })?;
        let asset_root = record
            .snapshot
            .map(|snapshot| snapshot.local_path)
            .unwrap_or_else(|| self.root.join("models").join("rvc"));
        Ok(TrainingPaths {
            python: adapter_python(&adapter),
            script: adapter.join("rvc_training.py"),
            trainer_root: adapter.join("source"),
            asset_root,
        })
    }

    fn job_request_path(&self, voice_id: Uuid, job_id: Uuid) -> PathBuf {
        self.store
            .layout(voice_id)
            .jobs
            .join(format!("{job_id}.request.json"))
    }
}

struct TrainingPaths {
    python: PathBuf,
    script: PathBuf,
    trainer_root: PathBuf,
    asset_root: PathBuf,
}

fn adapter_python(adapter: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        adapter.join("venv").join("Scripts").join("python.exe")
    }
    #[cfg(not(windows))]
    {
        adapter.join("venv").join("bin").join("python")
    }
}

fn process_is_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .ok()
            .is_some_and(|output| {
                String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
            })
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(format!("/proc/{pid}")).exists()
            || Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .is_ok_and(|status| status.success())
    }
}

fn process_matches_job(pid: u32, request_path: &Path) -> bool {
    if !process_is_running(pid) {
        return false;
    }
    #[cfg(windows)]
    {
        let command =
            format!("(Get-CimInstance Win32_Process -Filter \"ProcessId = {pid}\").CommandLine");
        return Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &command])
            .output()
            .ok()
            .is_some_and(|output| {
                let line = String::from_utf8_lossy(&output.stdout);
                line.contains(request_path.to_string_lossy().as_ref())
                    && line.contains("rvc_training.py")
            });
    }
    #[cfg(not(windows))]
    {
        fs::read(format!("/proc/{pid}/cmdline"))
            .ok()
            .is_some_and(|bytes| {
                let text = String::from_utf8_lossy(&bytes);
                text.contains("rvc_training.py")
                    && text.contains(request_path.to_string_lossy().as_ref())
            })
    }
}

fn terminate_owned_tree(pid: u32) -> TakokitResult<()> {
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map_err(storage)?;
        if !status.success() {
            return Err(invalid(format!(
                "taskkill could not terminate Takokit RVC job PID {pid}"
            )));
        }
    }
    #[cfg(not(windows))]
    {
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(storage)?;
        if !status.success() {
            return Err(invalid(format!(
                "could not terminate Takokit RVC job PID {pid}"
            )));
        }
    }
    Ok(())
}
