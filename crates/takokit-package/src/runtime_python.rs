//! Managed Python runtime and model-adapter lifecycle.

use crate::{
    runtime_command::{run_logged_command, runner_python_path, PathOrArg},
    runtime_python_specs::{
        adapter_dependency_overrides, adapter_spec, sanitized_adapter_requirements,
        AdapterSourceSpec, AdapterSpec, ADAPTER_SPECS,
    },
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::{Path, PathBuf};

mod adapter_records;
mod overlay;
mod prefetch;
mod shared;

use adapter_records::{
    ensure_adapter_manifest, lock_adapter_install, read_adapter_record, write_adapter_record,
};
use overlay::prune_shared_overlay;
pub(crate) use prefetch::prefetch_python_adapter_model;
use shared::{
    ensure_shared_python_runtime, run_logged_uv_command, shared_runtime_identity,
    venv_inherits_shared_packages,
};

pub(crate) fn write_python_adapter_manifests(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<()> {
    for spec in ADAPTER_SPECS {
        let adapter_dir = layout.adapters.join(spec.id);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        ensure_adapter_manifest(&manifest, spec)?;
    }
    Ok(())
}

pub fn python_adapter_records(takokit_root: &Path) -> PackageResult<Vec<AdapterRecord>> {
    let layout = python_managed_runner_layout(takokit_root);
    let mut records = Vec::new();
    if !layout.adapters.is_dir() {
        return Ok(records);
    }
    let mut entries = std::fs::read_dir(&layout.adapters)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let adapter = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().join("adapter.toml");
        let previous = entry.path().join("adapter.toml.previous");
        if path.is_file() || previous.is_file() {
            if let Some(spec) = adapter_spec(&adapter) {
                records.push(read_adapter_record(&path, spec)?);
            } else {
                let source = std::fs::read_to_string(path)?;
                records.push(toml::from_str::<AdapterRecord>(&source)?);
            }
        }
    }
    Ok(records)
}

pub fn python_adapter_is_current(takokit_root: &Path, adapter: &str) -> bool {
    let Some(spec) = adapter_spec(adapter) else {
        return false;
    };
    let Some(expected_script) = spec.script else {
        return false;
    };
    let adapter_dir = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter);
    let deployed_script = adapter_dir.join(format!("{adapter}.py"));
    let shared_marker = adapter_dir.join(".takokit-shared-runtime");
    let venv = adapter_dir.join("venv");
    python_adapter_record(takokit_root, adapter)
        .is_ok_and(|record| record.state == AdapterLifecycleState::Ready)
        && std::fs::read_to_string(deployed_script).is_ok_and(|script| script == expected_script)
        && std::fs::read_to_string(shared_marker)
            .is_ok_and(|version| version.trim() == shared_runtime_identity(spec.python))
        && venv_inherits_shared_packages(&venv)
        && spec
            .source
            .as_ref()
            .is_none_or(|source| adapter_source_is_current(&adapter_dir.join("source"), source))
}

pub fn python_adapter_record(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let spec = adapter_spec(adapter).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: adapter.to_string(),
        reason: "unknown managed adapter".to_string(),
    })?;
    let path = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter)
        .join("adapter.toml");
    read_adapter_record(&path, spec)
}

pub fn install_python_adapter(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let spec = adapter_spec(adapter).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: adapter.to_string(),
        reason: "unknown managed adapter".to_string(),
    })?;
    let layout = python_managed_runner_layout(takokit_root);
    let adapter_dir = layout.adapters.join(adapter);
    let _install_lock = lock_adapter_install(&adapter_dir, adapter)?;
    if python_adapter_is_current(takokit_root, adapter) {
        return python_adapter_record(takokit_root, adapter);
    }
    let manifest_path = adapter_dir.join("adapter.toml");
    ensure_adapter_manifest(&manifest_path, spec)?;
    let mut record = read_adapter_record(&manifest_path, spec)?;
    let reset_environment = matches!(
        record.state,
        AdapterLifecycleState::Failed | AdapterLifecycleState::Installing
    );
    record.state = AdapterLifecycleState::Installing;
    record.notes = "Takokit is installing a lightweight adapter overlay on the shared Python base."
        .to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = install_adapter_spec(takokit_root, &layout, spec, reset_environment);
    match result {
        Ok(note) => {
            record.state = AdapterLifecycleState::Ready;
            record.notes = note;
            write_adapter_record(&manifest_path, &record)?;
            Ok(record)
        }
        Err(error) => {
            record.state = AdapterLifecycleState::Failed;
            record.notes = format!("Adapter install failed: {error}");
            write_adapter_record(&manifest_path, &record)?;
            Err(error)
        }
    }
}

