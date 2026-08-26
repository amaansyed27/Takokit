use super::*;

#[derive(Debug, Clone)]
pub struct RvcVoiceLayout {
    pub root: PathBuf,
    pub project: PathBuf,
    pub voice: PathBuf,
    pub consent: PathBuf,
    pub samples_originals: PathBuf,
    pub samples_managed: PathBuf,
    pub sample_metadata: PathBuf,
    pub dataset_segments: PathBuf,
    pub dataset_f0: PathBuf,
    pub dataset_features: PathBuf,
    pub checkpoints: PathBuf,
    pub indexes: PathBuf,
    pub references: PathBuf,
    pub logs: PathBuf,
    pub jobs: PathBuf,
    pub packages: PathBuf,
}

impl RvcVoiceLayout {
    pub fn new(root: PathBuf) -> Self {
        Self {
            project: root.join("project.json"),
            voice: root.join("voice.json"),
            consent: root.join("consent.json"),
            samples_originals: root.join("samples").join("originals"),
            samples_managed: root.join("samples").join("managed"),
            sample_metadata: root.join("samples").join("metadata"),
            dataset_segments: root.join("dataset").join("segments"),
            dataset_f0: root.join("dataset").join("f0"),
            dataset_features: root.join("dataset").join("features"),
            checkpoints: root.join("checkpoints"),
            indexes: root.join("indexes"),
            references: root.join("references"),
            logs: root.join("logs"),
            jobs: root.join("jobs"),
            packages: root.join("packages"),
            root,
        }
    }

    pub(super) fn ensure(&self) -> TakokitResult<()> {
        for path in [
            &self.root,
            &self.samples_originals,
            &self.samples_managed,
            &self.sample_metadata,
            &self.dataset_segments,
            &self.dataset_f0,
            &self.dataset_features,
            &self.checkpoints,
            &self.indexes,
            &self.references,
            &self.logs,
            &self.jobs,
            &self.packages,
        ] {
            fs::create_dir_all(path).map_err(storage_error)?;
        }
        Ok(())
    }
}
