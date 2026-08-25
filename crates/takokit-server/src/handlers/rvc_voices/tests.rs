use super::*;
use takokit_core::{
    RvcF0Method, RvcTrainingConfig, RvcTrainingDevice, RvcTrainingJobStatus, RvcTrainingPrecision,
    RvcTrainingPreset, RvcTrainingStage,
};

fn fixture_job() -> RvcTrainingJob {
    RvcTrainingJob {
        id: Uuid::new_v4(),
        voice_id: Uuid::new_v4(),
        config: RvcTrainingConfig {
            preset: RvcTrainingPreset::Quick,
            epochs: 20,
            batch_size: 4,
            save_every_epochs: 5,
            sample_rate_hz: 40_000,
            model_version: "v2".into(),
            f0_enabled: true,
            f0_method: RvcF0Method::Rmvpe,
            device: RvcTrainingDevice::Auto,
            precision: RvcTrainingPrecision::Auto,
            cache_dataset_on_gpu: false,
        },
        status: RvcTrainingJobStatus::Running,
        stage: RvcTrainingStage::Train,
        created_at: 1,
        started_at: Some(2),
        finished_at: None,
        owner_pid: Some(123),
        child_pid: Some(456),
        log_path: PathBuf::from(r"C:\Takokit\logs\private.log"),
        checkpoint_ids: Vec::new(),
        failure: None,
        cancellation_requested: true,
    }
}

#[test]
fn public_training_job_does_not_expose_process_ownership_internals() {
    let value = public_job(&fixture_job());
    let object = value.as_object().unwrap();
    for forbidden in [
        "owner_pid",
        "child_pid",
        "log_path",
        "cancellation_requested",
    ] {
        assert!(!object.contains_key(forbidden), "leaked {forbidden}");
    }
    assert_eq!(value["status"], "running");
    assert_eq!(value["stage"], "train");
    assert_eq!(value["config"]["preset"], "quick");
}

#[test]
fn public_detail_scrubs_nested_active_job_only() {
    let job = fixture_job();
    let detail = RvcVoiceDetail {
        project: takokit_core::RvcVoiceProject {
            schema_version: 1,
            id: job.voice_id,
            name: "Voice ü".into(),
            engine: "rvc".into(),
            state: takokit_core::RvcVoiceProjectState::Training,
            imported: false,
            created_at: 1,
            updated_at: 2,
            latest_job_id: Some(job.id),
            active_checkpoint_id: None,
            active_index_id: None,
            last_error: None,
        },
        samples: Vec::new(),
        dataset: takokit_core::RvcDatasetInspection {
            voice_id: job.voice_id,
            ..Default::default()
        },
        managed: None,
        checkpoints: Vec::new(),
        indexes: Vec::new(),
        active_job: Some(job),
        conversion_target: None,
    };
    let value = public_detail(&detail);
    let active = value["active_job"].as_object().unwrap();
    assert!(!active.contains_key("owner_pid"));
    assert!(!active.contains_key("child_pid"));
    assert!(!active.contains_key("log_path"));
    assert!(!active.contains_key("cancellation_requested"));
    assert_eq!(value["project"]["name"], "Voice ü");
}
