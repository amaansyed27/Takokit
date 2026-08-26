use super::*;
use serde_json::Value;
use std::{fs, fs::File};
use takokit_store::sha256_file;

fn imported_service() -> (tempfile::TempDir, RvcVoiceService, RvcVoiceProject) {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let source = temp.path().join("model.pth");
    fs::write(&source, b"model").unwrap();
    let project = service
        .import_existing(ImportRvcVoiceRequest {
            checkpoint: source,
            index: None,
            name: "Pack ü".into(),
            consent_affirmed: true,
            consent_note: Some("test provenance".into()),
        })
        .unwrap();
    (temp, service, project)
}

fn import_named(service: &RvcVoiceService, root: &Path, name: &str) -> RvcVoiceProject {
    let source = root.join(format!("{name}.pth"));
    fs::write(&source, name.as_bytes()).unwrap();
    service
        .import_existing(ImportRvcVoiceRequest {
            checkpoint: source,
            index: None,
            name: name.into(),
            consent_affirmed: true,
            consent_note: Some("test provenance".into()),
        })
        .unwrap()
}

#[test]
fn runtime_manifest_never_escapes_conversion_root() {
    let (temp, service, project) = imported_service();
    let runtime = service.conversion_target_id(project.id);
    let manifest: Value =
        serde_json::from_reader(File::open(runtime.join("rvc.json")).unwrap()).unwrap();
    assert_eq!(manifest["checkpoint"], "checkpoint.pth");
    assert!(manifest["index"].is_null());
    assert!(!manifest.to_string().contains(".."));
    assert!(runtime.starts_with(temp.path()));
}

#[test]
fn imported_artifact_is_managed_copy_with_same_hash() {
    let (temp, service, project) = imported_service();
    let checkpoint = service
        .store
        .checkpoints(&project.id.to_string())
        .unwrap()
        .pop()
        .unwrap();
    assert!(checkpoint
        .path
        .starts_with(service.store.layout(project.id).root));
    assert_eq!(checkpoint.sha256, sha256_file(&checkpoint.path).unwrap());
    assert_ne!(checkpoint.path, temp.path().join("model.pth"));
}

#[test]
fn runtime_setup_failure_does_not_mark_audio_invalid_and_valid_reinspection_repairs_state() {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let project = service
        .create(CreateRvcVoiceRequest {
            name: "Inspection recovery".into(),
            consent_affirmed: true,
            consent_note: None,
        })
        .unwrap();
    let source = temp.path().join("sample ü.wav");
    fs::write(&source, b"fixture").unwrap();
    let sample = service
        .add_samples(
            &project.id.to_string(),
            AddRvcSamplesRequest {
                paths: vec![source],
            },
        )
        .unwrap()
        .pop()
        .unwrap();

    let setup_error = service.persist_inspection_result(
        sample.clone(),
        Err(TakokitError::Execution("adapter install failed".into())),
    );
    assert!(setup_error.is_err());
    let untouched = service.store.samples_id(project.id).unwrap().pop().unwrap();
    assert_eq!(untouched.state, RvcSampleState::Imported);
    assert!(untouched.inspection.is_none());
    assert!(untouched.warnings.is_empty());

    service
        .persist_inspection_result(
            untouched.clone(),
            Err(TakokitError::Audio("decoder rejected fixture".into())),
        )
        .unwrap();
    let invalid_sample = service.store.samples_id(project.id).unwrap().pop().unwrap();
    assert_eq!(invalid_sample.state, RvcSampleState::Invalid);

    service
        .persist_inspection_result(
            invalid_sample,
            Ok(WorkerInspection {
                duration_ms: Some(42_000),
                sample_rate: Some(44_100),
                channels: Some(2),
                codec: Some("PCM".into()),
                container: Some("WAV".into()),
                peak_dbfs: Some(-1.0),
                rms_dbfs: Some(-18.0),
                silence_ratio: Some(0.1),
                clipped_ratio: Some(0.0),
                warnings: vec![],
                valid: true,
            }),
        )
        .unwrap();
    let repaired = service.store.samples_id(project.id).unwrap().pop().unwrap();
    assert_eq!(repaired.state, RvcSampleState::Inspected);
    assert_eq!(repaired.inspection.unwrap().duration_ms, Some(42_000));
    assert!(repaired.warnings.is_empty());
}

