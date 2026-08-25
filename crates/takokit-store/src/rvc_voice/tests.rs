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
    let added = store
        .add_samples(&project.id.to_string(), &[source.clone(), source.clone()])
        .unwrap();
    assert_eq!(added.len(), 1);
    assert!(source.is_file());
    assert!(added[0].managed_path.is_file());
    store
        .remove_sample(&project.id.to_string(), added[0].id)
        .unwrap();
    assert!(source.is_file());
}

#[test]
fn removing_voice_is_blocked_by_active_job() {
    let temp = TempDir::new().unwrap();
    let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
    let project = store.create("Voice", true, None).unwrap();
    let config =
        takokit_core::RvcTrainingConfig::preset(takokit_core::RvcTrainingPreset::Quick).unwrap();
    let job = RvcTrainingJob {
        id: Uuid::new_v4(),
        voice_id: project.id,
        config,
        status: RvcTrainingJobStatus::Running,
        stage: takokit_core::RvcTrainingStage::Train,
        created_at: now_secs(),
        started_at: Some(now_secs()),
        finished_at: None,
        owner_pid: None,
        child_pid: None,
        log_path: PathBuf::new(),
        checkpoint_ids: vec![],
        failure: None,
        cancellation_requested: false,
    };
    store.save_job(&job).unwrap();
    assert!(store.remove(&project.id.to_string(), false).is_err());
}

#[test]
fn auxiliary_job_json_does_not_break_active_job_recovery() {
    let temp = TempDir::new().unwrap();
    let store = RvcVoiceStore::new(temp.path().join("voices/rvc"));
    let project = store.create("Voice", true, None).unwrap();
    let layout = store.layout(project.id);
    fs::write(
        layout.jobs.join("latest-result.json"),
        br#"{"checkpoint":"model.pth"}"#,
    )
    .unwrap();
    fs::write(
        layout.jobs.join("abc.request.json"),
        br#"{"operation":"train"}"#,
    )
    .unwrap();
    assert!(store.active_job(project.id).unwrap().is_none());
}