pub(crate) fn install_python_managed_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = python_managed_runner_layout(takokit_root);
    for path in [
        &layout.root,
        &layout.runtime,
        &layout.env,
        &layout.packages,
        &layout.wheels,
        &layout.logs,
        &layout.manifests,
        &layout.cache,
        &layout.adapters,
    ] {
        std::fs::create_dir_all(path)?;
    }
    write_python_adapter_manifests(&layout)?;
    let uv = bootstrap_uv(takokit_root)?;
    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Managed Python runtime is ready at {} using {}. A shared dependency base is installed once for each Python ABI when its first adapter is pulled.",
            layout.root.display(),
            uv.display()
        ),
    )
}

fn install_adapter_spec(
    takokit_root: &Path,
    layout: &PythonManagedRunnerLayout,
    spec: &AdapterSpec,
    reset_environment: bool,
) -> PackageResult<String> {
    let script = spec
        .script
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: format!("{} has no adapter script", spec.model_family),
        })?;
    if spec.packages.is_empty() && spec.source.is_none() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: "adapter has no dependency or source installation plan".to_string(),
        });
    }

    let adapter_dir = layout.adapters.join(spec.id);
    std::fs::create_dir_all(&adapter_dir)?;
    let venv = adapter_dir.join("venv");
    let log = adapter_dir.join("install.log");
    let shared_python = ensure_shared_python_runtime(takokit_root, layout, spec.python)?;
    let must_migrate = !venv_inherits_shared_packages(&venv)
        || std::fs::read_to_string(adapter_dir.join(".takokit-shared-runtime"))
            .map(|value| value.trim() != shared_runtime_identity(spec.python))
            .unwrap_or(true);
    if (reset_environment || must_migrate) && venv.exists() {
        std::fs::remove_dir_all(&venv)?;
    }
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_uv_command(
        takokit_root,
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            shared_python.clone().into(),
            "--system-site-packages".into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: spec.id.to_string(),
        reason: format!(
            "adapter environment has no Python executable: {}",
            venv.display()
        ),
    })?;

    let source_dir = match spec.source.as_ref() {
        Some(source) => Some(install_adapter_source(&adapter_dir, &log, source)?),
        None => None,
    };
    let dependency_overrides = adapter_dependency_overrides(spec.id);
    let dependency_override_file = if dependency_overrides.is_empty() {
        None
    } else {
        let path = adapter_dir.join("dependency-overrides.txt");
        std::fs::write(&path, format!("{}\n", dependency_overrides.join("\n")))?;
        Some(path)
    };
    if !spec.packages.is_empty() {
        uv_pip_install(
            takokit_root,
            &uv,
            &python,
            &log,
            spec.packages.iter().map(|item| (*item).into()),
        )?;
    }
    if !spec.no_deps_packages.is_empty() {
        uv_pip_install(
            takokit_root,
            &uv,
            &python,
            &log,
            std::iter::once("--no-deps".into())
                .chain(spec.no_deps_packages.iter().map(|item| (*item).into())),
        )?;
    }
    if let (Some(source), Some(source_dir)) = (spec.source.as_ref(), source_dir.as_ref()) {
        for requirements in source.requirement_files {
            let path = source_dir.join(requirements);
            if !path.is_file() {
                return Err(PackageError::ArtifactInstallFailed {
                    artifact: spec.id.to_string(),
                    reason: format!("required dependency file is missing: {}", path.display()),
                });
            }
            let install_path =
                prepare_adapter_requirements(spec.id, std::env::consts::OS, &path, &adapter_dir)?;
            let mut dependencies: Vec<PathOrArg> = Vec::new();
            if let Some(overrides) = dependency_override_file.as_ref() {
                dependencies.extend(["--override".into(), overrides.clone().into()]);
            }
            dependencies.extend(["-r".into(), install_path.into()]);
            uv_pip_install(takokit_root, &uv, &python, &log, dependencies)?;
        }
        if source.editable {
            let mut dependencies: Vec<PathOrArg> = Vec::new();
            if let Some(overrides) = dependency_override_file.as_ref() {
                dependencies.extend(["--override".into(), overrides.clone().into()]);
            }
            dependencies.extend(["-e".into(), source_dir.clone().into()]);
            uv_pip_install(takokit_root, &uv, &python, &log, dependencies)?;
        }
    }

    let inherited_package_count = prune_shared_overlay(
        takokit_root,
        &uv,
        &python,
        &shared_python,
        &adapter_dir,
        &log,
    )?;

    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;
    std::fs::write(
        adapter_dir.join(".takokit-shared-runtime"),
        shared_runtime_identity(spec.python),
    )?;
    Ok(format!(
        "Ready. {} Shared Python {} with thin adapter overlay: {}. {} exact package copies were replaced by inheritance from the ABI base. Source: {}. Install log: {}",
        spec.note,
        spec.python,
        venv.display(),
        inherited_package_count,
        source_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "package-managed".to_string()),
        log.display()
    ))
}

