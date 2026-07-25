//! Ollama-style model references and the Takokit registry control plane.

use crate::{ModelManifest, PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};

pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryIndex {
    pub schema_version: u32,
    pub namespace: String,
    pub generated_at: String,
    pub models: Vec<RegistryModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryModel {
    pub name: String,
    pub display_name: String,
    pub default_tag: String,
    pub summary: String,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub tags: Vec<RegistryTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryTag {
    pub tag: String,
    pub target: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub version: String,
    pub digest: String,
    #[serde(default)]
    pub size_bytes: u64,
    pub runner: String,
    #[serde(default)]
    pub adapter: Option<String>,
    pub license: String,
    pub kind: String,
    pub backend: String,
    pub hardware: RegistryHardware,
    pub source: RegistrySource,
    #[serde(default)]
    pub manifest_toml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryHardware {
    pub cpu: bool,
    pub gpu: bool,
    #[serde(default)]
    pub min_ram: Option<String>,
    #[serde(default)]
    pub min_vram: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySource {
    pub provider: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedModelReference {
    pub requested: String,
    pub canonical: String,
    pub namespace: String,
    pub name: String,
    pub tag: String,
    pub digest: String,
    pub target: String,
}

impl RegistryIndex {
    pub fn read(path: &Path) -> PackageResult<Self> {
        let bytes = std::fs::read(path)?;
        let index: Self = serde_json::from_slice(&bytes)?;
        index.validate()?;
        Ok(index)
    }

    pub fn validate(&self) -> PackageResult<()> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(invalid_registry(format!(
                "unsupported registry schema {}; expected {}",
                self.schema_version, REGISTRY_SCHEMA_VERSION
            )));
        }
        if self.namespace.trim().is_empty() {
            return Err(invalid_registry("registry namespace is empty"));
        }

        let mut names = HashSet::new();
        let mut aliases = HashSet::new();
        for model in &self.models {
            if model.name.trim().is_empty() || !names.insert(model.name.to_ascii_lowercase()) {
                return Err(invalid_registry(format!(
                    "duplicate or empty model family: {}",
                    model.name
                )));
            }
            let mut tags = HashSet::new();
            for release in &model.tags {
                if release.tag.trim().is_empty() || !tags.insert(release.tag.to_ascii_lowercase()) {
                    return Err(invalid_registry(format!(
                        "duplicate or empty tag for {}: {}",
                        model.name, release.tag
                    )));
                }
                validate_release(release)?;
                for alias in &release.aliases {
                    let key = alias.to_ascii_lowercase();
                    if !aliases.insert(key) {
                        return Err(invalid_registry(format!("duplicate model alias: {alias}")));
                    }
                }
            }
            if !model
                .tags
                .iter()
                .any(|release| release.tag.eq_ignore_ascii_case(&model.default_tag))
            {
                return Err(invalid_registry(format!(
                    "default tag {} is missing for {}",
                    model.default_tag, model.name
                )));
            }
        }
        Ok(())
    }

    pub fn resolve(&self, reference: &str) -> PackageResult<ResolvedModelReference> {
        let requested = reference.trim();
        if requested.is_empty() {
            return Err(PackageError::ModelNotFound(reference.to_string()));
        }
        let (without_digest, expected_digest) = requested
            .rsplit_once('@')
            .map(|(name, digest)| (name, Some(digest)))
            .unwrap_or((requested, None));
        let without_namespace = without_digest
            .strip_prefix(&format!("{}/", self.namespace))
            .unwrap_or(without_digest);
        let (name, explicit_tag) = split_name_tag(without_namespace);

        let (family, release) = if explicit_tag.is_none() {
            if let Some(family) = self
                .models
                .iter()
                .find(|family| family.name.eq_ignore_ascii_case(name))
            {
                let release = if let Some(digest) = expected_digest {
                    family
                        .tags
                        .iter()
                        .find(|release| digest_matches(&release.digest, digest))
                } else {
                    family
                        .tags
                        .iter()
                        .find(|release| release.tag.eq_ignore_ascii_case(&family.default_tag))
                };
                (family, release)
            } else if let Some((family, release)) = self.models.iter().find_map(|family| {
                family
                    .tags
                    .iter()
                    .find(|release| {
                        release
                            .aliases
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(name))
                    })
                    .map(|release| (family, Some(release)))
            }) {
                (family, release)
            } else {
                return Err(PackageError::ModelNotFound(reference.to_string()));
            }
        } else {
            let family = self
                .models
                .iter()
                .find(|family| {
                    family.name.eq_ignore_ascii_case(name)
                        || family
                            .aliases
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(name))
                })
                .ok_or_else(|| PackageError::ModelNotFound(reference.to_string()))?;
            let requested_tag = explicit_tag.expect("checked tag");
            let requested_tag = if requested_tag.eq_ignore_ascii_case("latest")
                && !family
                    .tags
                    .iter()
                    .any(|release| release.tag.eq_ignore_ascii_case("latest"))
            {
                family.default_tag.as_str()
            } else {
                requested_tag
            };
            (
                family,
                family
                    .tags
                    .iter()
                    .find(|release| release.tag.eq_ignore_ascii_case(requested_tag)),
            )
        };
        let release = release.ok_or_else(|| PackageError::ModelNotFound(reference.to_string()))?;
        if let Some(expected) = expected_digest {
            if !digest_matches(&release.digest, expected) {
                return Err(PackageError::ArtifactChecksumMismatch {
                    artifact: format!("{}:{} registry manifest", family.name, release.tag),
                    expected: expected.to_string(),
                    actual: release.digest.clone(),
                });
            }
        }

        Ok(ResolvedModelReference {
            requested: requested.to_string(),
            canonical: format!("{}:{}", family.name, release.tag),
            namespace: self.namespace.clone(),
            name: family.name.clone(),
            tag: release.tag.clone(),
            digest: release.digest.clone(),
            target: release.target.clone(),
        })
    }

    pub fn release(&self, resolved: &ResolvedModelReference) -> Option<&RegistryTag> {
        self.models
            .iter()
            .find(|model| model.name == resolved.name)
            .and_then(|model| model.tags.iter().find(|tag| tag.tag == resolved.tag))
    }

    pub fn canonical_for_target(&self, target: &str) -> Option<String> {
        self.models.iter().find_map(|model| {
            model
                .tags
                .iter()
                .find(|release| release.target == target)
                .map(|release| format!("{}:{}", model.name, release.tag))
        })
    }
}

pub fn manifest_digest(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n");
    format!("sha256:{:x}", Sha256::digest(normalized.as_bytes()))
}

fn validate_release(release: &RegistryTag) -> PackageResult<()> {
    if release.target.trim().is_empty() {
        return Err(invalid_registry(format!(
            "tag {} has an empty target",
            release.tag
        )));
    }
    let digest = release.digest.strip_prefix("sha256:").unwrap_or_default();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_registry(format!(
            "tag {} has an invalid manifest digest",
            release.tag
        )));
    }
    if let Some(source) = release.manifest_toml.as_deref() {
        let actual = manifest_digest(source);
        if !actual.eq_ignore_ascii_case(&release.digest) {
            return Err(PackageError::ArtifactChecksumMismatch {
                artifact: format!("{} registry manifest", release.target),
                expected: release.digest.clone(),
                actual,
            });
        }
        let manifest: ModelManifest = toml::from_str(source)?;
        if manifest.id != release.target {
            return Err(invalid_registry(format!(
                "tag {} targets {}, but its manifest identifies {}",
                release.tag, release.target, manifest.id
            )));
        }
    }
    Ok(())
}

