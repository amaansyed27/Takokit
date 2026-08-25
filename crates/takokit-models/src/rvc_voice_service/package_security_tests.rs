use super::*;
use std::{
    collections::BTreeMap,
    fs,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn imported_service_with_index() -> (tempfile::TempDir, RvcVoiceService, RvcVoiceProject) {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let checkpoint = temp.path().join("model.pth");
    let index = temp.path().join("model.index");
    fs::write(&checkpoint, b"checkpoint-v2").unwrap();
    fs::write(&index, b"faiss-index").unwrap();
    let project = service
        .import_existing(ImportRvcVoiceRequest {
            checkpoint,
            index: Some(index),
            name: "Secure Pack ü".into(),
            consent_affirmed: true,
            consent_note: Some("security test provenance".into()),
        })
        .unwrap();
    (temp, service, project)
}

fn export(
    service: &RvcVoiceService,
    project: &RvcVoiceProject,
    root: &Path,
    sign: bool,
) -> std::path::PathBuf {
    let package = root.join(if sign {
        "signed.takovoice"
    } else {
        "unsigned.takovoice"
    });
    service
        .export_package(
            &project.id.to_string(),
            ExportRvcVoiceRequest {
                output: package.clone(),
                sign,
                include_reference: false,
            },
        )
        .unwrap();
    package
}

fn read_entries(package: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut archive = ZipArchive::new(File::open(package).unwrap()).unwrap();
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        entries.insert(file.name().to_string(), bytes);
    }
    entries
}

fn rewrite(package: &Path, entries: &BTreeMap<String, Vec<u8>>) {
    let temp = package.with_extension("rewrite");
    let mut writer = ZipWriter::new(File::create(&temp).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
    fs::rename(temp, package).unwrap();
}

#[test]
fn unsigned_and_signed_packages_import_after_verification() {
    for sign in [false, true] {
        let (temp, service, project) = imported_service_with_index();
        let package = export(&service, &project, temp.path(), sign);
        let imported = service
            .import_package(ImportRvcPackageRequest {
                package,
                name: Some(if sign {
                    "Signed import".into()
                } else {
                    "Unsigned import".into()
                }),
                consent_affirmed: true,
                consent_note: Some("test".into()),
            })
            .unwrap();
        assert!(imported.imported);
        assert!(imported.active_checkpoint_id.is_some());
        assert!(imported.active_index_id.is_some());
    }
}

#[test]
fn signed_package_has_valid_ed25519_fingerprint() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), true);
    let report = service.verify_package(&package).unwrap();
    assert_eq!(report.signature_valid, Some(true));
    let fingerprint = report.signer_fingerprint.unwrap();
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn altered_manifest_invalidates_signature() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), true);
    let mut entries = read_entries(&package);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&entries["manifest.json"]).unwrap();
    manifest["voice_name"] = serde_json::json!("Altered voice");
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    rewrite(&package, &entries);
    let report = service.verify_package(&package).unwrap();
    assert_eq!(report.signature_valid, Some(false));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("signature")));
}

#[test]
fn altered_checkpoint_is_rejected_by_sha256_and_size_metadata() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), false);
    let mut entries = read_entries(&package);
    entries.insert("checkpoint.pth".into(), b"tampered-checkpoint".to_vec());
    rewrite(&package, &entries);
    let report = service.verify_package(&package).unwrap();
    assert!(!report.hashes_valid);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("checkpoint.pth")));
}

#[test]
fn altered_index_is_rejected() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), false);
    let mut entries = read_entries(&package);
    entries.insert("model.index".into(), b"tampered-index".to_vec());
    rewrite(&package, &entries);
    let report = service.verify_package(&package).unwrap();
    assert!(!report.hashes_valid);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("model.index")));
}

#[test]
fn explicit_manifest_sha256_mismatch_is_rejected() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), false);
    let mut entries = read_entries(&package);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&entries["manifest.json"]).unwrap();
    manifest["checkpoint"]["sha256"] = serde_json::json!("00".repeat(32));
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    rewrite(&package, &entries);
    assert!(!service.verify_package(&package).unwrap().hashes_valid);
}

#[test]
fn invalid_signature_metadata_is_rejected() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), true);
    let mut entries = read_entries(&package);
    let mut signature: serde_json::Value =
        serde_json::from_slice(&entries["signature.json"]).unwrap();
    signature["signature_hex"] = serde_json::json!("00".repeat(64));
    entries.insert(
        "signature.json".into(),
        serde_json::to_vec_pretty(&signature).unwrap(),
    );
    rewrite(&package, &entries);
    let report = service.verify_package(&package).unwrap();
    assert_eq!(report.signature_valid, Some(false));
}

#[test]
fn manifest_path_traversal_is_reported_and_import_refuses_it() {
    let (temp, service, project) = imported_service_with_index();
    let package = export(&service, &project, temp.path(), false);
    let mut entries = read_entries(&package);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&entries["manifest.json"]).unwrap();
    manifest["checkpoint"]["path"] = serde_json::json!("../checkpoint.pth");
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    rewrite(&package, &entries);
    let report = service.verify_package(&package).unwrap();
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("unsafe manifest artifact path")));
    assert!(service
        .import_package(ImportRvcPackageRequest {
            package,
            name: None,
            consent_affirmed: true,
            consent_note: None,
        })
        .is_err());
}

#[test]
fn unsafe_archive_entry_path_is_rejected_before_manifest_processing() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("traversal.takovoice");
    let mut writer = ZipWriter::new(File::create(&package).unwrap());
    let options = SimpleFileOptions::default();
    writer.start_file("../outside.pth", options).unwrap();
    writer.write_all(b"bad").unwrap();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap();
    let service = RvcVoiceService::new(temp.path());
    assert!(service.verify_package(&package).is_err());
}

#[test]
fn malformed_archive_and_oversized_manifest_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let service = RvcVoiceService::new(temp.path());
    let malformed = temp.path().join("malformed.takovoice");
    fs::write(&malformed, b"not a zip archive").unwrap();
    assert!(service.verify_package(&malformed).is_err());

    let oversized = temp.path().join("oversized.takovoice");
    let mut writer = ZipWriter::new(File::create(&oversized).unwrap());
    writer
        .start_file("manifest.json", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&vec![b' '; 1024 * 1024 + 1]).unwrap();
    writer.finish().unwrap();
    assert!(service.verify_package(&oversized).is_err());
}

#[test]
fn package_file_count_bound_is_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("many-files.takovoice");
    let mut writer = ZipWriter::new(File::create(&package).unwrap());
    let options = SimpleFileOptions::default();
    for index in 0..17 {
        writer
            .start_file(format!("entry-{index}"), options)
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    writer.finish().unwrap();
    let service = RvcVoiceService::new(temp.path());
    assert!(service.verify_package(&package).is_err());
}

#[test]
fn export_excludes_training_dataset_by_default() {
    let (temp, service, project) = imported_service_with_index();
    let sample = temp.path().join("training.wav");
    fs::write(&sample, b"training-dataset-bytes").unwrap();
    service
        .add_samples(
            &project.id.to_string(),
            AddRvcSamplesRequest {
                paths: vec![sample],
            },
        )
        .unwrap();
    let package = export(&service, &project, temp.path(), false);
    let entries = read_entries(&package);
    assert!(entries.contains_key("manifest.json"));
    assert!(entries.contains_key("checkpoint.pth"));
    assert!(entries.contains_key("model.index"));
    assert!(!entries.keys().any(|name| name.contains("sample")
        || name.contains("dataset")
        || name.contains("training.wav")));
}