fn install_adapter_source(
    adapter_dir: &Path,
    log: &Path,
    source: &AdapterSourceSpec,
) -> PackageResult<PathBuf> {
    let destination = adapter_dir.join("source");
    if adapter_source_is_current(&destination, source) {
        return Ok(destination);
    }
    if destination.exists() {
        std::fs::remove_dir_all(&destination)?;
    }
    let temporary = adapter_dir.join("source.download");
    if temporary.exists() {
        std::fs::remove_dir_all(&temporary)?;
    }
    let clone_args = if source.recursive {
        vec![
            "clone".into(),
            "--recursive".into(),
            "--no-checkout".into(),
            source.repository.into(),
            temporary.clone().into(),
        ]
    } else {
        vec![
            "clone".into(),
            "--no-checkout".into(),
            source.repository.into(),
            temporary.clone().into(),
        ]
    };
    run_logged_command(log, "git", &clone_args)?;
    run_logged_command(
        log,
        "git",
        &[
            "-C".into(),
            temporary.clone().into(),
            "checkout".into(),
            "--detach".into(),
            source.revision.into(),
        ],
    )?;
    if source.recursive {
        run_logged_command(
            log,
            "git",
            &[
                "-C".into(),
                temporary.clone().into(),
                "submodule".into(),
                "update".into(),
                "--init".into(),
                "--recursive".into(),
            ],
        )?;
    }
    std::fs::write(temporary.join(".takokit-revision"), source.revision)?;
    std::fs::rename(&temporary, &destination)?;
    Ok(destination)
}

fn adapter_source_is_current(destination: &Path, source: &AdapterSourceSpec) -> bool {
    destination.is_dir()
        && std::fs::read_to_string(destination.join(".takokit-revision"))
            .ok()
            .is_some_and(|revision| revision.trim() == source.revision)
        && source
            .required_files
            .iter()
            .all(|relative| destination.join(relative).is_file())
}

fn prepare_adapter_requirements(
    adapter: &str,
    target_os: &str,
    source: &Path,
    adapter_dir: &Path,
) -> PackageResult<PathBuf> {
    let requirements = std::fs::read_to_string(source)?;
    let Some(sanitized) = sanitized_adapter_requirements(adapter, target_os, &requirements) else {
        return Ok(source.to_path_buf());
    };
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("requirements.txt");
    let prepared = adapter_dir.join(format!(".takokit-{file_name}"));
    std::fs::write(&prepared, sanitized)?;
    Ok(prepared)
}

fn uv_pip_install(
    takokit_root: &Path,
    uv: &Path,
    python: &Path,
    log: &Path,
    dependencies: impl IntoIterator<Item = PathOrArg>,
) -> PackageResult<()> {
    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        python.to_path_buf().into(),
        "--no-progress".into(),
        "--torch-backend=auto".into(),
    ];
    arguments.extend(dependencies);
    run_logged_uv_command(takokit_root, log, uv, &arguments)
}

#[cfg(test)]
mod source_readiness_tests {
    use super::*;

    #[test]
    fn adapter_source_requires_revision_and_declared_runtime_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = AdapterSourceSpec {
            repository: "https://example.invalid/source.git",
            revision: "pinned-revision",
            recursive: false,
            requirement_files: &[],
            required_files: &["configs/v1/40k.json", "train/train.py"],
            editable: false,
        };
        std::fs::write(temp.path().join(".takokit-revision"), source.revision)
            .expect("revision marker");
        std::fs::create_dir_all(temp.path().join("configs/v1")).expect("config directory");
        std::fs::create_dir_all(temp.path().join("train")).expect("train directory");
        std::fs::write(temp.path().join("configs/v1/40k.json"), "{}").expect("config");

        assert!(!adapter_source_is_current(temp.path(), &source));
        std::fs::write(temp.path().join("train/train.py"), "# train").expect("trainer");
        assert!(adapter_source_is_current(temp.path(), &source));
    }
}
