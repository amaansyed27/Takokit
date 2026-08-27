use std::path::{Path, PathBuf};
use takokit_release::DistributionMetadata;

pub(crate) fn configure_installed_resources() {
    let Some(root) = application_root() else {
        return;
    };
    if distribution_metadata_at(&root).is_none() {
        return;
    }
    let gui = root.join("resources").join("gui");
    if std::env::var_os("TAKOKIT_GUI_DIST").is_none() && gui.join("index.html").is_file() {
        std::env::set_var("TAKOKIT_GUI_DIST", gui);
    }
    let registry = root.join("resources").join("registry");
    if std::env::var_os("TAKOKIT_REGISTRY_DIR").is_none() && registry.join("index.json").is_file() {
        std::env::set_var("TAKOKIT_REGISTRY_DIR", registry);
    }
}

pub(crate) fn distribution_metadata() -> Option<DistributionMetadata> {
    distribution_metadata_at(&application_root()?)
}

pub(crate) fn application_root() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    if directory.file_name().and_then(|name| name.to_str()) == Some("bin") {
        return directory.parent().map(Path::to_path_buf);
    }
    Some(directory.to_path_buf())
}

pub(crate) fn desktop_executable() -> Option<PathBuf> {
    let candidate = application_root()?.join("bin").join("Takokit.exe");
    candidate.is_file().then_some(candidate)
}

pub(crate) fn updater_executable() -> Option<PathBuf> {
    let candidate = application_root()?.join("bin").join("takokit-updater.exe");
    candidate.is_file().then_some(candidate)
}

fn distribution_metadata_at(root: &Path) -> Option<DistributionMetadata> {
    let path = root.join("distribution.json");
    let metadata: DistributionMetadata = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (metadata.product == takokit_release::PRODUCT).then_some(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_root_never_uses_current_working_directory() {
        let root = application_root().expect("current executable has a directory");
        assert!(root.is_absolute());
    }
}
