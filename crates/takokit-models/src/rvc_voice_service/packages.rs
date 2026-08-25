use super::*;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use takokit_store::sha256_file;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const MAX_PACKAGE_FILES: usize = 16;
const MAX_PACKAGE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

impl RvcVoiceService {
    pub fn export_package(
        &self,
        voice: &str,
        request: ExportRvcVoiceRequest,
    ) -> TakokitResult<PathBuf> {
        let project = self.store.load(voice)?;
        let runtime = self.conversion_target_id(project.id);
        let checkpoint = runtime.join("checkpoint.pth");
        let index = runtime.join("model.index");
        if !checkpoint.is_file() || !runtime.join("rvc.json").is_file() {
            return Err(invalid("voice has no ready managed checkpoint to export"));
        }
        ensure_takovoice_extension(&request.output)?;
        if let Some(parent) = request.output.parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let reference = request
            .include_reference
            .then(|| first_reference(&self.store.layout(project.id).references))
            .flatten();
        let manifest = RvcPackageManifest {
            schema_version: TAKOVOICE_SCHEMA_VERSION,
            engine: "rvc".into(),
            voice_name: project.name.clone(),
            exported_at: now(),
            checkpoint: package_artifact("checkpoint.pth", &checkpoint)?,
            index: index
                .is_file()
                .then(|| package_artifact("model.index", &index))
                .transpose()?,
            reference: reference
                .as_ref()
                .map(|path| package_artifact("reference.wav", path))
                .transpose()?,
            consent_acknowledged: true,
            provenance_note: "Takokit package provenance records local artifact integrity. It does not prove speaker identity, legal ownership, consent authenticity, or perceptual similarity.".into(),
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| invalid(error.to_string()))?;
        let mut zip = ZipWriter::new(File::create(&request.output).map_err(storage)?);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options).map_err(zip_error)?;
        zip.write_all(&manifest_bytes).map_err(storage)?;
        add_zip_file(&mut zip, "checkpoint.pth", &checkpoint, options)?;
        if index.is_file() {
            add_zip_file(&mut zip, "model.index", &index, options)?;
        }
        if let Some(reference) = reference.as_ref() {
            add_zip_file(&mut zip, "reference.wav", reference, options)?;
        }
        if request.sign {
            let signature = self.sign_manifest(&manifest_bytes)?;
            zip.start_file("signature.json", options).map_err(zip_error)?;
            zip.write_all(
                &serde_json::to_vec_pretty(&signature)
                    .map_err(|error| invalid(error.to_string()))?,
            )
            .map_err(storage)?;
        }
        zip.finish().map_err(zip_error)?;
        Ok(request.output)
    }

    pub fn verify_package(&self, package: &Path) -> TakokitResult<RvcPackageVerification> {
        let mut archive = open_package(package)?;
        validate_archive_bounds(&mut archive)?;
        let manifest_bytes = read_small_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
        let manifest: RvcPackageManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| invalid(format!("invalid voice package manifest: {error}")))?;
        let mut errors = validate_manifest(&manifest);
        let mut hashes_valid = true;
        let mut artifacts = vec![&manifest.checkpoint];
        if let Some(index) = manifest.index.as_ref() {
            artifacts.push(index);
        }
        if let Some(reference) = manifest.reference.as_ref() {
            artifacts.push(reference);
        }
        for artifact in artifacts {
            match hash_zip_entry(&mut archive, artifact) {
                Ok(true) => {}
                Ok(false) => {
                    hashes_valid = false;
                    errors.push(format!("artifact hash/size mismatch: {}", artifact.path));
                }
                Err(error) => {
                    hashes_valid = false;
                    errors.push(error.to_string());
                }
            }
        }
        let signature_bytes =
            read_optional_small_entry(&mut archive, "signature.json", MAX_MANIFEST_BYTES)?;
        let (signed, signature_valid, signer_fingerprint) = match signature_bytes {
            Some(bytes) => match serde_json::from_slice::<RvcPackageSignature>(&bytes) {
                Ok(signature) => {
                    let valid = verify_signature(&manifest_bytes, &signature).unwrap_or(false);
                    if !valid {
                        errors.push("voice package signature is invalid".into());
                    }
                    (true, Some(valid), Some(signature.signer_fingerprint))
                }
                Err(error) => {
                    errors.push(format!("invalid signature metadata: {error}"));
                    (true, Some(false), None)
                }
            },
            None => (false, None, None),
        };
        Ok(RvcPackageVerification {
            schema_version: manifest.schema_version,
            package_path: package.to_path_buf(),
            signed,
            signature_valid,
            signer_fingerprint,
            hashes_valid,
            voice_name: Some(manifest.voice_name),
            errors,
        })
    }

    pub fn import_package(
        &self,
        request: ImportRvcPackageRequest,
    ) -> TakokitResult<RvcVoiceProject> {
        if !request.consent_affirmed {
            return Err(invalid(
                "import requires permission/provenance acknowledgement",
            ));
        }
        let verification = self.verify_package(&request.package)?;
        if !verification.hashes_valid
            || verification.signature_valid == Some(false)
            || !verification.errors.is_empty()
        {
            return Err(invalid(format!(
                "voice package verification failed: {}",
                verification.errors.join("; ")
            )));
        }
        let mut archive = open_package(&request.package)?;
        let manifest_bytes = read_small_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
        let manifest: RvcPackageManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| invalid(error.to_string()))?;
        let name = request.name.unwrap_or(manifest.voice_name.clone());
        let mut project = self.store.create(&name, true, request.consent_note)?;
        project.imported = true;
        self.store.save_project(&project)?;
        let layout = self.store.layout(project.id);
        let temporary = layout.packages.join(format!("import-{}", Uuid::new_v4()));
        fs::create_dir_all(&temporary).map_err(storage)?;
        let checkpoint = temporary.join("checkpoint.pth");
        extract_entry(&mut archive, &manifest.checkpoint, &checkpoint)?;
        let index = match manifest.index.as_ref() {
            Some(meta) => {
                let path = temporary.join("model.index");
                extract_entry(&mut archive, meta, &path)?;
                Some(path)
            }
            None => None,
        };
        if let Some(reference) = manifest.reference.as_ref() {
            let path = layout.references.join("package-reference.wav");
            extract_entry(&mut archive, reference, &path)?;
        }
        self.import_artifacts(
            project.id,
            &checkpoint,
            index.as_deref(),
            Some(serde_json::json!({
                "package": request.package,
                "signed": verification.signed,
                "signer_fingerprint": verification.signer_fingerprint,
                "imported_at": now()
            })),
        )?;
        let _ = fs::remove_dir_all(temporary);
        self.store.load_id(project.id)
    }

    fn sign_manifest(&self, manifest: &[u8]) -> TakokitResult<RvcPackageSignature> {
        let directory = self.root.join("keys").join("voice-packages");
        fs::create_dir_all(&directory).map_err(storage)?;
        let path = directory.join("ed25519.key");
        let signing = if path.is_file() {
            let bytes = hex::decode(fs::read_to_string(&path).map_err(storage)?.trim())
                .map_err(|error| invalid(format!("invalid voice signing key: {error}")))?;
            let array: [u8; 32] = bytes
                .try_into()
                .map_err(|_| invalid("voice signing key has invalid length"))?;
            SigningKey::from_bytes(&array)
        } else {
            let key = SigningKey::generate(&mut OsRng);
            fs::write(&path, hex::encode(key.to_bytes())).map_err(storage)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(storage)?;
            }
            key
        };
        let verifying = signing.verifying_key();
        Ok(RvcPackageSignature {
            algorithm: "Ed25519".into(),
            public_key_hex: hex::encode(verifying.as_bytes()),
            signature_hex: hex::encode(signing.sign(manifest).to_bytes()),
            signer_fingerprint: hex::encode(Sha256::digest(verifying.as_bytes())),
        })
    }
}

