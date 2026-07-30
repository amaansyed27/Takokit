//! First-class model-license metadata and durable acceptance receipts.

use crate::{ModelManifest, PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use takokit_core::ModelLicenseInfo;

pub const CPML_ID: &str = "CPML";
const CPML_DIGEST: &str = "sha256:3dbb31aa8875793cde77882e71dbb5f80fe31b818ecca4a4a5812a430f7209c7";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicenseReceipt {
    pub license_id: String,
    pub license_name: String,
    pub license_url: String,
    pub license_version: String,
    pub license_digest: String,
    pub accepted_at_unix: u64,
    pub takokit_version: String,
    pub model_id: String,
}

pub fn model_license_info(model: &ModelManifest) -> Option<ModelLicenseInfo> {
    if model.license.eq_ignore_ascii_case(CPML_ID)
        || model.license.eq_ignore_ascii_case("coqui-public-model-license-check-required")
        || model.id == "xtts-v2"
    {
        Some(ModelLicenseInfo {
            id: CPML_ID.to_string(),
            name: "Coqui Public Model License".to_string(),
            version: "1.0.0".to_string(),
            url: "https://coqui.ai/cpml.txt".to_string(),
            digest: CPML_DIGEST.to_string(),
            requires_acceptance: true,
            commercial_use: false,
            notice: "Non-commercial use only. Review the CPML before downloading or using this model.".to_string(),
        })
    } else {
        None
    }
}

pub fn normalize_model_license(model: &mut ModelManifest) {
    if let Some(info) = model_license_info(model) {
        model.license = info.id;
    }
}

pub fn ensure_model_license_accepted(
    takokit_root: &Path,
    model: &ModelManifest,
    accepted_license: Option<&str>,
) -> PackageResult<Option<LicenseReceipt>> {
    let Some(info) = model_license_info(model) else {
        return Ok(None);
    };
    if let Some(receipt) = valid_license_receipt(takokit_root, model)? {
        return Ok(Some(receipt));
    }
    if accepted_license.is_some_and(|id| id.eq_ignore_ascii_case(&info.id)) {
        let receipt = LicenseReceipt {
            license_id: info.id,
            license_name: info.name,
            license_url: info.url,
            license_version: info.version,
            license_digest: info.digest,
            accepted_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            takokit_version: env!("CARGO_PKG_VERSION").to_string(),
            model_id: model.id.clone(),
        };
        write_receipt(takokit_root, &receipt)?;
        return Ok(Some(receipt));
    }
    if accepted_license.is_some() {
        return Err(PackageError::LicenseMismatch {
            model: model.id.clone(),
            expected: info.id,
            supplied: accepted_license.unwrap_or_default().to_string(),
        });
    }
    Err(PackageError::LicenseAcceptanceRequired {
        model: model.id.clone(),
        license: info.id,
        url: info.url,
    })
}

pub fn valid_license_receipt(
    takokit_root: &Path,
    model: &ModelManifest,
) -> PackageResult<Option<LicenseReceipt>> {
    let Some(info) = model_license_info(model) else {
        return Ok(None);
    };
    let path = receipt_path(takokit_root, &info.id, &model.id);
    let source = match std::fs::read(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let receipt: LicenseReceipt = serde_json::from_slice(&source)?;
    if receipt.license_id == info.id
        && receipt.license_version == info.version
        && receipt.license_digest == info.digest
        && receipt.license_url == info.url
        && receipt.model_id == model.id
    {
        Ok(Some(receipt))
    } else {
        Ok(None)
    }
}

pub fn list_license_receipts(takokit_root: &Path) -> PackageResult<Vec<LicenseReceipt>> {
    let root = receipt_root(takokit_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut receipts = Vec::new();
    for license_dir in std::fs::read_dir(root)? {
        let license_dir = license_dir?;
        if !license_dir.path().is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(license_dir.path())? {
            let entry = entry?;
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                receipts.push(serde_json::from_slice(&std::fs::read(entry.path())?)?);
            }
        }
    }
    receipts.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    Ok(receipts)
}

pub fn revoke_license_receipt(
    takokit_root: &Path,
    license_id: &str,
    model_id: Option<&str>,
) -> PackageResult<usize> {
    let root = receipt_root(takokit_root).join(safe_component(license_id));
    if let Some(model_id) = model_id {
        let path = root.join(format!("{}.json", safe_component(model_id)));
        return match std::fs::remove_file(path) {
            Ok(()) => Ok(1),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        };
    }
    if !root.exists() {
        return Ok(0);
    }
    let removed = std::fs::read_dir(&root)?.filter_map(Result::ok).count();
    std::fs::remove_dir_all(root)?;
    Ok(removed)
}

fn write_receipt(takokit_root: &Path, receipt: &LicenseReceipt) -> PackageResult<()> {
    let path = receipt_path(takokit_root, &receipt.license_id, &receipt.model_id);
    let parent = path.parent().expect("receipt path has parent");
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(receipt)?)?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn receipt_root(takokit_root: &Path) -> PathBuf {
    takokit_root.join("licenses").join("receipts")
}

fn receipt_path(takokit_root: &Path, license_id: &str, model_id: &str) -> PathBuf {
    receipt_root(takokit_root)
        .join(safe_component(license_id))
        .join(format!("{}.json", safe_component(model_id)))
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') { character } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactManifest, CapabilityManifest, HardwareManifest, ModelBackend, ModelKind};

    fn cpml_model() -> ModelManifest {
        ModelManifest {
            id: "xtts-v2".into(), name: "XTTS v2".into(), family: "xtts-v2".into(),
            version: "2".into(), kind: ModelKind::VoiceCloning, backend: ModelBackend::PythonManaged,
            runner: "takokit-python-managed".into(), required_adapter: Some("coqui_tts".into()),
            license: "CPML".into(), description: "fixture".into(),
            capabilities: CapabilityManifest::default(),
            hardware: HardwareManifest { cpu: true, gpu: true, min_ram: None, min_vram: None },
            source: None, artifacts: ArtifactManifest::default(),
        }
    }

    #[test]
    fn acceptance_is_persistent_and_revocable() {
        let root = tempfile::tempdir().expect("tempdir");
        let model = cpml_model();
        assert!(matches!(
            ensure_model_license_accepted(root.path(), &model, None),
            Err(PackageError::LicenseAcceptanceRequired { .. })
        ));
        ensure_model_license_accepted(root.path(), &model, Some("CPML")).expect("accept");
        assert!(valid_license_receipt(root.path(), &model).expect("read").is_some());
        assert_eq!(revoke_license_receipt(root.path(), "CPML", Some("xtts-v2")).expect("revoke"), 1);
        assert!(valid_license_receipt(root.path(), &model).expect("read").is_none());
    }

    #[test]
    fn wrong_or_changed_license_identity_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let model = cpml_model();
        assert!(matches!(
            ensure_model_license_accepted(root.path(), &model, Some("MIT")),
            Err(PackageError::LicenseMismatch { .. })
        ));
        ensure_model_license_accepted(root.path(), &model, Some("CPML")).expect("accept");
        let path = receipt_path(root.path(), "CPML", "xtts-v2");
        let mut receipt: LicenseReceipt = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        receipt.license_digest = "changed".into();
        std::fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert!(valid_license_receipt(root.path(), &model).expect("read").is_none());
    }
}
