//! Safe local custom-model manifests layered on verified Takokit runners.

use crate::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CUSTOM_MODEL_SCHEMA_VERSION: u32 = 1;

const SUPPORTED_PYTHON_BASES: &[&str] = &[
    "qwen3-tts",
    "qwen3-tts-0.6b-base",
    "qwen3-tts-1.7b-custom",
    "qwen3-tts-1.7b-base",
    "qwen3-tts-1.7b-voice-design",
    "chatterbox",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomModelSpec {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub extends: String,
    pub version: String,
    pub license: String,
    pub description: String,
    #[serde(default)]
    pub source: Option<ModelSourceManifest>,
    #[serde(default)]
    pub artifacts: ArtifactManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomModelRecord {
    pub spec: CustomModelSpec,
    pub manifest: ModelManifest,
    pub path: PathBuf,
    pub canonical_reference: String,
}

pub fn custom_models_dir(takokit_root: &Path) -> PathBuf {
    takokit_root
        .join("manifests")
        .join("custom")
        .join("models")
}

pub fn register_custom_model(
    takokit_root: &Path,
    registry: &PackageRegistry,
    manifest_path: &Path,
) -> PackageResult<CustomModelRecord> {
    let source = std::fs::read_to_string(manifest_path)?;
    let spec: CustomModelSpec = toml::from_str(&source)?;
    validate_custom_model_spec(registry, &spec)?;

    let directory = custom_models_dir(takokit_root);
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.toml", spec.id));
    if path.exists() {
        return Err(invalid_custom_model(format!(
            "custom model {} already exists; remove it before registering a replacement",
            spec.id
        )));
    }

    match registry.model(&spec.id) {
        Ok(_) => {
            return Err(invalid_custom_model(format!(
                "custom model id {} would shadow a registry model",
                spec.id
            )))
        }
        Err(PackageError::ModelNotFound(_)) => {}
        Err(error) => return Err(error),
    }

    let manifest = materialize_custom_model(registry, &spec)?;
    let encoded = toml::to_string_pretty(&spec)?;
    let temporary = path.with_extension(format!(
        "toml.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&temporary, encoded)?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        PackageError::Io(error)
    })?;

    Ok(CustomModelRecord {
        canonical_reference: format!("local/{}:latest", spec.id),
        spec,
        manifest,
        path,
    })
}

pub fn custom_model_records(
    takokit_root: &Path,
    registry: &PackageRegistry,
) -> PackageResult<Vec<CustomModelRecord>> {
    let directory = custom_models_dir(takokit_root);
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = std::fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut records = Vec::new();
    for entry in entries {
        if entry.path().extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let spec = read_custom_model_spec(&entry.path())?;
        let manifest = materialize_custom_model(registry, &spec)?;
        records.push(CustomModelRecord {
            canonical_reference: format!("local/{}:latest", spec.id),
            spec,
            manifest,
            path: entry.path(),
        });
    }
    Ok(records)
}

pub fn custom_model_record(
    takokit_root: &Path,
    registry: &PackageRegistry,
    reference: &str,
) -> PackageResult<CustomModelRecord> {
    let id = require_custom_model_id(reference)?;
    let path = custom_models_dir(takokit_root).join(format!("{id}.toml"));
    if !path.is_file() {
        return Err(PackageError::ModelNotFound(reference.to_string()));
    }
    let spec = read_custom_model_spec(&path)?;
    let manifest = materialize_custom_model(registry, &spec)?;
    Ok(CustomModelRecord {
        canonical_reference: format!("local/{id}:latest"),
        spec,
        manifest,
        path,
    })
}

pub fn remove_custom_model(takokit_root: &Path, reference: &str) -> PackageResult<bool> {
    let id = require_custom_model_id(reference)?;
    let path = custom_models_dir(takokit_root).join(format!("{id}.toml"));
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(PackageError::Io(error)),
    }
}

pub fn require_custom_model_id(reference: &str) -> PackageResult<String> {
    custom_reference_id(reference)?.ok_or_else(|| {
        invalid_custom_model(format!(
            "{reference} is not a local custom-model reference; use <id> or local/<id>:latest"
        ))
    })
}

pub(crate) fn custom_reference_id(reference: &str) -> PackageResult<Option<String>> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Ok(None);
    }
    if let Some(local) = reference.strip_prefix("local/") {
        if local.contains('@') {
            return Err(invalid_custom_model(
                "local custom models do not accept digest suffixes".to_string(),
            ));
        }
        let id = match local.split_once(':') {
            Some((id, "latest")) => id,
            Some((_, tag)) => {
                return Err(invalid_custom_model(format!(
                    "local custom models currently support only the latest tag, not {tag}"
                )))
            }
            None => local,
        };
        validate_custom_model_id(id)?;
        return Ok(Some(id.to_string()));
    }
    if reference.contains(['/', ':', '@']) {
        return Ok(None);
    }
    if validate_custom_model_id(reference).is_ok() {
        Ok(Some(reference.to_string()))
    } else {
        Ok(None)
    }
}

pub(crate) fn read_custom_model_spec(path: &Path) -> PackageResult<CustomModelSpec> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