fn validate_manifest(manifest: &RvcPackageManifest) -> Vec<String> {
    let mut errors = Vec::new();
    if manifest.schema_version != TAKOVOICE_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported package schema {}",
            manifest.schema_version
        ));
    }
    if manifest.engine != "rvc" {
        errors.push("package engine is not rvc".into());
    }
    if manifest.voice_name.trim().is_empty() || manifest.voice_name.chars().count() > 120 {
        errors.push("package voice name is invalid".into());
    }
    for path in std::iter::once(&manifest.checkpoint.path)
        .chain(manifest.index.iter().map(|item| &item.path))
        .chain(manifest.reference.iter().map(|item| &item.path))
    {
        let path = Path::new(path);
        if path.is_absolute()
            || path.components().count() != 1
            || path.components().any(|part| matches!(part, std::path::Component::ParentDir))
        {
            errors.push(format!("unsafe manifest artifact path: {}", path.display()));
        }
    }
    errors
}

fn ensure_takovoice_extension(path: &Path) -> TakokitResult<()> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("takovoice"))
    {
        Ok(())
    } else {
        Err(invalid("voice package output must use .takovoice"))
    }
}

fn package_artifact(name: &str, path: &Path) -> TakokitResult<RvcPackageArtifact> {
    Ok(RvcPackageArtifact {
        path: name.into(),
        sha256: sha256_file(path)?,
        bytes: fs::metadata(path).map_err(storage)?.len(),
    })
}