#[test]
fn artifact_discovery_rejects_another_voice_projects_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let voice_a = import_named(&service, temp.path(), "Voice A");
    let voice_b = import_named(&service, temp.path(), "Voice B");
    let b_checkpoint = service
        .store
        .checkpoints(&voice_b.id.to_string())
        .unwrap()
        .pop()
        .unwrap();
    let result = service
        .store
        .layout(voice_a.id)
        .jobs
        .join("latest-result.json");
    write_atomic_json(
        &result,
        &serde_json::json!({"checkpoint": b_checkpoint.path, "index": null}),
    )
    .unwrap();

    let error = service.checkpoints(&voice_a.id.to_string()).unwrap_err();

    assert!(error
        .to_string()
        .contains("does not belong to the selected voice project"));
    assert_eq!(
        service
            .store
            .checkpoints(&voice_a.id.to_string())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn completed_retrain_rebinds_same_index_bytes_to_new_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let project = service
        .create(CreateRvcVoiceRequest {
            name: "Retrain pairing".into(),
            consent_affirmed: true,
            consent_note: None,
        })
        .unwrap();
    let layout = service.store.layout(project.id);
    let old_checkpoint_path = layout.checkpoints.join("old.pth");
    let new_checkpoint_path = layout.checkpoints.join("new.pth");
    let index_path = layout.indexes.join("voice.index");
    fs::write(&old_checkpoint_path, b"old checkpoint").unwrap();
    fs::write(&new_checkpoint_path, b"new checkpoint").unwrap();
    fs::write(&index_path, b"same index bytes").unwrap();

    let old_checkpoint = RvcCheckpoint {
        id: Uuid::new_v4(),
        voice_id: project.id,
        path: old_checkpoint_path.clone(),
        sha256: sha256_file(&old_checkpoint_path).unwrap(),
        bytes: fs::metadata(&old_checkpoint_path).unwrap().len(),
        epoch: Some(1),
        sample_rate_hz: Some(40_000),
        model_version: Some("v2".into()),
        f0: Some(true),
        created_at: now(),
        valid_for_inference: true,
    };
    service.store.save_checkpoint(&old_checkpoint).unwrap();
    let stale_index = RvcIndexArtifact {
        id: Uuid::new_v4(),
        voice_id: project.id,
        path: index_path.clone(),
        sha256: sha256_file(&index_path).unwrap(),
        bytes: fs::metadata(&index_path).unwrap().len(),
        checkpoint_id: Some(old_checkpoint.id),
        created_at: now(),
        valid: true,
    };
    service.store.save_index(&stale_index).unwrap();
    write_atomic_json(
        &layout.jobs.join("latest-result.json"),
        &serde_json::json!({
            "checkpoint": new_checkpoint_path,
            "index": index_path,
            "epoch": 2
        }),
    )
    .unwrap();

    service
        .refresh_completed_artifacts(&project.id.to_string())
        .unwrap();

    let detail = service.show(&project.id.to_string()).unwrap();
    let active_checkpoint = detail.project.active_checkpoint_id.unwrap();
    let active_index = detail.project.active_index_id.unwrap();
    assert_ne!(active_checkpoint, old_checkpoint.id);
    let active_index_record = detail
        .indexes
        .into_iter()
        .find(|item| item.id == active_index)
        .unwrap();
    assert_eq!(active_index_record.checkpoint_id, Some(active_checkpoint));
    assert_eq!(detail.project.state, RvcVoiceProjectState::Ready);
}

#[test]
fn managed_test_and_normal_convert_resolve_the_same_project_target() {
    let (temp, service, project) = imported_service();
    let normal_target = service
        .resolve_conversion_target(&project.id.to_string())
        .unwrap();
    let test_target = service.conversion_target_id(project.id);
    let manifest: Value =
        serde_json::from_reader(File::open(test_target.join("rvc.json")).unwrap()).unwrap();

    assert_eq!(PathBuf::from(normal_target), test_target);
    assert_eq!(manifest["managed_voice_id"], project.id.to_string());
    assert!(test_target.starts_with(
        temp.path()
            .join("voices")
            .join("rvc")
            .join(project.id.to_string())
    ));
}

#[test]
fn unsigned_package_roundtrip_verifies_hashes() {
    let (temp, service, project) = imported_service();
    let package = temp.path().join("voice.takovoice");
    service
        .export_package(
            &project.id.to_string(),
            ExportRvcVoiceRequest {
                output: package.clone(),
                sign: false,
                include_reference: false,
            },
        )
        .unwrap();
    let report = service.verify_package(&package).unwrap();
    assert!(report.hashes_valid);
    assert!(!report.signed);
    assert!(report.errors.is_empty());
}

#[test]
fn signed_package_verifies_signature() {
    let (temp, service, project) = imported_service();
    let package = temp.path().join("voice.takovoice");
    service
        .export_package(
            &project.id.to_string(),
            ExportRvcVoiceRequest {
                output: package.clone(),
                sign: true,
                include_reference: false,
            },
        )
        .unwrap();
    let report = service.verify_package(&package).unwrap();
    assert!(report.hashes_valid);
    assert_eq!(report.signature_valid, Some(true));
    assert!(report.signer_fingerprint.is_some());
}
