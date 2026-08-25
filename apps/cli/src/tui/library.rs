use std::path::PathBuf;

use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};

#[derive(Debug, Clone)]
pub struct LibraryModelRow {
    pub reference: String,
    pub model_id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub installed: bool,
    pub ready: bool,
}

impl LibraryModelRow {
    pub fn pull_reference(&self) -> Option<&str> {
        (!self.ready).then_some(self.reference.as_str())
    }
}

pub fn load_library_rows(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<Vec<LibraryModelRow>> {
    let mut rows = Vec::new();
    for family in package_registry.registry_models()? {
        for release in &family.tags {
            let reference = format!("{}:{}", family.name, release.tag);
            let model = package_registry.model(&reference)?;
            let plan = plan_model(package_registry, installed_registry, &reference)?;
            let installed = installed_registry.is_model_installed(&model.id);
            let ready = installed && plan.executable;
            let state = if ready {
                "installed · ready"
            } else if installed {
                "installed · needs repair"
            } else {
                "available"
            };
            let action = if ready {
                "Already installed and ready. Enter opens it in Installed models."
            } else if installed {
                "Installed incompletely. Enter or P repairs/re-pulls it using the normal Takokit pull flow."
            } else {
                "Not installed. Enter or P pulls it using the normal Takokit pull flow."
            };
            let hardware = format_hardware(
                release.hardware.cpu,
                release.hardware.gpu,
                release.hardware.min_ram.as_deref(),
                release.hardware.min_vram.as_deref(),
            );
            let tasks = if family.tasks.is_empty() {
                "not listed".to_string()
            } else {
                family.tasks.join(", ")
            };

            rows.push(LibraryModelRow {
                reference: reference.clone(),
                model_id: model.id.clone(),
                title: format!("{} · {}", family.display_name, release.tag),
                state: state.to_string(),
                detail: format!(
                    "{}\n\nReference: {}\nModel: {}\nTasks: {}\nVersion: {}\nFamily: {}\nRunner: {}\nBackend: {}\nLicense: {}\nRegistry size: {}\nHardware: {}\nSource: {}\n\n{}",
                    family.summary,
                    reference,
                    model.name,
                    tasks,
                    release.version,
                    model.family,
                    release.runner,
                    release.backend,
                    release.license,
                    format_size(release.size_bytes),
                    hardware,
                    release.source.provider,
                    action,
                ),
                installed,
                ready,
            });
        }
    }
    rows.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(rows)
}

fn format_hardware(
    cpu: bool,
    gpu: bool,
    min_ram: Option<&str>,
    min_vram: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if cpu {
        parts.push("CPU".to_string());
    }
    if gpu {
        parts.push("GPU".to_string());
    }
    if let Some(value) = min_ram {
        parts.push(format!("RAM {value}"));
    }
    if let Some(value) = min_vram {
        parts.push(format!("VRAM {value}"));
    }
    if parts.is_empty() {
        "not specified".to_string()
    } else {
        parts.join(" · ")
    }
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        "not listed".to_string()
    } else if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_registry() -> PackageRegistry {
        PackageRegistry::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry"))
    }

    #[test]
    fn clean_store_library_exposes_pullable_kokoro() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = bundled_registry();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        let rows = load_library_rows(&registry, &installed).expect("library rows");
        let kokoro = rows
            .iter()
            .find(|row| row.model_id == "kokoro")
            .expect("kokoro in canonical registry");

        assert_eq!(kokoro.state, "available");
        assert!(!kokoro.installed);
        assert!(!kokoro.ready);
        assert_eq!(kokoro.pull_reference(), Some(kokoro.reference.as_str()));
    }

    #[test]
    fn clean_store_library_is_registry_backed_and_non_empty() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = bundled_registry();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        let rows = load_library_rows(&registry, &installed).expect("library rows");
        let release_count: usize = registry
            .registry_models()
            .expect("registry models")
            .iter()
            .map(|family| family.tags.len())
            .sum();

        assert_eq!(rows.len(), release_count);
        assert!(!rows.is_empty());
    }
}