fn add_zip_file(
    zip: &mut ZipWriter<File>,
    name: &str,
    path: &Path,
    options: SimpleFileOptions,
) -> TakokitResult<()> {
    zip.start_file(name, options).map_err(zip_error)?;
    std::io::copy(&mut File::open(path).map_err(storage)?, zip).map_err(storage)?;
    Ok(())
}

fn open_package(path: &Path) -> TakokitResult<ZipArchive<File>> {
    ensure_takovoice_extension(path)?;
    ZipArchive::new(File::open(path).map_err(storage)?).map_err(zip_error)
}

fn validate_archive_bounds(archive: &mut ZipArchive<File>) -> TakokitResult<()> {
    if archive.is_empty() || archive.len() > MAX_PACKAGE_FILES {
        return Err(invalid("voice package contains an invalid number of files"));
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(zip_error)?;
        if file.enclosed_name().is_none() {
            return Err(invalid(format!("unsafe package path: {}", file.name())));
        }
        total = total.saturating_add(file.size());
        if total > MAX_PACKAGE_BYTES {
            return Err(invalid("voice package exceeds the 10 GiB safety bound"));
        }
    }
    Ok(())
}

fn read_small_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    max: u64,
) -> TakokitResult<Vec<u8>> {
    let mut file = archive.by_name(name).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() > max {
        return Err(invalid(format!("invalid package entry: {name}")));
    }
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).map_err(storage)?;
    Ok(bytes)
}

fn read_optional_small_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    max: u64,
) -> TakokitResult<Option<Vec<u8>>> {
    match archive.by_name(name) {
        Ok(mut file) => {
            if file.enclosed_name().is_none() || file.size() > max {
                return Err(invalid(format!("invalid package entry: {name}")));
            }
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes).map_err(storage)?;
            Ok(Some(bytes))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(error) => Err(zip_error(error)),
    }
}

fn hash_zip_entry(
    archive: &mut ZipArchive<File>,
    artifact: &RvcPackageArtifact,
) -> TakokitResult<bool> {
    let mut file = archive.by_name(&artifact.path).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() != artifact.bytes {
        return Ok(false);
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(storage)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()) == artifact.sha256)
}

fn extract_entry(
    archive: &mut ZipArchive<File>,
    artifact: &RvcPackageArtifact,
    output: &Path,
) -> TakokitResult<()> {
    let mut file = archive.by_name(&artifact.path).map_err(zip_error)?;
    if file.enclosed_name().is_none() || file.size() != artifact.bytes {
        return Err(invalid("package artifact metadata mismatch"));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(storage)?;
    }
    let mut target = File::create(output).map_err(storage)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(storage)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        target.write_all(&buffer[..count]).map_err(storage)?;
    }
    if hex::encode(hash.finalize()) != artifact.sha256 {
        let _ = fs::remove_file(output);
        return Err(invalid("package artifact hash changed during extraction"));
    }
    Ok(())
}

fn verify_signature(
    manifest: &[u8],
    signature: &RvcPackageSignature,
) -> Result<bool, String> {
    if signature.algorithm != "Ed25519" {
        return Ok(false);
    }
    let public: [u8; 32] = hex::decode(&signature.public_key_hex)
        .map_err(|error| error.to_string())?
        .try_into()
        .map_err(|_| "invalid public key length".to_string())?;
    let verifying = VerifyingKey::from_bytes(&public).map_err(|error| error.to_string())?;
    let signature = Signature::from_slice(
        &hex::decode(&signature.signature_hex).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let fingerprint = hex::encode(Sha256::digest(verifying.as_bytes()));
    Ok(fingerprint == signature.signer_fingerprint
        && verifying.verify(manifest, &signature).is_ok())
}

fn first_reference(root: &Path) -> Option<PathBuf> {
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_file())
}

fn zip_error(error: zip::result::ZipError) -> TakokitError {
    TakokitError::Storage(error.to_string())
}
