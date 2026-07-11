//! Mutation and queries for locally installed models, runners, and artifacts.

use crate::{
    artifact_io::{install_artifact, sha256_file},
    artifact_reuse,
    install_support::*,
    registry::read_manifest_dir,
    *,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]

pub struct InstalledRegistry {
    root: PathBuf,
}

impl InstalledRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn installed_model(&self, id: &str) -> PackageResult<ModelManifest> {
        std::fs::read_to_string(self.model_manifest_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotInstalled(id.to_string()),

                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
    }

    pub fn installed_model_record(&self, id: &str) -> PackageResult<InstalledModelRecord> {
        std::fs::read_to_string(self.model_record_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotInstalled(id.to_string()),

                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
            .or_else(|error| match error {
                PackageError::ModelNotInstalled(_) if self.model_manifest_path(id).is_file() => {
                    let manifest = self.installed_model(id)?;

                    Ok(installed_model_record(
                        &manifest,
                        self.model_manifest_path(id),
                    ))
                }

                _ => Err(error),
            })
    }

    pub fn installed_runner_record(&self, id: &str) -> PackageResult<InstalledRunnerRecord> {
        std::fs::read_to_string(self.runner_record_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => {
                    PackageError::RunnerPackageNotInstalled(id.to_string())
                }

                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
            .or_else(|error| match error {
                PackageError::RunnerPackageNotInstalled(_)
                    if self.runner_manifest_path(id).is_file() =>
                {
                    let source = std::fs::read_to_string(self.runner_manifest_path(id))?;

                    let manifest: RunnerManifest = toml::from_str(&source)?;

                    Ok(installed_runner_record(
                        &manifest,
                        self.runner_manifest_path(id),
                    ))
                }

                _ => Err(error),
            })
    }

    pub fn installed_model_records(&self) -> PackageResult<Vec<InstalledModelRecord>> {
        read_manifest_dir(&self.root.join("installed-models"))
    }

    pub fn installed_runner_records(&self) -> PackageResult<Vec<InstalledRunnerRecord>> {
        read_manifest_dir(&self.root.join("installed-runners"))
    }

    pub fn is_model_installed(&self, id: &str) -> bool {
        self.model_record_path(id).is_file() || self.model_manifest_path(id).is_file()
    }

    pub fn is_runner_installed(&self, id: &str) -> bool {
        self.runner_record_path(id).is_file() || self.runner_manifest_path(id).is_file()
    }

    pub fn install_model(&self, manifest: &ModelManifest) -> PackageResult<PullReport> {
        self.install_model_with_options(manifest, InstallModelOptions::default())
    }

    pub fn install_model_with_options(
        &self,

        manifest: &ModelManifest,

        options: InstallModelOptions,
    ) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("models"))?;

        std::fs::create_dir_all(self.root.join("installed-models"))?;

        let path = self.model_manifest_path(&manifest.id);

        let previous = self.installed_model_record(&manifest.id).ok();

        if options.metadata_only
            && previous
                .as_ref()
                .is_some_and(|record| record.status == InstalledPackageStatus::Ready)
        {
            return Ok(PullReport {
                id: manifest.id.clone(),

                installed: false,

                manifest_path: path,

                note: "Metadata-only request preserved the existing verified ready installation."
                    .to_string(),
            });
        }

        if options.metadata_only
            && previous
                .as_ref()
                .is_some_and(|record| record.status == InstalledPackageStatus::MetadataOnly)
        {
            return Ok(PullReport {
                id: manifest.id.clone(),

                installed: false,

                manifest_path: path,

                note: "Metadata-only model record is already present.".to_string(),
            });
        }

        let artifact_set = self.install_artifacts(manifest, options, previous.as_ref())?;

        self.materialize_model_artifacts(manifest, &artifact_set)?;

        let record = installed_model_record_with_artifacts(manifest, path.clone(), artifact_set);

        write_model_install_files(
            &path,
            &self.model_record_path(&manifest.id),
            &toml::to_string_pretty(manifest)?,
            &toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),

            installed: true,

            manifest_path: path,

            note: record.note,
        })
    }

    pub fn install_runner(&self, manifest: &RunnerManifest) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("runners"))?;

        std::fs::create_dir_all(self.root.join("installed-runners"))?;

        let path = self.runner_manifest_path(&manifest.id);

        std::fs::write(&path, toml::to_string_pretty(manifest)?)?;

        let record = self
            .installed_runner_record(&manifest.id)
            .map(|mut record| {
                // Pulling a current manifest must never downgrade a ready or

                // failed runtime record back to a contract-only state.

                record.version = manifest.version.clone();

                record.kind = manifest.kind.as_str().to_string();

                record.manifest_path = path.clone();

                record.platforms = manifest.platforms.clone();

                record
            })
            .unwrap_or_else(|_| installed_runner_record(manifest, path.clone()));

        std::fs::write(
            self.runner_record_path(&manifest.id),
            toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),

            installed: true,

            manifest_path: path,

            note: record.note,
        })
    }

    pub fn install_runner_runtime(
        &self,

        manifest: &RunnerManifest,

        status: RunnerLifecycleState,

        note: impl Into<String>,
    ) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("runners"))?;

        std::fs::create_dir_all(self.root.join("installed-runners"))?;

        let path = self.runner_manifest_path(&manifest.id);

        if !path.is_file() {
            std::fs::write(&path, toml::to_string_pretty(manifest)?)?;
        }

        let mut record = self
            .installed_runner_record(&manifest.id)
            .unwrap_or_else(|_| installed_runner_record(manifest, path.clone()));

        record.status = status;

        record.note = note.into();

        record.installed_at = timestamp_now();

        std::fs::write(
            self.runner_record_path(&manifest.id),
            toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),

            installed: true,

            manifest_path: path,

            note: record.note,
        })
    }

    pub fn remove_model(&self, id: &str) -> PackageResult<bool> {
        let manifest_path = self.model_manifest_path(id);

        let record_path = self.model_record_path(id);

        if !manifest_path.exists() && !record_path.exists() {
            return Err(PackageError::ModelNotInstalled(id.to_string()));
        }

        remove_file_if_exists(manifest_path)?;

        remove_file_if_exists(record_path)?;

        Ok(true)
    }

    pub fn remove_runner(&self, id: &str) -> PackageResult<bool> {
        let manifest_path = self.runner_manifest_path(id);

        let record_path = self.runner_record_path(id);

        if !manifest_path.exists() && !record_path.exists() {
            return Err(PackageError::RunnerPackageNotInstalled(id.to_string()));
        }

        remove_file_if_exists(manifest_path)?;

        remove_file_if_exists(record_path)?;

        Ok(true)
    }

    pub(crate) fn model_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("models").join(format!("{id}.toml"))
    }

    fn runner_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("runners").join(format!("{id}.toml"))
    }

    pub(crate) fn model_record_path(&self, id: &str) -> PathBuf {
        self.root
            .join("installed-models")
            .join(format!("{id}.toml"))
    }

    fn runner_record_path(&self, id: &str) -> PathBuf {
        self.root
            .join("installed-runners")
            .join(format!("{id}.toml"))
    }

    pub fn storage_root(&self) -> PathBuf {
        self.root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.clone())
    }

    fn install_artifacts(
        &self,

        manifest: &ModelManifest,

        options: InstallModelOptions,

        previous: Option<&InstalledModelRecord>,
    ) -> PackageResult<InstalledArtifactSet> {
        let artifacts = manifest.artifacts.all().collect::<Vec<_>>();

        let metadata_only = options.metadata_only || manifest.artifacts.metadata_only;

        if artifacts.is_empty() || metadata_only {
            return Ok(InstalledArtifactSet {
                records: installed_artifacts(&manifest.artifacts),

                status: InstalledPackageStatus::MetadataOnly,

                note: if artifacts.is_empty() {
                    "Installed model metadata from local mock registry. No model weights were downloaded."

                        .to_string()
                } else {
                    "Installed model metadata from local mock registry. Artifact URLs are recorded, but downloads were skipped because this manifest is metadata-only."

                        .to_string()
                },
            });
        }

        let root = self.storage_root();

        let downloads_dir = root.join("cache").join("downloads");

        let blob_dir = root.join("blobs").join("sha256");

        std::fs::create_dir_all(&downloads_dir)?;

        std::fs::create_dir_all(&blob_dir)?;

        let mut records = Vec::new();

        for artifact in artifacts {
            let prior = previous.and_then(|record| {
                record
                    .artifacts
                    .iter()
                    .find(|record| record.name == artifact.name)
            });

            let local_path =
                match prior.filter(|record| artifact_reuse::is_verified(record, artifact)) {
                    Some(record) => record
                        .local_path
                        .clone()
                        .expect("verified artifact has path"),

                    None => install_artifact(manifest, artifact, &downloads_dir, &blob_dir)?,
                };

            records.push(InstalledArtifactRecord {
                name: artifact.name.clone(),

                sha256: artifact.sha256.clone(),

                bytes: artifact.bytes,

                url: artifact.url.clone(),

                role: artifact.role,

                local_path: Some(local_path),

                downloaded: true,
            });
        }

        Ok(InstalledArtifactSet {
            records,

            status: InstalledPackageStatus::Ready,

            note: "Installed model metadata and verified artifacts into content-addressed blobs."
                .to_string(),
        })
    }

    fn materialize_model_artifacts(
        &self,

        manifest: &ModelManifest,

        artifact_set: &InstalledArtifactSet,
    ) -> PackageResult<()> {
        if artifact_set.status != InstalledPackageStatus::Ready {
            return Ok(());
        }

        let model_dir = self.storage_root().join("models").join(&manifest.id);

        for artifact in &artifact_set.records {
            let relative = Path::new(&artifact.name);

            if relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            }) {
                return Err(PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),

                    reason: "artifact name must be a safe relative path".to_string(),
                });
            }

            let source = artifact.local_path.as_ref().ok_or_else(|| {
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),

                    reason: "downloaded artifact is missing its blob path".to_string(),
                }
            })?;

            let destination = model_dir.join(relative);

            let parent =
                destination
                    .parent()
                    .ok_or_else(|| PackageError::ArtifactInstallFailed {
                        artifact: artifact.name.clone(),

                        reason: "artifact path has no parent directory".to_string(),
                    })?;

            std::fs::create_dir_all(parent)?;

            if destination.is_file()
                && artifact.bytes.is_none_or(|expected| {
                    std::fs::metadata(&destination)
                        .map(|m| m.len() == expected)
                        .unwrap_or(false)
                })
                && sha256_file(&destination)
                    .map(|checksum| checksum == artifact.sha256)
                    .unwrap_or(false)
            {
                continue;
            }

            if destination.is_file() {
                std::fs::remove_file(&destination)?;
            }

            if std::fs::hard_link(source, &destination).is_err() {
                std::fs::copy(source, &destination)?;
            }
        }

        Ok(())
    }
}
