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
