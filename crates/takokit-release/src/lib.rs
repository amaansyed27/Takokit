use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use thiserror::Error;

pub const PRODUCT: &str = "Takokit";
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const STORAGE_SCHEMA_VERSION: u32 = 1;
pub const TEST_KEY_ID: &str = "takokit-test-fixture-v1";
pub const PRODUCTION_KEY_ID: &str = "takokit-release-v1";
const TEST_SIGNING_SEED_HEX: &str =
    "4242424242424242424242424242424242424242424242424242424242424242";

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("release metadata I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid release metadata: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid release version: {0}")]
    Version(#[from] semver::Error),
    #[error("invalid release signing key: {0}")]
    SigningKey(String),
    #[error("release signature verification failed: {0}")]
    Signature(String),
    #[error("release manifest rejected: {0}")]
    Rejected(String),
}

pub type ReleaseResult<T> = Result<T, ReleaseError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub product: String,
    pub version: String,
    pub channel: String,
    pub commit_sha: String,
    pub build_id: String,
    pub build_timestamp: String,
    pub os: String,
    pub architecture: String,
    pub registry_schema_version: u32,
    pub storage_schema: StorageSchemaCompatibility,
    pub minimum_compatible_version: String,
    pub signing_key_id: String,
    #[serde(default)]
    pub test_fixture: bool,
    pub artifacts: Vec<ReleaseArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageSchemaCompatibility {
    pub current: u32,
    pub minimum_readable: u32,
    pub maximum_readable: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseArtifact {
    pub role: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseIndex {
    pub schema_version: u32,
    pub product: String,
    pub version: String,
    pub channel: String,
    pub commit_sha: String,
    pub signing_key_id: String,
    #[serde(default)]
    pub test_fixture: bool,
    pub platforms: BTreeMap<String, PlatformManifestReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformManifestReference {
    pub os: String,
    pub architecture: String,
    pub manifest: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureEnvelope {
    pub algorithm: String,
    pub key_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributionMetadata {
    pub product: String,
    pub version: String,
    pub mode: String,
    #[serde(default)]
    pub install_root: Option<String>,
    /// Backward-compatible manifest URL for the default channel.
    #[serde(default)]
    pub update_manifest_url: Option<String>,
    /// Channel-specific signed-manifest locations used by automatic/manual checks.
    #[serde(default)]
    pub update_manifest_urls: BTreeMap<String, String>,
    #[serde(default = "default_channel")]
    pub default_channel: String,
}

impl DistributionMetadata {
    pub fn manifest_url_for_channel(&self, channel: &str) -> Option<String> {
        self.update_manifest_urls.get(channel).cloned().or_else(|| {
            (channel == self.default_channel)
                .then(|| self.update_manifest_url.clone())
                .flatten()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    Current,
    UpdateAvailable { version: Version },
}

fn default_channel() -> String {
    "stable".to_string()
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn sha256_file(path: &Path) -> ReleaseResult<String> {
    let bytes = fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

pub fn parse_manifest(bytes: &[u8]) -> ReleaseResult<ReleaseManifest> {
    Ok(serde_json::from_slice(bytes)?)
}

pub fn parse_signature(bytes: &[u8]) -> ReleaseResult<SignatureEnvelope> {
    Ok(serde_json::from_slice(bytes)?)
}

pub fn sign_with_seed(
    bytes: &[u8],
    seed_hex: &str,
    key_id: impl Into<String>,
) -> ReleaseResult<SignatureEnvelope> {
    let seed = decode_32(seed_hex, "private signing seed")?;
    let signing = SigningKey::from_bytes(&seed);
    let signature = signing.sign(bytes);
    Ok(SignatureEnvelope {
        algorithm: "ed25519".to_string(),
        key_id: key_id.into(),
        signature: hex::encode(signature.to_bytes()),
    })
}

pub fn sign_test_fixture(bytes: &[u8]) -> ReleaseResult<SignatureEnvelope> {
    sign_with_seed(bytes, TEST_SIGNING_SEED_HEX, TEST_KEY_ID)
}

pub fn verify_signature(
    bytes: &[u8],
    envelope: &SignatureEnvelope,
    allow_test_key: bool,
) -> ReleaseResult<()> {
    if envelope.algorithm != "ed25519" {
        return Err(ReleaseError::Signature(format!(
            "unsupported algorithm {}",
            envelope.algorithm
        )));
    }
    let verifying = if envelope.key_id == TEST_KEY_ID {
        if !allow_test_key {
            return Err(ReleaseError::Signature(
                "test-fixture trust is disabled".to_string(),
            ));
        }
        let seed = decode_32(TEST_SIGNING_SEED_HEX, "test signing seed")?;
        SigningKey::from_bytes(&seed).verifying_key()
    } else if envelope.key_id == PRODUCTION_KEY_ID {
        let public = option_env!("TAKOKIT_RELEASE_PUBLIC_KEY_HEX").ok_or_else(|| {
            ReleaseError::Signature(
                "this build has no production release public key configured".to_string(),
            )
        })?;
        VerifyingKey::from_bytes(&decode_32(public, "production public key")?)
            .map_err(|error| ReleaseError::SigningKey(format!("production public key: {error}")))?
    } else {
        return Err(ReleaseError::Signature(format!(
            "untrusted signing key id {}",
            envelope.key_id
        )));
    };
    let signature_bytes = hex::decode(&envelope.signature)
        .map_err(|error| ReleaseError::Signature(format!("signature is not hex: {error}")))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| ReleaseError::Signature(error.to_string()))?;
    verifying
        .verify(bytes, &signature)
        .map_err(|error| ReleaseError::Signature(error.to_string()))
}

pub fn validate_manifest(
    manifest: &ReleaseManifest,
    current_version: &str,
    selected_channel: &str,
    allow_test_key: bool,
) -> ReleaseResult<UpdateDecision> {
    if manifest.product != PRODUCT {
        return Err(ReleaseError::Rejected(format!(
            "wrong product {}",
            manifest.product
        )));
    }
    if normalize_os(&manifest.os) != normalize_os(std::env::consts::OS) {
        return Err(ReleaseError::Rejected(format!(
            "wrong operating system {}",
            manifest.os
        )));
    }
    if normalize_arch(&manifest.architecture) != normalize_arch(std::env::consts::ARCH) {
        return Err(ReleaseError::Rejected(format!(
            "wrong architecture {} for {}",
            manifest.architecture,
            std::env::consts::ARCH
        )));
    }
    if manifest.registry_schema_version > REGISTRY_SCHEMA_VERSION {
        return Err(ReleaseError::Rejected(format!(
            "registry schema {} is newer than supported schema {}",
            manifest.registry_schema_version, REGISTRY_SCHEMA_VERSION
        )));
    }
    if STORAGE_SCHEMA_VERSION < manifest.storage_schema.minimum_readable
        || STORAGE_SCHEMA_VERSION > manifest.storage_schema.maximum_readable
    {
        return Err(ReleaseError::Rejected(format!(
            "storage schema {} is outside update compatibility {}..={}",
            STORAGE_SCHEMA_VERSION,
            manifest.storage_schema.minimum_readable,
            manifest.storage_schema.maximum_readable
        )));
    }
    if manifest.test_fixture {
        if !allow_test_key || manifest.channel != "test" || manifest.signing_key_id != TEST_KEY_ID {
            return Err(ReleaseError::Rejected(
                "test fixture requires explicit test trust and the test channel".to_string(),
            ));
        }
    } else if manifest.channel != selected_channel {
        return Err(ReleaseError::Rejected(format!(
            "manifest channel {} does not match selected channel {}",
            manifest.channel, selected_channel
        )));
    }
    let current = Version::parse(current_version)?;
    let minimum = Version::parse(&manifest.minimum_compatible_version)?;
    if current < minimum {
        return Err(ReleaseError::Rejected(format!(
            "current version {current} is older than minimum compatible version {minimum}"
        )));
    }
    let offered = Version::parse(&manifest.version)?;
    if offered < current {
        return Err(ReleaseError::Rejected(format!(
            "downgrade from {current} to {offered} is not allowed"
        )));
    }
    if offered == current {
        Ok(UpdateDecision::Current)
    } else {
        Ok(UpdateDecision::UpdateAvailable { version: offered })
    }
}

pub fn validate_artifact(artifact: &ReleaseArtifact, bytes: &[u8]) -> ReleaseResult<()> {
    if artifact.size != bytes.len() as u64 {
        return Err(ReleaseError::Rejected(format!(
            "artifact {} size mismatch: expected {}, got {}",
            artifact.name,
            artifact.size,
            bytes.len()
        )));
    }
    let actual = sha256_bytes(bytes);
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(ReleaseError::Rejected(format!(
            "artifact {} SHA-256 mismatch",
            artifact.name
        )));
    }
    Ok(())
}

pub fn safe_artifact_name(name: &str) -> bool {
    let path = Path::new(name);
    !name.trim().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn normalize_arch(value: &str) -> &str {
    match value {
        "amd64" | "x64" => "x86_64",
        other => other,
    }
}

fn normalize_os(value: &str) -> &str {
    match value {
        "darwin" | "osx" => "macos",
        other => other,
    }
}

fn decode_32(value: &str, label: &str) -> ReleaseResult<[u8; 32]> {
    let decoded = hex::decode(value)
        .map_err(|error| ReleaseError::SigningKey(format!("{label}: {error}")))?;
    decoded
        .try_into()
        .map_err(|_| ReleaseError::SigningKey(format!("{label} must contain exactly 32 bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(version: &str) -> ReleaseManifest {
        ReleaseManifest {
            product: PRODUCT.to_string(),
            version: version.to_string(),
            channel: "test".to_string(),
            commit_sha: "abc".to_string(),
            build_id: "build".to_string(),
            build_timestamp: "2026-08-26T00:00:00Z".to_string(),
            os: "windows".to_string(),
            architecture: "x86_64".to_string(),
            registry_schema_version: REGISTRY_SCHEMA_VERSION,
            storage_schema: StorageSchemaCompatibility {
                current: STORAGE_SCHEMA_VERSION,
                minimum_readable: STORAGE_SCHEMA_VERSION,
                maximum_readable: STORAGE_SCHEMA_VERSION,
            },
            minimum_compatible_version: "0.0.1".to_string(),
            signing_key_id: TEST_KEY_ID.to_string(),
            test_fixture: true,
            artifacts: Vec::new(),
        }
    }

    #[test]
    fn test_fixture_signatures_require_explicit_trust() {
        let bytes = serde_json::to_vec_pretty(&fixture("0.0.2")).unwrap();
        let signature = sign_test_fixture(&bytes).unwrap();
        assert!(verify_signature(&bytes, &signature, false).is_err());
        verify_signature(&bytes, &signature, true).unwrap();
    }

    #[test]
    fn bad_signature_is_rejected() {
        let bytes = serde_json::to_vec_pretty(&fixture("0.0.2")).unwrap();
        let signature = sign_test_fixture(&bytes).unwrap();
        assert!(verify_signature(b"tampered", &signature, true).is_err());
    }

    #[test]
    fn update_validation_rejects_downgrade() {
        assert!(validate_manifest(&fixture("0.0.0"), "0.0.1", "stable", true).is_err());
    }

    #[test]
    fn signed_hash_authenticates_artifact_bytes() {
        let payload = b"update payload";
        let artifact = ReleaseArtifact {
            role: "update_bundle".to_string(),
            name: "update.zip".to_string(),
            size: payload.len() as u64,
            sha256: sha256_bytes(payload),
            url: None,
        };
        validate_artifact(&artifact, payload).unwrap();
        assert!(validate_artifact(&artifact, b"corrupt").is_err());
    }

    #[test]
    fn artifact_names_refuse_traversal() {
        assert!(safe_artifact_name("Takokit/update.zip"));
        assert!(!safe_artifact_name("../evil.exe"));
        assert!(!safe_artifact_name("/absolute.exe"));
    }

    #[test]
    fn distribution_metadata_prefers_channel_specific_manifest() {
        let metadata: DistributionMetadata = serde_json::from_str(
            r#"{"product":"Takokit","version":"0.0.1","mode":"installed","update_manifest_url":"https://example.test/stable.json","update_manifest_urls":{"stable":"https://example.test/stable-v2.json","preview":"https://example.test/preview.json"},"default_channel":"stable"}"#,
        )
        .unwrap();
        assert_eq!(
            metadata.manifest_url_for_channel("stable").as_deref(),
            Some("https://example.test/stable-v2.json")
        );
        assert_eq!(
            metadata.manifest_url_for_channel("preview").as_deref(),
            Some("https://example.test/preview.json")
        );
    }
}