fn split_name_tag(reference: &str) -> (&str, Option<&str>) {
    let slash = reference.rfind('/').map(|index| index + 1).unwrap_or(0);
    reference[slash..]
        .rfind(':')
        .map(|relative| slash + relative)
        .map(|index| (&reference[..index], Some(&reference[index + 1..])))
        .unwrap_or((reference, None))
}

fn digest_matches(actual: &str, expected: &str) -> bool {
    let actual = actual.strip_prefix("sha256:").unwrap_or(actual);
    let expected = expected.strip_prefix("sha256:").unwrap_or(expected);
    actual.eq_ignore_ascii_case(expected)
}

fn invalid_registry(reason: impl Into<String>) -> PackageError {
    PackageError::ArtifactInstallFailed {
        artifact: "takokit-registry".to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> RegistryIndex {
        let manifest = r#"id = "whisper-base"
name = "Whisper Base"
family = "whisper"
version = "0.1.0"
kind = "stt"
backend = "whispercpp"
runner = "takokit-whispercpp"
license = "mit"
description = "fixture"
[capabilities]
stt = true
[hardware]
cpu = true
gpu = false
[artifacts]
metadata_only = true
"#;
        RegistryIndex {
            schema_version: REGISTRY_SCHEMA_VERSION,
            namespace: "library".into(),
            generated_at: "0".into(),
            models: vec![RegistryModel {
                name: "whisper".into(),
                display_name: "Whisper".into(),
                default_tag: "base".into(),
                summary: "Speech recognition".into(),
                tasks: vec!["stt".into()],
                aliases: Vec::new(),
                tags: vec![RegistryTag {
                    tag: "base".into(),
                    target: "whisper-base".into(),
                    aliases: vec!["whisper-base".into()],
                    version: "0.1.0".into(),
                    digest: manifest_digest(manifest),
                    size_bytes: 1,
                    runner: "takokit-whispercpp".into(),
                    adapter: None,
                    license: "mit".into(),
                    kind: "stt".into(),
                    backend: "whispercpp".into(),
                    hardware: RegistryHardware {
                        cpu: true,
                        gpu: false,
                        min_ram: None,
                        min_vram: None,
                    },
                    source: RegistrySource {
                        provider: "artifact".into(),
                        repository: None,
                        revision: None,
                    },
                    manifest_toml: Some(manifest.into()),
                }],
            }],
        }
    }

    #[test]
    fn defaults_tags_aliases_namespaces_and_digests_resolve() {
        let index = fixture();
        index.validate().expect("valid index");
        let plain = index.resolve("whisper").expect("default tag");
        let latest = index.resolve("whisper:latest").expect("latest alias");
        let legacy = index.resolve("whisper-base").expect("legacy alias");
        let namespaced = index.resolve("library/whisper:base").expect("namespace");
        let pinned = index
            .resolve(&format!("whisper@{}", plain.digest))
            .expect("digest");
        for resolved in [latest, legacy, namespaced, pinned] {
            assert_eq!(resolved.canonical, "whisper:base");
            assert_eq!(resolved.target, "whisper-base");
        }
    }

    #[test]
    fn wrong_digest_is_rejected() {
        let error = fixture()
            .resolve(&format!("whisper@sha256:{}", "0".repeat(64)))
            .expect_err("wrong digest");
        assert!(matches!(error, PackageError::ModelNotFound(_)));
    }
}
