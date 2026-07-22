//! Managed Python runtime and model-adapter lifecycle.

use crate::{
    runtime_command::{run_logged_command, runner_python_path, PathOrArg},
    runtime_python_specs::{adapter_spec, AdapterSourceSpec, AdapterSpec, ADAPTER_SPECS},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::{Path, PathBuf};

mod prefetch;
pub(crate) use prefetch::prefetch_python_adapter_model;

pub(crate) fn write_python_adapter_manifests(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<()> {
    for spec in ADAPTER_SPECS {
        let adapter_dir = layout.adapters.join(spec.id);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        if !manifest.is_file() {
            write_adapter_record(
                &manifest,
                &AdapterRecord {
                    id: spec.id.to_string(),
                    model_family: spec.model_family.to_string(),
                    state: AdapterLifecycleState::NotInstalled,
                    dependency_strategy: "isolated-takokit-managed-python".to_string(),
                    input_contract: "typed JSON request on stdin".to_string(),
                    output_contract: "typed JSON response on stdout".to_string(),
                    logs: "install.log".to_string(),
                    notes: spec.note.to_string(),
                },
            )?;
        }
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
        let path = entry.path().join("adapter.toml");
        if path.is_file() {
            let source = std::fs::read_to_string(path)?;
            records.push(toml::from_str::<AdapterRecord>(&source)?);
        }
    }
    Ok(records)
}

pub fn python_adapter_record(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let path = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter)
        .join("adapter.toml");
    let source = std::fs::read_to_string(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: format!(
                "adapter is not available; run `takokit runner install takokit-python-managed`: {}",
                path.display()
            ),
        },
        _ => PackageError::Io(error),
    })?;
    Ok(toml::from_str::<AdapterRecord>(&source)?)
}

pub fn install_python_adapter(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let layout = python_managed_runner_layout(takokit_root);
    write_python_adapter_manifests(&layout)?;
    let manifest_path = layout.adapters.join(adapter).join("adapter.toml");
    let mut record = python_adapter_record(takokit_root, adapter)?;
    record.state = AdapterLifecycleState::Installing;
    record.notes = "Takokit is installing this adapter in an isolated environment.".to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = adapter_spec(adapter)
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: "unknown managed adapter".to_string(),
        })
        .and_then(|spec| install_adapter_spec(takokit_root, &layout, spec));
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
    let venv = layout.env.join("venv");
    let log = layout.logs.join("runtime-install.log");
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            "3.11".into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "managed Python runtime".to_string(),
        reason: format!(
            "uv created no Python executable below {}; see {}",
            venv.display(),
            log.display()
        ),
    })?;
    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Managed Python runtime is ready at {} using {}. Install per-model adapters with `takokit adapter install <id>`. Log: {}",
            layout.root.display(),
            python.display(),
            log.display()
        ),
    )
}

fn install_adapter_spec(
    takokit_root: &Path,
    layout: &PythonManagedRunnerLayout,
    spec: &AdapterSpec,
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
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            spec.python.into(),
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
    if !spec.packages.is_empty() {
        uv_pip_install(
            &uv,
            &python,
            &log,
            spec.packages.iter().map(|item| (*item).into()),
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
            uv_pip_install(&uv, &python, &log, ["-r".into(), path.into()].into_iter())?;
        }
        if source.editable {
            uv_pip_install(
                &uv,
                &python,
                &log,
                ["-e".into(), source_dir.clone().into()].into_iter(),
            )?;
        }
    }

    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;
    Ok(format!(
        "Ready. {} Environment: {}. Source: {}. Install log: {}",
        spec.note,
        venv.display(),
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
    let marker = destination.join(".takokit-revision");
    if destination.is_dir()
        && std::fs::read_to_string(&marker)
            .ok()
            .is_some_and(|revision| revision.trim() == source.revision)
    {
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

fn uv_pip_install(
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
    run_logged_command(log, uv, &arguments)
}

pub(crate) fn write_adapter_record(path: &Path, record: &AdapterRecord) -> PackageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: record.id.clone(),
            reason: "adapter manifest path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, toml::to_string_pretty(record)?)?;
    Ok(())
}