pub(crate) fn materialize_custom_model(
    registry: &PackageRegistry,
    spec: &CustomModelSpec,
) -> PackageResult<ModelManifest> {
    validate_custom_model_spec(registry, spec)?;
    let base = registry.read_bundled_model(&spec.extends)?;
    let family = if base.backend == ModelBackend::PythonManaged {
        base.id.clone()
    } else {
        base.family.clone()
    };
    Ok(ModelManifest {
        id: spec.id.clone(),
        name: spec.name.trim().to_string(),
        family,
        version: spec.version.trim().to_string(),
        kind: base.kind,
        backend: base.backend,
        runner: base.runner,
        required_adapter: base.required_adapter,
        license: spec.license.trim().to_string(),
        description: spec.description.trim().to_string(),
        capabilities: base.capabilities,
        hardware: base.hardware,
        source: spec.source.clone(),
        artifacts: spec.artifacts.clone(),
    })
}

fn validate_custom_model_spec(
    registry: &PackageRegistry,
    spec: &CustomModelSpec,
) -> PackageResult<()> {
    if spec.schema_version != CUSTOM_MODEL_SCHEMA_VERSION {
        return Err(invalid_custom_model(format!(
            "unsupported schema_version {}; expected {}",
            spec.schema_version, CUSTOM_MODEL_SCHEMA_VERSION
        )));
    }
    validate_custom_model_id(&spec.id)?;
    for (label, value) in [
        ("name", spec.name.as_str()),
        ("version", spec.version.as_str()),
        ("license", spec.license.as_str()),
        ("description", spec.description.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(invalid_custom_model(format!("{label} cannot be empty")));
        }
    }
    if spec.artifacts.metadata_only {
        return Err(invalid_custom_model(
            "custom models must contain executable checkpoint material, not metadata_only"
                .to_string(),
        ));
    }

    let base = registry.read_bundled_model(&spec.extends).map_err(|_| {
        invalid_custom_model(format!(
            "extends must name a bundled Takokit model, got {}",
            spec.extends
        ))
    })?;
    let python_supported = SUPPORTED_PYTHON_BASES.contains(&base.id.as_str());
    let whisper_supported = base.backend == ModelBackend::Whispercpp
        && base.family.eq_ignore_ascii_case("whisper")
        && base.capabilities.stt;
    if !python_supported && !whisper_supported {
        return Err(invalid_custom_model(format!(
            "{} is not a custom-model base with a verified generic runner contract",
            base.id
        )));
    }

    if python_supported {
        let source = spec.source.as_ref().ok_or_else(|| {
            invalid_custom_model(format!(
                "{} custom checkpoints require a pinned Hugging Face source",
                base.id
            ))
        })?;
        validate_hugging_face_source(source)?;
    } else {
        if spec.source.is_some() {
            return Err(invalid_custom_model(
                "custom whisper.cpp models use one checksum-pinned model artifact, not a source snapshot"
                    .to_string(),
            ));
        }
        let model_artifacts = spec
            .artifacts
            .weights
            .iter()
            .filter(|artifact| artifact.role == ArtifactRole::Model)
            .collect::<Vec<_>>();
        if model_artifacts.len() != 1 {
            return Err(invalid_custom_model(
                "custom whisper.cpp models require exactly one weight with role = \"model\""
                    .to_string(),
            ));
        }
    }

    for artifact in spec.artifacts.all() {
        let url = artifact.url.as_deref().ok_or_else(|| {
            invalid_custom_model(format!("artifact {} requires an HTTPS URL", artifact.name))
        })?;
        if !url.starts_with("https://") {
            return Err(invalid_custom_model(format!(
                "artifact {} URL must use HTTPS",
                artifact.name
            )));
        }
        if artifact.sha256.len() != 64
            || !artifact
                .sha256
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        {
            return Err(invalid_custom_model(format!(
                "artifact {} requires a 64-character SHA-256 digest",
                artifact.name
            )));
        }
    }
    if spec.source.is_none() && spec.artifacts.all().next().is_none() {
        return Err(invalid_custom_model(
            "custom model must provide a pinned source or checksum-pinned artifacts".to_string(),
        ));
    }
    Ok(())
}

fn validate_hugging_face_source(source: &ModelSourceManifest) -> PackageResult<()> {
    if source.provider != ModelSourceProvider::HuggingFace {
        return Err(invalid_custom_model(
            "only Hugging Face custom-model sources are supported".to_string(),
        ));
    }
    let parts = source.repository.split('/').collect::<Vec<_>>();
    if parts.len() != 2 || parts.iter().any(|part| part.trim().is_empty()) {
        return Err(invalid_custom_model(
            "Hugging Face repository must use owner/name".to_string(),
        ));
    }
    if source.revision.len() != 40
        || !source
            .revision
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(invalid_custom_model(
            "Hugging Face revision must be a pinned 40-character commit SHA".to_string(),
        ));
    }
    Ok(())
}

fn validate_custom_model_id(id: &str) -> PackageResult<()> {
    let id = id.trim();
    if id.is_empty()
        || id == "."
        || id == ".."
        || !id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
    {
        return Err(invalid_custom_model(
            "id must contain only ASCII letters, numbers, '.', '_' or '-'".to_string(),
        ));
    }
    Ok(())
}

fn invalid_custom_model(message: String) -> PackageError {
    PackageError::InvalidCustomModel(message)
}
