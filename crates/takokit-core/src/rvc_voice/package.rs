use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRvcVoiceRequest {
    pub output: PathBuf,
    #[serde(default)]
    pub sign: bool,
    #[serde(default)]
    pub include_reference: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRvcPackageRequest {
    pub package: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRvcPackageRequest {
    pub package: PathBuf,
    pub name: Option<String>,
    pub consent_affirmed: bool,
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcPackageArtifact {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcPackageManifest {
    pub schema_version: u32,
    pub engine: String,
    pub voice_name: String,
    pub exported_at: u64,
    pub checkpoint: RvcPackageArtifact,
    pub index: Option<RvcPackageArtifact>,
    pub reference: Option<RvcPackageArtifact>,
    pub consent_acknowledged: bool,
    pub provenance_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcPackageSignature {
    pub algorithm: String,
    pub public_key_hex: String,
    pub signature_hex: String,
    pub signer_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcPackageVerification {
    pub schema_version: u32,
    pub package_path: PathBuf,
    pub signed: bool,
    pub signature_valid: Option<bool>,
    pub signer_fingerprint: Option<String>,
    pub hashes_valid: bool,
    pub voice_name: Option<String>,
    pub errors: Vec<String>,
}
